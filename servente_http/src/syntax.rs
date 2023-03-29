// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! This module contains HTTP syntax semantics, valid across all representations
//! of HTTP: textual HTTP/1.x, binary HTTP/2 and HTTP/3.
//!
//! # References
//! * [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html)

use crate::{
    abnf,
    HttpParseError,
};

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

/// Is the given character a character that can occur anywhere in the string?
///
/// # [HTTP Semantics (RFC 9110) Definitions](https://www.rfc-editor.org/rfc/rfc9110.html#name-uri-references)
/// ```text
/// URI-reference = <URI-reference, see [URI], Section 4.1>
/// absolute-URI  = <absolute-URI, see [URI], Section 4.3>
/// relative-part = <relative-part, see [URI], Section 4.2>
/// authority     = <authority, see [URI], Section 3.2>
/// uri-host      = <host, see [URI], Section 3.2.2>
/// port          = <port, see [URI], Section 3.2.3>
/// path-abempty  = <path-abempty, see [URI], Section 3.3>
/// segment       = <segment, see [URI], Section 3.3>
/// query         = <query, see [URI], Section 3.4>
///
/// absolute-path = 1*( "/" segment )
/// partial-URI   = relative-part [ "?" query ]
/// ```
///
/// # [HTTP/1.1 (RFC 9112) Definitions](https://www.rfc-editor.org/rfc/rfc9112.html#name-request-target)
/// ```text
/// request-target = origin-form
///                / absolute-form
///                / authority-form
///                / asterisk-form
/// origin-form    = absolute-path [ "?" query ]
/// absolute-form  = absolute-URI
/// authority-form = uri-host ":" port
/// asterisk-form  = "*"
/// ```
///
/// # [URI (RFC 3986) Definitions](https://www.rfc-editor.org/rfc/rfc3986.html)
/// ```text
/// absolute-URI  = scheme ":" hier-part [ "?" query ]
/// [...]
/// ```
pub fn is_request_target_character(byte: u8) -> bool {
    !matches!(byte, 0x00..=0x1F | 0x80..=0xFF)
}

/// Is the given character a character that can occur (anywhere) in the string?
/// This is useful for early exits, but use [`validate_token`] after the
/// token is parsed.
///
/// ```text
/// tchar          = "!" / "#" / "$" / "%" / "&" / "'" / "*"
///                / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
///                / DIGIT / ALPHA
///                ; any VCHAR, except delimiters
/// ```
#[inline]
pub fn is_token_character(byte: u8) -> bool {
    validate_token_character(byte).is_ok()
}

/// Returns whether or not the character is whitespace according to the HTTP
/// specification. This is in effect just `U+0020 SPACE` and `U+0009 CHARACTER
/// TABULATION`.
///
/// This isn't an actual definition, but used in the `OWS` (optional whitespace),
/// `RWS` (required whitespace) and `BWS` (bad whitespace).
///
/// # Definition
/// ```text
/// OWS            = *( SP / HTAB )
///                ; optional whitespace
/// RWS            = 1*( SP / HTAB )
///                ; required whitespace
/// BWS            = OWS
///                ; "bad" whitespace
/// ```
///
/// # References
/// * [RFC 9110 Section 5.6.3](https://www.rfc-editor.org/rfc/rfc9110.html#name-whitespace)
#[inline]
pub fn is_whitespace_character(character: char) -> bool {
    character == ' ' || character == '\t'
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
