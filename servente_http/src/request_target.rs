// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

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
