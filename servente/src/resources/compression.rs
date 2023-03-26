// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{io::Write, fmt::Formatter, time::SystemTime};

use super::MediaType;

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

/// A struct that contains multiple encoded/compressed versions of the same
/// resource.
///
/// TODO: name this structure better, since it doesn't reflect the contents
///       anymore
#[derive(Default)]
pub struct ContentEncodedVersions {
    pub modified_date: Option<SystemTime>,
    pub cache_details: Option<super::cache::CachedFileDetails>,
    pub media_type: Option<MediaType>,

    /// The uncompressed version of the resource.
    pub uncompressed: Vec<u8>,

    /// The `br` encoded version of the resource.
    pub brotli: Option<Vec<u8>>,

    /// The `gzip` encoded version of the resource.
    pub gzip: Option<Vec<u8>>,
}

impl core::fmt::Debug for ContentEncodedVersions {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ContentEncodedVersions")
            .field("uncompressed", &self.uncompressed.len())
            .field("brotli", &self.brotli.as_ref().map_or(0, |v| v.len()))
            .field("gzip", &self.gzip.as_ref().map_or(0, |v| v.len()))
            .finish()
    }
}

/// Returns whether the given file should be compressed.
/// We shouldn't compress images, since they are already compressed.
fn should_compress_file(uncompressed: &Vec<u8>) -> bool {
    !uncompressed.starts_with(super::FILE_JPEG_MAGIC_NUMBER) && !uncompressed.starts_with(super::FILE_PNG_MAGIC_NUMBER)
}

impl ContentEncodedVersions {
    pub fn create(uncompressed: Vec<u8>) -> Self {
        let mut result = ContentEncodedVersions {
            uncompressed,
            ..Default::default()
        };

        if should_compress_file(&result.uncompressed) {
            result.brotli = ContentCoding::Brotli.encode(&result.uncompressed);
            result.gzip = ContentCoding::Gzip.encode(&result.uncompressed);
        }

        result
    }

    pub fn determine_best_version_from_accept_encoding(&self, accept_encoding: &str) -> Option<ContentCoding> {
        if self.gzip.as_ref().map_or(usize::MAX, |v| v.len()) > self.uncompressed.len()
                && self.brotli.as_ref().map_or(usize::MAX, |v| v.len()) > self.uncompressed.len() {
            return None;
        }

        if self.gzip.is_none() && self.brotli.is_none() {
            return None;
        }

        // The quality of all the other (and unspecified) encodings, indicated
        // by the wildcard '*'.
        let mut all_quality = 1.0;
        let mut brotli_quality = None;
        let mut gzip_quality = None;

        for encoding in accept_encoding.split(',') {
            let mut parts = encoding.split(';');
            let encoding = parts.next().unwrap().trim();
            let quality = parts.next().map(|q| q.trim_start_matches("q=")).unwrap_or("1.0").parse::<f32>().unwrap_or(1.0);

            match encoding {
                "*" => {
                    all_quality = quality;
                }
                "br" => {
                    if self.brotli.is_some() {
                        brotli_quality = Some(quality);
                    } else {
                        brotli_quality = Some(0.0);
                    }
                }
                "gzip" => {
                    if self.gzip.is_some() {
                        gzip_quality = Some(quality);
                    } else {
                        gzip_quality = Some(0.0);
                    }
                }
                _ => {}
            }
        }

        if brotli_quality.is_none() && gzip_quality.is_none() {
            // The client does not prefer any of the encodings, and the
            // HTTP-specification states that the missing encodings should
            // we treated as a quality of 1.0.
            if all_quality > 0.0 {
                return self.determine_smallest_file_size();
            }

            return None;
        }

        if let Some(brotli_quality) = brotli_quality {
            if let Some(gzip_quality) = gzip_quality {
                if gzip_quality == brotli_quality {
                    return self.determine_smallest_file_size();
                }

                if gzip_quality > brotli_quality {
                    if self.gzip.is_some() {
                        return Some(ContentCoding::Gzip);
                    }
                    return Some(ContentCoding::Brotli);
                }

                if self.brotli.is_some() {
                    return Some(ContentCoding::Brotli);
                }

                return Some(ContentCoding::Gzip);
            }

            if self.brotli.is_some() {
                return Some(ContentCoding::Brotli);
            }

            return None;
        }

        if self.gzip.is_some() && gzip_quality.unwrap_or(all_quality) > 0.0 {
            return Some(ContentCoding::Gzip);
        }

        None
    }

