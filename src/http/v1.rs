// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use tokio::{
    net::{TcpListener, TcpStream},
    task, io::{split, AsyncWriteExt, AsyncReadExt, BufReader, AsyncBufReadExt, BufWriter, AsyncSeekExt}, time::Instant,
};

use tokio_rustls::TlsAcceptor;

use std::{
    io::{self, SeekFrom},
    time::Duration, mem::swap
};

use crate::{
    http::{
        Error,
        error::HttpParseError,
        finish_response_error,
        finish_response_normal,
        handle_parse_error,
        handle_request,
        message::{
            BodyKind,
            ContentRangeHeaderValue,
            HttpVersion,
            HeaderName,
            HeaderMap,
            HeaderValue,
            HttpRangeList,
            Method,
            Range,
            Response,
            Request,
            RequestTarget,
            StatusCode,
            StatusCodeClass,
        },
    },
    ServenteConfig,
};

/// The threshold at which the response body is transferred using chunked
/// encoding.
const TRANSFER_ENCODING_THRESHOLD: u64 = 1000_000_000_000_000_000; // 1 MiB

/// Indicates the maximum length of a certain HTTP entity.
struct MaximumLength(pub usize);

impl MaximumLength {
    /// The maximum length of a method name.
    pub const METHOD: MaximumLength = MaximumLength(16);

    /// The maximum length of a request target, including the query string.
    pub const REQUEST_TARGET: MaximumLength = MaximumLength(1024);

    /// The maximum length of a full HTTP header (name + value), excluding the CRLF.
    pub const HEADER: MaximumLength = MaximumLength(4096);
}

/// The strategy to use for transferring the response body.
#[derive(Debug, Clone, PartialEq)]
pub enum TransferStrategy {
    Chunked,
    Full,
    Ranges { ranges: HttpRangeList },
}

/// Consume a `U+000D CARRIAGE RETURN` character (CR) and a `U+000A LINE FEED`
/// character (LF) from the stream.
async fn consume_crlf<R>(stream: &mut R) -> Result<(), Error>
        where R: AsyncBufReadExt + Unpin {
    let mut buffer = [0u8; 2];
    stream.read_exact(&mut buffer).await?;

    if buffer[0] != b'\r' || buffer[1] != b'\n' {
        return Err(Error::ParseError(HttpParseError::InvalidCRLF));
    }

    Ok(())
}

/// Plans out the best `TransferStrategy` for the given response.
#[must_use]
async fn determine_transfer_strategy(response: &mut Response, ranges: Option<HttpRangeList>) -> TransferStrategy {
    let Some(body) = &response.body else {
        if response.status.class() != StatusCodeClass::Informational {
            response.headers.set_content_length(0);
        }
        return TransferStrategy::Full;
    };

    match body {
        BodyKind::File(file) => {
            let file_size = file.metadata().await.unwrap().len();
            if let Some(ranges) = ranges {
                response.status = StatusCode::PartialContent;
                if let Some(range) = ranges.first_and_only() {
                    match range {
                        Range::Full => {
                            response.headers.set_content_range(ContentRangeHeaderValue::Range {
                                start: 0,
                                end: (file_size - 1) as _,
                                complete_length: Some(file_size as _),
                            });
                        }
                        Range::Points { start, end } => {
                            response.headers.set_content_range(ContentRangeHeaderValue::Range {
                                start: start as _,
                                end: end as _,
                                complete_length: Some(file_size as _),
                            });
                        }
                        Range::StartPointToEnd { start } => {
                            response.headers.set_content_range(ContentRangeHeaderValue::Range {
                                start: start as _,
                                end: (file_size - 1) as _,
                                complete_length: Some(file_size as _),
                            });
                        }
                        Range::Suffix { suffix } => {
                            response.headers.set_content_range(ContentRangeHeaderValue::Range {
                                start: (file_size - suffix) as _,
                                end: (file_size - 1) as _,
                                complete_length: Some(file_size as _),
                            });
                        }
                    }
                } else {
                    todo!("Support multiple ranges");
                }

                return TransferStrategy::Ranges { ranges };
            }

            if file_size > TRANSFER_ENCODING_THRESHOLD {
                response.headers.set(HeaderName::TransferEncoding, "chunked".into());
                return TransferStrategy::Chunked;
            }

            response.headers.set_content_length(file_size as _);
            TransferStrategy::Full
        }

        BodyKind::CachedBytes(bytes) => {
            response.headers.set_content_length(bytes.len());
            TransferStrategy::Full
        }

        BodyKind::Bytes(bytes) => {
            response.headers.set_content_length(bytes.len());
            TransferStrategy::Full
        }

        BodyKind::StaticString(string) => {
            response.headers.set_content_length(string.len());
            TransferStrategy::Full
        }

        BodyKind::String(string) => {
            response.headers.set_content_length(string.len());
            TransferStrategy::Full
        }
    }
}

