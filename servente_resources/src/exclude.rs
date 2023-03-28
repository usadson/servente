// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! This module contains functions for determining whether or not a file should
//! be excluded from caching and serving to clients.

use unicase::UniCase;

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
