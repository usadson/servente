// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use hashbrown::HashMap;
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
    headers: HashMap<HeaderName, HeaderValue>,
    set_cookie_values: Vec<HeaderValue>,
}

impl HeaderMap {
    pub fn new() -> HeaderMap {
        HeaderMap::default()
    }

    /// Append a header to the list of headers.
    ///
    /// If the field with the given `name` was already present, the values are
    /// concatenated with `, `, as specified by the HTTP specification.
    ///
    /// # References
    ///
    pub fn append(&mut self, name: HeaderName, value: HeaderValue) -> Result<(), HeaderMapInsertionError> {
        if name == HeaderName::SetCookie {
            self.set_cookie_values.push(value);
            return Ok(());
        }

        match self.headers.entry(name) {
            hashbrown::hash_map::Entry::Occupied(e) => {
                if e.key() == &HeaderName::ContentLength {
                    return Err(HeaderMapInsertionError::MultipleContentLength);
                }

                e.replace_entry_with(|_, old_value| {
                    let old_value_string_storage;
                    let old_value = if let Some(old_value) = old_value.as_str_no_convert() {
                        old_value
                    } else {
                        old_value_string_storage = old_value.to_string();
                        &old_value_string_storage
                    };

                    let extra_value_string_storage;
                    let extra_value = if let Some(extra_value) = value.as_str_no_convert() {
                        extra_value
                    } else {
                        extra_value_string_storage = value.to_string();
                        &extra_value_string_storage
                    };

                    Some(HeaderValue::from(format!("{}, {}", old_value, extra_value)))
                });
            }
            hashbrown::hash_map::Entry::Vacant(e) => {
                e.insert(value);
            }
        }

        Ok(())
    }

    pub fn append_or_override(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.insert(name, value);
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

    pub fn iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.headers.iter()
    }

    pub fn remove(&mut self, header_name: &HeaderName) {
        self.headers.retain(|name, _| name != header_name);
    }
}

/// While most duplicate header fields can be concatenated with a comma, some
/// header fields are explictly not interpretable as lists, for example the
/// `Content-Length` field.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeaderMapInsertionError {
    /// There were multiple `Content-Length` fields defined.
    MultipleContentLength,
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
        self.append_or_override(HeaderName::ContentLength, HeaderValue::Size(length));
    }

    pub fn set_content_range(&mut self, range: ContentRangeHeaderValue) {
        self.append_or_override(HeaderName::ContentRange, HeaderValue::ContentRange(range));
    }

    pub fn set_content_type(&mut self, media_type: MediaType) {
        self.append_or_override(HeaderName::ContentType, HeaderValue::MediaType(media_type));
    }

    pub fn set_last_modified(&mut self, date_time: SystemTime) {
        self.append_or_override(HeaderName::LastModified, HeaderValue::DateTime(date_time));
        if !self.contains(&HeaderName::ETag) {
            self.append_or_override(HeaderName::ETag, format_system_time_as_weak_etag(date_time).into());
        }
    }
}
