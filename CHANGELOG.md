# Changelog
All notable changes to this project will be documented in this file.

## [Unreleased]

## [0.2.0](https://github.com/usadson/servente/releases/tag/v0.2.0) - 2023-03-26
Beta build with support for HTTP/2 and caching.

### Added
- Beta HTTP/2 support
- HPACK support with compliant dynamic table
- Drafting a document describing sandboxing mechanisms
- Added Travis CI workflow for `FreeBSD` builds
- Added CI badges to `README`
- Validate HTTP/2 headers according to the HTTP/2 specification
- Validate HTTP/1.x headers

### Changed
- Improved `README` with extended feature support table and better wording

## [0.1.0](https://github.com/usadson/servente/releases/tag/v0.1.0) - 2023-03-17
Initial beta version.

### Added
- Mostly compliant HTTP/1.1 support
- File Caching in Memory using [stretto](https://docs.rs/stretto/latest/stretto/)
- Disallow certain files based on extensions (e.g. `*.log`)
- Verify integrity using GitHub Actions
- Add Brotli and GZip compression
- Automatically generate self-signed TLS certificates
- Provide security headers relating to XSS and CORS
- Prevent caching already optimal files (JPEG, PNG)
- Stabilize C10k/heavy load scenarios
- Supply `Last-Modified` file date and compare them, resulting in `304 Not Modified`
- Add weak ETag support for better browser support, since some (old) browsers might parse and encode `Last-Modified` dates incorrectly
- Add welcome page for when the `wwwroot` doesn't have an `index.html` file
- Add Dutch translation for the welcome page
- Support for `Range` requests, which is especially useful for large files, like `<video>` streaming
- Restrict request headers, method and URI sizes
- Try to end connections gracefully

### Work in Progress
- Static file analysis for client hints (`103 Early Hints` status code, `Link` header, etc.)
- `ktls` support for fast file serving using `sendfile`
- Experimental CGI support, currently only Rust-based infrastructure
- Experimental `1xx` response supprot
