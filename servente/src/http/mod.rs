// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{
    io,
    path::Path,
    sync::Arc,
    time::SystemTime, env::current_dir,
};

use crate::{
    resources::{
        self,
        cache::{
            self,
            CachedFileDetails,
        },
        MediaType,
        static_res,
    },
    ServenteConfig,
};

use self::{
    error::HttpParseError,
    hints::AcceptedLanguages,
    message::{
        BodyKind,
        HeaderName,
        HeaderValue,
        Method,
        Request,
        RequestTarget,
        Response,
        StatusCode,
        StatusCodeClass,
        format_system_time_as_weak_etag,
    },
};

pub mod error;
pub mod hints;
pub mod message;
pub mod v1;

#[cfg(feature = "http2")]
pub mod v2;

#[cfg(feature = "http3")]
pub mod v3;

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

/// Checks if the request is not modified and returns a 304 response if it isn't.
fn check_not_modified(request: &Request, path: &Path, modified_date: SystemTime) -> Option<Response> {
    if let Some(etag) = request.headers.get(&HeaderName::IfNoneMatch) {
        if etag.as_str_no_convert().unwrap() == format_system_time_as_weak_etag(modified_date) {
            let mut response = Response::with_status_and_string_body(StatusCode::NotModified, String::new());
            response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));
            response.headers.set(HeaderName::ETag, etag.clone());
            return Some(response);
        }
    }

    if let Some(if_modified_since) = request.headers.get(&HeaderName::IfModifiedSince) {
        if let Ok(if_modified_since_date) = if_modified_since.try_into() {
            if let Ok(duration) = modified_date.duration_since(if_modified_since_date) {
                if duration.as_secs() == 0 {
                    let mut response = Response::with_status_and_string_body(StatusCode::NotModified, String::new());
                    response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));
                    response.headers.set(HeaderName::LastModified, if_modified_since.to_owned());
                    return Some(response);
                }
            }

            // The file modified time is somehow earlier than
            // If-Modified-Date, but that's okay, since the normal file
            // handler will handle it.
        }
    }

    None
}

/// Finishes a response for an error response.
pub async fn finish_response_error(response: &mut Response) {
    response.headers.set(HeaderName::Connection, HeaderValue::from("close"));
    finish_response_general(response).await
}

/// Finishes a response for both normal and error response.
async fn finish_response_general(response: &mut Response) {
    if let Some(body) = &response.body {
        if !response.headers.contains(&HeaderName::LastModified) {
            if let BodyKind::File { metadata, ..} = body {
                if let Ok(modified_date) = metadata.modified() {
                    response.headers.set_last_modified(modified_date);
                }
            }
        }
    }

    response.headers.set(HeaderName::Server, HeaderValue::from("servente"));

    #[cfg(all(feature = "http2", not(feature = "http3")))]
    response.headers.set(HeaderName::AltSvc, HeaderValue::from("h2=\":8080\""));

    #[cfg(all(feature = "http2", feature = "http3"))]
    response.headers.set(HeaderName::AltSvc, HeaderValue::from("h2=\":8080\", h3=\":8080\""));

    response.headers.set(HeaderName::XFrameOptions, "DENY".into());
    response.headers.set(HeaderName::XXSSProtection, "X-XSS-Protection: 1; mode=block".into());
    response.headers.set(HeaderName::XContentTypeOptions, "nosniff".into());

    if !response.headers.contains(&HeaderName::Connection) {
        response.headers.set(HeaderName::Connection, HeaderValue::from("keep-alive"));
    }

    if !response.headers.contains(&HeaderName::Date) {
        response.headers.set(HeaderName::Date, SystemTime::now().into());
    }
}

/// Finishes a response for a normal (OK) response.
pub async fn finish_response_normal(request: &Request, response: &mut Response) {
    if response.body.is_some() {
        if !response.headers.contains(&HeaderName::ContentType) {
            response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(request.target.as_str()).clone()));
        }

        if response.status.class() == StatusCodeClass::Success && !response.headers.contains(&HeaderName::CacheControl) {
            response.headers.set(HeaderName::CacheControl, HeaderValue::from("max-age=120"));
        }
    }

    finish_response_general(response).await
}

