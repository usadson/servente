# Changelog
All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- Added roadmap describing upcoming features and changes
- Markdown to HTML renderer/converter
- Support **`OPTIONS`** request methods
- **Collaudatrice**, a tester for HTTP compliance and other web and
  webserver-related conformance and best practices.
- Add validation for request header field values ([RFC 9110 Section 5.5](https://www.rfc-editor.org/rfc/rfc9110.html#section-5.5)).
- Add recommended `pre-push` Git hook for running **Clippy** before pushing.
- Added tools to unify common code for working with HTTP Lists (`#rule`s)
- Validate `method` and `request-target` for *HTTP/1.x*
- Added read timeouts for *HTTP/1.x* to prevent some **Denial of Service**s
- Added integration tests for *HTTP/1.x* using **cURL**.
- Added support for [BoringSSL](https://boringssl.googlesource.com/boringssl/), use the `tls-boring` feature flag
- Added experimental support for plaintext HTTP
  - Disable the `rustls` feature
- Support for HTTP/2 trailers
- General performance improvements:
  - Support `Arc<str>` in headers to share with the dynamic table
  - Use `write!` instead of `String::push_str(format!(...))`
- Add support for CGI (Common Gateway Interface)
- License under the [`Apache License 2.0`](./COPYING)

### Changed
- Restructure repository for multi-crate config (Cargo workspace)
- Methods are now **case-sensitive**, as this conforms to the spec
  ([RFC 9110 Section 9.1](https://www.rfc-editor.org/rfc/rfc9110.html#section-9.1-5))
- Streamline HTTP parsing for shared semantics (e.g. `request-target`)
- More tests are added to prove code works at build-time, and still works after (internal) changes
- Mask `.htaccess` files so they now also return **`404 Not Found`**
- Updated dependencies

### Fixed
- **ALPN** and **`Alt-Svc`** weren't based on feature detection, so for example
  HTTP/2 would be listed on there, but wouldn't actually be available.
- Broken `Accept-Language` parsing for welcome pages
- Broken **HTTP/2** upgrade, this is now tested
- HTTP/2 is now almost fully supported and very usable
- Failing builds for builds without the `convert-markdown` feature flag
- Executable files won't be served now on *NIX (i.e. `chmod +x`) as a security measure and for CGI support

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
