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
        cache::{
            self,
            CacheDetails,
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
async fn check_not_modified(request: &Request, path: &Path, modified_date: SystemTime) -> Result<Option<Response>, io::Error> {
    if let Some(etag) = request.headers.get(&HeaderName::IfNoneMatch) {
        if etag.as_str_no_convert().unwrap() == format_system_time_as_weak_etag(modified_date) {
            let mut response = Response::with_status_and_string_body(StatusCode::NotModified, String::new());
            response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));
            response.headers.set(HeaderName::ETag, etag.clone());
            return Ok(Some(response));
        }
    }

    if let Some(if_modified_since) = request.headers.get(&HeaderName::IfModifiedSince) {
        if let Ok(if_modified_since_date) = if_modified_since.try_into() {
            if let Ok(duration) = modified_date.duration_since(if_modified_since_date) {
                if duration.as_secs() == 0 {
                    let mut response = Response::with_status_and_string_body(StatusCode::NotModified, String::new());
                    response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));
                    response.headers.set(HeaderName::LastModified, if_modified_since.to_owned());
                    return Ok(Some(response));
                }
            }

            // The file modified time is somehow earlier than
            // If-Modified-Date, but that's okay, since the normal file
            // handler will handle it.
        }
    }

    Ok(None)
}

/// Finishes a response for an error response.
pub async fn finish_response_error(response: &mut Response) -> Result<(), io::Error> {
    response.headers.set(HeaderName::Connection, HeaderValue::from("close"));
    finish_response_general(response).await
}

/// Finishes a response for both normal and error response.
async fn finish_response_general(response: &mut Response) -> Result<(), io::Error> {
    if let Some(body) = &response.body {
        if !response.headers.contains(&HeaderName::LastModified) {
            if let BodyKind::File(file) = body {
                if let Ok(metadata) = file.metadata().await {
                    if let Ok(modified_date) = metadata.modified() {
                        response.headers.set_last_modified(modified_date);
                    }
                }
            }
        }
    }

    response.headers.set(HeaderName::Server, HeaderValue::from("servente"));
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

    Ok(())
}

/// Finishes a response for a normal (OK) response.
pub async fn finish_response_normal(request: &Request, response: &mut Response) -> Result<(), io::Error> {
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

/// Handles a `HttpParseError`.
pub async fn handle_parse_error(error: HttpParseError) -> Response {
    match error {
        HttpParseError::HeaderDoesNotContainColon => Response::bad_request("Invalid header format"),
        HttpParseError::HeaderTooLarge => Response::with_status_and_string_body(StatusCode::RequestHeaderFieldsTooLarge, "Request Header Fields Too Large"),
        HttpParseError::InvalidContentLength => Response::bad_request("Malformed Content-Length"),
        HttpParseError::InvalidCRLF => Response::bad_request("Invalid CRLF"),
        HttpParseError::InvalidHttpVersion => Response::with_status_and_string_body(StatusCode::HTTPVersionNotSupported, "Invalid HTTP version"),
        HttpParseError::InvalidRequestTarget => Response::bad_request("Invalid request target"),
        HttpParseError::MethodTooLarge => Response::with_status_and_string_body(StatusCode::MethodNotAllowed, "Method Not Allowed"),
        HttpParseError::RequestTargetTooLarge => Response::with_status_and_string_body(StatusCode::URITooLong, "Invalid request target"),
    }
}

/// Handles a request.
pub async fn handle_request(request: &Request, config: &ServenteConfig) -> Result<Response, Error> {
    let controller = config.handler_controller.clone();
    if let Some(result) = controller.check_handle(request) {
        return result.map_err(|error| Error::Other(io::Error::new(io::ErrorKind::Other, error)));
    }

    if let RequestTarget::Origin { path, .. } = &request.target {
        let request_target = path.as_str();
        if request.method != Method::Get {
            return Ok(Response::with_status_and_string_body(StatusCode::MethodNotAllowed, "Method Not Allowed"));
        }

        let Ok(current_directory) = current_dir() else {
            return handle_welcome_page(request, request_target).await;
        };

        let root = current_directory.join("wwwroot/");
        let Ok(url_decoded) = urlencoding::decode(&request_target[1..]) else {
            return Ok(Response::with_status_and_string_body(StatusCode::BadRequest, "Bad Request"));
        };

        let path = root.join(url_decoded.into_owned());
        if !path.starts_with(&root) {
            return Ok(Response::with_status_and_string_body(StatusCode::Forbidden, format!("Forbidden\n{}\n{}", root.display(), path.display())));
        }

        if let Some(served_file_response) = serve_file(request, &path).await? {
            return Ok(served_file_response);
        };

        if let Ok(metadata) = std::fs::metadata(&path) {
            if metadata.is_dir() {
                let path = path.join("index.html");
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if metadata.is_file() {
                        drop(metadata);

                        if let Some(served_file_response) = serve_file(request, &path).await? {
                            return Ok(served_file_response);
                        }
                    }
                }
            }
        }

        if !root.join("/index.html").exists() {
            return handle_welcome_page(request, request_target).await;
        }

        return Ok(Response::with_status_and_string_body(StatusCode::NotFound, "Not Found"));
    }

    Ok(Response::with_status_and_string_body(StatusCode::BadRequest, "Invalid Target"))
}

