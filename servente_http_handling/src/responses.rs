// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! This module creates complete responses for handling common cases.

use servente_http::{Response, StatusCode};
use servente_resources::MediaType;

/// Create a response for when the request times out.
pub async fn create_request_timeout() -> Response {
    let mut response = Response::with_status_and_string_body(StatusCode::RequestTimeout, "Request Timed Out");
    response.headers.set_content_type(MediaType::PLAIN_TEXT);

    super::finish_response_error(&mut response).await;

    response
}
