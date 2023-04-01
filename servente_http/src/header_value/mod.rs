// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

pub mod lists;
pub mod sec_fetch_dest;

pub use sec_fetch_dest::*;

use std::{time::SystemTime, sync::Arc};
use std::fmt::Write;

use servente_resources::MediaType;

use crate::{
    ContentCoding,
    ContentRangeHeaderValue,
};

/// Represents a value of a header.
///
/// This makes transforming the response easier for shared code paths, for
/// example when the header is used in multiple places, this avoids
/// serializing and deserializing which improves performance.
///
/// `HeaderValue` can also be used to restrict the types of setting various
/// `HeaderName`s. Most headers have a strict format, making them less
/// error-prone for the handler.
///
/// Another advantage is that we can have a simpler API to use, such as the
/// `SecFetchDest` enum.
///
/// At last, this removes the deserialization strain from the handler to
/// the transport code.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum HeaderValue {
    SharedString(Arc<str>),
    StaticString(&'static str),
    String(String),
    ContentCoding(ContentCoding),
    ContentRange(ContentRangeHeaderValue),
    DateTime(SystemTime),
    MediaType(MediaType),
    SecFetchDest(SecFetchDest),
    Size(usize),
}

impl HeaderValue {
    /// Returns the value as a string, but does not convert it to a string if
    /// it is some other non-convertible type.
    #[must_use]
    pub fn as_str_no_convert(&self) -> Option<&str> {
        match self {
            HeaderValue::StaticString(string) => Some(string),
            HeaderValue::SharedString(string) => Some(string.as_ref()),
            HeaderValue::String(string) => Some(string),
            _ => None,
        }
    }

    pub fn append_to_message(&self, response_text: &mut String) {
        match self {
            HeaderValue::SharedString(string) => {
                response_text.push_str(string);
            }
            HeaderValue::StaticString(string) => {
                response_text.push_str(string);
            }
            HeaderValue::String(string) => {
                response_text.push_str(string);
            }
            HeaderValue::ContentCoding(content_coding) => {
                response_text.push_str(content_coding.http_identifier());
            }
            HeaderValue::ContentRange(content_range) => {
                match content_range {
                    ContentRangeHeaderValue::Range { start, end, complete_length } => {
                        debug_assert!(start < end, "`start` must be less than `end` for Content-Range");
                        match complete_length {
                            Some(complete_length) => {
                                debug_assert!(end < complete_length, "`end` must be less than `complete_length` for Content-Range");
                                _ = write!(response_text, "bytes {start}-{end}/{complete_length}");
                            }
                            None => _ = write!(response_text, "bytes {start}-{end}/*"),
                        }
                    }
                    ContentRangeHeaderValue::Unsatisfied { complete_length } => {
                        _ = write!(response_text, "bytes */{}", *complete_length);
                    }
                };
            }
            HeaderValue::DateTime(date_time) => {
                _ = write!(response_text, "{}", httpdate::HttpDate::from(*date_time));
            }
            HeaderValue::MediaType(media_type) => {
                response_text.push_str(media_type.as_str());
            }
            HeaderValue::SecFetchDest(sec_fetch_dest) => {
                response_text.push_str(sec_fetch_dest.as_str());
            }
            HeaderValue::Size(size) => {
                _ = write!(response_text, "{size}");
            }
        }
    }

    /// Get the header in string form.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        self.append_to_message(&mut result);
        result
    }

    /// Parses the value as a number.
    #[must_use]
    pub fn parse_number(&self) -> Option<usize> {
        match self {
            HeaderValue::StaticString(string) => string.parse().ok(),
            HeaderValue::String(string) => string.parse().ok(),
            HeaderValue::Size(size) => Some(*size),
            _ => None,
        }
    }

    /// Calculate the length of the header value in string characters.
    pub fn string_length(&self) -> usize {
        // Fast path, when the type is a string, or can easily be mapped into
        // one:
        match self {
            Self::SharedString(str) => return str.len(),
            Self::StaticString(str) => return str.len(),
            Self::String(str) => return str.len(),
            Self::ContentCoding(coding) => return coding.http_identifier().len(),
            Self::ContentRange(_) => (),
            Self::DateTime(_) => (),
            Self::MediaType(media_type) => return media_type.as_str().len(),
            Self::SecFetchDest(sec_fetch_dest) => return sec_fetch_dest.as_str().len(),
            Self::Size(_) => (),
        }

        // Otherwise slow path, format it into a new string and get the length
        // of the string after formatting.

        let mut tmp_str = String::new();
        self.append_to_message(&mut tmp_str);
        tmp_str.len()
    }
}

