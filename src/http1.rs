use tokio::{
    net::{TcpListener, TcpStream},
    task, io::{split, AsyncWriteExt, AsyncReadExt, BufReader, AsyncBufReadExt, BufWriter, AsyncSeekExt}, time::Instant,
};

use tokio_rustls::TlsAcceptor;

use std::{
    env::current_dir,
    fs,
    io::{self, SeekFrom},
    path::Path,
    time::{SystemTime, Duration},
};

use crate::{
    http::{
        message::{Request, Method, RequestTarget, HttpVersion, HeaderMap, HeaderName, Response, StatusCode, BodyKind, StatusCodeClass, HeaderValue, HttpRangeList, Range, ContentRangeHeaderValue},
        error::HttpParseError, hints::{SecFetchDest, AcceptedLanguages},
    },
    resources::{MediaType, static_res}, ServenteConfig,
};

const TRANSFER_ENCODING_THRESHOLD: u64 = 1024 * 1024; // 1 MiB

#[derive(Debug)]
pub enum Error {
    ParseError(HttpParseError),
    Other(io::Error),
}

impl From<HttpParseError> for Error {
    fn from(error: HttpParseError) -> Self {
        Error::ParseError(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Other(error)
    }
}

struct MaximumLength(pub usize);

impl MaximumLength {
    /// The maximum length of a method name.
    pub const METHOD: MaximumLength = MaximumLength(16);

    /// The maximum length of a request target, including the query string.
    pub const REQUEST_TARGET: MaximumLength = MaximumLength(1024);

