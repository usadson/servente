# 🚀 Servente
[<img alt="GitHub Actions build status" src="https://img.shields.io/github/actions/workflow/status/usadson/servente/rust.yml?logo=Github-Actions&style=for-the-badge" height="22">](https://github.com/usadson/servente/actions/)
[<img alt="Travis CI build status" src="https://img.shields.io/github/actions/workflow/status/usadson/servente/rust.yml?logo=travis&style=for-the-badge" height="22">](https://app.travis-ci.com/github/usadson/servente)

A web server designed for hosting static files and serves as a valuable tool for
testing new ideas, features, and specifications. Despite its focus on
experimentation, the server remains highly performant, delivering
⚡ lightning-fast speeds that allow for seamless content delivery.

## 💡 Feature Support Table
| Feature                   | Status | Description                                              | Notes                                                                                                                 |
| ------------------------- | ------ | -------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Accept-Ranges             | ✅     | Accepting range requests (especially useful for video's) | Fully supported                                                                                                       |
| Automatic Certificates    | 🧪     | Automatically create certificates for HTTPS              | Only self-signed at the moment, but we should implement the ACME protocol for supporting LetsEncrypt                  |
| Content-Encoding          | ✅     | Compressing data before sending                          | Fully implemented (brotli and gzip)                                                                                   |
| Common Gateway Interface  | 🧪     | Running code to generate pages and resources             | CGI 1.1 mostly supported, other implementations like FastCGI aren't yet.                                              |
| Custom API handlers       | 🧪     | Experimental API for adding custom handlers              | Experimental                                                                                                          |
| ETag Caching              | ✅     | Cache files using an identifier                          | Based on file modification date                                                                                       |
| HTTP/1.1                  | ✅     | HTTP version every client supports                       | Compliant                                                                                                             |
| HTTP/2                    | ✅     | Improved binary-format HTTP (2015)                       | Largely supported                                                                                                     |
| HTTP/2 Server Push        | 🚧     | Pushing resources to the client before requested         | Won't be implemented                                                                                                  |
| HTTP/3                    | ❌     | Improved binary-format HTTP (2022) with QUIC (UDP)       | Not implemented yet                                                                                                   |
| HTTP/3 Server Push        | 🚧     | Pushing resources to the client before requested         | Won't be implemented                                                                                                  |
| io_uring                  | ⚔️     | Asynchronous I/O for Linux                               | Blocked #1                                                                                                            |
| ktls                      | ⚔️     | Kernel TLS for Linux and FreeBSD                         | Blocked #2                                                                                                            |
| Last-Modified Caching     | ✅     | Cache files using the modification date                  | Fully supported                                                                                                       |
| Markdown Rendering        | 🧪     | Render Markdown files to HTML                            | Experimental                                                                                                          |
| Memory Cache              | ✅     | Cache files in memory for faster access                  | Uses [`stretto`](https://docs.rs/stretto/latest/stretto/)                                                             |
| OPTIONS method            | ✅     | Detecting server and resource capabilities               | Experimental                                                                                                          |
| TLS                       | ✅     | Transport Layer Security (HTTPS)                         | Uses [`rustls`](https://docs.rs/rustls/latest/rustls/) or [BoringSSL](https://boringssl.googlesource.com/boringssl/)  |
| Transfer-Encoding         | ✅     | Sending data in chunks                                   | `chunked` encoding is supported                                                                                       |
| WebSockets                | 🔨     | Real-time communication between client and server        | Work in progress yet                                                                                                  |


## 🛠️ Building
Servente is built using [🦀 Rust](https://www.rust-lang.org/), a modern systems
programming language. It can be built using [Cargo](https://doc.rust-lang.org/cargo/),
Rust's package manager and build tool.

```bash
# Clone the repository
git clone https://github.com/usadson/servente.git
cd servente

# Build the project
cargo build --release
```

## ⚙️ Running
Servente can be run using the `servente` binary, which can be found in the
`target/release` directory after building.

```bash
# Run the server
./target/release/servente
```

## 🏃 Configuring
Servente automatically creates a self-signed certificate and key for HTTPS. They
can be overriding by placing the `cert.der` and `key.der` files in the `.servente`
directory in current working directory.

Files are served from the `wwwroot/` directory in the current working directory.
You can override the welcome page by placing a `index.html` file in the `wwwroot/`.

## 🎁 Contributing
Contributions to Servente are welcome! If you find a bug or have a feature
request, please open an issue on GitHub. If you would like to contribute code,
please fork the repository and submit a pull request.

## 🔍 Documentation
The code is documented in-source, but for non-code information, you can read
more here:
* [Changelog](CHANGELOG.md)
* [Possible Security Flaws](docs/Security.md)
* [Roadmap](ROADMAP.md)
* [Quick-and-dirty Benchmarks](docs/Benchmark.md)
* [Sandboxing Research](docs/Sandboxing.md)

## 📚 Quick Links
* [CommonMark](https://spec.commonmark.org/0.30/)
* [RFC 3875: The Common Gateway Interface (CGI) Version 1.1](https://www.rfc-editor.org/rfc/rfc3875)
* [RFC 6455: The WebSocket Protocol](https://www.rfc-editor.org/rfc/rfc6455)
* [RFC 7541: HPACK Header Compression for HTTP/2](https://httpwg.org/specs/rfc7541.html)
* [RFC 9110: HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html)
* [RFC 9111: HTTP Caching](https://www.rfc-editor.org/rfc/rfc9111.html)
* [RFC 9111: HTTP/1.1](https://www.rfc-editor.org/rfc/rfc9112.html)
* [RFC 9112: HTTP/2](https://www.rfc-editor.org/rfc/rfc9113.html)
* [RFC 9113: HTTP/3](https://www.rfc-editor.org/rfc/rfc9114.html)
* [RFC 9204: QPACK Header Field Compression for HTTP/3](https://httpwg.org/specs/rfc9204.html)
* [HTTP Working Group](https://httpwg.org/)
* [WebSockets Standard](https://websockets.spec.whatwg.org)

## ⚖️ Copyright
**Servente**, and all of it's components, with the exception of
[third-party software](./THIRDPARTY), is licensed under the
[Apache License 2.0](./COPYING).

> Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org> \
> All Rights Reserved.
