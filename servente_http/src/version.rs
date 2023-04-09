// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HttpVersion {
    Http09,
    Http10,
    Http11,
    Http2,
    Http3,
}

impl HttpVersion {
    /// Formats the [`HttpVersion`] to a HTTP-Version, as specified by RFC 9112.
    ///
    /// # References
    /// * [RFC 3875 Section 4.1.16](https://www.rfc-editor.org/rfc/rfc3875.html#section-4.1.16)
    /// * [RFC 9112 Section 2.3](https://www.rfc-editor.org/rfc/rfc9112.html#name-http-version)
    pub fn to_http_version(&self) -> &'static str {
        match self {
            Self::Http09 => "HTTP/0.9",
            Self::Http10 => "HTTP/1.0",
            Self::Http11 => "HTTP/1.1",
            Self::Http2 => "HTTP/2.0",
            Self::Http3 => "HTTP/3.0",
        }
    }
}