    /// The maximum length of a full HTTP header (name + value), excluding the CRLF.
    pub const HEADER: MaximumLength = MaximumLength(4096);
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransferStrategy {
    Chunked,
    Full,
    Ranges { ranges: HttpRangeList },
}

/// Checks if the request is not modified and returns a 304 response if it isn't.
async fn check_not_modified(request: &Request, _path: &Path, metadata: &fs::Metadata) -> Result<Option<Response>, io::Error> {
    if let Some(if_modified_since) = request.headers.get(&HeaderName::IfModifiedSince) {
        if let Ok(modified_date) = metadata.modified() {
            if let Ok(if_modified_since_date) = if_modified_since.try_into() {
                match modified_date.duration_since(if_modified_since_date) {
                    Ok(duration) => {
                        if duration.as_secs() == 0 {
                            let mut response = Response::with_status_and_string_body(StatusCode::NotModified, String::new());
                            response.headers.set(HeaderName::LastModified, if_modified_since.to_owned());
                            return Ok(Some(response));
                        }
                    },
                    // The file modified time is somehow earlier than
                    // If-Modified-Date, but that's okay, since the normal file
                    // handler will handle it.
                    Err(_) => (),
                }
            }
        }
    }

    // TODO support etags and stuff
    return Ok(None);
}

async fn consume_crlf<R>(stream: &mut R) -> Result<(), Error>
        where R: AsyncBufReadExt + Unpin {
    let mut buffer = [0u8; 2];
    stream.read_exact(&mut buffer).await?;

    if buffer[0] != b'\r' || buffer[1] != b'\n' {
        return Err(Error::ParseError(HttpParseError::InvalidCRLF));
    }

    Ok(())
}

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
                if ranges.ranges.len() == 1 {
                    match ranges.ranges.first().unwrap() {
                        Range::Full => {
                            response.headers.set_content_range(ContentRangeHeaderValue::Range {
                                start: 0,
                                end: (file_size - 1) as _,
                                complete_length: Some(file_size as _),
                            });
                        }
                        Range::Points { start, end } => {
                            response.headers.set_content_range(ContentRangeHeaderValue::Range {
                                start: *start as _,
                                end: *end as _,
                                complete_length: Some(file_size as _),
                            });
                        }
                        Range::StartPointToEnd { start } => {
                            response.headers.set_content_range(ContentRangeHeaderValue::Range {
                                start: *start as _,
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
                    todo!();
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

async fn discard_request(stream: &mut TcpStream) -> Result<(), Error> {
    let mut buffer = BufReader::new(stream);
    loop {
        let line = read_crlf_line(&mut buffer, MaximumLength::HEADER).await?;
        if line.len() == 0 {
            return Ok(());
        }
    }
}

async fn finish_response_error(response: &mut Response) -> Result<(), io::Error>{
    response.headers.set(HeaderName::Connection, HeaderValue::from("close"));
    finish_response_general(response).await
}

async fn finish_response_general(response: &mut Response) -> Result<(), io::Error>{
    if let Some(body) = &response.body {
        if !response.headers.contains(&HeaderName::LastModified) {
            if let BodyKind::File(file) = body {
                if let Ok(metadata) = file.metadata().await {
                    if let Ok(modified_date) = metadata.modified() {
                        response.headers.set(HeaderName::LastModified, HeaderValue::from(modified_date));
                    }
                }
            }
        }
    }

    response.headers.set(HeaderName::Server, HeaderValue::from("servente"));
    response.headers.set(HeaderName::AltSvc, HeaderValue::from("h2=\":8080\", h3=\":8080\""));

    if !response.headers.contains(&HeaderName::Connection) {
        response.headers.set(HeaderName::Connection, HeaderValue::from("keep-alive"));
    }

    if !response.headers.contains(&HeaderName::Date) {
        response.headers.set(HeaderName::Date, SystemTime::now().into());
    }

    Ok(())
}

async fn finish_response_normal(request: &Request, response: &mut Response) -> Result<(), io::Error>{
    if response.body.is_some() {
        if !response.headers.contains(&HeaderName::ContentType) {
            response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(request.target.as_str()).clone()));
        }

        if response.status.class() == StatusCodeClass::Success {
            if !response.headers.contains(&HeaderName::CacheControl) {
                response.headers.set(HeaderName::CacheControl, HeaderValue::from("max-age=120"));
            }
        }
    }

    finish_response_general(response).await
}

async fn handle_exchange<R, W>(reader: &mut R, writer: &mut W, config: &ServenteConfig) -> Result<(), io::Error>
        where R: AsyncBufReadExt + Unpin, W: AsyncWriteExt + Unpin {
    let start_full = Instant::now();

    let request = match read_request_excluding_body(reader).await {
        Ok(request) => request,
        Err(error) => {
            match error {
                Error::ParseError(error) => {
                    let mut response = handle_parse_error(error).await;
                    finish_response_error(&mut response).await.unwrap();
                    send_response(writer, response, None).await?;
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "Parse error"));
                }
                Error::Other(error) => {
                    return Err(error);
                }
            }
        }
    };

    let start_handling = Instant::now();
    let mut response = match handle_request(reader, &request, config).await {
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
                .and_then(|range| HttpRangeList::parse(range))
    ).await?;

    println!("{:?}>: {:?} (f={}ms, h={}ms, b={}ms)", request.method, request.target, start_full.elapsed().as_millis(), start_handling.elapsed().as_millis(), sent_body.as_millis());

    Ok(())
}

async fn handle_parse_error(error: HttpParseError) -> Response {
    match error {
        HttpParseError::HeaderTooLarge => Response::with_status_and_string_body(StatusCode::RequestHeaderFieldsTooLarge, "Request Header Fields Too Large"),
        HttpParseError::InvalidCRLF => Response::bad_request("Invalid CRLF"),
        HttpParseError::InvalidHttpVersion => Response::with_status_and_string_body(StatusCode::HTTPVersionNotSupported, "Invalid HTTP version"),
        HttpParseError::InvalidRequestTarget => Response::bad_request("Invalid request target"),
        HttpParseError::MethodTooLarge => Response::with_status_and_string_body(StatusCode::MethodNotAllowed, "Method Not Allowed"),
        HttpParseError::RequestTargetTooLarge => Response::with_status_and_string_body(StatusCode::URITooLong, "Invalid request target"),
    }
}

async fn handle_request<R>(stream: &mut R, request: &Request, config: &ServenteConfig) -> Result<Response, Error>
        where R: AsyncBufReadExt + Unpin {
    let controller = config.handler_controller.clone();
    if let Some(result) = controller.check_handle(&request) {
        return result.map_err(|error| Error::Other(io::Error::new(io::ErrorKind::Other, error)));
    }

    if let RequestTarget::Origin { path, .. } = &request.target {
        let request_target = path.as_str();
        if request.method != Method::Get {
            return Ok(Response::with_status_and_string_body(StatusCode::MethodNotAllowed, "Method Not Allowed"));
        }

        let root = current_dir().unwrap().join("wwwroot/");
        let path = root.join(urlencoding::decode(&request_target[1..]).unwrap().into_owned());
        if !path.starts_with(&root) {
            return Ok(Response::with_status_and_string_body(StatusCode::Forbidden, format!("Forbidden\n{}\n{}", root.display(), path.display())));
        }

        if let Ok(metadata) = std::fs::metadata(&path) {
            if metadata.is_file() {
                if let Some(not_modified_response) = check_not_modified(request, &path, &metadata).await? {
                    return Ok(not_modified_response);
                }

                let file = tokio::fs::File::open(&path).await.unwrap();

                let mut response = Response::with_status(StatusCode::Ok);
                response.body = Some(BodyKind::File(file));

                if let Ok(modified_date) = metadata.modified() {
                    response.headers.set(HeaderName::LastModified, modified_date.into());
                }

                if request.headers.sec_fetch_dest() == Some(SecFetchDest::Document) {
                    response.prelude_response.push(Response{
                        version: request.version,
                        status: StatusCode::EarlyHints,
                        headers: HeaderMap::new_with_vec(vec![
                            (HeaderName::Link, HeaderValue::from("<HTML%20Standard_bestanden/spec.css>; rel=preload; as=style")),
                            (HeaderName::Link, HeaderValue::from("<HTML%20Standard_bestanden/standard.css>; rel=preload; as=style")),
                            (HeaderName::Link, HeaderValue::from("<HTML%20Standard_bestanden/standard-shared-with-dev.css>; rel=preload; as=style")),
                            (HeaderName::Link, HeaderValue::from("<HTML%20Standard_bestanden/styles.css>; rel=preload; as=style")),
                            (HeaderName::Link, HeaderValue::from("<script.js>; rel=preload; as=script")),
                        ]),
                        body: None,
                        prelude_response: vec![],
                    });
                    response.headers = HeaderMap::new_with_vec(vec![
                        (HeaderName::Link, HeaderValue::from("<HTML%20Standard_bestanden/spec.css>; rel=preload; as=style")),
                        (HeaderName::Link, HeaderValue::from("<HTML%20Standard_bestanden/standard.css>; rel=preload; as=style")),
                        (HeaderName::Link, HeaderValue::from("<HTML%20Standard_bestanden/standard-shared-with-dev.css>; rel=preload; as=style")),
                        (HeaderName::Link, HeaderValue::from("<HTML%20Standard_bestanden/styles.css>; rel=preload; as=style")),
                    ]);
                }

                return Ok(response);
            }
        }

        if !root.join("/index.html").exists() {
            return handle_welcome_page(request, request_target).await;
        }

        return Ok(Response::with_status_and_string_body(StatusCode::NotFound, "Not Found"));
    }

    _ = stream;
    Ok(Response::with_status_and_string_body(StatusCode::BadRequest, "Invalid Target"))
}

async fn handle_welcome_page(request: &Request, request_target: &str) -> Result<Response, Error> {
    let mut response = Response::with_status(StatusCode::Ok);
    response.headers.set_content_type(MediaType::HTML);
    response.headers.set(HeaderName::CacheControl, "public, max-age=600".into());
    response.headers.set(HeaderName::ContentSecurityPolicy, "default-src 'self'; upgrade-insecure-requests; style-src-elem 'self' 'unsafe-inline'".into());

    response.body = Some(BodyKind::StaticString(static_res::WELCOME_HTML));
    response.headers.set(HeaderName::ContentLanguage, "en".into());

    match request_target {
        "/" | "/index" | "/index.html" => {
            if let Some(accepted_languages) = request.headers.get(&HeaderName::AcceptLanguage) {
                if let Some(accepted_languages) = AcceptedLanguages::parse(accepted_languages.as_str_no_convert().unwrap()) {
                    if let Some(best) = accepted_languages.match_best(vec!["nl", "en"]) {
                        if best == "nl" {
                            response.body = Some(BodyKind::StaticString(static_res::WELCOME_HTML_NL));
                            response.headers.set(HeaderName::ContentLanguage, "nl".into());
                        }
                    }
                }
            }
        }
        "/welcome.en.html" => (),
        "/welcome.nl.html" => {
            response.body = Some(BodyKind::StaticString(static_res::WELCOME_HTML_NL));
            response.headers.set(HeaderName::ContentLanguage, "nl".into());
        }
        _ => return Ok(Response::with_status_and_string_body(StatusCode::NotFound, "Not Found")),
    }

    return Ok(response);
}

async fn process_socket(mut stream: TcpStream, config: ServenteConfig) {
    println!("Client connected: {}", stream.peer_addr().unwrap());
    let mut buf = [0u8; 4];
    if let Ok(length) = stream.peek(&mut buf).await {
        if length >= 3 && &buf[0..3] == b"GET" {
            if let Err(e) = discard_request(&mut stream).await {
                println!("Client Error discarding non-HTTPS: {:?}", e);
                return;
            }

            send_http_upgrade(&mut stream).await.unwrap();
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
            println!("Client Error: {:?}", e);
            return;
        }
    }
}

async fn read_crlf_line<R>(stream: &mut R, maximum_length: MaximumLength) -> Result<String, Error>
        where R: AsyncBufReadExt + Unpin {
    let mut string = String::new();

    while string.len() < maximum_length.0 {
        let byte = stream.read_u8().await?;
        if byte == '\r' as u8 {
            let byte = stream.read_u8().await?;
            if byte == '\n' as u8 {
                return Ok(string);
            }
            return Err(Error::ParseError(HttpParseError::InvalidCRLF));
        }

        string.push(byte as char);
    }

    Err(Error::ParseError(HttpParseError::HeaderTooLarge))
}

async fn read_headers<R>(stream: &mut R) -> Result<HeaderMap, Error>
        where R: AsyncBufReadExt + Unpin {
    let mut headers = Vec::new();

    loop {
        let line = read_crlf_line(stream, MaximumLength::HEADER).await?;
        if line.len() == 0 {
            return Ok(HeaderMap::new_with_vec(headers));
        }

        let mut parts = line.splitn(2, ':');
        let name = parts.next().unwrap().trim().to_string();
        let value = parts.next().unwrap().trim().to_string();

        let name = HeaderName::from_str(name);
        if let HeaderName::Other(name) = &name {
            #[cfg(debug_assertions)]
            println!("[DEBUG] Unknown header name: \"{}\" with value: \"{}\"", name, value);
        }
        headers.push((name, HeaderValue::from(value)));
    }
}

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

async fn read_request_excluding_body<R>(stream: &mut R) -> Result<Request, Error>
        where R: AsyncBufReadExt + Unpin {
    let (method, target, version) = read_request_line(stream).await?;
    let headers = read_headers(stream).await?;
    Ok(Request { method, target, version, headers })
}

async fn read_request_line<R>(stream: &mut R) -> Result<(Method, RequestTarget, HttpVersion), Error>
        where R: AsyncBufReadExt + Unpin {

    let method = Method::from_str(read_string_until_character(stream, ' ' as u8, MaximumLength::METHOD, HttpParseError::MethodTooLarge).await?);

    // TODO skip OWS
    let target = read_request_target(stream).await?;

    // TODO skip OWS

    let version = read_http_version(stream).await?;
    consume_crlf(stream).await?;

    Ok((method, target, version))
}

async fn read_request_target<R>(stream: &mut R) -> Result<RequestTarget, Error>
        where R: AsyncBufReadExt + Unpin {
    let str = read_string_until_character(stream, ' ' as u8, MaximumLength::REQUEST_TARGET, HttpParseError::RequestTargetTooLarge).await?;

    if str == "*" {
        return Ok(RequestTarget::Asterisk);
    }

    if str.starts_with("/") {
        let mut parts = str.splitn(2, '?');
        return Ok(RequestTarget::Origin {
            path: parts.next().unwrap().to_string(),
            query: parts.next().unwrap_or("").to_string(),
        });
    }

    // TODO
    if str.starts_with("http://") || str.starts_with("https://") {
        return Ok(RequestTarget::Absolute(str));
    }

    Err(Error::ParseError(HttpParseError::InvalidRequestTarget))
}

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
    _ = stream.write_all(message.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

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
            BodyKind::StaticString(response) => stream.write_all(response.as_bytes()).await?,
            BodyKind::String(response) => stream.write_all(response.as_bytes()).await?,
        }
    }
    _ = stream.flush().await;
    Ok(start.elapsed())
}


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

async fn transfer_body_full<O, I>(output: &mut O, input: &mut I) -> Result<(), io::Error>
        where O: AsyncWriteExt + Unpin,
              I: AsyncReadExt + Unpin {
    tokio::io::copy(input, output).await?;
    Ok(())
}

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
