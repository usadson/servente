// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

mod read;

pub(crate) use read::*;

use tokio::{
    net::{TcpListener, TcpStream},
    task, io::{split, AsyncWriteExt, AsyncReadExt, BufReader, AsyncBufReadExt, BufWriter, AsyncSeekExt}, time::{Instant, timeout},
};

#[cfg(feature = "ktls")]
use tokio::io::{AsyncRead, AsyncWrite};

#[cfg(feature = "rustls")]
use tokio_rustls::TlsAcceptor;

#[cfg(feature = "rustls")]
use std::sync::Arc;

use std::{
    io::{self, SeekFrom},
    mem::swap,
    time::Duration,
};

#[cfg(feature = "ktls")]
use std::{ops::DerefMut, pin};

use servente_http::{
    Error,
    HttpParseError,
};

use servente_http_handling::{
    finish_response_error,
    finish_response_normal,
    handle_parse_error,
    handle_request, ServenteConfig, responses, ServenteSettings,
};

use servente_http::{
    BodyKind,
    ContentRangeHeaderValue,
    HttpVersion,
    HeaderName,
    HttpRangeList,
    Method,
    Range,
    Response,
    Request,
    StatusCode,
    StatusCodeClass,
};

use servente_resources::ContentCoding;

/// The threshold at which the response body is transferred using chunked
/// encoding.
const TRANSFER_ENCODING_THRESHOLD: u64 = 1_000_000_000_000_000_000; // 1 MiB

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

#[derive(Debug)]
pub enum ExchangeError {
    MalformedData,
    Http2Upgrade,
    TimedOut,
    Io(io::Error),
}

impl From<io::Error> for ExchangeError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}


#[cfg(feature = "ktls")]
enum StreamWrapper {
    Normal(TlsStream<TcpStream>),
    KtlsStream(ktls::KtlsStream<TcpStream>),
}


#[cfg(feature = "ktls")]
impl Unpin for StreamWrapper {}


