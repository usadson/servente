// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! This crate contains various utilities for working with static resources. It
//! contains compressions and caching methods, amongst other small tools.

pub mod cache;
pub mod content_coding;
pub mod compression;
pub mod exclude;
mod magic;
pub mod media_type;
pub mod static_resources;

pub use cache::*;
pub use content_coding::*;
pub use compression::*;
pub use exclude::*;
pub use media_type::*;
