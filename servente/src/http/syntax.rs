// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! This module contains HTTP syntax semantics, valid across all representations
//! of HTTP: textual HTTP/1.x, binary HTTP/2 and HTTP/3.
//!
//! # References
//! * [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html)

use crate::abnf;

use super::error::HttpParseError;

/// Validate a field character. Note that in HTTP, UTF-8 is optional (US-ASCII),
/// and isn't used before validating the content.
///
/// ```text
/// field-vchar    = VCHAR / obs-text
/// ```
#[inline]
fn is_field_value_character(byte: u8) -> bool {
    abnf::is_visible_character(byte) || validate_obs_text(byte)
}

/// Validate obs-text.
/// ```text
/// obs-text       = %x80-FF
/// ```
#[inline]
fn validate_obs_text(byte: u8) -> bool {
    matches!(byte, 0x80..=0xFF)
}

/// Validate a field character. Note that in HTTP, UTF-8 is optional (US-ASCII),
/// and isn't used before validating the content.
pub fn validate_field_content(value: &[u8]) -> Result<(), HttpParseError> {
    if value.iter().all(|byte| is_field_value_character(*byte) || *byte == b' ' || *byte == b'\t') {
        Ok(())
    } else {
        Err(HttpParseError::FieldValueContainsInvalidCharacters)
    }
}

pub fn validate_token(value: &str) -> Result<(), HttpParseError> {
    if value.is_empty() {
        return Err(HttpParseError::TokenEmpty);
    }

    for character in value.bytes() {
        validate_token_character(character)?;
    }

    Ok(())
}

/// Validate a token character.
///
/// ```text
/// tchar          = "!" / "#" / "$" / "%" / "&" / "'" / "*"
///                / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
///                / DIGIT / ALPHA
///                ; any VCHAR, except delimiters
/// ```
fn validate_token_character(character: u8) -> Result<(), HttpParseError> {
    match character {
        b' ' | b'\t' => Err(HttpParseError::TokenContainsWhitespace),

        b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' |
        b'^' | b'_' | b'`' | b'|' | b'~' => Ok(()),

        b'0'..=b'9' => Ok(()),
        b'A'..=b'Z' => Ok(()),
        b'a'..=b'z' => Ok(()),

        b'"' | b'(' | b')' | b',' | b'/' | b':' | b';' | b'<' | b'=' | b'>' |
        b'?' | b'@' | b'[' | b'\\' | b']' | b'{' | b'}' => Err(HttpParseError::TokenContainsDelimiter),

        _ => Err(HttpParseError::TokenContainsNonVisibleAscii),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(0x00, false)]
    #[case(0x10, false)]
    #[case(0x1F, false)]
    #[case(b'A', true)]
    #[case(b'z', true)]
    #[case(b'0', true)]
    #[case(b'9', true)]
    #[case(0xFF, true)]
    #[test]
    fn test_is_field_value_character(#[case] input: u8, #[case] expected: bool) {
        assert_eq!(is_field_value_character(input), expected, "character isn't matching: {}", input);
    }

    #[test]
    fn test_validate_token() {
        assert_eq!(validate_token(""), Err(HttpParseError::TokenEmpty));
        assert_eq!(validate_token("hello"), Ok(()));
        assert_eq!(validate_token(" hello"), Err(HttpParseError::TokenContainsWhitespace));
        assert_eq!(validate_token("hello "), Err(HttpParseError::TokenContainsWhitespace));
        assert_eq!(validate_token("hel lo"), Err(HttpParseError::TokenContainsWhitespace));
    }

    #[rstest]
    #[case(b' ', Err(HttpParseError::TokenContainsWhitespace))]
    #[case(b'\t', Err(HttpParseError::TokenContainsWhitespace))]
    #[case(b'!', Ok(()))]
    #[case(b'"', Err(HttpParseError::TokenContainsDelimiter))]
    #[case(0x00, Err(HttpParseError::TokenContainsNonVisibleAscii))]
    #[case(b'~', Ok(()))]
    #[case(0x7F, Err(HttpParseError::TokenContainsNonVisibleAscii))]
    fn test_validate_token_character(#[case] input: u8, #[case] expected: Result<(), HttpParseError>) {
        assert_eq!(validate_token_character(input), expected);
    }
}
