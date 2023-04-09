// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::borrow::Cow;

use phf::phf_map;
use unicase::UniCase;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeaderName {
    Other(String),

    Accept,
    AcceptCharset,
    AcceptEncoding,
    AcceptLanguage,
    AcceptRanges,
    AccessControlAllowCredentials,
    AccessControlAllowHeaders,
    AccessControlAllowMethods,
    AccessControlAllowOrigin,
    AccessControlExposeHeaders,
    AccessControlMaxAge,
    AccessControlRequestHeaders,
    AccessControlRequestMethod,
    Age,
    Allow,
    AltSvc,
    Authorization,
    CacheControl,
    CacheStatus,
    Close,
    Connection,
    Cookie,
    ContentDisposition,
    ContentEncoding,
    ContentLanguage,
    ContentLength,
    ContentLocation,
    ContentRange,
    ContentSecurityPolicy,
    ContentSecurityPolicyReportOnly,
    ContentType,
    CrossOriginResourcePolicy,
    Date,
    DNT,
    EarlyData,
    ETag,
    Expect,
    ExpectCT,
    Expires,
    Forwarded,
    From,
    Host,
    IfMatch,
    IfModifiedSince,
    IfNoneMatch,
    IfRange,
    IfUnmodifiedSince,
    KeepAlive,
    LastModified,
    Link,
    Location,
    MaxForwards,
    ProxyStatus,
    Origin,
    Pragma,
    ProxyAuthenticate,
    ProxyAuthorization,
    ProxyConnection,
    Purpose,
    Range,
    Referer,
    ReferrerPolicy,
    Refresh,
    RetryAfter,
    SecChUa,
    SecChUaMobile,
    SecChUaPlatform,
    SecFetchDest,
    SecFetchMode,
    SecFetchSite,
    SecFetchUser,
    SecPurpose,
    SecWebSocketAccept,
    SecWebSocketExtensions,
    SecWebSocketKey,
    SecWebSocketProtocol,
    SecWebSocketVersion,
    Server,
    ServerTiming,
    SetCookie,
    Status,
    StrictTransportSecurity,
    TE,
    TimingAllowOrigin,
    Trailer,
    TransferEncoding,
    Upgrade,
    UpgradeInsecureRequests,
    UserAgent,
    Vary,
    Via,
    WwwAuthenticate,
    XContentTypeOptions,
    XForwaredFor,
    XFrameOptions,
    XRequestedWith,
    XXSSProtection,
}