/// Handle an OPTIONS request.
///
/// This request queries the capabilities of the server, or of a specific
/// target-resource.
///
/// # References
/// * [RFC 9110 Section 9.3.7](https://www.rfc-editor.org/rfc/rfc9110.html#name-options)
async fn handle_options(request: &Request, config: &ServenteConfig) -> Response {
    if request.target == RequestTarget::Asterisk {
        return handle_options_asterisk();
    }

    if let Some(response) = config.handler_controller.check_handle_options(request) {
        return response;
    }

    // TODO: support static resources.
    // if let RequestTarget::Origin { path, .. } = &request.target {
    //
    // }

    Response::not_found("Not Found")
}

/// Handle an `OPTIONS` request for the '*' resource, meaning the global
/// capabilities of the server.
fn handle_options_asterisk() -> Response {
    let mut response = Response::with_status(StatusCode::Ok);
    response.headers.set(HeaderName::Allow, "GET, HEAD, OPTIONS, POST".into());
    response.headers.set(HeaderName::Allow, "GET, HEAD".into());
    response.headers.set_content_length(0);
    response
}

/// Handles a `HttpParseError`.
///
/// Servers SHOULD explain the error to the client, but this might be a security
/// risk, so we might want to make this optional.
pub async fn handle_parse_error(error: HttpParseError) -> Response {
    let body = format!("<h1>Bad Request<h1>
<hr>
<p>{}</p>", error.as_ref());
    let mut response = Response::with_status_and_string_body(StatusCode::BadRequest, body);
    response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::HTML));
    response
}

/// Handles a request.
pub async fn handle_request(request: &Request, config: &ServenteConfig) -> Response {
    if request.method == Method::Options {
        return handle_options(request, config).await;
    }

    // Method is not OPTIONS, so a request-target of "*" is not allowed anymore.
    if request.target == RequestTarget::Asterisk {
        return Response::with_status_and_string_body(StatusCode::BadRequest, "Invalid Target");
    }

    let controller = config.handler_controller.clone();
    if let Some(result) = controller.check_handle(request) {
        return match result {
            Ok(res) => res,
            Err(e) => {
                #[cfg(feature = "debugging")]
                println!("[HTTP] Failed to invoke handler: {:#?}", e);
                _ = e;

                let mut response = Response::with_status_and_string_body(StatusCode::InternalServerError, "Internal Server Error");
                finish_response_general(&mut response).await;
                response
            }
        };
    }

    if let RequestTarget::Origin { path, .. } = &request.target {
        let request_target = path.as_str();
        if request.method != Method::Get {
            let mut response = Response::with_status_and_string_body(StatusCode::MethodNotAllowed, "Method Not Allowed");
            response.headers.set(HeaderName::Allow, "GET".into());
            return response;
        }

        let Ok(current_directory) = current_dir() else {
            return handle_welcome_page(request, request_target).await;
        };

        let root = current_directory.join("wwwroot");
        let Ok(url_decoded) = urlencoding::decode(&request_target[1..]) else {
            return Response::with_status_and_string_body(StatusCode::BadRequest, "Bad Request");
        };

        let path = root.join(url_decoded.into_owned());
        if !path.starts_with(&root) {
            return Response::with_status_and_string_body(StatusCode::Forbidden, format!("Forbidden\n{}\n{}", root.display(), path.display()));
        }

        for component in path.components() {
            if let std::path::Component::ParentDir = component {
                return Response::with_status_and_string_body(StatusCode::Forbidden, "Forbidden");
            }
        }

        if let Some(served_file_response) = serve_file(request, &path).await {
            return served_file_response;
        };

        if let Ok(metadata) = std::fs::metadata(&path) {
            if metadata.is_dir() {
                let path = path.join("index.html");
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if metadata.is_file() {
                        if let Some(served_file_response) = serve_file(request, &path).await {
                            return served_file_response;
                        }
                    }
                }
            }
        }

        if !root.join("/index.html").exists() {
            return handle_welcome_page(request, request_target).await;
        }

        return Response::with_status_and_string_body(StatusCode::NotFound, "Not Found");
    }

    Response::with_status_and_string_body(StatusCode::BadRequest, "Invalid Target")
}

