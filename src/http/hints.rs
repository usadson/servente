// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

/// The `Sec-Fetch-Dest` header indicates the destination of the request.
///
/// ### References
/// * [MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Sec-Fetch-Dest)
/// * [Specification](https://wicg.github.io/sec-fetch-metadata/#sec-fetch-dest-header)
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecFetchDest {
    Audio,
    AudioWorklet,
    Document,
    Embed,
    #[default]
    Empty,
    Font,
    Frame,
    Iframe,
    Image,
    Manifest,
    Object,
    PaintWorklet,
    Report,
    Script,
    ServiceWorker,
    SharedWorker,
    Style,
    Track,
    Video,
    Worker,
    Xslt,
}

impl SecFetchDest {
    /// Parse a `SecFetchDest` from a string.
    ///
    /// ### Note
    /// This value is a `token` as defined by *RFC 8941*¹, meaning it isn't
    /// case-sensitive.
    ///
    /// ### References
    /// 1. [RFC 8941](https://datatracker.ietf.org/doc/html/rfc8941#name-parsing-a-token)
    /// 2. [MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Sec-Fetch-Dest)
    /// 3. [Specification](https://wicg.github.io/sec-fetch-metadata/#sec-fetch-dest-header)
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "audio" => SecFetchDest::Audio,
            "audioworklet" => SecFetchDest::AudioWorklet,
            "document" => SecFetchDest::Document,
            "embed" => SecFetchDest::Embed,
            "empty" => SecFetchDest::Empty,
            "font" => SecFetchDest::Font,
            "frame" => SecFetchDest::Frame,
            "iframe" => SecFetchDest::Iframe,
            "image" => SecFetchDest::Image,
            "manifest" => SecFetchDest::Manifest,
            "object" => SecFetchDest::Object,
            "paintworklet" => SecFetchDest::PaintWorklet,
            "report" => SecFetchDest::Report,
            "script" => SecFetchDest::Script,
            "serviceworker" => SecFetchDest::ServiceWorker,
            "sharedworker" => SecFetchDest::SharedWorker,
            "style" => SecFetchDest::Style,
            "track" => SecFetchDest::Track,
            "video" => SecFetchDest::Video,
            "worker" => SecFetchDest::Worker,
            "xslt" => SecFetchDest::Xslt,
            _ => return None,
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SecFetchDest::Audio => "audio",
            SecFetchDest::AudioWorklet => "audioworklet",
            SecFetchDest::Document => "document",
            SecFetchDest::Embed => "embed",
            SecFetchDest::Empty => "empty",
            SecFetchDest::Font => "font",
            SecFetchDest::Frame => "frame",
            SecFetchDest::Iframe => "iframe",
            SecFetchDest::Image => "image",
            SecFetchDest::Manifest => "manifest",
            SecFetchDest::Object => "object",
            SecFetchDest::PaintWorklet => "paintworklet",
            SecFetchDest::Report => "report",
            SecFetchDest::Script => "script",
            SecFetchDest::ServiceWorker => "serviceworker",
            SecFetchDest::SharedWorker => "sharedworker",
            SecFetchDest::Style => "style",
            SecFetchDest::Track => "track",
            SecFetchDest::Video => "video",
            SecFetchDest::Worker => "worker",
            SecFetchDest::Xslt => "xslt",
        }
    }
}

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


