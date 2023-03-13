// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{
    fs,
    io::{
        self,
    },
    path::Path,
    sync::Arc,
    time::{SystemTime, Duration}, env::current_dir,
};

use lazy_static::lazy_static;
use stretto::AsyncCache;
use tokio::io::AsyncReadExt;

use crate::{resources::{MediaType, static_res}, ServenteConfig};

use self::{
    error::HttpParseError,
    message::{
        BodyKind,
        HeaderName,
        HeaderValue,
        Request,
        Response,
        StatusCode,
        StatusCodeClass, HeaderMap, RequestTarget, Method,
    }, hints::{AcceptedLanguages, SecFetchDest},
};

pub mod error;
pub mod hints;
pub mod message;
pub mod v1;

#[cfg(feature = "http3")]
pub mod v3;

/// The maximum size of a file that can be cached in memory.
const FILE_CACHE_MAXIMUM_SIZE: u64 = 50_000_000; // 50 MB

lazy_static! {
    static ref FILE_CACHE: AsyncCache<String, Arc<Vec<u8>>> = AsyncCache::new(12960, 1e6 as i64, tokio::spawn).unwrap();
}

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
async fn check_not_modified(request: &Request, path: &Path, metadata: &fs::Metadata) -> Result<Option<Response>, io::Error> {
    if let Some(if_modified_since) = request.headers.get(&HeaderName::IfModifiedSince) {
        if let Ok(modified_date) = metadata.modified() {
            if let Ok(if_modified_since_date) = if_modified_since.try_into() {
                match modified_date.duration_since(if_modified_since_date) {
                    Ok(duration) => {
                        if duration.as_secs() == 0 {
                            let mut response = Response::with_status_and_string_body(StatusCode::NotModified, String::new());
                            response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));
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

pub async fn finish_response_error(response: &mut Response) -> Result<(), io::Error>{
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

pub async fn finish_response_normal(request: &Request, response: &mut Response) -> Result<(), io::Error>{
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

pub async fn handle_parse_error(error: HttpParseError) -> Response {
    match error {
        HttpParseError::HeaderTooLarge => Response::with_status_and_string_body(StatusCode::RequestHeaderFieldsTooLarge, "Request Header Fields Too Large"),
        HttpParseError::InvalidContentLength => Response::bad_request("Malformed Content-Length"),
        HttpParseError::InvalidCRLF => Response::bad_request("Invalid CRLF"),
        HttpParseError::InvalidHttpVersion => Response::with_status_and_string_body(StatusCode::HTTPVersionNotSupported, "Invalid HTTP version"),
        HttpParseError::InvalidRequestTarget => Response::bad_request("Invalid request target"),
        HttpParseError::MethodTooLarge => Response::with_status_and_string_body(StatusCode::MethodNotAllowed, "Method Not Allowed"),
        HttpParseError::RequestTargetTooLarge => Response::with_status_and_string_body(StatusCode::URITooLong, "Invalid request target"),
    }
}

pub async fn handle_request(request: &Request, config: &ServenteConfig) -> Result<Response, Error> {
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
                return serve_file(request, &path, &metadata).await;
            }

            if metadata.is_dir() {
                let path = path.join("index.html");
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if metadata.is_file() {
                        return serve_file(request, &path, &metadata).await;
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

async fn maybe_cache_file(path: &Path) {
    let Ok(mut file) = tokio::fs::File::open(path).await else {
        return;
    };

    let path = path.to_owned();

    tokio::task::spawn(async move {
        let metadata = file.metadata().await.unwrap();

        if metadata.len() > FILE_CACHE_MAXIMUM_SIZE {
            return;
        }

        let mut data = Vec::with_capacity(metadata.len() as usize);
        _ = file.read_to_end(&mut data).await;

        FILE_CACHE.insert_with_ttl(path.to_str().unwrap().to_string(), Arc::new(data), 0, Duration::from_secs(60)).await;
    });
}

async fn serve_file(request: &Request, path: &Path, metadata: &fs::Metadata) -> Result<Response, Error> {
    if let Some(not_modified_response) = check_not_modified(request, &path, &metadata).await? {
        return Ok(not_modified_response);
    }

    if let Some(cached) = FILE_CACHE.get(path.to_string_lossy().as_ref()) {
        let mut response = Response::with_status(StatusCode::Ok);
        response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));
        response.headers.set(HeaderName::CacheStatus, "ServenteCache; hit; detail=MEMORY".into());
        response.body = Some(BodyKind::CachedBytes(Arc::clone(cached.value())));
        if let Ok(modified_date) = metadata.modified() {
            response.headers.set(HeaderName::LastModified, modified_date.into());
        }
        return Ok(response);
    }

    let file = tokio::fs::File::open(&path).await?;
    maybe_cache_file(&path).await;

    let mut response = Response::with_status(StatusCode::Ok);
    response.body = Some(BodyKind::File(file));

    if let Ok(modified_date) = metadata.modified() {
        response.headers.set(HeaderName::LastModified, modified_date.into());
    }

    response.headers.set(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));

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
    }

    return Ok(response);
}
