// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

///    A brief, single-entry Atom Feed Document:
///
/// ```xml
/// <?xml version="1.0" encoding="utf-8"?>
/// <feed xmlns="http://www.w3.org/2005/Atom">
///
///   <title>Example Feed</title>
///   <link href="http://example.org/"/>
///   <updated>2003-12-13T18:30:02Z</updated>
///   <author>
///     <name>John Doe</name>
///   </author>
///   <id>urn:uuid:60a76c80-d399-11d9-b93C-0003939e0af6</id>
///
///   <entry>
///     <title>Atom-Powered Robots Run Amok</title>
///     <link href="http://example.org/2003/12/13/atom03"/>
///     <id>urn:uuid:1225c695-cfb8-4ebb-aaaa-80da344efa6a</id>
///     <updated>2003-12-13T18:30:02Z</updated>
///     <summary>Some text.</summary>
///   </entry>
///
/// </feed>
/// ```


pub trait Entry {
    fn summary(&self) -> &str;
    fn title(&self) -> &str;
}

pub trait Feed {
    /// Get the name of the author.
    fn authors(&self) -> Box<dyn Iterator<Item = &dyn Person> + '_>;

    fn entries(&self) -> Box<dyn Iterator<Item = &dyn Entry> + '_>;

    fn title(&self) -> &str;
}

/// atomPersonConstruct =
///    atomCommonAttributes,
///    (element atom:name { text }
///     & element atom:uri { atomUri }?
///     & element atom:email { atomEmailAddress }?
///     & extensionElement*)
pub trait Person {
    fn name(&self) -> &str;
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleEntry {
    summary: String,
    title: String,
}

impl Entry for SimpleEntry {
    fn summary(&self) -> &str {
        self.summary.as_str()
    }

    fn title(&self) -> &str {
        self.title.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleFeed {
    authors: Vec<SimplePerson>,
    entries: Vec<SimpleEntry>,
    title: String,
}

impl Feed for SimpleFeed {
    fn authors(&self) -> Box<dyn Iterator<Item = &dyn Person> + '_> {
        Box::new(self.authors.iter().map(|p| p as &dyn Person))
    }

    fn entries(&self) -> Box<dyn Iterator<Item = &dyn Entry> + '_> {
        Box::new(self.entries.iter().map(|e| e as &dyn Entry))
    }

    fn title(&self) -> &str {
        self.title.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimplePerson {
    name: String,
}

impl From<String> for SimplePerson {
    fn from(value: String) -> Self {
        Self {
            name: value
        }
    }
}

impl Person for SimplePerson {
    fn name(&self) -> &str {
        self.name.as_str()
    }
}