#[cfg(feature = "ktls")]
impl AsyncRead for StreamWrapper {
    fn poll_read(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<io::Result<()>> {
        match self.get_mut() {
            StreamWrapper::Normal(stream) => std::pin::pin!(stream).poll_read(cx, buf),
            StreamWrapper::KtlsStream(stream) => std::pin::pin!(stream).poll_read(cx, buf),
        }
    }
}

#[cfg(feature = "ktls")]
impl AsyncWrite for StreamWrapper {
    fn poll_write(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<io::Result<usize>> {
        match self.get_mut() {
            StreamWrapper::Normal(stream) => std::pin::pin!(stream).poll_write(cx, buf),
            StreamWrapper::KtlsStream(stream) => std::pin::pin!(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<io::Result<()>> {
        match self.get_mut() {
            StreamWrapper::Normal(stream) => std::pin::pin!(stream).poll_flush(cx),
            StreamWrapper::KtlsStream(stream) => std::pin::pin!(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<io::Result<()>> {
        match self.get_mut() {
            StreamWrapper::Normal(stream) => std::pin::pin!(stream).poll_shutdown(cx),
            StreamWrapper::KtlsStream(stream) => std::pin::pin!(stream).poll_shutdown(cx),
        }
    }
}

/// Plans out the best `TransferStrategy` for the given response.
async fn determine_transfer_strategy(response: &mut Response, ranges: Option<HttpRangeList>) -> TransferStrategy {
    let Some(body) = &response.body else {
        if response.status.class() != StatusCodeClass::Informational {
            response.headers.set_content_length(0);
        }
        return TransferStrategy::Full;
    };

    match body {
        BodyKind::File { metadata, .. } => {
            let file_size = metadata.len();
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
                response.headers.append_or_override(HeaderName::TransferEncoding, "chunked".into());
                return TransferStrategy::Chunked;
            }

            response.headers.set_content_length(file_size as _);
            TransferStrategy::Full
        }

        BodyKind::CachedBytes(bytes, coding) => {
            response.headers.set_content_length(bytes.get_version(*coding).len());
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
pub async fn handle_exchange<R, W>(reader: &mut R, writer: &mut W, settings: &ServenteSettings) -> Result<(), ExchangeError>
        where R: AsyncBufReadExt + Unpin,
              W: AsyncWriteExt + Unpin {
    #[cfg(feature = "debugging")]
    let start_full = Instant::now();

    let request = match timeout(settings.read_headers_timeout, read_request_excluding_body(reader)).await {
        Ok(request) => request,
        Err(_) => {
            _ = send_response(writer, responses::create_request_timeout().await, None).await;
            return Err(ExchangeError::TimedOut);
        }
    };

    let mut request = match request {
        Ok(request) => request,
        Err(error) => {
            match error {
                Error::ParseError(error) => {
                    let mut response = handle_parse_error(error).await;
                    finish_response_error(&mut response).await;
                    send_response(writer, response, None).await?;
                    return Err(ExchangeError::MalformedData);
                }
                Error::Other(error) => {
                    return Err(error.into());
                }
            }
        }
    };

    // This should be done before reading the request body, since the PRI
    // method is special in that it doesn't convey a way for a normal HTTP/1.1
    // server to know that it contains a body using `Content-Length` or a
    // related mechanism, but it actually does.
    if request.method == Method::Pri {
        return handle_pri_method(reader, writer, request).await;
    }

    // TODO some handlers might prefer to read the body themselves.
    let body_result = match timeout(settings.read_body_timeout, read_request_body(reader, &mut request)).await {
        Ok(body_result) => body_result,
        Err(_) => {
            _ = send_response(writer, responses::create_request_timeout().await, None).await;
            return Err(ExchangeError::TimedOut);
        }
    };

    if let Err(error) = body_result {
        match error {
            Error::ParseError(error) => {
                let mut response = handle_parse_error(error).await;
                finish_response_error(&mut response).await;
                send_response(writer, response, None).await?;
                return Err(ExchangeError::MalformedData);
            }
            Error::Other(error) => {
                return Err(error.into());
            }
        }
    }

    #[cfg(feature = "debugging")]
    let start_handling = Instant::now();
    let mut response = handle_request(&request, settings).await;
    finish_response_normal(&request, &mut response).await;

    if let Some(BodyKind::File { metadata, .. }) = &response.body {
        if !metadata.is_file() {
            let mut response = Response::with_status(StatusCode::InternalServerError);

            #[cfg(feature = "debugging")]
            {
                dbg!("Warning: tried to send a non-file as response body: {}", metadata);
                response.body = Some(BodyKind::StaticString("Warning: tried to send a non-file as response body"));
            }

            finish_response_error(&mut response).await;
            send_response(writer, response, None).await?;

            return Ok(());
        }
    }

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

/// The 'PRI' method is used for upgrading HTTP/1.1 connections to HTTP/2. It
/// achieves this by using a special preface:
/// ```text
/// PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n
/// ```
async fn handle_pri_method<R, W>(reader: &mut R, writer: &mut W, request: Request) -> Result<(), ExchangeError>
        where R: AsyncBufReadExt + Unpin,
              W: AsyncWriteExt + Unpin {
    fn validate(index: usize, byte: u8) -> Result<(), Error> {
        if b"\r\nSM\r\n\r\n"[index] != byte {
            Err(Error::ParseError(HttpParseError::InvalidHttp2PriUpgradeBody))
        } else {
            Ok(())
        }
    }

    if let Err(Error::ParseError(HttpParseError::InvalidHttp2PriUpgradeBody)) = consume_exact_verify(reader, 8, validate).await {
        #[cfg(feature = "debugging")]
        println!("[HTTP/2] [PRI Upgrade] Invalid body!");

        let mut response = Response::with_status_and_string_body(StatusCode::BadRequest,
            "Invalid HTTP/2 PRI upgrade body");
        response.headers.append_or_override(HeaderName::Connection, "close".into());
        finish_response_error(&mut response).await;
        send_response(writer, response, None).await?;
        return Err(ExchangeError::MalformedData);
    }

    if request.version != HttpVersion::Http2 {
        let mut response = Response::with_status_and_string_body(StatusCode::HTTPVersionNotSupported,
                "Invalid HTTP upgrade using PRI: expected version HTTP/2.0");
        response.headers.append_or_override(HeaderName::Connection, "close".into());
        finish_response_error(&mut response).await;
        send_response(writer, response, None).await?;
        return Err(ExchangeError::MalformedData);
    }

    // PRI method should be exactly like the preface, so no headers
    if request.headers.iter().next().is_some() {
        let mut response = Response::with_status_and_string_body(StatusCode::BadRequest,
            "Invalid preface start request");
        response.headers.append_or_override(HeaderName::Connection, "close".into());
        finish_response_error(&mut response).await;
        send_response(writer, response, None).await?;
        return Err(ExchangeError::MalformedData);
    }

    // Notify the caller that the HTTP connection should be upgraded to version
    // HTTP/2.
    #[cfg(feature = "http2")]
    return Err(ExchangeError::Http2Upgrade);

    #[cfg(not(feature = "http2"))]
    return handle_pri_method_http2_not_enabled(writer).await;
}

#[cfg(not(feature = "http2"))]
async fn handle_pri_method_http2_not_enabled<W>(writer: &mut W) -> Result<(), ExchangeError>
        where W: AsyncWriteExt + Unpin {
    const FRAME_HTTP_1_1_REQUIRED: &'static [u8; 35] = &[
        // Settings Acknowledge
        0x00, 0x00, 0x00,       // length = 0
        0x04,                   // type = 0x04 SETTINGS
        0b00_00_00_01,          // flags = ACK
        0x00, 0x00, 0x00, 0x00, // reserved = 0, stream = 0

        // Settings from server (0)
        0x00, 0x00, 0x00,       // length = 0
        0x04,                   // type = 0x04 SETTINGS
        0b00_00_00_00,          // flags = 0
        0x00, 0x00, 0x00, 0x00, // reserved = 0, stream = 0

        // Goaway
        0x00, 0x00, 0x04,       // length = 4 (stream + error code)
        0x07,                   // type = 0x07 GOAWAY
        0x00,                   // flags = 0
        0x00, 0x00, 0x00, 0x00, // reserved = 0, stream = 0
        0x00, 0x00, 0x00, 0x00, // reserved = 0, last stream = 0
        0x0d, 0x00, 0x00, 0x00, // error code = 0x0d HTTP_1_1_REQUIRED
    ];

    writer.write_all(FRAME_HTTP_1_1_REQUIRED).await?;
    Err(ExchangeError::MalformedData)
}

/// Process a single socket connection.
async fn process_socket(stream: TcpStream, config: ServenteConfig) {
    #[cfg(any(feature = "rustls", feature = "tls-boring"))]
    let stream = {
        let mut stream = stream;
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

        #[cfg(feature = "rustls")]
        {
            let acceptor = TlsAcceptor::from(Arc::clone(&config.tls_config));
            match acceptor.accept(stream).await {
                Ok(stream) => stream,
                Err(_) => return,
            }
        }

        #[cfg(feature = "tls-boring")]
        match tokio_boring::accept(&config.tls_config, stream).await {
            Ok(stream) => stream,
            Err(_) => return,
        }
    };

    #[cfg(feature = "ktls")]
    let stream = match ktls::config_ktls_server(stream) {
        Ok(stream) => StreamWrapper::KtlsStream(stream),
        Err(e) => {
            //#[cfg(feature = "debugging")]
            println!("ktls error: {:?}", e);

            #[cfg(not(feature = "debugging"))]
            { _ = e }
            return;
        },
    };

    let (reader, writer) = split(stream);
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    loop {
        if let Err(e) = handle_exchange(&mut reader, &mut writer, &config.settings).await {
            #[cfg(feature = "http2")]
            if let ExchangeError::Http2Upgrade = e {
                servente_http2::handle_client(reader, writer, std::sync::Arc::new(config)).await;
                return;
            }

            #[cfg(feature = "debugging")]
            println!("Client Error: {:?}", e);

            #[cfg(not(feature = "debugging"))]
            { _ = e }
            return;
        }
    }
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
    if let Some(body) = response.body {
        match body {
            BodyKind::File { mut handle, .. } => {
                match transfer_strategy {
                    TransferStrategy::Full => transfer_body_full(stream, &mut handle).await?,
                    TransferStrategy::Chunked => transfer_body_chunked(stream, &mut handle).await?,
                    TransferStrategy::Ranges { ranges } => {
                        transfer_body_ranges(stream, &mut handle, ranges).await?
                    }
                }
            }
            BodyKind::Bytes(response) => stream.write_all(&response).await?,
            BodyKind::CachedBytes(cached_version, encoding) => match encoding {
                Some(ContentCoding::Brotli) => {
                    if cached_version.brotli.is_some() {
                        stream.write_all(cached_version.brotli.as_ref().unwrap()).await?;
                    } else if let Some(compressed_on_the_fly) = ContentCoding::Brotli.encode(&cached_version.uncompressed) {
                        stream.write_all(&compressed_on_the_fly).await?;
                    } else {
                        // TODO this isn't really a condition we should be in
                        debug_assert!(false, "Brotli was set as the ContentEncoding, but the cached version was not brotli-compressed and we failed to compress it on the fly.");
                        stream.write_all(&cached_version.uncompressed).await?;
                    }
                }
                Some(ContentCoding::Gzip) => {
                    if cached_version.gzip.is_some() {
                        stream.write_all(cached_version.gzip.as_ref().unwrap()).await?;
                    } else if let Some(compressed_on_the_fly) = ContentCoding::Gzip.encode(&cached_version.uncompressed) {
                        stream.write_all(&compressed_on_the_fly).await?;
                    } else {
                        // TODO this isn't really a condition we should be in
                        debug_assert!(false, "Gzip was set as the ContentEncoding, but the cached version was not gzip-compressed and we failed to compress it on the fly.");
                        stream.write_all(&cached_version.uncompressed).await?;
                    }
                }
                _ => stream.write_all(&cached_version.uncompressed).await?,
            }
            BodyKind::StaticString(response) => stream.write_all(response.as_bytes()).await?,
            BodyKind::String(response) => stream.write_all(response.as_bytes()).await?,
        }
    }
    _ = stream.flush().await;
    Ok(start.elapsed())
}

/// Start the HTTPv1 server on the given address.
pub async fn start(address: &str, config: ServenteConfig) -> io::Result<()> {
    let listener = TcpListener::bind(address).await?;
    println!("Started listening on {}", address);

    loop {
        let (stream, _) = match listener.accept().await {
            Ok((stream, addr)) => (stream, addr),
            Err(e) => {
                #[cfg(unix)]
                if let Some(os_error) = e.raw_os_error() {
                    if os_error == servente_common::platform::unix::ERRNO_EMFILE {
                        task::yield_now().await;
                        continue;
                    }
                }

                println!("[FATAL] Error accepting connection: {}", e);
                continue;
            }
        };
        let config = config.clone();
        task::spawn(async move {
            process_socket(stream, config).await;
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
    use std::time::Duration;

    use rstest::rstest;

    use super::*;
    use servente_http_handling::{handler::HandlerController, ServenteSettings};

    use servente_http::{
        HeaderMap,
        RequestTarget
    };

    /// The connection preface, as defined in [RFC 9113 Section 3.4](https://www.rfc-editor.org/rfc/rfc9113.html#name-http-2-connection-preface).
    #[allow(dead_code)] // Linting doesn't understand that this is used in tests :(
    const HTTP2_CONNECTION_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

    lazy_static::lazy_static! {
        //static ref HUFFMAN_TREE: Box<BinaryTreeNode> = BinaryTreeNode::construct(HUFFMAN_CODE);
        static ref SETTINGS: ServenteSettings = ServenteSettings {
            handler_controller: HandlerController::new(),
            read_headers_timeout: Duration::from_secs(5),
            read_body_timeout: Duration::from_secs(5),
        };
    }

    #[tokio::test]
    async fn test_consume_exact_verify() {
        let mut buf = std::io::Cursor::new(&[0, 1, 2, 3, 4, 5, 7]);

        let result = consume_exact_verify(&mut buf, 6, |idx, byte| {
            if idx == byte as usize {
                Ok(())
            } else {
                Err(Error::ParseError(HttpParseError::InvalidOctetInMethod))
            }
        }).await;

        assert_eq!(result.unwrap(), vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(buf.position(), 6);
        assert_eq!(consume_exact_verify(&mut buf, 0, |_,_| Ok(())).await.unwrap(), Vec::new());

        let eof_result = consume_exact_verify(&mut tokio::io::empty(), 3, |_,_| Ok(())).await;
        assert!(matches!(eof_result, Err(Error::Other(_))));
        if let Error::Other(io_error) = eof_result.unwrap_err() {
            assert_eq!(io_error.kind(), std::io::ErrorKind::UnexpectedEof);
        }
    }

    #[rstest]
    #[case("Connection: \rkeep-alive", HttpParseError::InvalidCRLF)]
    #[case("Connection keep-alive", HttpParseError::HeaderDoesNotContainColon)]
    #[case("Connection keep-alive", HttpParseError::HeaderDoesNotContainColon)]
    #[tokio::test]
    async fn read_headers_name_validation(#[case] line: &str, #[case] expected: HttpParseError) {
        let mut stream = std::io::Cursor::new(format!("{}\r\n\r\n", line));
        let headers = super::read_headers(&mut stream).await;
        assert!(headers.is_err());
        assert!(matches!(headers.err().unwrap(), Error::ParseError(e) if e == expected));
    }

    #[cfg(feature = "http2")]
    #[tokio::test]
    async fn http2_upgrade_read_request_excluding_body() {
        let mut data = std::io::Cursor::new(HTTP2_CONNECTION_PREFACE);
        let request = read_request_excluding_body(&mut data).await.unwrap();
        assert_eq!(request.method, Method::Pri);
        assert_eq!(request.target, RequestTarget::Asterisk);
        assert_eq!(request.version, HttpVersion::Http2);
        assert!(request.headers.is_empty());
        assert_eq!(data.position() as usize, b"PRI * HTTP/2.0\r\n".len());
    }

    #[cfg(feature = "http2")]
    #[tokio::test]
    async fn http2_upgrade_handle_pri_method() {
        const DATA: &[u8] = b"\r\nSM\r\n\r\n";
        let mut data = std::io::Cursor::new(DATA);
        let mut writer = Vec::new();
        let request = Request {
            method: Method::Pri,
            target: RequestTarget::Asterisk,
            version: HttpVersion::Http2,
            headers: HeaderMap::new(),
            body: None,
        };
        let exchange_error = handle_pri_method(&mut data, &mut writer, request).await.unwrap_err();
        assert_eq!(data.position() as usize, DATA.len());
        assert_eq!(writer, Vec::new());
        assert!(matches!(exchange_error, ExchangeError::Http2Upgrade), "Invalid error: {exchange_error:#?} written: {}", String::from_utf8_lossy(writer.as_slice()));
    }

    #[cfg(feature = "http2")]
    #[tokio::test]

    async fn http2_upgrade_handle_exchange() {
        let mut data = std::io::Cursor::new(HTTP2_CONNECTION_PREFACE);
        let mut writer = Vec::new();
        let exchange_error = handle_exchange(&mut data, &mut writer, &SETTINGS).await.unwrap_err();
        assert!(matches!(exchange_error, ExchangeError::Http2Upgrade), "Invalid error: {exchange_error:#?} written: {}", String::from_utf8_lossy(writer.as_slice()));
    }
}