/// Discards the full request.
async fn discard_request(stream: &mut TcpStream) -> Result<(), Error> {
    let mut buffer = BufReader::new(stream);
    loop {
        let line = read_crlf_line(&mut buffer, MaximumLength::HEADER).await?;
        if line.is_empty() {
            return Ok(());
        }
    }
}

/// Reads a single response, handles it and sends the response back to the
/// client.
async fn handle_exchange<R, W>(reader: &mut R, writer: &mut W, config: &ServenteConfig) -> Result<(), io::Error>
        where R: AsyncBufReadExt + Unpin, W: AsyncWriteExt + Unpin {
    #[cfg(feature = "debugging")]
    let start_full = Instant::now();

    let mut request = match read_request_excluding_body(reader).await {
        Ok(request) => request,
        Err(error) => {
            match error {
                Error::ParseError(error) => {
                    let mut response = handle_parse_error(error).await;
                    finish_response_error(&mut response).await?;
                    send_response(writer, response, None).await?;
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "Parse error"));
                }
                Error::Other(error) => {
                    return Err(error);
                }
            }
        }
    };

    // TODO some handlers might prefer to read the body themselves.
    if let Err(error) = read_request_body(reader, &mut request).await {
        match error {
            Error::ParseError(error) => {
                let mut response = handle_parse_error(error).await;
                finish_response_error(&mut response).await?;
                send_response(writer, response, None).await?;
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Parse error"));
            }
            Error::Other(error) => {
                return Err(error);
            }
        }
    }

    #[cfg(feature = "debugging")]
    let start_handling = Instant::now();
    let mut response = match handle_request(&request, config).await {
        Ok(response) => response,
        Err(error) => {
            println!("{:?}>: {:?} => {:?}", request.method, request.target, error);
            let mut response = Response::with_status_and_string_body(StatusCode::InternalServerError, String::from("Internal Server Error"));
            finish_response_error(&mut response).await?;
            response
        }
    };
    finish_response_normal(&request, &mut response).await?;

    for response in response.prelude_response {
        send_response(writer, response, None).await?;
    }
    response.prelude_response = Vec::new();

    let sent_body = send_response(writer, response,
        request.headers.get(&HeaderName::Range)
                .and_then(|range| range.as_str_no_convert())
                .and_then(HttpRangeList::parse)
    ).await?;

    #[cfg(feature = "debugging")]
    println!("{:?}>: {:?} (f={}ms, h={}ms, b={}ms)", request.method, request.target, start_full.elapsed().as_millis(), start_handling.elapsed().as_millis(), sent_body.as_millis());

    #[cfg(not(feature = "debugging"))]
    { _ = sent_body }

    Ok(())
}

