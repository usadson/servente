# Project Roadmap
This document outlines the upcoming features, improvements and enhancements
planned for the project.

## Short-Term
The following are goals to be executed within a couple of weeks.

### Documentation
* Add a security document describing common/possible security flaws, and how this project addresses them
* Add a sandboxing document describing

### Features
* Fully compliant HTTP/2 support
* Finished HTTP/3 support
* [`OPTIONS`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/OPTIONS) method to allow clients to identify the allowed mechanisms for a specific resource, and detect global server features
* Sandboxed server environment, even without Docker, to protect the computer even on the improbably possibility of remote code execution, kernel-based disallowal of files outside of `wwwroot`
* Denial of Service protection for long-term connections like HTTP/1.x keep-alive, HTTP/2 and HTTP/3

#### Multimedia
* Video streaming using [MPEG-DASH](https://en.wikipedia.org/wiki/Dynamic_Adaptive_Streaming_over_HTTP) / [HTTP Live Streaming](https://en.wikipedia.org/wiki/HTTP_Live_Streaming)
* Image downscaling for large files
* SVG rasterization

#### Configuration
* A Git repository acting as `wwwroot`
* ZIP/Tar archives acting as directories, or as `wwwroot`.
* [Docker](https://docker.com/) prebuilt containers


## Intermediate-Term Goals
* Multiple processes handling the same socket, to allow restarting the server without dropping connections
* For a development environment, inject autoreload script in `HTML` files

## Long-Term Goals
These goals are desirable things, but require a lot of work, need extensive
research, and/or need major infrastructural changes.

* Autoconfiguration for TLS certificates ([ACME](https://www.rfc-editor.org/rfc/rfc8555)) using [LetsEncrypt](https://letsencrypt.org)
* `ktls` support for [Linux `sendfile(2)`](https://man7.org/linux/man-pages/man2/sendfile.2.html) and [FreeBSD `sendfile(2)`](https://man.freebsd.org/cgi/man.cgi?query=sendfile&sektion=2&format=html)
* Kernel-level asynchronous I/O for Linux using [io_uring](https://man.archlinux.org/man/io_uring.7)
* [Common Gateway Interface](https://en.wikipedia.org/wiki/Common_Gateway_Interface) support, FastCGI support and accelerated PHP for applications like WordPress.
* [Reverse proxy](https://www.cloudflare.com/learning/cdn/glossary/reverse-proxy/) support

### Reconsiderations
The following aspects might be reconsidered, when previously marked as `won't be implemented`.

#### [`PUSH_PROMISE`](https://http3-explained.haxx.se/en/h3/h3-push) for HTTP/2 and HTTP/3
These might be beneficial for high-speed internet connections, and we might be
able to do so using with RTT heuristics.