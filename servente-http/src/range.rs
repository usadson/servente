// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Range {
    Full,
    StartPointToEnd { start: u64 },
    Points {
        start: u64,
        end: u64,
    },
    Suffix { suffix: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRangeList {
    pub ranges: Vec<Range>,
}

impl HttpRangeList {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        if !value.starts_with("bytes=") {
            return None;
        }
        let mut ranges = Vec::new();
        for range in value[6..].split(',') {
            let range = range.trim();
            if range.is_empty() {
                continue;
            }
            let range = if let Some(suffix) = range.strip_prefix('-') {
                Range::Suffix { suffix: suffix.parse().ok()? }
            } else if let Some(start) = range.strip_suffix('-') {
                Range::StartPointToEnd { start: start.parse().ok()? }
            } else {
                let mut range = range.splitn(2, '-');
                let start = range.next()?.parse().ok()?;
                let end = range.next()?.parse().ok()?;
                Range::Points { start, end }
            };
            ranges.push(range);
        }
        Some(Self { ranges })
    }

    /// Returns the first and only range if there is only one range.
    /// Otherwise, when there are 0 or more than one, returns `None`.
    #[must_use]
    pub fn first_and_only(&self) -> Option<Range> {
        if self.ranges.len() == 1 {
            Some(self.ranges[0])
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Range> {
        self.ranges.iter()
    }
}

/// The `Content-Range` header field indicates where in a full body a partial
/// message belongs.
///
/// ### References
/// * [RFC 9110](https://httpwg.org/specs/rfc9110.html#field.content-range)
/// * [MDN `Content-Range` header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Range)
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContentRangeHeaderValue {
    Range {
        /// The start of the range, inclusive.
        start: usize,

        /// The end of the range, inclusive.
        end: usize,

        /// Complete length of the **resource**, not the body.
        complete_length: Option<usize>,
    },

    /// Used for 416 Range Not Satisfiable.
    ///
    /// ### RFC 9110, section 14.4:
    /// > A server generating a 416 (Range Not Satisfiable) response to a
    /// byte-range request SHOULD send a Content-Range header field with an
    /// unsatisfied-range value, as in the following example:
    /// > ```text
    /// > Content-Range: bytes */1234`
    /// > ```
    /// > The complete-length in a 416 response indicates the current length of
    /// > the selected representation.
    ///
    /// ### References
    /// * [MDN `416 Range Not Satisfiable` HTTP status code](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/416)
    /// * [MDN `Content-Range` header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Range)
    /// * [RFC 9110](https://httpwg.org/specs/rfc9110.html#field.content-range)
    Unsatisfied {
        /// The complete length of the resource.
        complete_length: usize
    },
}
