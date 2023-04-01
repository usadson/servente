// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestTarget {
    Origin {
        path: String,
        query: String,
    },
    Absolute(String),
    Authority(String),
    Asterisk,
}

impl RequestTarget {
    pub fn parse<'a>(input: impl Into<Cow<'a, str>>) -> Option<Self> {
        let input = input.into();
        if input == "*" {
            return Some(Self::Asterisk);
        }

        if input.starts_with('/') {
            if let Some((path, query)) = input.split_once('?') {
                return Some(Self::Origin {
                    path: path.to_string(),
                    query: query.to_string(),
                });
            }

            return Some(Self::Origin { path: input.to_string(), query: String::new() });
        }

        // TODO: correctly parse the URI.
        if input.starts_with("http://") || input.starts_with("https://") {
            return Some(RequestTarget::Absolute(input.into_owned()));
        }

        None
    }

    /// Returns the request target as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            RequestTarget::Origin{ path, .. } => path,
            RequestTarget::Absolute(string) => string,
            RequestTarget::Authority(string) => string,
            RequestTarget::Asterisk => "*",
        }
    }
}
