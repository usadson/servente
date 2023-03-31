// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use strum_macros::AsRefStr;

use std::io;

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

/// An error that can occur while parsing an HTTP request.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, AsRefStr)]
pub enum HttpParseError {
    /// The header didn't contain a colon, it's only the name.
    ///
    /// ## Example:
    /// ```text
    /// Content-Type
    /// ```
    HeaderDoesNotContainColon,

    /// The header (name + value) was too large.
    ///
    /// ## Example:
    /// ```
    /// // Maximum = 256, length = 300
    /// ```
    /// ```text
    /// Content-Type: aaaa...aaaa
    /// ```
    HeaderTooLarge,

    /// The `Content-Length` field was malformed, meaning it contained non-numeric
    /// characters, was too large, was negative, or was the empty string.
    ///
    /// ## Example:
    /// ```text
    /// Content-Length: 123abc
    /// ```
    InvalidContentLength,

    /// The line ended with CR but not followed by an LF.
    ///
    /// ## Example:
    /// ```text
    /// Content-Length: 123\r
    /// ```
    InvalidCRLF,

    /// The HTTP version was invalid.
    ///
    /// ## Sytax
    /// The HTTP version must be in the format `HTTP/<digit>.<digit>`, where
    /// `<digit>` is a single digit (0 - 9).
    ///
    /// ## HTTP/1.1 (TCP)
    /// On connections using TCP, only the following are valid:
    /// * `HTTP/1.0`
    /// * `HTTP/1.1`
    /// * `HTTP/2.0` - for HTTP/2 Upgrading
    ///
    /// ## Examples:
    /// ```text
    /// HTTP/1.
    /// REST/1.1
    /// HTTP/1.1.1
    /// H/1.1
    /// ```
    InvalidHttpVersion,

    /// The request-target format is unknown.
    ///
    /// ## Syntax
    /// The request-target can be one of the following:
    /// * `*` - for OPTIONS requests
    /// * `origin-form` - for all other requests
    /// * `absolute-form` - for CONNECT requests
    /// * `authority-form` - for CONNECT requests
    ///
    /// ## Examples:
    /// ```text
    /// GET not-beginning-with-a-solidus HTTP/1.1
    /// OPTIONS *** HTTP/1.1
    /// GET ?query=string HTTP/1.1
    /// ```
    InvalidRequestTarget,

    /// The method was too large.
    ///
    /// ## Example:
    /// ```
    /// // Maximum = 16, length = 53
    /// ```
    /// ```text
    /// THIS-IS-A-VERY-LONG-METHOD-CONTAINING-MANY-CHARACTERS / HTTP/1.1
    /// ```
    MethodTooLarge,

    /// The request-target (e.g. URI) was too large.
    ///
    /// ## Example:
    /// ```
    /// // Maximum = 2048, length = 3000
    /// ```
    /// ```text
    /// GET /this-is-a-very-long-request-target-containing-many-characters[...] HTTP/1.1
    /// ```
    RequestTargetTooLarge,

    TokenContainsDelimiter,
    TokenContainsNonVisibleAscii,
    TokenContainsWhitespace,
    TokenEmpty,

    FieldValueContainsInvalidCharacters,

    InvalidOctetInMethod,
    InvalidOctetInRequestTarget,

    InvalidHttp2PriUpgradeBody,
}