/// Process a single socket connection.
async fn process_socket(mut stream: TcpStream, config: ServenteConfig) {
    //println!("Client connected: {}", stream.peer_addr().unwrap());
    let mut buf = [0u8; 4];
    if let Ok(length) = stream.peek(&mut buf).await {
        if length >= 3 && &buf[0..3] == b"GET" {
            if let Err(e) = discard_request(&mut stream).await {
                #[cfg(feature = "debugging")]
                println!("Client Error discarding non-HTTPS: {:?}", e);

                #[cfg(not(feature = "debugging"))]
                { _ = e }
                return;
            }

            if send_http_upgrade(&mut stream).await.is_err() {
                _ = stream.shutdown().await;
                return;
            }
        }
    }

    let acceptor = TlsAcceptor::from(config.tls_config.clone());
    let stream = match acceptor.accept(stream).await {
        Ok(stream) => stream,
        Err(_) => return,
    };

    let (reader, writer) = split(stream);
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    loop {
        if let Err(e) = handle_exchange(&mut reader, &mut writer, &config).await {
            #[cfg(feature = "debugging")]
            println!("Client Error: {:?}", e);

            #[cfg(not(feature = "debugging"))]
            { _ = e }
            return;
        }
    }
}

/// Reads a line from the stream, up to the maximum length.
#[must_use]
async fn read_crlf_line<R>(stream: &mut R, maximum_length: MaximumLength) -> Result<String, Error>
        where R: AsyncBufReadExt + Unpin {
    let mut string = String::new();

    while string.len() < maximum_length.0 {
        let byte = stream.read_u8().await?;
        if byte == b'\r' {
            let byte = stream.read_u8().await?;
            if byte == b'\n' {
                return Ok(string);
            }
            return Err(Error::ParseError(HttpParseError::InvalidCRLF));
        }

        string.push(byte as char);
    }

    Err(Error::ParseError(HttpParseError::HeaderTooLarge))
}

/// Reads the headers from the stream.
#[must_use]
async fn read_headers<R>(stream: &mut R) -> Result<HeaderMap, Error>
        where R: AsyncBufReadExt + Unpin {
    let mut headers = Vec::new();

    loop {
        let line = read_crlf_line(stream, MaximumLength::HEADER).await?;
        if line.is_empty() {
            return Ok(HeaderMap::new_with_vec(headers));
        }

        let Some((name, value)) = line.split_once(':') else {
            return Err(Error::ParseError(HttpParseError::HeaderDoesNotContainColon));
        };
        let name = name.trim().to_string();
        let value = value.trim().to_string();

        let name = HeaderName::from(name);
        if let HeaderName::Other(name) = &name {
            #[cfg(debug_assertions)]
            println!("[DEBUG] Unknown header name: \"{}\" with value: \"{}\"", name, value);
        }
        headers.push((name, HeaderValue::from(value)));
    }
}

/// Reads the HTTP version from the stream.
#[must_use]
async fn read_http_version<R>(stream: &mut R) -> Result<HttpVersion, Error>
        where R: AsyncBufReadExt + Unpin {
    let mut version_buffer = [0u8; 8];
    stream.read_exact(&mut version_buffer).await?;

    Ok(match &version_buffer {
        b"HTTP/1.0" => HttpVersion::Http10,
        b"HTTP/1.1" => HttpVersion::Http11,
        b"HTTP/2.0" => HttpVersion::Http2,
        _ => return Err(Error::ParseError(HttpParseError::InvalidHttpVersion)),
    })
}

/// Reads a string from the stream until the given character is found, or the
/// maximum length is reached.
#[must_use]
async fn read_string_until_character<R>(stream: &mut R, char: u8, maximum_length: MaximumLength, length_error: HttpParseError) -> Result<String, Error>
        where R: AsyncBufReadExt + Unpin {
    let mut buffer = String::new();

    while buffer.len() < maximum_length.0 {
        let byte = stream.read_u8().await?;
        if byte == char {
            return Ok(buffer);
        }

        buffer.push(byte as char);
    }

    Err(Error::ParseError(length_error))
}

/// Reads the request body from the stream and stores it in the request.
async fn read_request_body<R>(stream: &mut R, request: &mut Request) -> Result<(), Error>
        where R: AsyncBufReadExt + Unpin {
    if let Some(content_length) = request.headers.get(&HeaderName::ContentLength) {
        request.body = Some(read_request_body_content_length(stream, request, content_length).await?);
        return Ok(());
    }

    if request.headers.get(&HeaderName::TransferEncoding).is_some() {
        request.body = Some(read_request_body_chunked(stream).await?);
        return Ok(());
    }

    Ok(())
}