impl From<Arc<str>> for HeaderValue {
    fn from(value: Arc<str>) -> Self {
        HeaderValue::SharedString(value)
    }
}

impl From<ContentCoding> for HeaderValue {
    fn from(content_coding: ContentCoding) -> HeaderValue {
        HeaderValue::ContentCoding(content_coding)
    }
}

impl From<&'static str> for HeaderValue {
    fn from(string: &'static str) -> HeaderValue {
        HeaderValue::StaticString(string)
    }
}

impl From<String> for HeaderValue {
    fn from(string: String) -> HeaderValue {
        HeaderValue::String(string)
    }
}

impl From<SystemTime> for HeaderValue {
    fn from(date_time: SystemTime) -> HeaderValue {
        HeaderValue::DateTime(date_time)
    }
}

impl From<MediaType> for HeaderValue {
    fn from(media_type: MediaType) -> HeaderValue {
        HeaderValue::MediaType(media_type)
    }
}

impl From<SecFetchDest> for HeaderValue {
    fn from(sec_fetch_dest: SecFetchDest) -> HeaderValue {
        HeaderValue::SecFetchDest(sec_fetch_dest)
    }
}

impl From<usize> for HeaderValue {
    fn from(size: usize) -> HeaderValue {
        HeaderValue::Size(size)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeaderValueDateTimeParseError {
    InvalidFormat,
}

impl TryInto<SystemTime> for &HeaderValue {
    type Error = HeaderValueDateTimeParseError;

    fn try_into(self) -> Result<SystemTime, Self::Error> {
        match self {
            HeaderValue::StaticString(string) => httpdate::parse_http_date(string).map_err(|_| HeaderValueDateTimeParseError::InvalidFormat),
            HeaderValue::String(string) => httpdate::parse_http_date(string).map_err(|_| HeaderValueDateTimeParseError::InvalidFormat),
            HeaderValue::DateTime(date_time) => Ok(*date_time),
            _ => Err(HeaderValueDateTimeParseError::InvalidFormat),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_header_value_string_length() {
        assert_eq!(HeaderValue::StaticString("hello").string_length(), 5);
        assert_eq!(HeaderValue::String(String::new()).string_length(), 0);
        assert_eq!(HeaderValue::String(String::from("This is a line.")).string_length(), 15);
        assert_eq!(HeaderValue::ContentCoding(ContentCoding::Brotli).string_length(), 2);
        assert_eq!(HeaderValue::ContentCoding(ContentCoding::Gzip).string_length(), 4);
        assert_eq!(HeaderValue::ContentRange(ContentRangeHeaderValue::Range { start: 99, end: 4783, complete_length: None }).string_length(), "bytes 99-4783/*".len());
        assert_eq!(HeaderValue::ContentRange(ContentRangeHeaderValue::Range { start: 0, end: 4, complete_length: Some(5) }).string_length(), "bytes 0-4/5".len());
        assert_eq!(HeaderValue::ContentRange(ContentRangeHeaderValue::Range { start: 0, end: 4, complete_length: Some(60) }).string_length(), "bytes 0-4/60".len());
        assert_eq!(HeaderValue::ContentRange(ContentRangeHeaderValue::Unsatisfied { complete_length: 10 }).string_length(), "bytes */10".len());
        assert_eq!(HeaderValue::MediaType(MediaType::HTML).string_length(), MediaType::HTML.as_str().len());
        assert_eq!(HeaderValue::SecFetchDest(SecFetchDest::Document).string_length(), "document".len());
        assert_eq!(HeaderValue::Size(100).string_length(), 3);
    }
}
