// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use servente_http::{HeaderName, Method, StatusCode};
use servente_http2::hpack::DynamicTableEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaticMethod {
    Connect,
    Delete,
    Get,
    Head,
    Options,
    Post,
    Put,
}

impl Into<Method> for StaticMethod {
    fn into(self) -> Method {
        match self {
            StaticMethod::Connect => Method::Connect,
            StaticMethod::Delete => Method::Delete,
            StaticMethod::Get => Method::Get,
            StaticMethod::Head => Method::Head,
            StaticMethod::Options => Method::Options,
            StaticMethod::Post => Method::Post,
            StaticMethod::Put => Method::Put,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Entry {
    Authority,
    /// :path   /
    Path,
    Header(HeaderName, &'static str),
    Method(StaticMethod),
    SchemeHttp,
    SchemeHttps,
    StatusCode(StatusCode),
}

impl Entry {
    pub fn into_dynamic_table_entry_with_value_from_client(&self, value: String) -> Option<DynamicTableEntry> {
        Some(match self {
            Entry::Authority => DynamicTableEntry::Authority(value.into()),
            Entry::Path => return None,
            Entry::Header(name, _) => DynamicTableEntry::Header { name: name.clone(), value: value.into() },
            Entry::Method(_) => DynamicTableEntry::Method(value.into()),
            Entry::SchemeHttp | Entry::SchemeHttps => DynamicTableEntry::Scheme(value.into()),
            Entry::StatusCode(_) => return None,
        })
    }
}

pub const TABLE: &[Entry; 99] = &[
    Entry::Authority,
    Entry::Path,
    Entry::Header(HeaderName::Age, "0"),
    Entry::Header(HeaderName::ContentDisposition, ""),
    Entry::Header(HeaderName::ContentLength, "0"),
    Entry::Header(HeaderName::Cookie, ""),
    Entry::Header(HeaderName::Date, ""),
    Entry::Header(HeaderName::ETag, ""),
    Entry::Header(HeaderName::IfModifiedSince, ""),
    Entry::Header(HeaderName::IfNoneMatch, ""),
    Entry::Header(HeaderName::LastModified, ""),
    Entry::Header(HeaderName::Link, ""),
    Entry::Header(HeaderName::Location, ""),
    Entry::Header(HeaderName::Referer, ""),
    Entry::Header(HeaderName::SetCookie, ""),
    Entry::Method(StaticMethod::Connect),
    Entry::Method(StaticMethod::Delete),
    Entry::Method(StaticMethod::Get),
    Entry::Method(StaticMethod::Head),
    Entry::Method(StaticMethod::Options),
    Entry::Method(StaticMethod::Post),
    Entry::Method(StaticMethod::Put),
    Entry::SchemeHttp,
    Entry::SchemeHttps,
    Entry::StatusCode(StatusCode::EarlyHints),
    Entry::StatusCode(StatusCode::Ok),
    Entry::StatusCode(StatusCode::NotModified),
    Entry::StatusCode(StatusCode::NotFound),
    Entry::StatusCode(StatusCode::ServiceUnavailable),
    Entry::Header(HeaderName::Accept, "*/*"),
    Entry::Header(HeaderName::Accept, "application/dns-message"),
    Entry::Header(HeaderName::AcceptEncoding, "gzip, deflate, br"),
    Entry::Header(HeaderName::AcceptRanges, "bytes"),
    Entry::Header(HeaderName::AccessControlAllowHeaders, "cache-control"),
    Entry::Header(HeaderName::AccessControlAllowHeaders, "content-type"),
    Entry::Header(HeaderName::AccessControlAllowOrigin, "*"),
    Entry::Header(HeaderName::CacheControl, "max-age=0"),
    Entry::Header(HeaderName::CacheControl, "max-age=2592000"),
    Entry::Header(HeaderName::CacheControl, "max-age=604800"),
    Entry::Header(HeaderName::CacheControl, "no-cache"),
    Entry::Header(HeaderName::CacheControl, "no-store"),
    Entry::Header(HeaderName::CacheControl, "public, max-age=31536000"),
    Entry::Header(HeaderName::ContentEncoding, "br"),
    Entry::Header(HeaderName::ContentEncoding, "gzip"),
    Entry::Header(HeaderName::ContentType, "application/dns-message"),
    Entry::Header(HeaderName::ContentType, "application/javascript"),
    Entry::Header(HeaderName::ContentType, "application/json"),
    Entry::Header(HeaderName::ContentType, "application/x-www-form-urlencoded"),
    Entry::Header(HeaderName::ContentType, "image/gif"),
    Entry::Header(HeaderName::ContentType, "image/jpeg"),
    Entry::Header(HeaderName::ContentType, "image/png"),
    Entry::Header(HeaderName::ContentType, "text/css"),
    Entry::Header(HeaderName::ContentType, "text/html; charset=utf-8"),
    Entry::Header(HeaderName::ContentType, "text/plain"),
    Entry::Header(HeaderName::ContentType, "text/plain;charset=utf-8"),
    Entry::Header(HeaderName::Range, "bytes=0-"),
    Entry::Header(HeaderName::StrictTransportSecurity, "max-age=31536000"),
    Entry::Header(HeaderName::StrictTransportSecurity, "max-age=31536000; includesubdomains"),
    Entry::Header(HeaderName::StrictTransportSecurity, "max-age=31536000; includesubdomains; preload"),
    Entry::Header(HeaderName::Vary, "accept-encoding"),
    Entry::Header(HeaderName::Vary, "origin"),
    Entry::Header(HeaderName::XContentTypeOptions, "nosniff"),
    Entry::Header(HeaderName::XXSSProtection, "1; mode=block"),
    Entry::StatusCode(StatusCode::Continue),
    Entry::StatusCode(StatusCode::NoContent),
    Entry::StatusCode(StatusCode::PartialContent),
    Entry::StatusCode(StatusCode::Found),
    Entry::StatusCode(StatusCode::BadRequest),
    Entry::StatusCode(StatusCode::Forbidden),
    Entry::StatusCode(StatusCode::MisdirectedRequest),
    Entry::StatusCode(StatusCode::TooEarly),
    Entry::StatusCode(StatusCode::InternalServerError),
    Entry::Header(HeaderName::AcceptLanguage, ""),
    Entry::Header(HeaderName::AccessControlAllowCredentials, "FALSE"),
    Entry::Header(HeaderName::AccessControlAllowCredentials, "TRUE"),
    Entry::Header(HeaderName::AccessControlAllowHeaders, "*"),
    Entry::Header(HeaderName::AccessControlAllowMethods, "get"),
    Entry::Header(HeaderName::AccessControlAllowMethods, "get, post, options"),
    Entry::Header(HeaderName::AccessControlAllowMethods, "options"),
    Entry::Header(HeaderName::AccessControlExposeHeaders, "content-length"),
    Entry::Header(HeaderName::AccessControlRequestHeaders, "content-type"),
    Entry::Header(HeaderName::AccessControlRequestMethod, "get"),
    Entry::Header(HeaderName::AccessControlRequestMethod, "post"),
    Entry::Header(HeaderName::AltSvc, "clear"),
    Entry::Header(HeaderName::Authorization, ""),
    Entry::Header(HeaderName::ContentSecurityPolicy, "script-src 'none'; object-src 'none'; base-uri 'none'"),
    Entry::Header(HeaderName::EarlyData, "1"),
    Entry::Header(HeaderName::ExpectCT, ""),
    Entry::Header(HeaderName::Forwarded, ""),
    Entry::Header(HeaderName::IfRange, ""),
    Entry::Header(HeaderName::Origin, ""),
    Entry::Header(HeaderName::Purpose, "prefetch"),
    Entry::Header(HeaderName::Server, ""),
    Entry::Header(HeaderName::TimingAllowOrigin, "*"),
    Entry::Header(HeaderName::UpgradeInsecureRequests, "1"),
    Entry::Header(HeaderName::UserAgent, ""),
    Entry::Header(HeaderName::XForwaredFor, ""),
    Entry::Header(HeaderName::XFrameOptions, "deny"),
    Entry::Header(HeaderName::XFrameOptions, "sameorigin"),
];
