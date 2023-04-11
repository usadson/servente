// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use phf::phf_map;

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

static METHOD_MAP: phf::Map<&'static str, Method> = phf_map!(
    "ACL" => Method::Acl,
    "BASELINE-CONTROL" => Method::BaselineControl,
    "BIND" => Method::Bind,
    "CHECKIN" => Method::CheckIn,
    "CHECKOUT" => Method::CheckOut,
    "CONNECT" => Method::Connect,
    "COPY" => Method::Copy,
    "DELETE" => Method::Delete,
    "GET" => Method::Get,
    "HEAD" => Method::Head,
    "LABEL" => Method::Label,
    "LINK" => Method::Link,
    "LOCK" => Method::Lock,
    "MERGE" => Method::Merge,
    "MKACTIVITY" => Method::MkActivity,
    "MKCALENDAR" => Method::MkCalendar,
    "MKCOL" => Method::MkCol,
    "MKREDIRECTREF" => Method::MkRedirectRef,
    "MKWORKSPACE" => Method::MkWorkspace,
    "MOVE" => Method::Move,
    "OPTIONS" => Method::Options,
    "ORDERPATCH" => Method::OrderPatch,
    "PATCH" => Method::Patch,
    "POST" => Method::Post,
    "PRI" => Method::Pri,
    "PROPFIND" => Method::PropFind,
    "PROPPATCH" => Method::PropPatch,
    "PUT" => Method::Put,
    "REBIND" => Method::Rebind,
    "REPORT" => Method::Report,
    "SEARCH" => Method::Search,
    "TRACE" => Method::Trace,
    "UNBIND" => Method::Unbind,
    "UNCHECKOUT" => Method::Uncheckout,
    "UNLINK" => Method::Unlink,
    "UNLOCK" => Method::Unlock,
    "UPDATE" => Method::Update,
    "UPDATEREDIRECTREF" => Method::UpdateRedirectRef,
    "VERSION-CONTROL" => Method::VersionControl,
);

impl From<String> for Method {
    /// Methods in HTTP are case-sensitive, as per
    /// [RFC 9110 Section 9.1](https://www.rfc-editor.org/rfc/rfc9110.html#section-9.1-5)
    fn from(value: String) -> Self {
        match METHOD_MAP.get(value.as_str()) {
            Some(method) => method.clone(),
            None => Method::Other(value),
        }
    }
}

impl From<&str> for Method {
    /// Methods in HTTP are case-sensitive, as per
    /// [RFC 9110 Section 9.1](https://www.rfc-editor.org/rfc/rfc9110.html#section-9.1-5)
    fn from(value: &str) -> Self {
        match METHOD_MAP.get(value) {
            Some(method) => method.clone(),
            None => Method::Other(value.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use super::*;

    #[test]
    fn test_to_and_from_string() {
        for (from_string_identifier, method) in METHOD_MAP.entries() {
            assert_eq!(*from_string_identifier, method.as_string(),
                "Invalid entry: \"{from_string_identifier}\" and \"{}\"",
                method.as_string());
        }
    }

    #[rstest]
    #[case("get", Method::Other(String::from("get")))]
    #[case("GET", Method::Get)]
    #[case("Post", Method::Other(String::from("Post")))]
    #[case("POST", Method::Post)]
    #[case("pRI", Method::Other(String::from("pRI")))]
    #[case("Pri", Method::Other(String::from("Pri")))]
    #[case("PrI", Method::Other(String::from("PrI")))]
    #[case("PRi", Method::Other(String::from("PRi")))]
    #[case("PRI", Method::Pri)]
    #[test]
    fn test_case_sensitivity(#[case] input: &str, #[case] expected: Method) {
        assert_eq!(Method::from(input), expected);
    }
}