    /// Determine the ContentCoding with the smallest file size.
    pub fn determine_smallest_file_size(&self) -> Option<ContentCoding> {
        let Some(brotli) = &self.brotli else {
            if let Some(gzip) = &self.gzip {
                if gzip.len() < self.uncompressed.len() {
                    return Some(ContentCoding::Gzip);
                }

                return None;
            }
            return None;
        };

        let Some(gzip) = &self.gzip else {
            if brotli.len() < self.uncompressed.len() {
                return Some(ContentCoding::Brotli);
            }

            // Gzip is unavailable, but brotli is larger than the uncompressed
            // version.
            return None;
        };

        if gzip.len() < self.uncompressed.len() {
            if brotli.len() <= gzip.len() {
                return Some(ContentCoding::Brotli);
            }

            return Some(ContentCoding::Gzip);
        }

        if brotli.len() < self.uncompressed.len() {
            return Some(ContentCoding::Brotli);
        }

        None
    }

    pub fn get_version(&self, coding: Option<ContentCoding>) -> &Vec<u8> {
        match coding {
            Some(ContentCoding::Brotli) => self.brotli.as_ref().unwrap(),
            Some(ContentCoding::Gzip) => self.gzip.as_ref().unwrap(),
            None => &self.uncompressed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ContentCoding,
        ContentEncodedVersions,
    };

    #[test]
    pub fn determine_smallest_file_size_only_uncompressed() {
        let versions = ContentEncodedVersions {
            uncompressed: vec![1, 2, 3, 4, 5],
            ..Default::default()
        };

        assert_eq!(versions.determine_smallest_file_size(), None);
    }

    #[test]
    pub fn determine_smallest_file_size_uncompressed_smaller() {
        let versions = ContentEncodedVersions {
            uncompressed: vec![1, 2, 3, 4, 5],
            brotli: Some(vec![1, 2, 3, 4, 5, 6]),
            gzip: Some(vec![1, 2, 3, 4, 5, 6, 7]),
            ..Default::default()
        };

        assert_eq!(versions.determine_smallest_file_size(), None);
    }

    /// When both brotli and gzip are smaller than the uncompressed file, the
    /// file with the smallest size is preferred.
    #[test]
    pub fn determine_smallest_file_size_brotli_smaller() {
        let mut versions = ContentEncodedVersions {
            uncompressed: vec![1, 2, 3, 4, 5, 6, 7],
            brotli: Some(vec![1, 2, 3, 4, 5, 6]),
            gzip: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            ..Default::default()
        };

        assert_eq!(versions.determine_smallest_file_size(), Some(ContentCoding::Brotli));

        versions.gzip = None;
        assert_eq!(versions.determine_smallest_file_size(), Some(ContentCoding::Brotli));
    }


    /// When both brotli and gzip are smaller than the uncompressed file, the
    /// file with the smallest size is preferred.
    #[test]
    pub fn determine_smallest_file_size_gzip_smaller() {
        let mut versions = ContentEncodedVersions {
            uncompressed: vec![1, 2, 3, 4, 5, 6, 7, 8],
            brotli: Some(vec![1, 2, 3, 4, 5, 6, 7]),
            gzip: Some(vec![1, 2, 3, 4, 5, 6]),
            ..Default::default()
        };

        assert_eq!(versions.determine_smallest_file_size(), Some(ContentCoding::Gzip));

        versions.brotli = None;
        assert_eq!(versions.determine_smallest_file_size(), Some(ContentCoding::Gzip));
    }

    /// When both brotli and gzip are smaller than the uncompressed file, the
    /// file with the smallest size is preferred.
    #[test]
    pub fn determine_smallest_file_size_brotli_and_gzip_smaller() {
        let versions = ContentEncodedVersions {
            uncompressed: vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            brotli: Some(vec![1, 2, 3, 4, 5, 6, 7]),
            gzip: Some(vec![1, 2, 3, 4, 5, 6]),
            ..Default::default()
        };

        assert_eq!(versions.determine_smallest_file_size(), Some(ContentCoding::Gzip));
    }

    /// When both brotli and gzip are smaller than the uncompressed file, the
    /// file with the smallest size is preferred.
    #[test]
    pub fn determine_smallest_file_size_brotli_and_gzip_smaller_2() {
        let versions = ContentEncodedVersions {
            uncompressed: vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            brotli: Some(vec![1, 2, 3, 4, 5, 6, 7]),
            gzip: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            ..Default::default()
        };

        assert_eq!(versions.determine_smallest_file_size(), Some(ContentCoding::Brotli));
    }

    /// When both brotli and gzip are smaller than the uncompressed file, the
    /// brotli file is preferred.
    #[test]
    pub fn determine_smallest_file_size_brotli_preference() {
        let versions = ContentEncodedVersions {
            uncompressed: vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            brotli: Some(vec![1, 2, 3, 4, 5, 6, 7]),
            gzip: Some(vec![1, 2, 3, 4, 5, 6, 7]),
            ..Default::default()
        };

        assert_eq!(versions.determine_smallest_file_size(), Some(ContentCoding::Brotli));
    }

    /// When uncompressed is the same as brotli and gzip, the uncompressed file
    /// is preferred.
    #[test]
    pub fn determine_smallest_file_size_all_the_same_size() {
        let versions = ContentEncodedVersions {
            uncompressed: vec![1],
            brotli: Some(vec![1]),
            gzip: Some(vec![1]),
            ..Default::default()
        };

        assert_eq!(versions.determine_smallest_file_size(), None);
    }

    #[test]
    pub fn get_version() {
        let versions = ContentEncodedVersions {
            uncompressed: vec![1],
            brotli: Some(vec![2, 3]),
            gzip: Some(vec![4, 5, 6]),
            ..Default::default()
        };

        assert_eq!(versions.get_version(None), &versions.uncompressed);
        assert_eq!(versions.get_version(Some(ContentCoding::Brotli)), versions.brotli.as_ref().unwrap());
        assert_eq!(versions.get_version(Some(ContentCoding::Gzip)), versions.gzip.as_ref().unwrap());
    }

    #[test]
    pub fn determine_best_version_from_accept_encoding() {
        let versions = ContentEncodedVersions {
            uncompressed: vec![1],
            brotli: Some(vec![2, 3]),
            gzip: Some(vec![4, 5, 6]),
            ..Default::default()
        };

        assert_eq!(versions.determine_best_version_from_accept_encoding(""), None, "with empty accept-encoding, but uncompressed is still smaller");
        assert_eq!(versions.determine_best_version_from_accept_encoding("gzip, brotli, deflate"), None, "with accept-encoding, but uncompressed is still smaller");
        assert_eq!(versions.determine_best_version_from_accept_encoding("*;q=0.0"), None, "without encoding");
    }

    #[test]
    #[ignore = "fixme"]
    pub fn determine_best_version_from_accept_encoding_brotli() {
        let versions = ContentEncodedVersions {
            uncompressed: vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            brotli: Some(vec![1, 2, 3, 4, 5, 6, 7]),
            gzip: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            ..Default::default()
        };

        assert_eq!(versions.determine_best_version_from_accept_encoding("brotli"), Some(ContentCoding::Brotli), "with brotli");
        assert_eq!(versions.determine_best_version_from_accept_encoding("brotli, gzip"), Some(ContentCoding::Brotli), "with brotli and gzip");
        assert_eq!(versions.determine_best_version_from_accept_encoding("brotli, gzip;q=0.5"), Some(ContentCoding::Brotli), "with brotli and gzip;q=0.5");
        assert_eq!(versions.determine_best_version_from_accept_encoding("brotli;q=0.5, gzip"), Some(ContentCoding::Gzip), "with brotli;q=0.5 and gzip");
        assert_eq!(versions.determine_best_version_from_accept_encoding("brotli;q=0.5, gzip;q=0.5"), Some(ContentCoding::Brotli), "with brotli;q=0.5 and gzip;q=0.5");
        assert_eq!(versions.determine_best_version_from_accept_encoding("*;q=0.0"), None, "with *,q=0.0");
        assert_eq!(versions.determine_best_version_from_accept_encoding("*;q=0.0,gzip;q=1.0"), Some(ContentCoding::Gzip));
    }
}