/// Reads the request-body
#[must_use]
async fn read_request_body_content_length<R>(stream: &mut R, request: &Request, content_length: &HeaderValue) -> Result<BodyKind, Error>
        where R: AsyncBufReadExt + Unpin {
    let content_length = content_length.parse_number().ok_or(Error::ParseError(HttpParseError::InvalidContentLength))?;
    let mut body = Vec::with_capacity(content_length);

    stream.read_exact(body.as_mut_slice()).await?;

    if let Some(media_type) = request.headers.get(&HeaderName::ContentType) {
        // TODO: correctly parse the Media Type
        if media_type.as_str_no_convert().unwrap().starts_with("text/") {
            match String::from_utf8(body) {
                Ok(body) => return Ok(BodyKind::String(body)),
                // String conversion was impossible (possibly not UTF-8), so just return the bytes.
                Err(error) => return Ok(BodyKind::Bytes(error.into_bytes())),
            }
        }
    }
    Ok(BodyKind::Bytes(body))
}

/// Reads the body of a request, assuming that the body is encoded using chunked
/// transfer encoding.
#[must_use]
async fn read_request_body_chunked<R>(_stream: &mut R) -> Result<BodyKind, Error>
        where R: AsyncBufReadExt + Unpin {
    // TODO: support chunked encoding
    Err(Error::Other(io::Error::new(io::ErrorKind::InvalidData, "TODO: support chunked encoding")))
}

/// Read the request-line and headers from the stream, without reading the body.
///
/// We do not read the body here, because the handler might prefer to use their
/// own method of reading the body. This is especially useful for streaming
/// data, directly reading and writing without buffering, etc.
async fn read_request_excluding_body<R>(stream: &mut R) -> Result<Request, Error>
        where R: AsyncBufReadExt + Unpin {
    let (method, target, version) = read_request_line(stream).await?;
    let headers = read_headers(stream).await?;
    Ok(Request { method, target, version, headers, body: None })
}

/// Read the request-line from the stream.
#[must_use]
async fn read_request_line<R>(stream: &mut R) -> Result<(Method, RequestTarget, HttpVersion), Error>
        where R: AsyncBufReadExt + Unpin {

    let method = Method::from(read_string_until_character(stream, b' ', MaximumLength::METHOD, HttpParseError::MethodTooLarge).await?);

    // TODO skip OWS
    let target = read_request_target(stream).await?;

    // TODO skip OWS

    let version = read_http_version(stream).await?;
    consume_crlf(stream).await?;

    Ok((method, target, version))
}

/// Reads the request-target from the stream.
///
/// ### References
/// * [RFC 9112, Section 3.2. Request Target](https://www.rfc-editor.org/rfc/rfc9112.html#name-request-target)
#[must_use]
async fn read_request_target<R>(stream: &mut R) -> Result<RequestTarget, Error>
        where R: AsyncBufReadExt + Unpin {
    let str = read_string_until_character(stream, b' ', MaximumLength::REQUEST_TARGET, HttpParseError::RequestTargetTooLarge).await?;

    if str == "*" {
        return Ok(RequestTarget::Asterisk);
    }

    if str.starts_with('/') {
        let mut parts = str.splitn(2, '?');
        return Ok(RequestTarget::Origin {
            path: parts.next().unwrap_or("").to_string(),
            query: parts.next().unwrap_or("").to_string(),
        });
    }

    // TODO: correctly parse the URI.
    if str.starts_with("http://") || str.starts_with("https://") {
        return Ok(RequestTarget::Absolute(str));
    }

    Err(Error::ParseError(HttpParseError::InvalidRequestTarget))
}