static STRING_TO_HEADER_NAME_MAP: phf::Map<UniCase<&'static str>, HeaderName> = phf_map!(
    UniCase::ascii("accept") => HeaderName::Accept,
    UniCase::ascii("accept-charset") => HeaderName::AcceptCharset,
    UniCase::ascii("accept-encoding") => HeaderName::AcceptEncoding,
    UniCase::ascii("accept-language") => HeaderName::AcceptLanguage,
    UniCase::ascii("accept-ranges") => HeaderName::AcceptRanges,
    UniCase::ascii("access-control-allow-credentials") => HeaderName::AccessControlAllowCredentials,
    UniCase::ascii("access-control-allow-headers") => HeaderName::AccessControlAllowHeaders,
    UniCase::ascii("access-control-allow-methods") => HeaderName::AccessControlAllowMethods,
    UniCase::ascii("access-control-allow-origin") => HeaderName::AccessControlAllowOrigin,
    UniCase::ascii("access-control-expose-headers") => HeaderName::AccessControlExposeHeaders,
    UniCase::ascii("access-control-max-age") => HeaderName::AccessControlMaxAge,
    UniCase::ascii("access-control-request-headers") => HeaderName::AccessControlRequestHeaders,
    UniCase::ascii("access-control-request-method") => HeaderName::AccessControlRequestMethod,
    UniCase::ascii("age") => HeaderName::Age,
    UniCase::ascii("allow") => HeaderName::Allow,
    UniCase::ascii("alt-svc") => HeaderName::AltSvc,
    UniCase::ascii("authorization") => HeaderName::Authorization,
    UniCase::ascii("cache-control") => HeaderName::CacheControl,
    UniCase::ascii("cache-status") => HeaderName::CacheStatus,
    UniCase::ascii("close") => HeaderName::Close,
    UniCase::ascii("connection") => HeaderName::Connection,
    UniCase::ascii("cookie") => HeaderName::Cookie,
    UniCase::ascii("content-disposition") => HeaderName::ContentDisposition,
    UniCase::ascii("content-encoding") => HeaderName::ContentEncoding,
    UniCase::ascii("content-length") => HeaderName::ContentLength,
    UniCase::ascii("content-language") => HeaderName::ContentLanguage,
    UniCase::ascii("content-location") => HeaderName::ContentLocation,
    UniCase::ascii("content-range") => HeaderName::ContentRange,
    UniCase::ascii("content-security-policy") => HeaderName::ContentSecurityPolicy,
    UniCase::ascii("content-security-policy-report-only") => HeaderName::ContentSecurityPolicyReportOnly,
    UniCase::ascii("content-type") => HeaderName::ContentType,
    UniCase::ascii("cross-origin-resource-policy") => HeaderName::CrossOriginResourcePolicy,
    UniCase::ascii("date") => HeaderName::Date,
    UniCase::ascii("dnt") => HeaderName::DNT,
    UniCase::ascii("early-data") => HeaderName::EarlyData,
    UniCase::ascii("etag") => HeaderName::ETag,
    UniCase::ascii("expect") => HeaderName::Expect,
    UniCase::ascii("expect-ct") => HeaderName::ExpectCT,
    UniCase::ascii("expires") => HeaderName::Expires,
    UniCase::ascii("forwarded") => HeaderName::Forwarded,
    UniCase::ascii("from") => HeaderName::From,
    UniCase::ascii("host") => HeaderName::Host,
    UniCase::ascii("if-match") => HeaderName::IfMatch,
    UniCase::ascii("if-modified-since") => HeaderName::IfModifiedSince,
    UniCase::ascii("if-none-match") => HeaderName::IfNoneMatch,
    UniCase::ascii("if-range") => HeaderName::IfRange,
    UniCase::ascii("if-unmodified-since") => HeaderName::IfUnmodifiedSince,
    UniCase::ascii("keep-alive") => HeaderName::KeepAlive,
    UniCase::ascii("last-modified") => HeaderName::LastModified,
    UniCase::ascii("link") => HeaderName::Link,
    UniCase::ascii("location") => HeaderName::Location,
    UniCase::ascii("max-forwards") => HeaderName::MaxForwards,
    UniCase::ascii("origin") => HeaderName::Origin,
    UniCase::ascii("pragma") => HeaderName::Pragma,
    UniCase::ascii("proxy-authenticate") => HeaderName::ProxyAuthenticate,
    UniCase::ascii("proxy-authorization") => HeaderName::ProxyAuthorization,
    UniCase::ascii("proxy-connection") => HeaderName::ProxyConnection,
    UniCase::ascii("purpose") => HeaderName::Purpose,
    UniCase::ascii("proxy-status") => HeaderName::ProxyStatus,
    UniCase::ascii("range") => HeaderName::Range,
    UniCase::ascii("referer") => HeaderName::Referer,
    UniCase::ascii("referrer-policy") => HeaderName::ReferrerPolicy,
    UniCase::ascii("refresh") => HeaderName::Refresh,
    UniCase::ascii("retry-after") => HeaderName::RetryAfter,
    UniCase::ascii("sec-ch-ua") => HeaderName::SecChUa,
    UniCase::ascii("sec-ch-ua-mobile") => HeaderName::SecChUaMobile,
    UniCase::ascii("sec-ch-ua-platform") => HeaderName::SecChUaPlatform,
    UniCase::ascii("sec-fetch-dest") => HeaderName::SecFetchDest,
    UniCase::ascii("sec-fetch-mode") => HeaderName::SecFetchMode,
    UniCase::ascii("sec-fetch-site") => HeaderName::SecFetchSite,
    UniCase::ascii("sec-fetch-user") => HeaderName::SecFetchUser,
    UniCase::ascii("sec-purpose") => HeaderName::SecPurpose,
    UniCase::ascii("sec-websocket-accept") => HeaderName::SecWebSocketAccept,
    UniCase::ascii("sec-websocket-extensions") => HeaderName::SecWebSocketExtensions,
    UniCase::ascii("sec-websocket-key") => HeaderName::SecWebSocketKey,
    UniCase::ascii("sec-websocket-protocol") => HeaderName::SecWebSocketProtocol,
    UniCase::ascii("sec-websocket-version") => HeaderName::SecWebSocketVersion,
    UniCase::ascii("server") => HeaderName::Server,
    UniCase::ascii("server-timing") => HeaderName::ServerTiming,
    UniCase::ascii("set-cookie") => HeaderName::SetCookie,
    UniCase::ascii("status") => HeaderName::Status,
    UniCase::ascii("strict-transport-security") => HeaderName::StrictTransportSecurity,
    UniCase::ascii("te") => HeaderName::TE,
    UniCase::ascii("timing-allow-origin") => HeaderName::TimingAllowOrigin,
    UniCase::ascii("trailer") => HeaderName::Trailer,
    UniCase::ascii("transfer-encoding") => HeaderName::TransferEncoding,
    UniCase::ascii("upgrade") => HeaderName::Upgrade,
    UniCase::ascii("upgrade-insecure-requests") => HeaderName::UpgradeInsecureRequests,
    UniCase::ascii("user-agent") => HeaderName::UserAgent,
    UniCase::ascii("vary") => HeaderName::Vary,
    UniCase::ascii("via") => HeaderName::Via,
    UniCase::ascii("www-authenticate") => HeaderName::WwwAuthenticate,
    UniCase::ascii("x-content-type-options") => HeaderName::XContentTypeOptions,
    UniCase::ascii("x-forwarded-for") => HeaderName::XForwaredFor,
    UniCase::ascii("x-frame-options") => HeaderName::XFrameOptions,
    UniCase::ascii("x-requested-with") => HeaderName::XRequestedWith,
    UniCase::ascii("x-xss-protection") => HeaderName::XXSSProtection,
);

