// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::io::Write;

/// A list of supported content encodings.
///
/// ## About
/// This enum is used to specify the content encoding of a resource. Servente
/// only provides support for the `gzip` and `br` encodings, since the other
/// encodings are not widely used.
///
/// Chromium for example, [only supports](https://source.chromium.org/chromium/chromium/src/+/main:ui/base/resource/resource_bundle.cc;l=178;drc=4cc7ba01d3c5dc996ddc98f9d0bd709e3d5bbfd3;bpv=1;bpt=1)
/// `gzip` and `br` encodings.
///
/// ### Deflate
/// Historically, `deflate` was commonly used to compress response data, but
/// Microsoft's web services were not compliant with the HTTP specification.
/// This is also the reason that this encoding is not supported in Servente.
///
/// ## References
/// * [IANA HTTP Content Coding Registry](https://www.iana.org/assignments/http-parameters/http-parameters.xhtml#content-coding)
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContentCoding {
    /// The `br` content encoding.
    ///
    /// ## References
    /// * [Wikipedia](https://en.wikipedia.org/wiki/Brotli)
    /// * [RFC 7932: Brotli Compressed Data Format](https://datatracker.ietf.org/doc/html/rfc7932)
    Brotli,

    /// The `gzip` content encoding.
    ///
    /// ## References
    /// * [Wikipedia](https://en.wikipedia.org/wiki/Gzip)
    /// * [RFC 1952: GZIP file format specification version 4.3](https://datatracker.ietf.org/doc/html/rfc1952)
    Gzip,
}

impl ContentCoding {
    /// Encodes the given data using the specified content encoding.
    pub fn encode(&self, data: &Vec<u8>) -> Option<Vec<u8>> {
        let mut result = Vec::new();
        match self {
            ContentCoding::Brotli => {
                let mut reader = brotli::CompressorReader::new(std::io::Cursor::new(&data), 4096, 11, 22);
                std::io::copy(&mut reader, &mut result).unwrap();
                Some(result)
            }
            ContentCoding::Gzip => {
                let mut encoder = flate2::write::GzEncoder::new(result, flate2::Compression::default());
                encoder.write_all(data).unwrap();
                Some(encoder.finish().unwrap())
            }
        }
    }

    /// Returns the HTTP identifier for the content encoding, as specified in
    /// the IANA Registry (name field).
    ///
    /// * [IANA HTTP Content Coding Registry](https://www.iana.org/assignments/http-parameters/http-parameters.xhtml#content-coding)
    pub fn http_identifier(&self) -> &'static str {
        match self {
            ContentCoding::Brotli => "br",
            ContentCoding::Gzip => "gzip",
        }
    }
}