/// Send the HTTPS upgrade to the client.
///
/// ### TODO
/// In the future, we should prefer using a 3xx response, redirecting the client
/// to the https-scheme.
async fn send_http_upgrade(stream: &mut TcpStream) -> Result<(), io::Error> {
    let body = "HTTPS is required.";
    let message = format!(
        concat!("HTTP/1.1 426 Upgrade Required\r\n",
                "Upgrade: TLS/1.2, HTTP/1.1\r\n",
                "Connection: Upgrade\r\n",
                "Content-Length: {}\r\n",
                "Content-Type: text/plain;charset=utf-8\r\n",
                "\r\n",
                "{}"
        ),
        body.len(), body
    );
    stream.write_all(message.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

/// Send the response to the client.
#[must_use]
async fn send_response<R>(stream: &mut R, mut response: Response, ranges: Option<HttpRangeList>) -> Result<Duration, io::Error>
        where R: AsyncWriteExt + Unpin {
    let transfer_strategy = determine_transfer_strategy(&mut response, ranges).await;

    let mut response_text = String::with_capacity(1024);
    response_text.push_str("HTTP/1.1 ");
    response_text.push_str(&response.status.to_string());
    response_text.push_str("\r\n");

    for (name, value) in response.headers.iter() {
        response_text.push_str(name.to_string_h1());
        response_text.push_str(": ");
        value.append_to_message(&mut response_text);
        response_text.push_str("\r\n");
    }

    response_text.push_str("\r\n");

    stream.write_all(response_text.as_bytes()).await?;


    let start = Instant::now();
    if let Some(response) = response.body {
        match response {
            BodyKind::File(mut response) => {
                match transfer_strategy {
                    TransferStrategy::Full => transfer_body_full(stream, &mut response).await?,
                    TransferStrategy::Chunked => transfer_body_chunked(stream, &mut response).await?,
                    TransferStrategy::Ranges { ranges } => {
                        transfer_body_ranges(stream, &mut response, ranges).await?
                    }
                }
            }
            BodyKind::Bytes(response) => stream.write_all(&response).await?,
            BodyKind::CachedBytes(response) => stream.write_all(&response).await?,
            BodyKind::StaticString(response) => stream.write_all(response.as_bytes()).await?,
            BodyKind::String(response) => stream.write_all(response.as_bytes()).await?,
        }
    }
    _ = stream.flush().await;
    Ok(start.elapsed())
}

/// Start the HTTPv1 server on the given address.
#[must_use]
pub async fn start(address: &str, config: ServenteConfig) -> io::Result<()> {
    let listener = TcpListener::bind(address).await?;
    println!("Started listening on {}", address);

    loop {
        let (stream, _) = listener.accept().await?;
        let config = config.clone();
        task::spawn(async move {
            process_socket(stream, config.clone()).await;
        });
    }
}

/// Transfer the body, using the `Transfer-Encoding: chunked` algorithm.
async fn transfer_body_chunked<O, I>(output: &mut O, input: &mut I) -> Result<(), io::Error>
        where O: AsyncWriteExt + Unpin,
              I: AsyncReadExt + Unpin {
    let mut buf: [u8; 16384] = [0; 16384];
    loop {
        let len = input.read(&mut buf).await?;

        if len == 0 {
            break;
        }

        output.write_all(format!("{:X}\r\n", len).as_bytes()).await?;

        output.write_all(&buf[0..len]).await?;

        output.write_all(b"\r\n").await?;
    }

    output.write_all(b"0\r\n\r\n").await?;

    Ok(())
}

/// Transfer the body, using the full contents of the input, without and
/// `Transfer-Encoding` or `range`s.
async fn transfer_body_full<O, I>(output: &mut O, input: &mut I) -> Result<(), io::Error>
        where O: AsyncWriteExt + Unpin,
              I: AsyncReadExt + Unpin {
    let mut buf1 = [0; 8192];
    let mut buf2 = [0; 8192];

    let mut front_buf = &mut buf1;
    let mut back_buf = &mut buf2;

    let mut len = input.read(front_buf).await?;

    loop {
        if len == 0 {
            break;
        }

        let write_fut = output.write_all(&front_buf[0..len]);
        len = input.read(back_buf).await?;
        write_fut.await?;

        swap(&mut front_buf, &mut back_buf);
    }

    Ok(())
}

/// Transfer the body, using the ranges specified in the request.
async fn transfer_body_ranges<O, I>(output: &mut O, input: &mut I, ranges: HttpRangeList) -> Result<(), io::Error>
        where O: AsyncWriteExt + Unpin,
              I: AsyncReadExt + AsyncSeekExt + Unpin {
    for range in ranges.iter() {
        match range {
            Range::Full => {
                return transfer_body_full(output, input).await;
            }
            Range::StartPointToEnd { start } => {
                let mut buf: [u8; 8192] = [0; 8192];
                input.seek(SeekFrom::Start(*start as _)).await?;
                loop {
                    let len = input.read(&mut buf).await?;

                    if len == 0 {
                        break;
                    }

                    output.write_all(&buf[0..len]).await?;
                }
            }
            Range::Points { start, end } => {
                let mut buf: [u8; 8192] = [0; 8192];
                input.seek(SeekFrom::Start(*start as _)).await?;
                let mut remaining = (end - start) as usize;
                while remaining > 0 {
                    let len = input.read(&mut buf).await?;

                    if len == 0 {
                        break;
                    }

                    let len = std::cmp::min(len, remaining);
                    output.write_all(&buf[0..len]).await?;
                    remaining -= len;
                }
            }
            Range::Suffix { suffix } => {
                let mut buf: [u8; 8192] = [0; 8192];
                let len = input.seek(SeekFrom::End(0)).await?;
                let start = (len - *suffix) as usize;
                input.seek(SeekFrom::Start(start as _)).await?;
                let mut remaining = *suffix as _;
                while remaining > 0 {
                    let len = input.read(&mut buf).await?;

                    if len == 0 {
                        break;
                    }

                    let len = std::cmp::min(len, remaining);
                    output.write_all(&buf[0..len]).await?;
                    remaining -= len;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::http::{message::{Method, RequestTarget, HttpVersion}, error::HttpParseError, Error};

    #[tokio::test]
    async fn read_request_line_normal() {
        let mut stream = std::io::Cursor::new(b"GET / HTTP/1.1\r\n");
        let request_line = super::read_request_line(&mut stream).await.unwrap();
        assert_eq!(request_line.0, Method::Get);
        assert_eq!(request_line.1 , RequestTarget::Origin { path: "/".to_string(), query: String::new() });
        assert_eq!(request_line.2, HttpVersion::Http11);
    }

    #[rstest]
    #[case(b"DELETE / HTTP/1.1\r\n", Method::Delete)]
    #[case(b"GET / HTTP/1.1\r\n", Method::Get)]
    #[case(b"get / HTTP/1.1\r\n", Method::Get)]
    #[case(b"POST / HTTP/1.1\r\n", Method::Post)]
    #[case(b"PUT / HTTP/1.1\r\n", Method::Put)]
    #[case(b"OPTIONS * HTTP/1.1\r\n", Method::Options)]
    #[case(b"NEW-METHOD / HTTP/1.1\r\n", Method::Other(String::from("NEW-METHOD")))]
    #[tokio::test]
    async fn read_request_line_methods(#[case] input: &[u8], #[case] expected: Method) {
        let mut stream = std::io::Cursor::new(input);
        let request_line = super::read_request_line(&mut stream).await.unwrap();
        assert_eq!(request_line.0, expected);
        assert_eq!(request_line.2, HttpVersion::Http11);
    }

    #[tokio::test]
    async fn read_request_line_long_method() {
        let mut stream = std::io::Cursor::new(b"THIS-IS-A-VERY-LONG-METHOD / HTTP/1.1\r\n");
        let request_line = super::read_request_line(&mut stream).await;
        assert!(request_line.is_err());
        let error = request_line.err().unwrap();
        match &error {
            Error::ParseError(HttpParseError::MethodTooLarge) => {},
            _ => panic!("Unexpected error: {:?}", error),
        }
    }
}
