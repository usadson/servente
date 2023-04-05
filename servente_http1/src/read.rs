// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use servente_http::{
    BodyKind,
    Error,
    HeaderMap,
    HeaderName,
    HeaderValue,
    HttpParseError,
    HttpVersion,
    Method,
    Request,
    RequestTarget,
    syntax,
};

use tokio::io::{
    AsyncBufReadExt,
    AsyncReadExt,
};

use crate::{
    MaximumLength
};

use std::io;

/// Consume a `U+000D CARRIAGE RETURN` character (CR) and a `U+000A LINE FEED`
/// character (LF) from the stream.
async fn consume_crlf<R>(stream: &mut R) -> Result<(), Error>
        where R: AsyncBufReadExt + Unpin {
    _ = consume_exact_verify(stream, 2, |index, byte| {
        if index == 0 && byte != b'\r' {
            return Err(Error::ParseError(HttpParseError::InvalidCRLF));
        }

        if index == 1 && byte != b'\n' {
            return Err(Error::ParseError(HttpParseError::InvalidCRLF));
        }

        Ok(())
    }).await?;
    Ok(())
}

pub(crate) async fn consume_exact_verify<R>(stream: &mut R, length: usize, byte_validator: fn(usize, u8) -> Result<(), Error>) -> Result<Vec<u8>, Error>
        where R: AsyncBufReadExt + Unpin {
    if length == 0 {
        return Ok(Vec::new());
    }

    let mut buffer = Vec::<u8>::new();
    buffer.resize(length, 0);

    let mut idx = 0;
    while idx != length {
        let read = stream.read(&mut buffer[idx..(length - idx)]).await?;
        if read == 0 {
            return Err(Error::Other(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "EOF")));
        }
        for i in 0..read {
            byte_validator(idx + i, buffer[i])?;
        }
        idx += read;
    }

    Ok(buffer)
}

/// Reads a line from the stream, up to the maximum length.
pub(crate) async fn read_crlf_line<R>(stream: &mut R, maximum_length: MaximumLength) -> Result<String, Error>
        where R: AsyncBufReadExt + Unpin {
    let mut string = String::new();

    while string.len() < maximum_length.0 {
        let byte = stream.read_u8().await?;
        if byte == b'\r' {
            let byte = stream.read_u8().await?;
            if byte == b'\n' {
                return Ok(string);
            }
            return Err(Error::ParseError(HttpParseError::InvalidCRLF));
        }

        string.push(byte as char);
    }

    Err(Error::ParseError(HttpParseError::HeaderTooLarge))
}

/// Reads the headers from the stream.
pub(crate) async fn read_headers<R>(stream: &mut R) -> Result<HeaderMap, Error>
        where R: AsyncBufReadExt + Unpin {
    let mut header_map = HeaderMap::new();

    loop {
        let line = read_crlf_line(stream, MaximumLength::HEADER).await?;
        if line.is_empty() {
            return Ok(header_map);
        }

        let Some((name, value)) = line.split_once(':') else {
            return Err(Error::ParseError(HttpParseError::HeaderDoesNotContainColon));
        };
        let name = name.trim().to_string();
        let value = value.trim().to_string();

        servente_http::syntax::validate_token(&name)?;
        servente_http::syntax::validate_field_content(value.as_bytes())?;

        let name = HeaderName::from(name);
        if let HeaderName::Other(name) = &name {
            #[cfg(debug_assertions)]
            println!("[DEBUG] Unknown header name: \"{}\" with value: \"{}\"", name, value);
        }

        header_map.append(name, HeaderValue::from(value))?;
    }
}

