# Servente
A web server for static files written in Rust, used as a learning project and a way to test new ideas, features, specifications and more.


## Feature Support Table
| Feature               | Status | Description                                              | Notes                                 |
| --------------------- | ------ | -------------------------------------------------------- | ------------------------------------- |
| Accept-Ranges         | ✅     | Accepting range requests (especially useful for video's) | Fully supported                       |
| Content-Encoding      | ✅     | Compressing data before sending                          | Fully implemented (brotli and gzip)   |
| Custom API handlers   | 🤕     | Experimental API for adding custom handlers              | Experimental                          |
| ETag Caching          | ✅     | Cache files using an identifier                          | Based on file modification date       |
| HTTP/1.1              | ✅     | HTTP version every client supports                       | Compliant                             |
| HTTP/2                | ❌     | Improved binary-format HTTP (2015)                       | Not implemented yet                   |
| HTTP/2 Server Push    | ❎     | Pushing resources to the client before requested         | Won't be implemented                  |
| HTTP/3                | ❌     | Improved binary-format HTTP (2022) with QUIC (UDP)       | Not implemented yet                   |
| io_uring              | 🚧     | Asynchronous I/O for Linux                               | Blocked #1                            |
| ktls                  | 🚧     | Kernel TLS for Linux and FreeBSD                         | Blocked #2                            |
| Memory Cache          | ✅     | Cache files in memory for faster access                  | Uses `stretto`                        |
| OPTIONS method        | ❌     | Detecting server and resource capabilities               | Not implemented yet                   |
| TLS                   | ✅     | Transport Layer Security (HTTPS)                         | Uses rustls                           |
| Transfer-Encoding     | ✅     | Sending data in chunks                                   | `chunked` encoding is supported       |
| WebSockets            | ❌     | Real-time communication between client and server        | Not implemented yet                   |


## Building
Servente is built using [Rust](https://www.rust-lang.org/), a modern systems
programming language. It can be built using [Cargo](https://doc.rust-lang.org/cargo/),
Rust's package manager and build tool.

```bash
# Clone the repository
git clone https://github.com/usadson/servente.git
cd servente

# Build the project
cargo build --release
```

## Running
Servente can be run using the `servente` binary, which can be found in the
`target/release` directory after building.

```bash
# Run the server
./target/release/servente
```

## Configuring
Servente automatically creates a self-signed certificate and key for HTTPS. They
can be overriding by placing the `cert.der` and `key.der` files in the `.servente`
directory in current working directory.

Files are served from the `wwwroot/` directory in the current working directory.
You can override the welcome page by placing a `index.html` file in the `wwwroot/`.

## Quick Links
* [RFC 7541: HPACK Header Compression for HTTP/2](https://httpwg.org/specs/rfc7541.html)
* [RFC 9110: HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html)
* [RFC 9111: HTTP Caching](https://www.rfc-editor.org/rfc/rfc9111.html)
* [RFC 9111: HTTP/1.1](https://www.rfc-editor.org/rfc/rfc9112.html)
* [RFC 9112: HTTP/2](https://www.rfc-editor.org/rfc/rfc9113.html)
* [RFC 9113: HTTP/3](https://www.rfc-editor.org/rfc/rfc9114.html)
* [RFC 9204: QPACK Header Field Compression for HTTP/3](https://httpwg.org/specs/rfc9204.html)
* [HTTP Working Group](https://httpwg.org/)
