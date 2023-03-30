// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! This module provides support for the HTTP Lists `#rule` ABNF extension.
//!
//! # Definition for Recipients
//! ```text
//! #element => [ element ] *( OWS "," OWS [ element ] )
//! ```
//!
//! # References
//! * [RFC 9110 Section 5.6.1](https://www.rfc-editor.org/rfc/rfc9110.html#section-5.6.1)

use unicase::UniCase;

use crate::{syntax::is_whitespace_character, abnf};

struct HttpListElementIterator<'a> {
    value: &'a str,
}

impl<'a> Iterator for HttpListElementIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.value.is_empty() {
            return None;
        }

        while let Some((element, rest)) = self.value.split_once(',') {
            self.value = rest.trim_matches(is_whitespace_character);

            let result = element.trim_matches(is_whitespace_character);
            if !result.is_empty() {
                return Some(result);
            }
        }

        if self.value.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.value))
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct HttpWeightedValue<'a> {
    pub name: &'a str,
    pub weight: f32,
}

struct HttpWeightedListValueIterator<'a> {
    inner: HttpListElementIterator<'a>,
}

impl<'a> Iterator for HttpWeightedListValueIterator<'a> {
    type Item = HttpWeightedValue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(value) = self.inner.next() else {
            return None;
        };

        let mut split = value.split(';');
        let name = split.next()?;

        let mut weight = 1.0;
        if let Some(qvalue) = split.next() {
            if let Some((_prefix, value)) = qvalue.trim_start_matches(is_whitespace_character).split_once("q=") {
                weight = parse_quality_value(value);
            }
        }

        Some(HttpWeightedValue { name, weight })
    }
}

/// Finds the best matching element in the given field-value of an header,
/// returning `None` if no such match could be found and the .
///
/// Assumes a case-sensitive token, for example as for `content-coding`.
pub fn find_best_match_in_weighted_list(header_field_value: &str, supported_values: &[&str], default_wildstart_weight: f32) -> Option<usize> {
    let list: Vec<_> = parse_http_weighted_list(header_field_value)
        .collect();

    // The weight of values that aren't specified in the header list. This can
    // be disabled by the user agent by specifying "*" with a weight of 0.0:
    // ```text
    // gzip, *;q=0.0
    // ```
    let non_header_specified_element_weigth = list.iter()
        .filter(|entry| entry.name == "*")
        .take(1)
        .map(|entry| entry.weight)
        .next().unwrap_or(default_wildstart_weight);

    let mut best_match: Option<(usize, f32)> = None;

    for (supported_value_index, supported_value_name) in supported_values.iter().enumerate() {
        let field_weight = list.iter()
                .filter(|entry| UniCase::ascii(supported_value_name) == UniCase::ascii(entry.name))
                .map(|entry| entry.weight)
                .next().unwrap_or(non_header_specified_element_weigth);

        if field_weight == 0.0 {
            continue;
        }

        let better = if let Some((_, best_weight)) = best_match {
            best_weight < field_weight
        } else {
            true
        };

        if better {
            best_match = Some((supported_value_index, field_weight));
        }
    }

    best_match.map(|(best_supported_values_index, _) | best_supported_values_index)
}

/// This function parses a field-value and returns an iterator of list elements
/// for HTTP. The iterator will never return the empty string, as those cannot
/// occur in HTTP lists and will be ignored.
///
/// # Definition for Recipients
/// ```text
/// #element => [ element ] *( OWS "," OWS [ element ] )
/// ```
///
/// # References
/// * [RFC 9110 Section 5.6.1](https://www.rfc-editor.org/rfc/rfc9110.html#section-5.6.1)
pub fn parse_http_list(value: &str) -> impl Iterator<Item = &'_ str> {
    HttpListElementIterator { value }
}