/// Reads the HTTP version from the stream.
async fn read_http_version<R>(stream: &mut R) -> Result<HttpVersion, Error>
        where R: AsyncBufReadExt + Unpin {
    _ = consume_exact_verify(stream, 5, |index, byte| {
        if b"HTTP/"[index] == byte {
            Ok(())
        } else {
            Err(Error::ParseError(HttpParseError::InvalidHttpVersion))
        }
    }).await?;

    let mut version_buffer = [0u8; 3];
    stream.read_exact(&mut version_buffer).await?;

    Ok(match &version_buffer {
        b"1.0" => HttpVersion::Http10,
        b"1.1" => HttpVersion::Http11,
        b"2.0" => HttpVersion::Http2,
        _ => return Err(Error::ParseError(HttpParseError::InvalidHttpVersion)),
    })
}

/// Reads a string from the stream until the given character is found, or the
/// maximum length is reached.
async fn read_string_until_character<R>(stream: &mut R, char: u8, maximum_length: MaximumLength, length_error: HttpParseError,
        byte_validator: fn(u8) -> Result<(), HttpParseError>) -> Result<String, Error>
        where R: AsyncBufReadExt + Unpin {
    let mut buffer = String::new();

    while buffer.len() < maximum_length.0 {
        let byte = stream.read_u8().await?;
        if byte == char {
            return Ok(buffer);
        }

        byte_validator(byte)?;
        buffer.push(byte as char);
    }

    Err(Error::ParseError(length_error))
}

/// Reads the request body from the stream and stores it in the request.
pub(crate) async fn read_request_body<R>(stream: &mut R, request: &mut Request) -> Result<(), Error>
        where R: AsyncBufReadExt + Unpin {
    if let Some(content_length) = request.headers.get(&HeaderName::ContentLength) {
        request.body = Some(read_request_body_content_length(stream, request, content_length).await?);
        return Ok(());
    }

    if request.headers.get(&HeaderName::TransferEncoding).is_some() {
        request.body = Some(read_request_body_chunked(stream).await?);
        return Ok(());
    }

    Ok(())
}

/// Reads the request-body
async fn read_request_body_content_length<R>(stream: &mut R, request: &Request, content_length: &HeaderValue) -> Result<BodyKind, Error>
        where R: AsyncBufReadExt + Unpin {
    let content_length = content_length.parse_number().ok_or(Error::ParseError(HttpParseError::InvalidContentLength))?;
    let mut body = Vec::with_capacity(content_length);

    stream.read_exact(body.as_mut_slice()).await?;

    if let Some(media_type) = request.headers.get(&HeaderName::ContentType) {
        // TODO: correctly parse the Media Type
        if media_type.as_str_no_convert().unwrap().starts_with("text/") {
            match String::from_utf8(body) {
                Ok(body) => return Ok(BodyKind::String(body)),
                // String conversion was impossible (possibly not UTF-8), so just return the bytes.
                Err(error) => return Ok(BodyKind::Bytes(error.into_bytes())),
            }
        }
    }
    Ok(BodyKind::Bytes(body))
}

/// Reads the body of a request, assuming that the body is encoded using chunked
/// transfer encoding.
async fn read_request_body_chunked<R>(_stream: &mut R) -> Result<BodyKind, Error>
        where R: AsyncBufReadExt + Unpin {
    // TODO: support chunked encoding
    Err(Error::Other(io::Error::new(io::ErrorKind::InvalidData, "TODO: support chunked encoding")))
}

/// Read the request-line and headers from the stream, without reading the body.
///
/// We do not read the body here, because the handler might prefer to use their
/// own method of reading the body. This is especially useful for streaming
/// data, directly reading and writing without buffering, etc.
pub(crate) async fn read_request_excluding_body<R>(stream: &mut R) -> Result<Request, Error>
        where R: AsyncBufReadExt + Unpin {
    let (method, target, version) = read_request_line(stream).await?;
    let headers = if version == HttpVersion::Http2 {
        HeaderMap::new()
    } else {
        read_headers(stream).await?
    };
    Ok(Request { method, target, version, headers, body: None })
}