impl From<String> for HeaderName {
    #[must_use]
    fn from(mut value: String) -> Self {
        match STRING_TO_HEADER_NAME_MAP.get(&UniCase::ascii(&value)) {
            Some(header_name) => header_name.clone(),
            None => {
                value.make_ascii_lowercase();
                HeaderName::Other(value)
            }
        }
    }
}

impl HeaderName {
    /// This class defines which specification and/or behavior the field name
    /// belongs to.
    pub fn class(&self) -> HeaderNameClass {
        match self {
            HeaderName::Connection
                | HeaderName::KeepAlive
                | HeaderName::TE
                | HeaderName::TransferEncoding
                | HeaderName::Upgrade => HeaderNameClass::ConnectionSpecific,
            HeaderName::Other(_) if self.is_cgi_extension_field() => HeaderNameClass::CgiExtension,
            _ => HeaderNameClass::Other,
        }
    }

    /// Returns whether or not this field is an CGI extension field. These may
    /// convey information between a CGI script and a web server.
    ///
    /// # References
    /// * [RFC 3875 Section 6.3.5](https://www.rfc-editor.org/rfc/rfc3875#section-6.3.5)
    pub fn is_cgi_extension_field(&self) -> bool {
        let Self::Other(str) = self else {
            return false;
        };

        const HEADER_NAME_PREFIX_X_CGI: &str = "x-cgi-";
        if str.len() < HEADER_NAME_PREFIX_X_CGI.len() {
            return false;
        }

        unicase::eq_ascii(&str[0..HEADER_NAME_PREFIX_X_CGI.len()], str)
    }