/// This function parses a field-value and returns an iterator of weighted list
/// elements. Weights are optional, and will default to `1.0`. Multi-parameter
/// values are not supported, and in the best case this will parse only the
/// quality property (`q`).
///
/// # Definition
/// ```text
/// weight = OWS ";" OWS "q=" qvalue
/// qvalue = ( "0" [ "." 0*3DIGIT ] )
///        / ( "1" [ "." 0*3("0") ] )
/// ```
///
/// # References
/// * [RFC 9110 Section 12.4.2](https://www.rfc-editor.org/rfc/rfc9110.html#name-quality-values)
pub fn parse_http_weighted_list(value: &str) -> impl Iterator<Item = HttpWeightedValue<'_>> {
    HttpWeightedListValueIterator {
        inner: HttpListElementIterator { value }
    }
}

/// Parses a `qvalue` as defined by
/// [RFC 9110, section 12.4.2](https://www.rfc-editor.org/rfc/rfc9110.html#name-quality-values).
///
/// # Definition
/// ```text
/// qvalue = ( "0" [ "." 0*3DIGIT ] )
///        / ( "1" [ "." 0*3("0") ] )
/// ```
///
/// # Invalid Syntax
/// The sender MUST NOT generate these values, but there isn't an explicit
/// definition of what should be done if an endpoint receives these. In that
/// case, we should go with the default value of `1.0`.
///
/// # References
/// * [RFC 9110 Section 12.4.2](https://www.rfc-editor.org/rfc/rfc9110.html#name-quality-values)
fn parse_quality_value(value: &str) -> f32 {
    const DEFAULT_VALUE_FOR_INVALID_SYNTAX: f32 = 1.0;

    // Length restrictions
    if value.is_empty() || value.len() > 5 {
        return DEFAULT_VALUE_FOR_INVALID_SYNTAX;
    }

    if matches!(value, "0" | "0." | "0.0") {
        return 0.0;
    }

    let first_character = value.chars().next().unwrap();

    // This covers valid cases and invalid cases, since it can never be more
    // than `1.0`.
    if first_character == '1' {
        return 1.0;
    }

    if first_character != '0' || value.len() == 1 || value.chars().nth(1).unwrap() != '.' {
        return DEFAULT_VALUE_FOR_INVALID_SYNTAX;
    }

    let mut fractional = 0.0;
    for (idx, character) in value[2..].char_indices().take(3) {
        if let Some(digit) = abnf::parse_digit_character(character) {
            fractional += digit as f32 * 10_f32.powi(-(idx as i32 + 1));
        }
    }

    fractional
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use float_cmp::approx_eq;

    #[rstest]
    #[case("gzip, deflate, br", &["br", "gzip", "deflate"], Some(0))]
    #[case("br", &["br", "gzip", "deflate"], Some(0))]
    #[case("gzip, deflate, br", &[], None)]
    #[case("", &["br"], Some(0))]
    #[case("", &["br", "gzip", "brotli"], Some(0))]
    #[case("", &[], None)]
    #[case("    ,    ", &[], None)]
    #[case("*;q=0.0", &["gzip", "br", "deflate"], None)]
    #[case("*;q=0.0, gzip;q=0.001", &["br", "deflate", "gzip"], Some(2))]
    fn test_find_best_match_in_weighted_list(#[case] input_user_agent: &str,
            #[case] input_server: &[&str], #[case] expected: Option<usize>) {
        assert_eq!(find_best_match_in_weighted_list(input_user_agent, input_server, 1.0), expected);
    }

    #[rstest]
    #[case("en-US", &["en-US"])]
    #[case("foo,bar", &["foo", "bar"])]
    #[case("foo , bar,", &["foo", "bar"])]
    #[case("foo , ,bar,charlie", &["foo", "bar", "charlie"])]
    #[case("", &[])]
    #[case(",", &[])]
    #[case(",     ,", &[])]
    #[case(",     ,  ", &[])]
    #[case("gzip, br, deflate", &["gzip", "br", "deflate"])]
    fn test_parse_http_list(#[case] input: &str, #[case] expected: &[&str]) {
        assert_eq!(parse_http_list(input).collect::<Vec<&str>>(), expected.to_vec());
    }

    #[rstest]
    #[case("gzip", &[HttpWeightedValue{ name: "gzip", weight: 1.0 }])]
    #[case("*", &[HttpWeightedValue{ name: "*", weight: 1.0 }])]
    #[case("*;q=1.0", &[HttpWeightedValue{ name: "*", weight: 1.0 }])]
    #[case("*;q=0.001", &[HttpWeightedValue{ name: "*", weight: 0.001 }])]
    #[case("*;q=0.0", &[HttpWeightedValue{ name: "*", weight: 0.0 }])]
    #[case("gzip;q=0.0", &[HttpWeightedValue{ name: "gzip", weight: 0.0 }])]
    #[case("gzip;q=0.5", &[HttpWeightedValue{ name: "gzip", weight: 0.5 }])]
    #[case("*;q=0.0, gzip;q=0.001", &[HttpWeightedValue{ name: "*", weight: 0.0 }, HttpWeightedValue{ name: "gzip", weight: 0.001 }])]
    #[case("gzip, br, deflate", &[HttpWeightedValue{ name: "gzip", weight: 1.0 },
            HttpWeightedValue{ name: "br", weight: 1.0 }, HttpWeightedValue{ name: "deflate", weight: 1.0 }])]
    #[case("gzip, br;q=0.1, deflate;q=not-a-weight", &[HttpWeightedValue{ name: "gzip", weight: 1.0 },
            HttpWeightedValue{ name: "br", weight: 0.1 }, HttpWeightedValue{ name: "deflate", weight: 1.0 }])]
    #[case("gzip;q=1.000, br;q=0.5, deflate;q=0.1, *;q=0.0", &[HttpWeightedValue{ name: "gzip", weight: 1.0 },
            HttpWeightedValue{ name: "br", weight: 0.5 }, HttpWeightedValue{ name: "deflate", weight: 0.1 },
            HttpWeightedValue{ name: "*", weight: 0.0 }])]
    fn test_parse_http_weighted_list(#[case] input: &str, #[case] expected: &[HttpWeightedValue<'static>]) {
        assert_eq!(parse_http_weighted_list(input), expected.to_vec());
    }

    fn parse_http_weighted_list(input: &str) -> Vec<HttpWeightedValue<'_>> {
        super::parse_http_weighted_list(input).collect()
    }

    #[rstest]
    #[case("0", 0.0)]
    #[case("0.", 0.0)]
    #[case("0.0", 0.0)]
    #[case("0.00", 0.0)]
    #[case("0.000", 0.0)]
    #[case("1", 1.0)]
    #[case("1.", 1.0)]
    #[case("1.0", 1.0)]
    #[case("1.00", 1.0)]
    #[case("1.000", 1.0)]
    #[case("0.5", 0.5)]
    #[case("0.05", 0.05)]
    #[case("0.001", 0.001)]
    #[case("0.123", 0.123)]
    #[case("0.089", 0.089)]
    fn test_parse_quality_value_valid(#[case] input: &str, #[case] expected: f32) {
        let outcome = parse_quality_value(input);
        assert!(approx_eq!(f32, outcome, expected, ulps = 3), "Incorrect, outcome={outcome}, expected={expected} for input=\"{input}\"");
    }
    #[rstest]
    #[case("ABCFDGNSDG")]
    #[case("")]
    #[case("-0.0")]
    #[case("+0.0")]
    #[case("+0")]
    #[case("+1")]
    #[case("-1.0")]
    #[case("0.000005")]
    #[case("-.582")]
    #[case("1.5")]
    #[case("2")]
    #[case("2.0")]
    #[case("2.001")]
    fn test_parse_quality_value_invalid(#[case] input: &str) {
        let outcome = parse_quality_value(input);
        assert!(approx_eq!(f32, outcome, 1.0, ulps = 3), "Incorrect, outcome={outcome}");
    }

}
