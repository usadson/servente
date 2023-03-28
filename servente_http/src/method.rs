// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use phf::phf_map;
use unicase::UniCase;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Method {
    Other(String),
    Acl,
    BaselineControl,
    Bind,
    CheckIn,
    CheckOut,
    Connect,
    Copy,
    Delete,
    Get,
    Head,
    Label,
    Link,
    Lock,
    Merge,
    MkActivity,
    MkCalendar,
    MkCol,
    MkRedirectRef,
    MkWorkspace,
    Move,
    Options,
    OrderPatch,
    Patch,
    Post,
    Pri,
    PropFind,
    PropPatch,
    Put,
    Rebind,
    Report,
    Search,
    Trace,
    Unbind,
    Uncheckout,
    Unlink,
    Unlock,
    Update,
    UpdateRedirectRef,
    VersionControl,
}

impl Method {
    /// Get the method in string form.
    ///
    /// # Notes
    /// Header names are case-sensitive, as per
    /// [RFC 9110 - Section 9.1](https://www.rfc-editor.org/rfc/rfc9110.html#section-9.1-5):
    /// > The method token is case-sensitive because it might be used as a
    /// > gateway to object-based systems with case-sensitive method names. By
    /// > convention, standardized methods are defined in all-uppercase US-ASCII
    /// > letters.
    ///
    /// # References
    /// * [RFC 9110 - Section 9. Methods](https://www.rfc-editor.org/rfc/rfc9110.html#section-9)
    /// * [IANA Hypertext Transfer Protocol (HTTP) Method Registry](https://www.iana.org/assignments/http-methods/http-methods.xhtml)
    pub fn as_string(&self) -> &str {
        match self {
            Self::Other(str) => str,
            Self::Acl => "ACL",
            Self::BaselineControl => "BASELINE-CONTROL",
            Self::Bind => "BIND",
            Self::CheckIn => "CHECKIN",
            Self::CheckOut => "CHECKOUT",
            Self::Connect => "CONNECT",
            Self::Copy => "COPY",
            Self::Delete => "DELETE",
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Label => "LABEL",
            Self::Link => "LINK",
            Self::Lock => "LOCK",
            Self::Merge => "MERGE",
            Self::MkActivity => "MKACTIVITY",
            Self::MkCalendar => "MKCALENDAR",
            Self::MkCol => "MKCOL",
            Self::MkRedirectRef => "MKREDIRECTREF",
            Self::MkWorkspace => "MKWORKSPACE",
            Self::Move => "MOVE",
            Self::Options => "OPTIONS",
            Self::OrderPatch => "ORDERPATCH",
            Self::Patch => "PATCH",
            Self::Post => "POST",
            Self::Pri => "PRI",
            Self::PropFind => "PROPFIND",
            Self::PropPatch => "PROPPATCH",
            Self::Put => "PUT",
            Self::Rebind => "REBIND",
            Self::Report => "REPORT",
            Self::Search => "SEARCH",
            Self::Trace => "TRACE",
            Self::Unbind => "UNBIND",
            Self::Uncheckout => "UNCHECKOUT",
            Self::Unlink => "UNLINK",
            Self::Unlock => "UNLOCK",
            Self::Update => "UPDATE",
            Self::UpdateRedirectRef => "UPDATEREDIRECTREF",
            Self::VersionControl => "VERSION-CONTROL",
        }
    }
}

static METHOD_MAP: phf::Map<UniCase<&'static str>, Method> = phf_map!(
    UniCase::ascii("acl") => Method::Acl,
    UniCase::ascii("baseline-control") => Method::BaselineControl,
    UniCase::ascii("bind") => Method::Bind,
    UniCase::ascii("checkin") => Method::CheckIn,
    UniCase::ascii("checkout") => Method::CheckOut,
    UniCase::ascii("connect") => Method::Connect,
    UniCase::ascii("copy") => Method::Copy,
    UniCase::ascii("delete") => Method::Delete,
    UniCase::ascii("get") => Method::Get,
    UniCase::ascii("head") => Method::Head,
    UniCase::ascii("label") => Method::Label,
    UniCase::ascii("link") => Method::Link,
    UniCase::ascii("lock") => Method::Lock,
    UniCase::ascii("merge") => Method::Merge,
    UniCase::ascii("mkactivity") => Method::MkActivity,
    UniCase::ascii("mkcalendar") => Method::MkCalendar,
    UniCase::ascii("mkcol") => Method::MkCol,
    UniCase::ascii("mkredirectref") => Method::MkRedirectRef,
    UniCase::ascii("mkworkspace") => Method::MkWorkspace,
    UniCase::ascii("move") => Method::Move,
    UniCase::ascii("options") => Method::Options,
    UniCase::ascii("orderpatch") => Method::OrderPatch,
    UniCase::ascii("patch") => Method::Patch,
    UniCase::ascii("post") => Method::Post,
    UniCase::ascii("pri") => Method::Pri,
    UniCase::ascii("propfind") => Method::PropFind,
    UniCase::ascii("proppatch") => Method::PropPatch,
    UniCase::ascii("put") => Method::Put,
    UniCase::ascii("rebind") => Method::Rebind,
    UniCase::ascii("report") => Method::Report,
    UniCase::ascii("search") => Method::Search,
    UniCase::ascii("trace") => Method::Trace,
    UniCase::ascii("unbind") => Method::Unbind,
    UniCase::ascii("uncheckout") => Method::Uncheckout,
    UniCase::ascii("unlink") => Method::Unlink,
    UniCase::ascii("unlock") => Method::Unlock,
    UniCase::ascii("update") => Method::Update,
    UniCase::ascii("updateredirectref") => Method::UpdateRedirectRef,
    UniCase::ascii("version-control") => Method::VersionControl,
);

impl From<String> for Method {
    fn from(value: String) -> Self {
        match METHOD_MAP.get(&UniCase::ascii(&value)) {
            Some(method) => method.clone(),
            None => Method::Other(value),
        }
    }
}

impl From<&str> for Method {
    fn from(value: &str) -> Self {
        match METHOD_MAP.get(&UniCase::ascii(value)) {
            Some(method) => method.clone(),
            None => Method::Other(value.to_string()),
        }
    }
}