    #[must_use]
    pub fn to_string_h1(&self) -> &str {
        match self {
            HeaderName::Other(str) => str,

            HeaderName::Accept => "Accept",
            HeaderName::AcceptCharset => "Accept-Charset",
            HeaderName::AcceptEncoding => "Accept-Encoding",
            HeaderName::AcceptLanguage => "Accept-Language",
            HeaderName::AcceptRanges => "Accept-Ranges",
            HeaderName::AccessControlAllowCredentials => "Access-Control-Allow-Credentials",
            HeaderName::AccessControlAllowHeaders => "Access-Control-Allow-Headers",
            HeaderName::AccessControlAllowMethods => "Access-Control-Allow-Methods",
            HeaderName::AccessControlAllowOrigin => "Access-Control-Allow-Origin",
            HeaderName::AccessControlExposeHeaders => "Access-Control-Expose-Headers",
            HeaderName::AccessControlMaxAge => "Access-Control-Max-Age",
            HeaderName::AccessControlRequestHeaders => "Access-Control-Request-Headers",
            HeaderName::AccessControlRequestMethod => "Access-Control-Request-Method",
            HeaderName::Age => "Age",
            HeaderName::Allow => "Allow",
            HeaderName::AltSvc => "Alt-Svc",
            HeaderName::Authorization => "Authorization",
            HeaderName::CacheControl => "Cache-Control",
            HeaderName::CacheStatus => "Cache-Status",
            HeaderName::Close => "Close",
            HeaderName::Connection => "Connection",
            HeaderName::Cookie => "Cookie",
            HeaderName::ContentDisposition => "Content-Disposition",
            HeaderName::ContentEncoding => "Content-Encoding",
            HeaderName::ContentLanguage => "Content-Language",
            HeaderName::ContentLength => "Content-Length",
            HeaderName::ContentLocation => "Content-Location",
            HeaderName::ContentRange => "Content-Range",
            HeaderName::ContentSecurityPolicy => "Content-Security-Policy",
            HeaderName::ContentSecurityPolicyReportOnly => "Content-Security-Policy-Report-Only",
            HeaderName::ContentType => "Content-Type",
            HeaderName::CrossOriginResourcePolicy => "Cross-Origin-Resource-Policy",
            HeaderName::Date => "Date",
            HeaderName::DNT => "DNT",
            HeaderName::EarlyData => "Early-Data",
            HeaderName::ETag => "ETag",
            HeaderName::Expect => "Expect",
            HeaderName::ExpectCT => "Expect-CT",
            HeaderName::Expires => "Expires",
            HeaderName::Forwarded => "Forwarded",
            HeaderName::From => "From",
            HeaderName::Host => "Host",
            HeaderName::IfMatch => "If-Match",
            HeaderName::IfModifiedSince => "If-Modified-Since",
            HeaderName::IfNoneMatch => "If-None-Match",
            HeaderName::IfRange => "If-Range",
            HeaderName::IfUnmodifiedSince => "If-Unmodified-Since",
            HeaderName::KeepAlive => "Keep-Alive",
            HeaderName::LastModified => "Last-Modified",
            HeaderName::Link => "Link",
            HeaderName::Location => "Location",
            HeaderName::MaxForwards => "Max-Forwards",
            HeaderName::Origin => "Origin",
            HeaderName::Pragma => "Pragma",
            HeaderName::ProxyAuthenticate => "Proxy-Authenticate",
            HeaderName::ProxyAuthorization => "Proxy-Authorization",
            HeaderName::ProxyConnection => "Proxy-Connection",
            HeaderName::ProxyStatus => "Proxy-Status",
            HeaderName::Purpose => "Purpose",
            HeaderName::Range => "Range",
            HeaderName::Referer => "Referer",
            HeaderName::ReferrerPolicy => "Referrer-Policy",
            HeaderName::Refresh => "Refresh",
            HeaderName::RetryAfter => "Retry-After",
            HeaderName::SecChUa => "Sec-Ch-Ua",
            HeaderName::SecChUaMobile => "Sec-Ch-Ua-Mobile",
            HeaderName::SecChUaPlatform => "Sec-Ch-Ua-Platform",
            HeaderName::SecFetchDest => "Sec-Fetch-Dest",
            HeaderName::SecFetchMode => "Sec-Fetch-Mode",
            HeaderName::SecFetchSite => "Sec-Fetch-Site",
            HeaderName::SecFetchUser => "Sec-Fetch-User",
            HeaderName::SecPurpose => "Sec-Purpose",
            HeaderName::SecWebSocketAccept => "Sec-WebSocket-Accept",
            HeaderName::SecWebSocketExtensions => "Sec-WebSocket-Extensions",
            HeaderName::SecWebSocketKey => "Sec-WebSocket-Key",
            HeaderName::SecWebSocketProtocol => "Sec-WebSocket-Protocol",
            HeaderName::SecWebSocketVersion => "Sec-WebSocket-Version",
            HeaderName::Server => "Server",
            HeaderName::ServerTiming => "Server-Timing",
            HeaderName::SetCookie => "Set-Cookie",
            HeaderName::Status => "Status",
            HeaderName::StrictTransportSecurity => "Strict-Transport-Security",
            HeaderName::TE => "TE",
            HeaderName::TimingAllowOrigin => "Timing-Allow-Origin",
            HeaderName::Trailer => "Trailer",
            HeaderName::TransferEncoding => "Transfer-Encoding",
            HeaderName::Upgrade => "Upgrade",
            HeaderName::UpgradeInsecureRequests => "Upgrade-Insecure-Requests",
            HeaderName::UserAgent => "User-Agent",
            HeaderName::Vary => "Vary",
            HeaderName::Via => "Via",
            HeaderName::WwwAuthenticate => "WWW-Authenticate",
            HeaderName::XContentTypeOptions => "X-Content-Type-Options",
            HeaderName::XForwaredFor => "X-Forwarded-For",
            HeaderName::XFrameOptions => "X-Frame-Options",
            HeaderName::XRequestedWith => "X-Requested-With",
            HeaderName::XXSSProtection => "X-XSS-Protection",
        }
    }

