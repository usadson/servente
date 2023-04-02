// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::borrow::Cow;

use servente_resources::MediaType;

use crate::{
    BodyKind,
    HeaderMap,
    HeaderName,
    HeaderValue,
    HttpVersion,
    StatusCode,
};

#[derive(Debug)]
pub struct Response {
    /// Responses that are sent before this one, commonly 1xx response.
    /// E.g. 103 Early Hints.
    pub prelude_response: Vec<Response>,
    pub version: HttpVersion,
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Option<BodyKind>,
}

impl Response {
    pub fn with_status(status: StatusCode) -> Self {
        Self {
            prelude_response: Vec::new(),
            version: HttpVersion::Http11,
            status,
            headers: HeaderMap::new(),
            body: None,
        }
    }

    pub fn with_status_and_string_body(status: StatusCode, body: impl Into<Cow<'static, str>>) -> Self {
        let mut headers = HeaderMap::new();
        headers.append_or_override(HeaderName::ContentType, HeaderValue::from(MediaType::PLAIN_TEXT));
        Self {
            prelude_response: Vec::new(),
            version: HttpVersion::Http11,
            status,
            headers,
            body: match body.into() {
                Cow::Owned(body) => Some(BodyKind::String(body)),
                Cow::Borrowed(body) => Some(BodyKind::StaticString(body)),
            },
        }
    }

    pub fn bad_request(message: &'static str) -> Self {
        let mut response = Self::with_status(StatusCode::BadRequest);
        response.body = Some(BodyKind::StaticString(message));
        response
    }

    pub fn forbidden(message: &'static str) -> Self {
        let mut response = Self::with_status(StatusCode::Forbidden);
        response.body = Some(BodyKind::StaticString(message));
        response
    }

    pub fn not_found(message: &'static str) -> Self {
        let mut response = Self::with_status(StatusCode::NotFound);
        response.body = Some(BodyKind::StaticString(message));
        response
    }
}
