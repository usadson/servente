// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

pub mod cache;
pub mod compression;
pub mod static_res;

use phf::phf_map;
use unicase::UniCase;

pub use compression::ContentCoding;

pub const FILE_JPEG_MAGIC_NUMBER: &'static [u8; 3] = &[0xFF, 0xD8, 0xFF];
pub const FILE_PNG_MAGIC_NUMBER: &'static [u8; 8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MediaType {
    Common(&'static str),
    #[allow(unused)]
    Custom(String),
}

impl MediaType {
    pub fn as_str(&self) -> &str {
        match self {
            MediaType::Common(s) => s,
            MediaType::Custom(s) => s,
        }
    }
}

impl MediaType {
    //
    // General
    //
    pub const OCTET_STREAM: MediaType = MediaType::Common("application/octet-stream");

    //
    // Text
    //
    pub const CASCADING_STYLE_SHEETS: MediaType = MediaType::Common("text/css; charset=utf-8");
    pub const HTML: MediaType = MediaType::Common("text/html; charset=utf-8");
    pub const JAVASCRIPT: MediaType = MediaType::Common("text/javascript; charset=utf-8");
    pub const MARKDOWN: MediaType = MediaType::Common("text/markdown; charset=utf-8");
    pub const PLAIN_TEXT: MediaType = MediaType::Common("text/plain; charset=utf-8");
    pub const YAML: MediaType = MediaType::Common("text/yaml; charset=utf-8");


    //
    // Application
    //
    pub const JSON: MediaType = MediaType::Common("application/json; charset=utf-8");
    pub const PDF: MediaType = MediaType::Common("application/pdf");
    pub const ZIP: MediaType = MediaType::Common("application/zip");
    pub const GZIP: MediaType = MediaType::Common("application/gzip");
    pub const BZIP2: MediaType = MediaType::Common("application/x-bzip2");
    pub const XZ: MediaType = MediaType::Common("application/x-xz");
    pub const TAR: MediaType = MediaType::Common("application/x-tar");
    pub const XML: MediaType = MediaType::Common("application/xml; charset=utf-8");
    pub const ATOM: MediaType = MediaType::Common("application/atom+xml; charset=utf-8");
    pub const RSS: MediaType = MediaType::Common("application/rss+xml; charset=utf-8");

    //
    // Image
    //
    pub const GIF: MediaType = MediaType::Common("image/gif");
    pub const ICO: MediaType = MediaType::Common("image/x-icon");
    pub const JPEG: MediaType = MediaType::Common("image/jpeg");
    pub const PNG: MediaType = MediaType::Common("image/png");
    pub const SVG: MediaType = MediaType::Common("image/svg+xml");
    pub const WEBP: MediaType = MediaType::Common("image/webp");

    //
    // Audio
    //
    pub const MP3: MediaType = MediaType::Common("audio/mpeg");
    pub const WAV: MediaType = MediaType::Common("audio/wav");
    pub const OGG_AUDIO: MediaType = MediaType::Common("audio/ogg");

    //
    // Video
    //
    pub const MP4: MediaType = MediaType::Common("video/mp4");
    pub const WEBM: MediaType = MediaType::Common("video/webm");
    pub const OGG_VIDEO: MediaType = MediaType::Common("video/ogg");
    pub const QUICKTIME: MediaType = MediaType::Common("video/quicktime");
    pub const MPEG: MediaType = MediaType::Common("video/mpeg");
    pub const AVI: MediaType = MediaType::Common("video/x-msvideo");
    pub const FLV: MediaType = MediaType::Common("video/x-flv");
    pub const WMV: MediaType = MediaType::Common("video/x-ms-wmv");

    //
    // Font
    //
    pub const WOFF: MediaType = MediaType::Common("font/woff");
    pub const WOFF2: MediaType = MediaType::Common("font/woff2");
    pub const TTF: MediaType = MediaType::Common("font/ttf");
    pub const OTF: MediaType = MediaType::Common("font/otf");
    pub const EOT: MediaType = MediaType::Common("font/eot");
    pub const SFNT: MediaType = MediaType::Common("font/sfnt");
    pub const SVG_FONT: MediaType = MediaType::Common("font/svg");

    /// Returns the media type for the given extension.
    #[must_use]
    pub fn from_extension(extension: &str) -> &'static MediaType {
        MEDIA_TYPE_BY_EXTENSION.get(&UniCase::ascii(extension)).unwrap_or(&MediaType::OCTET_STREAM)
    }

    #[must_use]
    pub fn from_path(path: &str) -> &'static MediaType {
        let extension = path.rsplit('.').next().unwrap_or("");
        MediaType::from_extension(extension)
    }
}

