// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

pub mod config;
pub mod handler;
pub mod middleware;
pub mod responses;

use std::path::PathBuf;
use std::{
    path::Path,
    sync::Arc,
    time::SystemTime, env::current_dir,
};

use middleware::ExchangeState;
use servente_http::{HttpParseError, lists::find_best_match_in_weighted_list};

use servente_http::*;
use servente_resources::{MediaType, static_resources, CachedFileDetails, cache};

pub use config::{
    ServenteConfig,
    ServenteSettings,
};

pub use middleware::Middleware;

/// Checks if the request is not modified and returns a 304 response if it isn't.
fn check_not_modified(request: &Request, path: &Path, modified_date: SystemTime) -> Option<Response> {
    if let Some(etag) = request.headers.get(&HeaderName::IfNoneMatch) {
        if let Some(etag_as_str) = etag.as_str_no_convert() {
            if etag_as_str == format_system_time_as_weak_etag(modified_date) {
                let mut response = Response::with_status_and_string_body(StatusCode::NotModified, String::new());
                response.headers.append_or_override(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));
                response.headers.append_or_override(HeaderName::ETag, etag.clone());
                return Some(response);
            }
        }
    }

    if let Some(if_modified_since) = request.headers.get(&HeaderName::IfModifiedSince) {
        if let Ok(if_modified_since_date) = if_modified_since.try_into() {
            if let Ok(duration) = modified_date.duration_since(if_modified_since_date) {
                if duration.as_secs() == 0 {
                    let mut response = Response::with_status_and_string_body(StatusCode::NotModified, String::new());
                    response.headers.append_or_override(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));
                    response.headers.append_or_override(HeaderName::LastModified, if_modified_since.to_owned());
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

/// Finds the file as provided by the request path in the specified `wwwroot`.
///
/// This function also validates the contents of the request path, hence
/// returning an error of type [`Response`] when it occurs.
pub fn find_request_path_in_wwwroot(root: &Path, request_target: &str) -> Result<PathBuf, Response> {
    let Ok(url_decoded) = urlencoding::decode(&request_target[1..]) else {
        return Err(Response::with_status_and_string_body(StatusCode::BadRequest, "Bad Request"));
    };

    let path = root.join(url_decoded.into_owned());
    if !path.starts_with(&root) {
        return Err(Response::with_status_and_string_body(StatusCode::Forbidden, format!("Forbidden\n{}\n{}", root.display(), path.display())));
    }

    for component in path.components() {
        if let std::path::Component::ParentDir = component {
            return Err(Response::with_status_and_string_body(StatusCode::Forbidden, "Forbidden"));
        }
    }

    Ok(path)
}

/// Finishes a response for an error response.
pub async fn finish_response_error(response: &mut Response) {
    response.headers.append_or_override(HeaderName::Connection, HeaderValue::from("close"));
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

    response.headers.append_or_override(HeaderName::Server, HeaderValue::from("servente"));

    #[cfg(feature = "http2")]
    { _ = response.headers.append(HeaderName::AltSvc, HeaderValue::from("h2=\":8080\"")) }

    #[cfg(feature = "http3")]
    { _ = response.headers.append(HeaderName::AltSvc, HeaderValue::from("h3=\":8080\"")) }

    response.headers.append_or_override(HeaderName::XFrameOptions, "DENY".into());
    response.headers.append_or_override(HeaderName::XXSSProtection, "X-XSS-Protection: 1; mode=block".into());
    response.headers.append_or_override(HeaderName::XContentTypeOptions, "nosniff".into());

    if !response.headers.contains(&HeaderName::Connection) {
        _ = response.headers.append(HeaderName::Connection, HeaderValue::from("keep-alive"));
    }

    if !response.headers.contains(&HeaderName::Date) {
        response.headers.append_or_override(HeaderName::Date, SystemTime::now().into());
    }
}

/// Finishes a response for a normal (OK) response.
pub async fn finish_response_normal(request: &Request, response: &mut Response) {
    if response.body.is_some() {
        if !response.headers.contains(&HeaderName::ContentType) {
            response.headers.append_or_override(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(request.target.as_str()).clone()));
        }

        if response.status.class() == StatusCodeClass::Success && !response.headers.contains(&HeaderName::CacheControl) {
            _ = response.headers.append(HeaderName::CacheControl, HeaderValue::from("max-age=120"));
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
async fn handle_options(request: &Request, settings: &ServenteSettings) -> Response {
    if request.target == RequestTarget::Asterisk {
        return handle_options_asterisk();
    }

    if let Some(response) = settings.handler_controller.check_handle_options(request) {
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
    _ = response.headers.append(HeaderName::Allow, "GET, HEAD, OPTIONS, POST".into());
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
    response.headers.append_or_override(HeaderName::ContentType, HeaderValue::from(MediaType::HTML));
    response
}

/// Handles a request.
pub async fn handle_request(request: &Request, settings: &ServenteSettings) -> Response {
    let mut exchange_state = ExchangeState {
        request,
        response: handle_request_inner(request, settings).await,
    };

    for middleware in &settings.middleware {
        let mut middleware = Arc::clone(middleware);
        let middleware = dyn_clone::arc_make_mut(&mut middleware);

        if let Err(e) = middleware.invoke(&mut exchange_state).await {
            #[cfg(debug_assertions)]
            match e {
                middleware::MiddlewareError::RecoverableError(e) => {
                    println!("[Middleware] Recoverable error in middleware occurred: {}", e);
                }

                middleware::MiddlewareError::UnrecoverableError(e) => {
                    let mut response = Response::with_status_and_string_body(StatusCode::ServiceUnavailable,
                        format!(
                            concat!(
                                "<h1>Service Unavailable</h1>",
                                "<hr>",
                                "<p>An internal error occurred whilst processing your request in middleware: <b>{}</b></p>",
                                "<h2>Error Information</h2>",
                                "<pre>{}</pre>"
                            ),
                            middleware.debug_identifier(),
                            e
                        )
                    );
                    response.headers.set_content_type(MediaType::HTML);
                    return response;
                }
            }

            #[cfg(not(debug_assertions))]
            match e {
                middleware::MiddlewareError::RecoverableError(_) => (),
                middleware::MiddlewareError::UnrecoverableError(_) => {
                    return Response::with_status_and_string_body(StatusCode::ServiceUnavailable, "Service Unavailable");
                }
            }
        }
    }

    exchange_state.response
}

async fn handle_request_inner(request: &Request, settings: &ServenteSettings) -> Response {
    if request.method == Method::Options {
        return handle_options(request, settings).await;
    }

    // Method is not OPTIONS, so a request-target of "*" is not allowed anymore.
    if request.target == RequestTarget::Asterisk {
        return Response::with_status_and_string_body(StatusCode::BadRequest, "Invalid Target");
    }

    let controller = settings.handler_controller.clone();
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
            _ = response.headers.append(HeaderName::Allow, "GET".into());
            return response;
        }

        let Ok(current_directory) = current_dir() else {
            return handle_welcome_page(request, request_target).await;
        };

        let root = current_directory.join("wwwroot");
        let path = match find_request_path_in_wwwroot(&root, request_target) {
            Ok(path) => path,
            Err(response) => return response,
        };

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
    _ = response.headers.append(HeaderName::CacheControl, "public, max-age=600".into());
    _ = response.headers.append(HeaderName::ContentSecurityPolicy, "default-src 'self'; upgrade-insecure-requests; style-src-elem 'self' 'unsafe-inline'".into());

    response.body = Some(BodyKind::StaticString(static_resources::WELCOME_HTML));
    _ = response.headers.append(HeaderName::ContentLanguage, "en".into());
    response.headers.append_or_override(HeaderName::LastModified, HeaderValue::from(SystemTime::UNIX_EPOCH));
    response.headers.append_or_override(HeaderName::ETag, "welcome-en".into());
    _ = response.headers.append(HeaderName::Vary, "Content-Language".into());

    let request_etag = if let Some(etag) = request.headers.get(&HeaderName::IfNoneMatch) {
        etag.as_str_no_convert()
    } else {
        None
    };

    match request_target {
        "/" | "/index" | "/index.html" => {
            if let Some(accepted_languages) = request.headers.get(&HeaderName::AcceptLanguage) {
                if let Some(accepted_languages) = accepted_languages.as_str_no_convert() {
                    match find_best_match_in_weighted_list(accepted_languages, &["nl", "en"], 0.0) {
                        Some(0) => {
                            response.body = Some(BodyKind::StaticString(static_resources::WELCOME_HTML_NL));
                            response.headers.append_or_override(HeaderName::ContentLanguage, "nl".into());
                            response.headers.append_or_override(HeaderName::ETag, "welcome-nl".into());
                            if request_etag == Some("welcome-nl") {
                                return serve_welcome_page_not_modified(request);
                            }
                        }
                        _ => if request_etag == Some("welcome-en") {
                            return serve_welcome_page_not_modified(request);
                        },
                    }
                }
            }
        }
        "/welcome.en.html" => (),
        "/welcome.nl.html" => {
            response.body = Some(BodyKind::StaticString(static_resources::WELCOME_HTML_NL));
            response.headers.append_or_override(HeaderName::ContentLanguage, "nl".into());
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
    if !servente_resources::is_file_allowed_to_be_served(path.to_string_lossy().as_ref()) {
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

    #[cfg(unix)]
    {
        use servente_resources::fs::PermissionsExt;

        // Executable files are disallowed from being served, and can only be
        // accessed through systems like CGI.
        if metadata.permissions().is_executable() {
            return None;
        }
    }

    cache::maybe_cache_file(path).await;

    let mut response = Response::with_status(StatusCode::Ok);

    if let Ok(modified_date) = metadata.modified() {
        response.headers.set_last_modified(modified_date);
    }

    response.body = Some(BodyKind::File { handle: file, metadata });
    response.headers.append_or_override(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));

    Some(response)
}

/// Serves a file from the cache if it is available.
fn serve_file_from_cache(request: &Request, path: &Path) -> Option<Response> {
    let Some(cached) = cache::FILE_CACHE.get(path.to_string_lossy().as_ref()) else {
        return None
    };

    #[cfg(feature = "convert-markdown")]
    let cached = match &cached.value().cache_details {
        Some(CachedFileDetails::Markdown { html_rendered }) => Arc::clone(html_rendered),
        _ => Arc::clone(cached.value())
    };

    #[cfg(not(feature = "convert-markdown"))]
    let cached = Arc::clone(cached.value());

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
        response.headers.append_or_override(HeaderName::ContentEncoding, encoding.into());
    }

    if let Some(media_type) = cached.media_type.clone() {
        response.headers.append_or_override(HeaderName::ContentType, HeaderValue::from(media_type));
    } else {
        response.headers.append_or_override(HeaderName::ContentType, HeaderValue::from(MediaType::from_path(path.to_string_lossy().as_ref()).clone()));
    }

    response.headers.append_or_override(HeaderName::CacheStatus, "ServenteCache; hit; detail=MEMORY".into());
    if let Some(modified_date) = cached.modified_date {
        response.headers.set_last_modified(modified_date);
    }

    if let Some(CachedFileDetails::Document { link_preloads }) = &cached.cache_details{
        for link_preload in link_preloads {
            _ = response.headers.append(HeaderName::Link, link_preload.clone().into());
        }
    }

    response.body = Some(BodyKind::CachedBytes(cached, encoding));

    Some(response)
}

/// Serve the welcome page response with a 304 Not Modified status code.
fn serve_welcome_page_not_modified(request: &Request) -> Response {
    let mut response = Response::with_status(StatusCode::NotModified);
    response.headers.append_or_override(HeaderName::Vary, "Content-Language".into());

    if let Some(etag) = request.headers.get(&HeaderName::ETag) {
        response.headers.append_or_override(HeaderName::ETag, etag.clone());
    }

    if let Some(if_modified_since) = request.headers.get(&HeaderName::ETag) {
        response.headers.append_or_override(HeaderName::LastModified, if_modified_since.clone());
    }

    response
}
