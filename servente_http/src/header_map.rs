// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::time::{SystemTime, Duration};

use servente_resources::MediaType;

use crate::{
    ContentRangeHeaderValue,
    HeaderName,
    HeaderValue,
    SecFetchDest,
};

#[derive(Clone, Debug, Default)]
pub struct HeaderMap {
    headers: Vec<(HeaderName, HeaderValue)>,
}

impl HeaderMap {
    pub fn new() -> HeaderMap {
        HeaderMap::default()
    }

    pub fn new_with_vec(headers: Vec<(HeaderName, HeaderValue)>) -> HeaderMap {
        HeaderMap { headers }
    }

    /// Appends a header to the list of headers. This is used for headers that
    /// can be duplicated, such as `Set-Cookie` and `Link`.
    pub fn append_possible_duplicate(&mut self, header_name: HeaderName, value: HeaderValue) {
        self.headers.push((header_name, value));
    }

    #[must_use]
    pub fn contains(&self, header_name: &HeaderName) -> bool {
        for (name, _) in &self.headers {
            if name == header_name {
                return true;
            }
        }

        false
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.headers.len()
    }

    #[must_use]
    pub fn get(&self, header_name: &HeaderName) -> Option<&HeaderValue> {
        for (name, value) in &self.headers {
            if name == header_name {
                return Some(value);
            }
        }

        None
    }

    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(HeaderName, HeaderValue)> {
        self.headers.iter()
    }

    pub fn remove(&mut self, header_name: &HeaderName) {
        self.headers.retain(|(name, _)| name != header_name);
    }

    pub fn set(&mut self, header_name: HeaderName, value: HeaderValue) {
        for (name, existing_value) in &mut self.headers {
            if name == &header_name {
                *existing_value = value;
                return;
            }
        }

        self.headers.push((header_name, value));
    }
}

#[must_use]
pub fn format_system_time_as_weak_etag(date_time: SystemTime) -> String {
    format!("W/{:X}", date_time.duration_since(SystemTime::UNIX_EPOCH).unwrap_or(Duration::default()).as_secs())
}

//
// Header-specific methods
//
impl HeaderMap {
    #[must_use]
    pub fn sec_fetch_dest(&self) -> Option<SecFetchDest> {
        self.get(&HeaderName::SecFetchDest)
            .and_then(|value| value.as_str_no_convert())
            .and_then(SecFetchDest::parse)
    }

    pub fn set_content_length(&mut self, length: usize) {
        self.set(HeaderName::ContentLength, HeaderValue::Size(length));
    }

    pub fn set_content_range(&mut self, range: ContentRangeHeaderValue) {
        self.set(HeaderName::ContentRange, HeaderValue::ContentRange(range));
    }

    pub fn set_content_type(&mut self, media_type: MediaType) {
        self.set(HeaderName::ContentType, HeaderValue::MediaType(media_type));
    }

    pub fn set_last_modified(&mut self, date_time: SystemTime) {
        self.set(HeaderName::LastModified, HeaderValue::DateTime(date_time));
        if !self.contains(&HeaderName::ETag) {
            self.set(HeaderName::ETag, format_system_time_as_weak_etag(date_time).into());
        }
    }
}