/// Serves the welcome page to the client if the `wwwroot/index.html` file does
/// not exist.
async fn handle_welcome_page(request: &Request, request_target: &str) -> Response {
    if !request.headers.contains(&HeaderName::ETag) {
        if let Some(modified_since) = request.headers.get(&HeaderName::IfModifiedSince) {
            if let Some(modified_since) = modified_since.as_str_no_convert() {
                if let Ok(modified_since) = httpdate::parse_http_date(modified_since) {
                    if let Ok(duration) = SystemTime::UNIX_EPOCH.duration_since(modified_since) {
                        if duration.as_secs() < 600 {
                            return serve_welcome_page_not_modified(request);
                        }
                    }
                }
            }
        }
    }

    let mut response = Response::with_status(StatusCode::Ok);
    response.headers.set_content_type(MediaType::HTML);
    response.headers.set(HeaderName::CacheControl, "public, max-age=600".into());
    response.headers.set(HeaderName::ContentSecurityPolicy, "default-src 'self'; upgrade-insecure-requests; style-src-elem 'self' 'unsafe-inline'".into());

    response.body = Some(BodyKind::StaticString(static_res::WELCOME_HTML));
    response.headers.set(HeaderName::ContentLanguage, "en".into());
    response.headers.set(HeaderName::LastModified, HeaderValue::from(SystemTime::UNIX_EPOCH));
    response.headers.set(HeaderName::ETag, "welcome-en".into());
    response.headers.set(HeaderName::Vary, "Content-Language".into());

    let request_etag = request.headers.get(&HeaderName::IfNoneMatch).map(|etag| etag.as_str_no_convert().unwrap());

    match request_target {
        "/" | "/index" | "/index.html" => {
            if let Some(accepted_languages) = request.headers.get(&HeaderName::AcceptLanguage) {
                if let Some(accepted_languages) = AcceptedLanguages::parse(accepted_languages.as_str_no_convert().unwrap()) {
                    if let Some(best) = accepted_languages.match_best(vec!["nl", "en"]) {
                        if best == "nl" {
                            response.body = Some(BodyKind::StaticString(static_res::WELCOME_HTML_NL));
                            response.headers.set(HeaderName::ContentLanguage, "nl".into());
                            response.headers.set(HeaderName::ETag, "welcome-nl".into());
                            if request_etag == Some("welcome-nl") {
                                return serve_welcome_page_not_modified(request);
                            }
                        } else if request_etag == Some("welcome-en") {
                            return serve_welcome_page_not_modified(request);
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
        _ => return Response::with_status_and_string_body(StatusCode::NotFound, "Not Found"),
    }

    response
}

async fn serve_file(request: &Request, path: &Path) -> Option<Response> {
    if let Some(response) = serve_file_from_cache(request, path) {
        return Some(response);
    }

    serve_file_from_disk(path).await
}

/// Serves a file from the cache if it is available.
async fn serve_file_from_disk(path: &Path) -> Option<Response> {
    // Check if the file is allowed to be served. The cache already checked
    // this, but we need to check it again for files that are not cached.
    if !resources::is_file_allowed_to_be_served(path.to_string_lossy().as_ref()) {
        return None;
    }

    let Ok(file) = tokio::fs::File::open(path).await else {
        return None;
    };

    let Ok(metadata) = file.metadata().await else {
        return None;
    };

    if !metadata.is_file() {
        return None;
    }

    cache::maybe_cache_file(path).await;

    let mut response = Response::with_status(StatusCode::Ok);

    if let Ok(modified_date) = metadata.modified() {
        response.headers.set_last_modified(modified_date);
    }

    response.body = Some(BodyKind::File { handle: file, metadata });
    response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));

    Some(response)
}

/// Serves a file from the cache if it is available.
fn serve_file_from_cache(request: &Request, path: &Path) -> Option<Response> {
    let Some(cached) = cache::FILE_CACHE.get(path.to_string_lossy().as_ref()) else {
        return None
    };

    let cached = match &cached.value().cache_details {
        Some(CachedFileDetails::Markdown { html_rendered }) => Arc::clone(html_rendered),
        _ => Arc::clone(cached.value())
    };

    if let Some(modified_date) = cached.modified_date {
        if let Some(not_modified_response) = check_not_modified(request, path, modified_date) {
            return Some(not_modified_response);
        }
    }

    let mut response = Response::with_status(StatusCode::Ok);

    let encoding = if let Some(accept_encoding) = request.headers.get(&HeaderName::AcceptEncoding) {
        if let Some(accept_encoding) = accept_encoding.as_str_no_convert() {
            cached.determine_best_version_from_accept_encoding(accept_encoding)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(encoding) = encoding {
        response.headers.set(HeaderName::ContentEncoding, encoding.into());
    }

    if let Some(media_type) = cached.media_type.clone() {
        response.headers.set(HeaderName::ContentType, HeaderValue::from(media_type));
    } else {
        response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));
    }

    response.headers.set(HeaderName::CacheStatus, "ServenteCache; hit; detail=MEMORY".into());
    if let Some(modified_date) = cached.modified_date {
        response.headers.set_last_modified(modified_date);
    }

    if let Some(CachedFileDetails::Document { link_preloads }) = &cached.cache_details{
        for link_preload in link_preloads {
            response.headers.append_possible_duplicate(HeaderName::Link, link_preload.clone().into());
        }
    }

    response.body = Some(BodyKind::CachedBytes(cached, encoding));

    Some(response)
}

/// Serve the welcome page response with a 304 Not Modified status code.
fn serve_welcome_page_not_modified(request: &Request) -> Response {
    let mut response = Response::with_status(StatusCode::NotModified);
    response.headers.set(HeaderName::Vary, "Content-Language".into());

    if let Some(etag) = request.headers.get(&HeaderName::ETag) {
        response.headers.set(HeaderName::ETag, etag.clone());
    }

    if let Some(if_modified_since) = request.headers.get(&HeaderName::ETag) {
        response.headers.set(HeaderName::LastModified, if_modified_since.clone());
    }

    response
}

fn validate_token(value: &str) -> Result<(), HttpParseError> {
    if value.is_empty() {
        return Err(HttpParseError::TokenEmpty);
    }

    for character in value.bytes() {
        validate_token_character(character)?;
    }

    Ok(())
}

/// Validate a token character.
///
/// ```text
/// tchar          = "!" / "#" / "$" / "%" / "&" / "'" / "*"
///                / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
///                / DIGIT / ALPHA
///                ; any VCHAR, except delimiters
/// ```
fn validate_token_character(character: u8) -> Result<(), HttpParseError> {
    match character {
        b' ' | b'\t' => Err(HttpParseError::TokenContainsWhitespace),

        b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' |
        b'^' | b'_' | b'`' | b'|' | b'~' => Ok(()),

        b'0'..=b'9' => Ok(()),
        b'A'..=b'Z' => Ok(()),
        b'a'..=b'z' => Ok(()),

        b'"' | b'(' | b')' | b',' | b'/' | b':' | b';' | b'<' | b'=' | b'>' |
        b'?' | b'@' | b'[' | b'\\' | b']' | b'{' | b'}' => Err(HttpParseError::TokenContainsDelimiter),

        _ => Err(HttpParseError::TokenContainsNonVisibleAscii),
    }
}
