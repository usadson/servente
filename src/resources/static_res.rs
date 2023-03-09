// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.
//
// This file contains various static resources that are used by the
// application. These resources are embedded into the binary using the
// `include!` macros.

/// The HTML page that is shown when the user visits the root of the
/// application, without having overridden the default welcome page.
pub const WELCOME_HTML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/welcome.html"));