/// Read the request-line from the stream.
async fn read_request_line<R>(stream: &mut R) -> Result<(Method, RequestTarget, HttpVersion), Error>
        where R: AsyncBufReadExt + Unpin {

    let method = Method::from(read_string_until_character(stream, b' ', MaximumLength::METHOD, HttpParseError::MethodTooLarge,
        |b| if syntax::is_token_character(b) { Ok(()) } else { dbg!(b); Err(HttpParseError::InvalidOctetInMethod) }).await?);

    let target = read_request_target(stream).await?;
    let version = read_http_version(stream).await?;

    consume_crlf(stream).await?;

    Ok((method, target, version))
}

/// Reads the request-target from the stream.
///
/// ### References
/// * [RFC 9112, Section 3.2. Request Target](https://www.rfc-editor.org/rfc/rfc9112.html#name-request-target)
async fn read_request_target<R>(stream: &mut R) -> Result<RequestTarget, Error>
        where R: AsyncBufReadExt + Unpin {
    let str = read_string_until_character(stream, b' ', MaximumLength::REQUEST_TARGET,
        HttpParseError::RequestTargetTooLarge,
        |b| if syntax::is_request_target_character(b) { Ok(()) } else { Err(HttpParseError::InvalidOctetInRequestTarget) }).await?;

    RequestTarget::parse(str).ok_or(Error::ParseError(HttpParseError::InvalidRequestTarget))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    /// The connection preface, as defined in [RFC 9113 Section 3.4](https://www.rfc-editor.org/rfc/rfc9113.html#name-http-2-connection-preface).
    #[allow(dead_code)] // Linting doesn't understand that this is used in tests :(
    const HTTP2_CONNECTION_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

    #[tokio::test]
    async fn read_request_line_normal() {
        let mut stream = std::io::Cursor::new(b"GET / HTTP/1.1\r\n");
        let request_line = super::read_request_line(&mut stream).await.unwrap();
        assert_eq!(request_line.0, Method::Get);
        assert_eq!(request_line.1 , RequestTarget::Origin { path: "/".to_string(), query: String::new() });
        assert_eq!(request_line.2, HttpVersion::Http11);
    }

    #[rstest]
    #[case(b"DELETE / HTTP/1.1\r\n", Method::Delete)]
    #[case(b"GET / HTTP/1.1\r\n", Method::Get)]
    #[case(b"get / HTTP/1.1\r\n", Method::Other(String::from("get")))]
    #[case(b"POST / HTTP/1.1\r\n", Method::Post)]
    #[case(b"PUT / HTTP/1.1\r\n", Method::Put)]
    #[case(b"OPTIONS * HTTP/1.1\r\n", Method::Options)]
    #[case(b"NEW-METHOD / HTTP/1.1\r\n", Method::Other(String::from("NEW-METHOD")))]
    #[tokio::test]
    async fn read_request_line_methods(#[case] input: &[u8], #[case] expected: Method) {
        let mut stream = std::io::Cursor::new(input);
        let request_line = super::read_request_line(&mut stream).await.unwrap();
        assert_eq!(request_line.0, expected);
        assert_eq!(request_line.2, HttpVersion::Http11);
    }

    #[tokio::test]
    async fn read_request_line_long_method() {
        let mut stream = std::io::Cursor::new(b"THIS-IS-A-VERY-LONG-METHOD / HTTP/1.1\r\n");
        let request_line = super::read_request_line(&mut stream).await;
        assert!(request_line.is_err());
        let error = request_line.err().unwrap();
        match &error {
            Error::ParseError(HttpParseError::MethodTooLarge) => {},
            _ => panic!("Unexpected error: {:?}", error),
        }
    }

    #[cfg(feature = "http2")]
    #[tokio::test]
    async fn http2_upgrade_read_request_line() {
        let mut data = std::io::Cursor::new(HTTP2_CONNECTION_PREFACE);
        let (method, request_target, version) = read_request_line(&mut data).await.unwrap();
        assert_eq!(method, Method::Pri);
        assert_eq!(request_target, RequestTarget::Asterisk);
        assert_eq!(version, HttpVersion::Http2);
        assert_eq!(data.position() as usize, b"PRI * HTTP/2.0\r\n".len());
    }
}