    /// Get the lowercase format of the header, for use in HTTP/2.
    ///
    /// # HTTP/2
    /// _RFC 9113, section 8.2 Header Fields_ states:
    /// > Field names MUST be converted to lowercase when constructing an
    /// > HTTP/2 message.
    pub fn to_string_lowercase(&self) -> Cow<'static, str> {
        Cow::Borrowed(match self {
            HeaderName::Other(str) => return Cow::Owned(str.to_ascii_lowercase()),

            HeaderName::Accept => "accept",
            HeaderName::AcceptCharset => "accept-charset",
            HeaderName::AcceptEncoding => "accept-encoding",
            HeaderName::AcceptLanguage => "accept-language",
            HeaderName::AcceptRanges => "accept-ranges",
            HeaderName::AccessControlAllowCredentials => "access-control-allow-credentials",
            HeaderName::AccessControlAllowHeaders => "access-control-allow-headers",
            HeaderName::AccessControlAllowMethods => "access-control-allow-methods",
            HeaderName::AccessControlAllowOrigin => "access-control-allow-origin",
            HeaderName::AccessControlExposeHeaders => "access-control-expose-headers",
            HeaderName::AccessControlMaxAge => "access-control-max-age",
            HeaderName::AccessControlRequestHeaders => "access-control-request-headers",
            HeaderName::AccessControlRequestMethod => "access-control-request-method",
            HeaderName::Age => "age",
            HeaderName::Allow => "allow",
            HeaderName::AltSvc => "alt-svc",
            HeaderName::Authorization => "authorization",
            HeaderName::CacheControl => "cache-control",
            HeaderName::CacheStatus => "cache-status",
            HeaderName::Close => "close",
            HeaderName::Connection => "connection",
            HeaderName::Cookie => "cookie",
            HeaderName::ContentDisposition => "content-disposition",
            HeaderName::ContentEncoding => "content-encoding",
            HeaderName::ContentLanguage => "content-language",
            HeaderName::ContentLength => "content-length",
            HeaderName::ContentLocation => "content-location",
            HeaderName::ContentRange => "content-range",
            HeaderName::ContentSecurityPolicy => "content-security-policy",
            HeaderName::ContentSecurityPolicyReportOnly => "content-security-policy-report-only",
            HeaderName::ContentType => "content-type",
            HeaderName::CrossOriginResourcePolicy => "cross-origin-resource-policy",
            HeaderName::Date => "date",
            HeaderName::DNT => "dnt",
            HeaderName::EarlyData => "early-data",
            HeaderName::ETag => "etag",
            HeaderName::Expect => "expect",
            HeaderName::ExpectCT => "expect-ct",
            HeaderName::Expires => "expires",
            HeaderName::Forwarded => "forwarded",
            HeaderName::From => "from",
            HeaderName::Host => "host",
            HeaderName::IfMatch => "if-match",
            HeaderName::IfModifiedSince => "if-modified-since",
            HeaderName::IfNoneMatch => "if-none-match",
            HeaderName::IfRange => "if-range",
            HeaderName::IfUnmodifiedSince => "if-unmodified-since",
            HeaderName::KeepAlive => "keep-alive",
            HeaderName::LastModified => "last-modified",
            HeaderName::Link => "link",
            HeaderName::Location => "location",
            HeaderName::MaxForwards => "max-forwards",
            HeaderName::Origin => "origin",
            HeaderName::Pragma => "pragma",
            HeaderName::ProxyAuthenticate => "proxy-authenticate",
            HeaderName::ProxyAuthorization => "proxy-authorization",
            HeaderName::ProxyConnection => "proxy-connection",
            HeaderName::ProxyStatus => "proxy-status",
            HeaderName::Purpose => "purpose",
            HeaderName::Range => "range",
            HeaderName::Referer => "referer",
            HeaderName::ReferrerPolicy => "referrer-policy",
            HeaderName::Refresh => "refresh",
            HeaderName::RetryAfter => "retry-after",
            HeaderName::SecChUa => "sec-ch-ua",
            HeaderName::SecChUaMobile => "sec-ch-ua-mobile",
            HeaderName::SecChUaPlatform => "sec-ch-ua-platform",
            HeaderName::SecFetchDest => "sec-fetch-dest",
            HeaderName::SecFetchMode => "sec-fetch-mode",
            HeaderName::SecFetchSite => "sec-fetch-site",
            HeaderName::SecFetchUser => "sec-fetch-user",
            HeaderName::SecPurpose => "sec-purpose",
            HeaderName::SecWebSocketAccept => "sec-websocket-accept",
            HeaderName::SecWebSocketExtensions => "sec-websocket-extensions",
            HeaderName::SecWebSocketKey => "sec-websocket-key",
            HeaderName::SecWebSocketProtocol => "sec-websocket-protocol",
            HeaderName::SecWebSocketVersion => "sec-websocket-version",
            HeaderName::Server => "server",
            HeaderName::ServerTiming => "server-timing",
            HeaderName::SetCookie => "set-cookie",
            HeaderName::Status => "status",
            HeaderName::StrictTransportSecurity => "strict-transport-security",
            HeaderName::TE => "te",
            HeaderName::TimingAllowOrigin => "timing-allow-origin",
            HeaderName::Trailer => "trailer",
            HeaderName::TransferEncoding => "transfer-encoding",
            HeaderName::Upgrade => "upgrade",
            HeaderName::UpgradeInsecureRequests => "upgrade-insecure-requests",
            HeaderName::UserAgent => "user-agent",
            HeaderName::Vary => "vary",
            HeaderName::Via => "via",
            HeaderName::WwwAuthenticate => "www-authenticate",
            HeaderName::XContentTypeOptions => "x-content-type-options",
            HeaderName::XForwaredFor => "x-forwarded-for",
            HeaderName::XFrameOptions => "x-frame-options",
            HeaderName::XRequestedWith => "x-requested-with",
            HeaderName::XXSSProtection => "x-xss-protection",
        })
    }
}

