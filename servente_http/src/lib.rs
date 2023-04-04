// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! This crate contains the common HTTP logic for all HTTP versions (1.x, 2, 3).

pub mod abnf;
pub mod error;
pub mod header_map;
pub mod header_name;
pub mod header_value;
pub mod method;
pub mod range;
pub mod request;
pub mod request_target;
pub mod response;
pub mod status;
pub mod syntax;
pub mod version;

use std::sync::Arc;

pub use error::*;
pub use method::*;
pub use header_map::*;
pub use header_name::*;
pub use header_value::*;
pub use method::*;
pub use range::*;
pub use request::*;
pub use request_target::*;
pub use response::*;
pub use status::*;
pub use version::*;

use servente_resources::{
    ContentCoding,
    ContentEncodedVersions,
};

#[derive(Debug)]
pub enum BodyKind {
    Bytes(Vec<u8>),
    CachedBytes(Arc<ContentEncodedVersions>, Option<ContentCoding>),
    File {
        handle: tokio::fs::File,
        metadata: std::fs::Metadata,
    },
    StaticString(&'static str),
    String(String),
}

impl From<&'static str> for BodyKind {
    fn from(value: &'static str) -> Self {
        Self::StaticString(value)
    }
}

impl From<String> for BodyKind {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