static MEDIA_TYPE_BY_EXTENSION: phf::Map<UniCase<&'static str>, MediaType> = phf_map!(
    UniCase::ascii("css") => MediaType::CASCADING_STYLE_SHEETS,
    UniCase::ascii("htm") => MediaType::HTML,
    UniCase::ascii("html") => MediaType::HTML,
    UniCase::ascii("js") => MediaType::JAVASCRIPT,
    UniCase::ascii("md") => MediaType::MARKDOWN,
    UniCase::ascii("txt") => MediaType::PLAIN_TEXT,
    UniCase::ascii("yaml") => MediaType::YAML,

    UniCase::ascii("json") => MediaType::JSON,
    UniCase::ascii("pdf") => MediaType::PDF,
    UniCase::ascii("zip") => MediaType::ZIP,
    UniCase::ascii("gz") => MediaType::GZIP,
    UniCase::ascii("bz2") => MediaType::BZIP2,
    UniCase::ascii("xz") => MediaType::XZ,
    UniCase::ascii("tar") => MediaType::TAR,
    UniCase::ascii("xml") => MediaType::XML,
    UniCase::ascii("atom") => MediaType::ATOM,
    UniCase::ascii("rss") => MediaType::RSS,

    UniCase::ascii("gif") => MediaType::GIF,
    UniCase::ascii("ico") => MediaType::ICO,
    UniCase::ascii("jpeg") => MediaType::JPEG,
    UniCase::ascii("jpg") => MediaType::JPEG,
    UniCase::ascii("png") => MediaType::PNG,
    UniCase::ascii("svg") => MediaType::SVG,
    UniCase::ascii("webp") => MediaType::WEBP,

    UniCase::ascii("mp3") => MediaType::MP3,
    UniCase::ascii("wav") => MediaType::WAV,
    UniCase::ascii("ogg") => MediaType::OGG_AUDIO,

    UniCase::ascii("mp4") => MediaType::MP4,
    UniCase::ascii("webm") => MediaType::WEBM,
    UniCase::ascii("ogv") => MediaType::OGG_VIDEO,
    UniCase::ascii("mov") => MediaType::QUICKTIME,
    UniCase::ascii("mpeg") => MediaType::MPEG,
    UniCase::ascii("avi") => MediaType::AVI,
    UniCase::ascii("flv") => MediaType::FLV,
    UniCase::ascii("wmv") => MediaType::WMV,

    UniCase::ascii("woff") => MediaType::WOFF,
    UniCase::ascii("woff2") => MediaType::WOFF2,
    UniCase::ascii("ttf") => MediaType::TTF,
    UniCase::ascii("otf") => MediaType::OTF,
    UniCase::ascii("eot") => MediaType::EOT,
    UniCase::ascii("sfnt") => MediaType::SFNT,
    UniCase::ascii("svgf") => MediaType::SVG_FONT,
);

/// Extensions that are not allowed to be served. This is to prevent serving
/// files that are not meant to be served. This list is not exhaustive, but it
/// acts as a last line of defense.
///
/// ## Important Note
/// In no way should this list be considered as something to rely on. It is
/// possible to circumvent this list by using a different extension, but can be
/// a last line of defense.
///
/// Even so, the file extensions is not a reliable way to determine the file
/// type. For example, a file with the extension `.txt` could be a plain text
/// file, or it could contain sensitive information that should never be
/// served, but was given the `.txt` extension erroneously.
static DISALLOWED_RESPONSE_FILE_EXTENSIONS: phf::Set<UniCase<&'static str>> = phf::phf_set!(
    UniCase::ascii("crdownload"),       // Chrome temporary files
    UniCase::ascii("db"),               // Database files
    UniCase::ascii("der"),              // Certificate files
    UniCase::ascii("gitignore"),        // Git ignore files (might be used to find hidden files)
    UniCase::ascii("key"),              // Certificate files
    UniCase::ascii("lock"),             // Lock files
    UniCase::ascii("log"),              // Log files
    UniCase::ascii("part"),             // Partial downloads
    UniCase::ascii("pem"),              // Certificate files
    UniCase::ascii("tmp"),              // Temporary files
    UniCase::ascii("temp"),             // Temporary files
    UniCase::ascii("vim"),              // Vim temporary files
    UniCase::ascii("ytdl"),             // youtube-dl temporary files
);

/// Returns `true` if the file is allowed to be served.
///
/// This is to prevent serving files that are not meant to be served. The list
/// is not exhaustive, but it should cover most cases.
///
/// ## Important Note
/// In no way should this list be considered as something to rely on. It is
/// possible to circumvent this list by using a different extension, but can be
/// a last line of defense.
///
/// Even so, the file extensions is not a reliable way to determine the file
/// type. For example, a file with the extension `.txt` could be a plain text
/// file, or it could contain sensitive information that should never be
/// served, but was given the `.txt` extension erroneously.
///
/// ## TODO
/// 1. There should be a way to explicitly allow a file to be served.
pub fn is_file_allowed_to_be_served(path: &str) -> bool {
    match path.rsplit('.').next() {
        Some(extension) => !DISALLOWED_RESPONSE_FILE_EXTENSIONS.contains(&UniCase::ascii(extension)),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_file_allowed_to_be_served() {
        assert!(is_file_allowed_to_be_served("/"));
        assert!(is_file_allowed_to_be_served("/index.html"));
        assert!(is_file_allowed_to_be_served("/index.html"));
        assert!(is_file_allowed_to_be_served("/index.html"));
        assert!(is_file_allowed_to_be_served("/this/is-a/path/to/a/file.html"));
        assert!(is_file_allowed_to_be_served("/test.php"));
        assert!(is_file_allowed_to_be_served("/test.php?file=../file.txt"));
        assert!(is_file_allowed_to_be_served("file.txt"));
        assert!(is_file_allowed_to_be_served("file"));
    }

    #[test]
    fn test_is_file_disallowed_to_be_served() {
        assert!(!is_file_allowed_to_be_served("/this/is-a/path/to/a/file.tmp"));
        assert!(!is_file_allowed_to_be_served("file.log"));
        assert!(!is_file_allowed_to_be_served("/index.part"));
        assert!(!is_file_allowed_to_be_served("file.part"));
        assert!(!is_file_allowed_to_be_served("file.ytdl"));
    }
}
