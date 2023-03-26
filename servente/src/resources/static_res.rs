// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! This file contains various static resources that are used by the
//! application. These resources are embedded into the binary using the
//! `include!` macros.

// The path to the `/resources` directory in the repository root.
//
// There isn't an environment variable like `CARGO_WORKSPACE_DIR` yet, so
// we resort to using relative paths instead.
//
// `CARGO_MANIFEST_DIR` will return `[repository]/servente/` so `..` will
// get the workspace directory.

/// The HTML page that is shown when the user visits the root of the
/// application, without having overridden the default welcome page.
pub const WELCOME_HTML: &str = include_str!(concat!(concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/"), "welcome.html"));

/// The HTML page that is shown when the user visits the root of the
/// application, without having overridden the default welcome page.
pub const WELCOME_HTML_NL: &str = include_str!(concat!(concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/"), "welcome.nl.html"));