/// Serves the welcome page to the client if the `wwwroot/index.html` file does
/// not exist.
async fn handle_welcome_page(request: &Request, request_target: &str) -> Result<Response, Error> {
    if !request.headers.contains(&HeaderName::ETag) {
        if let Some(modified_since) = request.headers.get(&HeaderName::IfModifiedSince) {
            if let Some(modified_since) = modified_since.as_str_no_convert() {
                if let Ok(modified_since) = httpdate::parse_http_date(modified_since) {
                    if let Ok(duration) = SystemTime::UNIX_EPOCH.duration_since(modified_since) {
                        if duration.as_secs() < 600 {
                            return Ok(serve_welcome_page_not_modified(request));
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
                                return Ok(serve_welcome_page_not_modified(request));
                            }
                        } else if request_etag == Some("welcome-en") {
                            return Ok(serve_welcome_page_not_modified(request));
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

    Ok(response)
}

async fn serve_file(request: &Request, path: &Path) -> Result<Option<Response>, Error> {
    if let Some(cached) = cache::FILE_CACHE.get(path.to_string_lossy().as_ref()) {
        if let Some(modified_date) = cached.value().modified_date {
            if let Some(not_modified_response) = check_not_modified(request, path, modified_date).await? {
                return Ok(Some(not_modified_response));
            }
        }

        let mut response = Response::with_status(StatusCode::Ok);

        let encoding = if let Some(accept_encoding) = request.headers.get(&HeaderName::AcceptEncoding) {
            if let Some(accept_encoding) = accept_encoding.as_str_no_convert() {
                cached.value().determine_best_version_from_accept_encoding(accept_encoding)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(encoding) = encoding {
            response.headers.set(HeaderName::ContentEncoding, encoding.into());
        }

        response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));
        response.headers.set(HeaderName::CacheStatus, "ServenteCache; hit; detail=MEMORY".into());
        response.body = Some(BodyKind::CachedBytes(Arc::clone(cached.value()), encoding));
        if let Some(modified_date) = cached.value().modified_date {
            response.headers.set_last_modified(modified_date);
        }

        if let Some(CacheDetails::Document { link_preloads }) = &cached.value().cache_details{
            for link_preload in link_preloads {
                response.headers.append_possible_duplicate(HeaderName::Link, link_preload.clone().into());
            }
        }

        return Ok(Some(response));
    }

    let Ok(file) = tokio::fs::File::open(path).await else {
        return Ok(None);
    };

    let Ok(metadata) = file.metadata().await else {
        return Ok(None);
    };

    if !metadata.is_file() {
        return Ok(None);
    }

    cache::maybe_cache_file(path).await;

    let mut response = Response::with_status(StatusCode::Ok);
    response.body = Some(BodyKind::File(file));

    if let Ok(modified_date) = metadata.modified() {
        response.headers.set_last_modified(modified_date);
    }

    response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));

    Ok(Some(response))
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
