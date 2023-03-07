use tokio::{
    net::{TcpListener, TcpStream},
    task, io::{split, AsyncWriteExt, AsyncReadExt, BufReader, AsyncBufReadExt, BufWriter},
};

use rustls::{
    ServerConfig
};
use tokio_rustls::TlsAcceptor;

use std::{io, sync::Arc, borrow::Cow, time::SystemTime, env::current_dir, path::Path, fs};

use crate::http::{
    message::{Request, Method, RequestTarget, HttpVersion, HeaderMap, HeaderName, Response, StatusCode, BodyKind},
    error::HttpParseError,
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

/// Checks if the request is not modified and returns a 304 response if it isn't.
async fn check_not_modified(request: &Request, _path: &Path, metadata: &fs::Metadata) -> Result<Option<Response>, io::Error> {
    if let Some(if_modified_since) = request.headers.get(&HeaderName::IfModifiedSince) {
        if let Ok(modified_date) = metadata.modified() {
            if let Ok(if_modified_since_date) = httpdate::parse_http_date(if_modified_since) {
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

async fn detect_media_type<'a>(request: &Request, response: &Response) -> Cow<'a, str> {
    if let RequestTarget::Origin(path) = &request.target {
        if let Some(extension) = path.rfind(".").and_then(|index| path.get((1 + index)..)) {
            match extension {
                "htm" => return Cow::Borrowed("text/html;charset=utf-8"),
                "html" => return Cow::Borrowed("text/html;charset=utf-8"),
                "css" => return Cow::Borrowed("text/css;charset=utf-8"),
                "js" => return Cow::Borrowed("application/javascript;charset=utf-8"),
                "json" => return Cow::Borrowed("application/json;charset=utf-8"),
                "svg" => return Cow::Borrowed("image/svg+xml"),
                "png" => return Cow::Borrowed("image/png"),
                "jpg" => return Cow::Borrowed("image/jpeg"),
                "jpeg" => return Cow::Borrowed("image/jpeg"),
                "gif" => return Cow::Borrowed("image/gif"),
                "ico" => return Cow::Borrowed("image/x-icon"),
                "txt" => return Cow::Borrowed("text/plain;charset=utf-8"),
                "xml" => return Cow::Borrowed("application/xml;charset=utf-8"),
                "pdf" => return Cow::Borrowed("application/pdf"),
                "zip" => return Cow::Borrowed("application/zip"),
                "rar" => return Cow::Borrowed("application/x-rar-compressed"),
                "7z" => return Cow::Borrowed("application/x-7z-compressed"),
                _ => (),
            }
        }
    }

    _ = response;
    Cow::Borrowed("application/octet-stream")
}

async fn discard_request(stream: &mut TcpStream) -> Result<(), Error> {
    let mut buffer = BufReader::new(stream);
    loop {
        let line = read_crlf_line(&mut buffer, MaximumLength::HEADER).await?;
        println!("Discarded: {} ({} len)", line, line.len());
        if line.len() == 0 {
            return Ok(());
        }
    }
}

async fn finish_response_error(response: &mut Response) -> Result<(), Error>{
    finish_response_general(response).await
}

async fn finish_response_general(response: &mut Response) -> Result<(), Error>{
    if let Some(body) = &response.body {
        if !response.headers.contains(&HeaderName::LastModified) {
            if let BodyKind::File(file) = body {
                if let Ok(metadata) = file.metadata().await {
                    if let Ok(modified_date) = metadata.modified() {
                        response.headers.set(HeaderName::LastModified, httpdate::fmt_http_date(modified_date));
                    }
                }
            }
        }
    }

    response.headers.set(HeaderName::Server, String::from("servente"));

    if !response.headers.contains(&HeaderName::Connection) {
        response.headers.set(HeaderName::Connection, String::from("keep-alive"));
    }

    if !response.headers.contains(&HeaderName::Date) {
        response.headers.set(HeaderName::Date, httpdate::fmt_http_date(SystemTime::now()));
    }

    Ok(())
}

async fn finish_response_normal(request: &Request, response: &mut Response) -> Result<(), Error>{
    if response.body.is_some() {
        if !response.headers.contains(&HeaderName::ContentType) {
            response.headers.set(HeaderName::ContentType, detect_media_type(request, response).await.into_owned());
        }
    }
    finish_response_general(response).await
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

async fn handle_request<R>(stream: &mut R, request: &Request) -> Result<Response, Error>
        where R: AsyncBufReadExt + Unpin {
    if let RequestTarget::Origin(request_target) = &request.target {
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
                    response.headers.set(HeaderName::LastModified, httpdate::fmt_http_date(modified_date));
                }

                println!("Path: {:?}", path.display());
                if request.headers.get(&HeaderName::SecFetchDest) == Some(&String::from("document")) {
                    response.prelude_response.push(Response{
                        version: request.version,
                        status: StatusCode::EarlyHints,
                        headers: HeaderMap::new_with_vec(vec![
                            (HeaderName::Link, String::from("<HTML%20Standard_bestanden/spec.css>; rel=preload; as=style")),
                            (HeaderName::Link, String::from("<HTML%20Standard_bestanden/standard.css>; rel=preload; as=style")),
                            (HeaderName::Link, String::from("<HTML%20Standard_bestanden/standard-shared-with-dev.css>; rel=preload; as=style")),
                            (HeaderName::Link, String::from("<HTML%20Standard_bestanden/styles.css>; rel=preload; as=style")),
                            (HeaderName::Link, String::from("<script.js>; rel=preload; as=script")),
                        ]),
                        body: None,
                        prelude_response: vec![],
                    });
                    response.headers = HeaderMap::new_with_vec(vec![
                        (HeaderName::Link, String::from("<HTML%20Standard_bestanden/spec.css>; rel=preload; as=style")),
                        (HeaderName::Link, String::from("<HTML%20Standard_bestanden/standard.css>; rel=preload; as=style")),
                        (HeaderName::Link, String::from("<HTML%20Standard_bestanden/standard-shared-with-dev.css>; rel=preload; as=style")),
                        (HeaderName::Link, String::from("<HTML%20Standard_bestanden/styles.css>; rel=preload; as=style")),
                    ]);
                }

                return Ok(response);
            }
        }

        return Ok(Response::with_status_and_string_body(StatusCode::NotFound, "Not Found"));
    }

    _ = stream;
    Ok(Response::with_status_and_string_body(StatusCode::BadRequest, "Invalid Target"))
}

async fn process_socket(mut stream: TcpStream, tls_config: Arc<ServerConfig>) {
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

    let acceptor = TlsAcceptor::from(tls_config);
    let stream = match acceptor.accept(stream).await {
        Ok(stream) => stream,
        Err(e) => {
            println!("Client Error: {:?}", e);
            return;
        }
    };

    let (reader, writer) = split(stream);
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    loop {
        let request = match read_request_excluding_body(&mut reader).await {
            Ok(request) => request,
            Err(e) => {
                println!("Client Error: {:?}", e);
                match e {
                    Error::ParseError(error) => {
                        let mut response = handle_parse_error(error).await;
                        finish_response_error(&mut response).await.unwrap();
                        send_response(&mut writer, response).await.unwrap();
                        continue;
                    }
                    _ => return,
                }
            }
        };

        println!("{:?}>: {:?}", request.method, request.target);

        let mut response = handle_request(&mut reader, &request).await.unwrap();
        finish_response_normal(&request, &mut response).await.unwrap();

        for response in response.prelude_response {
            send_response(&mut writer, response).await.unwrap();
        }
        response.prelude_response = Vec::new();

        send_response(&mut writer, response).await.unwrap();
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

        headers.push((HeaderName::from_str(name), value));
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
        return Ok(RequestTarget::Origin(str));
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

async fn send_response<R>(stream: &mut R, mut response: Response) -> Result<(), io::Error>
        where R: AsyncWriteExt + Unpin {
    let mut use_transfer_encoding = false;
    if let Some(body) = &response.body {
        match body {
            BodyKind::File(file) => {
                let len = file.metadata().await.unwrap().len();
                if len > TRANSFER_ENCODING_THRESHOLD {
                    use_transfer_encoding = true;
                    response.headers.set(HeaderName::TransferEncoding, "chunked".to_owned());
                } else {
                    response.headers.set(HeaderName::ContentLength, len.to_string());
                }
            }
            BodyKind::Bytes(bytes) => {
                response.headers.set(HeaderName::ContentLength, bytes.len().to_string());
            }
            BodyKind::StaticString(string) => {
                response.headers.set(HeaderName::ContentLength, string.len().to_string())
            }
            BodyKind::String(string) => {
                response.headers.set(HeaderName::ContentLength, string.len().to_string())
            }
        }
    }

    let mut response_text = String::with_capacity(1024);
    response_text.push_str("HTTP/1.1 ");
    response_text.push_str(&response.status.to_string());
    response_text.push_str("\r\n");

    for (name, value) in response.headers.iter() {
        response_text.push_str(name.to_string_h1());
        response_text.push_str(": ");
        response_text.push_str(value);
        response_text.push_str("\r\n");
    }

    response_text.push_str("\r\n");

    stream.write_all(response_text.as_bytes()).await?;


    if let Some(response) = response.body {
        match response {
            BodyKind::File(mut response) => {
                if use_transfer_encoding {
                    let mut buf: [u8; 4096] = [0; 4096];
                    loop {
                        let len = response.read(&mut buf).await?;

                        if len == 0 {
                            break;
                        }

                        stream.write_all(format!("{:X}\r\n", len).as_bytes()).await?;

                        stream.write_all(&buf[0..len]).await?;

                        stream.write_all(b"\r\n").await?;
                    }

                    stream.write_all(b"0\r\n\r\n").await?;
                } else {
                    tokio::io::copy(&mut response, stream).await?;
                }
            }
            BodyKind::Bytes(response) => stream.write_all(&response).await?,
            BodyKind::StaticString(response) => stream.write_all(response.as_bytes()).await?,
            BodyKind::String(response) => stream.write_all(response.as_bytes()).await?,
        }
    }
    _ = stream.flush().await;
    Ok(())
}

pub async fn start(address: &str, tls_config: Arc<ServerConfig>) -> io::Result<()> {
    let listener = TcpListener::bind(address).await?;
    println!("Started listening on {}", address);

    loop {
        let (stream, _) = listener.accept().await?;
        let tls_config = Arc::clone(&tls_config);
        task::spawn(async {
            process_socket(stream, tls_config).await;
        });
    }
}
