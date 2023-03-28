// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

#[derive(Clone, Debug, Default, PartialEq, PartialOrd)]
pub struct AcceptedLanguages<'a> {
    ranges: Vec<LanguageRange<'a>>,
}

impl<'a> AcceptedLanguages<'a> {
    pub fn parse(value: &'a str) -> Option<Self> {
        let mut ranges = Vec::new();
        for range in value.split(',') {
            let mut parts = range.split(';');
            let language = parts.next()?;
            let q = parts
                .next()
                .and_then(|part| part.strip_prefix("q="))
                .and_then(|q| q.parse().ok())
                .unwrap_or(1.0);
            ranges.push(LanguageRange { language, q });
        }
        ranges.sort_by(|a, b| b.q.partial_cmp(&a.q).unwrap_or(std::cmp::Ordering::Equal));
        Some(AcceptedLanguages { ranges })
    }

    pub fn match_best(&self, language: Vec<&'a str>) -> Option<&'a str> {
        for range in &self.ranges{
            for lang in &language {
                if range.language == *lang {
                    return Some(range.language);
                }
            }
        }
        language.first().copied()
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd)]
pub struct LanguageRange<'a> {
    pub language: &'a str,
    pub q: f32,
}
