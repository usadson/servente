// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! This module can transform CommonMark to HTML files.
//!
//! # References
//! * [CommonMark Spec 0.30](https://spec.commonmark.org/0.30)

/// The fallback name of the document, if a more appropriate one can't be found.
const FALLBACK_DOCUMENT_TITLE: &str = "Document";

struct Converter<'c> {
    input: &'c str,

    /// The title of the document.
    title: Option<String>,

    /// The output of the data, for placing into the `<body>` HTML element.
    body: String,
}

impl<'c> Converter<'c> {
    pub fn new(input: &'c str) -> Self {
        Self {
            input,
            title: None,
            body: String::new(),
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_html(&mut self) -> String {
        let mut first_heading = true;
        for line in self.input.lines() {
            match Token::from(line) {
                Token::AtxHeading { level, content } => {
                    if first_heading && level == 1 {
                        self.title = Some(content.to_string());
                    }
                    first_heading = false;

                    self.body += &format!("<h{level}>{content}</h{level}>");
                }
                Token::Content(content) => {
                    self.body += "<p>";
                    self.body += content;
                    self.body += "</p>";
                }
                Token::ThematicBreak => {
                    self.body += "<hr>";
                }
            }
        }

        format!(
            concat!(
                "<!DOCTYPE html>",
                "<html lang=\"en\">",
                "<head>",
                "<meta charset=\"utf-8\">",
                "<meta name=\"generator\" content=\"Servente\">",
                "<title>{title}</title>",
                "</head>",
                "<body>{body}</body>",
                "</html>",
            ),
            title = match &self.title {
                Some(title) => title,
                None => FALLBACK_DOCUMENT_TITLE,
            },
            body = self.body
        )
    }
}

/// Convert the CommonMark file to HTML.
pub fn convert_to_html(input: &str) -> String {
    Converter::new(input).to_html()
}

enum Token<'i> {
    AtxHeading {
        level: u8,
        content: &'i str,
    },

    /// Normal content
    Content(&'i str),

    /// # References
    /// * [CommonMark Section 4.1](https://spec.commonmark.org/0.30/#thematic-breaks)
    ThematicBreak,
}

impl<'i> From<&'i str> for Token<'i> {
    fn from(value: &'i str) -> Self {
        let space_trimmed_start = value.trim_start_matches(|c| c == ' ');

        if space_trimmed_start.starts_with('#') {
            // Heading
            if let Some((prefix, rest)) = space_trimmed_start.split_once(|c: char| c.is_whitespace()) {
                let level = prefix.chars().filter(|c| *c == '#').count();
                if level != prefix.chars().count() || level > 6 {
                    return Token::Content(value);
                }
                return Token::AtxHeading { level: level as _, content: rest };
            }

            let level = space_trimmed_start.chars().filter(|c| *c == '#').count();
            if level != space_trimmed_start.chars().count()  {
                return Token::ThematicBreak;
            }
        }

        Token::Content(value)
    }
}