/// This class defines which specification and/or behavior the field name
/// belongs to.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HeaderNameClass {
    /// The header name is specific to communication between the server and the
    /// CGI script.
    CgiExtension,

    /// The header name is a connection-specific value, applicable for HTTP/1.x
    /// connections.
    ConnectionSpecific,

    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks whether or not all the names in the `STRING_TO_HEADER_NAME_MAP`
    /// are valid, whether the `to_string_h1()` and `to_string_lowercase()`
    /// methods return the same string case-insensitve,
    /// and whether the `to_string_lowercase()` method returns a string
    /// that is all lowercase.
    #[test]
    fn test_header_name_to_string() {
        for (str, name) in STRING_TO_HEADER_NAME_MAP.entries() {
            assert_eq!(str, &UniCase::ascii(name.to_string_h1()));
            assert_eq!(str, &UniCase::ascii(name.to_string_lowercase()));

            assert!(name.to_string_h1().is_ascii());
            assert!(name.to_string_lowercase().is_ascii());

            assert!(!name.to_string_lowercase().bytes().any(|b| (b as char).is_uppercase()));

            assert!(!name.to_string_h1().split('-').any(|str| !str.is_empty() && str.chars().next().unwrap().is_ascii_lowercase()),
                "HTTP/1.1 Header names should have uppercase letters");
        }
    }
}
