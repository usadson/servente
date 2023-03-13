// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

/// An error that can occur while parsing an HTTP request.
#[derive(Debug)]
pub enum HttpParseError {
    /// The header (name + value) was too large.
    HeaderTooLarge,

    /// The `Content-Length` field was malformed.
    InvalidContentLength,

    /// The line ended with CR but not followed by an LF.
    InvalidCRLF,

    /// The HTTP version was invalid.
    InvalidHttpVersion,

    /// The request-target format is unknown.
    InvalidRequestTarget,

    /// The method was too large.
    MethodTooLarge,

    /// The request-target (e.g. URI) was too large.
    RequestTargetTooLarge,
}
