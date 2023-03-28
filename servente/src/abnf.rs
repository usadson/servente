// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! Various Augmented BNF (ABNF) tools and utilities.
//!
//! # Ranges
//! Ranges are inclusive:
//! ```text
//! DIGIT       =  %x30-39
//! ```
//! is equivalent to:
//! ```text
//! DIGIT       =  "0" / "1" / "2" / "3" / "4" / "5" / "6" /
//!                "7" / "8" / "9"
//! ```
//!
//! # References
//! * [RFC 5234 Augmented BNF for Syntax Specifications: ABNF](https://www.rfc-editor.org/rfc/rfc5234.html)

/// Is the character a visible (printing) character.
///
/// ```text
/// VCHAR          =  %x21-7E
/// ```
#[inline]
pub fn is_visible_character(byte: u8) -> bool {
    matches!(byte, 0x21..=0x7E)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(b'!', true)]
    #[case(b'!', true)]
    #[case(b'0', true)]
    #[case(b'9', true)]
    #[case(b'a', true)]
    #[case(b'z', true)]
    #[case(b'A', true)]
    #[case(b'B', true)]
    #[case(b' ', false)]
    #[case(b'\t', false)]
    #[case(b'\r', false)]
    #[case(b'\n', false)]
    #[case(b'~', true)]
    #[case(0x00, false)]
    #[case(0x01, false)]
    #[case(0x1F, false)]
    #[case(0x7F, false)]
    fn test_is_visible_character(#[case] character: u8, #[case] expected: bool) {
        assert_eq!(is_visible_character(character), expected);
    }
}
