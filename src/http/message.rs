// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{borrow::Cow, sync::Arc, time::{SystemTime, Duration}, fs::Metadata};

use phf::phf_map;
use unicase::UniCase;

use crate::resources::{
    ContentCoding,
    compression::ContentEncodedVersions,
    MediaType,
};

use super::hints::SecFetchDest;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HttpVersion {
    Http09,
    Http10,
    Http11,
    Http2,
    Http3,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatusCodeClass {
    /// 1xx: Informational
    Informational,

    /// 2xx: Success
    Success,

    /// 3xx: Redirection
    Redirection,

    /// 4xx: Client Error
    ClientError,

    /// 5xx: Server Error
    ServerError,
}

/// RFC 9110: https://httpwg.org/specs/rfc9110.html#status.codes
/// IANA: https://www.iana.org/assignments/http-status-codes/http-status-codes.xhtml
/// Wikipedia: https://en.wikipedia.org/wiki/List_of_HTTP_status_codes
/// MDN: https://developer.mozilla.org/en-US/docs/Web/HTTP/Status
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u16)]
pub enum StatusCode {
    Continue = 100,
    SwitchingProtocols = 101,
    Processing = 102,
    EarlyHints = 103,

    Ok = 200,
    Created = 201,
    Accepted = 202,
    NonAuthoritativeInformation = 203,
    NoContent = 204,
    ResetContent = 205,
    PartialContent = 206,
    MultiStatus = 207,
    AlreadyReported = 208,
    IMUsed = 226,

    MultipleChoices = 300,
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,
    UseProxy = 305,

    TemporaryRedirect = 307,
    PermanentRedirect = 308,

    BadRequest = 400,
    Unauthorized = 401,
    PaymentRequired = 402,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    ProxyAuthenticationRequired = 407,
    RequestTimeout = 408,
    Conflict = 409,
    Gone = 410,
    LengthRequired = 411,
    PreconditionFailed = 412,
    ContentTooLarge = 413,
    URITooLong = 414,
    UnsupportedMediaType = 415,
    RangeNotSatisfiable = 416,
    ExpectationFailed = 417,

    #[deprecated(note = "IANA Reserved since RFC 9110")]
    IMATeapot = 418,

    MisdirectedRequest = 421,
    UnprocessableContent = 422,
    Locked = 423,
    FailedDependency = 424,
    TooEarly = 425,
    UpgradeRequired = 426,
    PreconditionRequired = 428,
    TooManyRequests = 429,

    RequestHeaderFieldsTooLarge = 431,
    UnavailableForLegalReasons = 451,

    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    HTTPVersionNotSupported = 505,
    VariantAlsoNegotiates = 506,
    InsufficientStorage = 507,
    LoopDetected = 508,

    #[deprecated]
    NotExtended = 510,

    NetworkAuthenticationRequired = 511,
}

impl StatusCode {
    /// Returns the class of this status code.
    #[must_use]
    pub fn class(&self) -> StatusCodeClass {
        match *self as u16 {
            100..=199 => StatusCodeClass::Informational,
            200..=299 => StatusCodeClass::Success,
            300..=399 => StatusCodeClass::Redirection,
            400..=499 => StatusCodeClass::ClientError,
            500..=599 => StatusCodeClass::ServerError,
            _ => unreachable!(),
        }
    }

    #[must_use]
    pub fn to_string<'a>(&self) -> Cow<'a, str> {
        Cow::Borrowed(match self {
            StatusCode::Continue => "100 Continue",
            StatusCode::SwitchingProtocols => "101 Switching Protocols",
            StatusCode::Processing => "102 Processing",
            StatusCode::EarlyHints => "103 Early Hints",

            StatusCode::Ok => "200 OK",
            StatusCode::Created => "201 Created",
            StatusCode::Accepted => "202 Accepted",
            StatusCode::NonAuthoritativeInformation => "203 Non-Authoritative Information",
            StatusCode::NoContent => "204 No Content",
            StatusCode::ResetContent => "205 Reset Content",
            StatusCode::PartialContent => "206 Partial Content",
            StatusCode::MultiStatus => "207 Multi-Status",
            StatusCode::AlreadyReported => "208 Already Reported",
            StatusCode::IMUsed => "226 IM Used",
            StatusCode::MultipleChoices => "300 Multiple Choices",
            StatusCode::MovedPermanently => "301 Moved Permanently",
            StatusCode::Found => "302 Found",
            StatusCode::SeeOther => "303 See Other",
            StatusCode::NotModified => "304 Not Modified",
            StatusCode::UseProxy => "305 Use Proxy",
            StatusCode::TemporaryRedirect => "307 Temporary Redirect",
            StatusCode::PermanentRedirect => "308 Permanent Redirect",
            StatusCode::BadRequest => "400 Bad Request",
            StatusCode::Unauthorized => "401 Unauthorized",
            StatusCode::PaymentRequired => "402 Payment Required",
            StatusCode::Forbidden => "403 Forbidden",
            StatusCode::NotFound => "404 Not Found",
            StatusCode::MethodNotAllowed => "405 Method Not Allowed",
            StatusCode::NotAcceptable => "406 Not Acceptable",
            StatusCode::ProxyAuthenticationRequired => "407 Proxy Authentication Required",
            StatusCode::RequestTimeout => "408 Request Timeout",
            StatusCode::Conflict => "409 Conflict",
            StatusCode::Gone => "410 Gone",
            StatusCode::LengthRequired => "411 Length Required",
            StatusCode::PreconditionFailed => "412 Precondition Failed",
            StatusCode::ContentTooLarge => "413 Payload Too Large",
            StatusCode::URITooLong => "414 URI Too Long",
            StatusCode::UnsupportedMediaType => "415 Unsupported Media Type",
            StatusCode::RangeNotSatisfiable => "416 Range Not Satisfiable",
            StatusCode::ExpectationFailed => "417 Expectation Failed",
            #[allow(deprecated)]
            StatusCode::IMATeapot => "418 I'm a teapot",
            StatusCode::MisdirectedRequest => "421 Misdirected Request",
            StatusCode::UnprocessableContent => "422 Unprocessable Entity",
            StatusCode::Locked => "423 Locked",
            StatusCode::FailedDependency => "424 Failed Dependency",
            StatusCode::TooEarly => "425 Too Early",
            StatusCode::UpgradeRequired => "426 Upgrade Required",
            StatusCode::PreconditionRequired => "428 Precondition Required",
            StatusCode::TooManyRequests => "429 Too Many Requests",
            StatusCode::RequestHeaderFieldsTooLarge => "431 Request Header Fields Too Large",
            StatusCode::UnavailableForLegalReasons => "451 Unavailable For Legal Reasons",

            StatusCode::InternalServerError => "500 Internal Server Error",
            StatusCode::NotImplemented => "501 Not Implemented",
            StatusCode::BadGateway => "502 Bad Gateway",
            StatusCode::ServiceUnavailable => "503 Service Unavailable",
            StatusCode::GatewayTimeout => "504 Gateway Timeout",
            StatusCode::HTTPVersionNotSupported => "505 HTTP Version Not Supported",
            StatusCode::VariantAlsoNegotiates => "506 Variant Also Negotiates",
            StatusCode::InsufficientStorage => "507 Insufficient Storage",
            StatusCode::LoopDetected => "508 Loop Detected",
            #[allow(deprecated)]
            StatusCode::NotExtended => "510 Not Extended",
            StatusCode::NetworkAuthenticationRequired => "511 Network Authentication Required",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

/// Represents a value of a header.
///
/// This makes transforming the response easier for shared code paths, for
/// example when the header is used in multiple places, this avoids
/// serializing and deserializing which improves performance.
///
/// `HeaderValue` can also be used to restrict the types of setting various
/// `HeaderName`s. Most headers have a strict format, making them less
/// error-prone for the handler.
///
/// Another advantage is that we can have a simpler API to use, such as the
/// `SecFetchDest` enum.
///
/// At last, this removes the deserialization strain from the handler to
/// the transport code.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum HeaderValue {
    StaticString(&'static str),
    String(String),
    ContentCoding(ContentCoding),
    ContentRange(ContentRangeHeaderValue),
    DateTime(SystemTime),
    MediaType(MediaType),
    SecFetchDest(SecFetchDest),
    Size(usize),
}

impl HeaderValue {
    /// Returns the value as a string, but does not convert it to a string if
    /// it is some other non-convertible type.
    #[must_use]
    pub fn as_str_no_convert(&self) -> Option<&str> {
        match self {
            HeaderValue::StaticString(string) => Some(string),
            HeaderValue::String(string) => Some(string),
            _ => None,
        }
    }

    pub fn append_to_message(&self, response_text: &mut String) {
        match self {
            HeaderValue::StaticString(string) => {
                response_text.push_str(string);
            }
            HeaderValue::String(string) => {
                response_text.push_str(string);
            }
            HeaderValue::ContentCoding(content_coding) => {
                response_text.push_str(content_coding.http_identifier());
            }
            HeaderValue::ContentRange(content_range) => {
                response_text.push_str(&match content_range {
                    ContentRangeHeaderValue::Range { start, end, complete_length } => {
                        debug_assert!(start < end, "`start` must be less than `end` for Content-Range");
                        match complete_length {
                            Some(complete_length) => {
                                debug_assert!(end < complete_length, "`end` must be less than `complete_length` for Content-Range");
                                format!("bytes {}-{}/{}", start, end, complete_length)
                            }
                            None => format!("bytes {}-{}/*", start, end),
                        }
                    }
                    ContentRangeHeaderValue::Unsatisfied { complete_length } => {
                        format!("bytes */{}", complete_length)
                    }
                });
            }
            HeaderValue::DateTime(date_time) => {
                response_text.push_str(&httpdate::fmt_http_date(*date_time));
            }
            HeaderValue::MediaType(media_type) => {
                response_text.push_str(media_type.as_str());
            }
            HeaderValue::SecFetchDest(sec_fetch_dest) => {
                response_text.push_str(sec_fetch_dest.as_str());
            }
            HeaderValue::Size(size) => {
                response_text.push_str(&size.to_string());
            }
        }
    }

    /// Get the header in string form.
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        self.append_to_message(&mut result);
        result
    }

    /// Parses the value as a number.
    #[must_use]
    pub fn parse_number(&self) -> Option<usize> {
        match self {
            HeaderValue::StaticString(string) => string.parse().ok(),
            HeaderValue::String(string) => string.parse().ok(),
            HeaderValue::Size(size) => Some(*size),
            _ => None,
        }
    }

    /// Calculate the length of the header value in string characters.
    pub fn string_length(&self) -> usize {
        // Fast path, when the type is a string, or can easily be mapped into
        // one:
        match self {
            Self::StaticString(str) => return str.len(),
            Self::String(str) => return str.len(),
            Self::ContentCoding(coding) => return coding.http_identifier().len(),
            Self::ContentRange(_) => (),
            Self::DateTime(_) => (),
            Self::MediaType(media_type) => return media_type.as_str().len(),
            Self::SecFetchDest(sec_fetch_dest) => return sec_fetch_dest.as_str().len(),
            Self::Size(_) => (),
        }

        // Otherwise slow path, format it into a new string and get the length
        // of the string after formatting.

        let mut tmp_str = String::new();
        self.append_to_message(&mut tmp_str);
        tmp_str.len()
    }
}

impl From<ContentCoding> for HeaderValue {
    fn from(content_coding: ContentCoding) -> HeaderValue {
        HeaderValue::ContentCoding(content_coding)
    }
}

impl From<&'static str> for HeaderValue {
    fn from(string: &'static str) -> HeaderValue {
        HeaderValue::StaticString(string)
    }
}

impl From<String> for HeaderValue {
    fn from(string: String) -> HeaderValue {
        HeaderValue::String(string)
    }
}

impl From<SystemTime> for HeaderValue {
    fn from(date_time: SystemTime) -> HeaderValue {
        HeaderValue::DateTime(date_time)
    }
}

impl From<MediaType> for HeaderValue {
    fn from(media_type: MediaType) -> HeaderValue {
        HeaderValue::MediaType(media_type)
    }
}

impl From<SecFetchDest> for HeaderValue {
    fn from(sec_fetch_dest: SecFetchDest) -> HeaderValue {
        HeaderValue::SecFetchDest(sec_fetch_dest)
    }
}

impl From<usize> for HeaderValue {
    fn from(size: usize) -> HeaderValue {
        HeaderValue::Size(size)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeaderValueDateTimeParseError {
    InvalidFormat,
}

impl TryInto<SystemTime> for &HeaderValue {
    type Error = HeaderValueDateTimeParseError;

    fn try_into(self) -> Result<SystemTime, Self::Error> {
        match self {
            HeaderValue::StaticString(string) => httpdate::parse_http_date(string).map_err(|_| HeaderValueDateTimeParseError::InvalidFormat),
            HeaderValue::String(string) => httpdate::parse_http_date(string).map_err(|_| HeaderValueDateTimeParseError::InvalidFormat),
            HeaderValue::DateTime(date_time) => Ok(*date_time),
            _ => Err(HeaderValueDateTimeParseError::InvalidFormat),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct HeaderMap {
    headers: Vec<(HeaderName, HeaderValue)>,
}

impl HeaderMap {
    pub fn new() -> HeaderMap {
        HeaderMap::default()
    }

    pub fn new_with_vec(headers: Vec<(HeaderName, HeaderValue)>) -> HeaderMap {
        HeaderMap { headers }
    }

    /// Appends a header to the list of headers. This is used for headers that
    /// can be duplicated, such as `Set-Cookie` and `Link`.
    pub fn append_possible_duplicate(&mut self, header_name: HeaderName, value: HeaderValue) {
        self.headers.push((header_name, value));
    }

    #[must_use]
    pub fn contains(&self, header_name: &HeaderName) -> bool {
        for (name, _) in &self.headers {
            if name == header_name {
                return true;
            }
        }

        false
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.headers.len()
    }

    #[must_use]
    pub fn get(&self, header_name: &HeaderName) -> Option<&HeaderValue> {
        for (name, value) in &self.headers {
            if name == header_name {
                return Some(value);
            }
        }

        None
    }

    pub fn set(&mut self, header_name: HeaderName, value: HeaderValue) {
        for (name, existing_value) in &mut self.headers {
            if name == &header_name {
                *existing_value = value;
                return;
            }
        }

        self.headers.push((header_name, value));
    }

    pub fn remove(&mut self, header_name: &HeaderName) {
        self.headers.retain(|(name, _)| name != header_name);
    }

    pub fn iter(&self) -> impl Iterator<Item = &(HeaderName, HeaderValue)> {
        self.headers.iter()
    }
}

#[must_use]
pub fn format_system_time_as_weak_etag(date_time: SystemTime) -> String {
    format!("W/{:X}", date_time.duration_since(SystemTime::UNIX_EPOCH).unwrap_or(Duration::default()).as_secs())
}

//
// Header-specific methods
//
impl HeaderMap {
    #[must_use]
    pub fn sec_fetch_dest(&self) -> Option<SecFetchDest> {
        self.get(&HeaderName::SecFetchDest)
            .and_then(|value| value.as_str_no_convert())
            .and_then(SecFetchDest::parse)
    }

    pub fn set_content_length(&mut self, length: usize) {
        self.set(HeaderName::ContentLength, HeaderValue::Size(length));
    }

    pub fn set_content_range(&mut self, range: ContentRangeHeaderValue) {
        self.set(HeaderName::ContentRange, HeaderValue::ContentRange(range));
    }

    pub fn set_content_type(&mut self, media_type: MediaType) {
        self.set(HeaderName::ContentType, HeaderValue::MediaType(media_type));
    }

    pub fn set_last_modified(&mut self, date_time: SystemTime) {
        self.set(HeaderName::LastModified, HeaderValue::DateTime(date_time));
        if !self.contains(&HeaderName::ETag) {
            self.set(HeaderName::ETag, format_system_time_as_weak_etag(date_time).into());
        }
    }
}

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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestTarget {
    Origin {
        path: String,
        query: String,
    },
    Absolute(String),
    Authority(String),
    Asterisk,
}

impl RequestTarget {
    /// Returns the request target as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            RequestTarget::Origin{ path, .. } => path,
            RequestTarget::Absolute(string) => string,
            RequestTarget::Authority(string) => string,
            RequestTarget::Asterisk => "*",
        }
    }
}

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub target: RequestTarget,
    pub version: HttpVersion,
    pub headers: HeaderMap,
    pub body: Option<BodyKind>,
}

#[derive(Debug)]
pub enum BodyKind {
    Bytes(Vec<u8>),
    CachedBytes(Arc<ContentEncodedVersions>, Option<ContentCoding>),
    File {
        handle: tokio::fs::File,
        metadata: Metadata,
    },
    StaticString(&'static str),
    String(String),
}

#[derive(Debug)]
pub struct Response {
    /// Responses that are sent before this one, commonly 1xx response.
    /// E.g. 103 Early Hints.
    pub prelude_response: Vec<Response>,
    pub version: HttpVersion,
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Option<BodyKind>,
}

impl Response {
    pub fn with_status(status: StatusCode) -> Self {
        Self {
            prelude_response: Vec::new(),
            version: HttpVersion::Http11,
            status,
            headers: HeaderMap::new(),
            body: None,
        }
    }

    pub fn with_status_and_string_body(status: StatusCode, body: impl Into<Cow<'static, str>>) -> Self {
        let mut headers = HeaderMap::new();
        headers.set(HeaderName::ContentType, HeaderValue::from("text/plain; charset=utf-8"));
        Self {
            prelude_response: Vec::new(),
            version: HttpVersion::Http11,
            status,
            headers,
            body: match body.into() {
                Cow::Owned(body) => Some(BodyKind::String(body)),
                Cow::Borrowed(body) => Some(BodyKind::StaticString(body)),
            },
        }
    }

    pub fn bad_request(message: &'static str) -> Self {
        let mut response = Self::with_status(StatusCode::BadRequest);
        response.body = Some(BodyKind::StaticString(message));
        response
    }

    pub fn forbidden(message: &'static str) -> Self {
        let mut response = Self::with_status(StatusCode::Forbidden);
        response.body = Some(BodyKind::StaticString(message));
        response
    }

    pub fn not_found(message: &'static str) -> Self {
        let mut response = Self::with_status(StatusCode::NotFound);
        response.body = Some(BodyKind::StaticString(message));
        response
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Range {
    Full,
    StartPointToEnd { start: u64 },
    Points {
        start: u64,
        end: u64,
    },
    Suffix { suffix: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRangeList {
    pub ranges: Vec<Range>,
}

impl HttpRangeList {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        if !value.starts_with("bytes=") {
            return None;
        }
        let mut ranges = Vec::new();
        for range in value[6..].split(',') {
            let range = range.trim();
            if range.is_empty() {
                continue;
            }
            let range = if let Some(suffix) = range.strip_prefix('-') {
                Range::Suffix { suffix: suffix.parse().ok()? }
            } else if let Some(start) = range.strip_suffix('-') {
                Range::StartPointToEnd { start: start.parse().ok()? }
            } else {
                let mut range = range.splitn(2, '-');
                let start = range.next()?.parse().ok()?;
                let end = range.next()?.parse().ok()?;
                Range::Points { start, end }
            };
            ranges.push(range);
        }
        Some(Self { ranges })
    }

    /// Returns the first and only range if there is only one range.
    /// Otherwise, when there are 0 or more than one, returns `None`.
    #[must_use]
    pub fn first_and_only(&self) -> Option<Range> {
        if self.ranges.len() == 1 {
            Some(self.ranges[0])
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Range> {
        self.ranges.iter()
    }
}

/// The `Content-Range` header field indicates where in a full body a partial
/// message belongs.
///
/// ### References
/// * [RFC 9110](https://httpwg.org/specs/rfc9110.html#field.content-range)
/// * [MDN `Content-Range` header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Range)
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContentRangeHeaderValue {
    Range {
        /// The start of the range, inclusive.
        start: usize,

        /// The end of the range, inclusive.
        end: usize,

        /// Complete length of the **resource**, not the body.
        complete_length: Option<usize>,
    },

    /// Used for 416 Range Not Satisfiable.
    ///
    /// ### RFC 9110, section 14.4:
    /// > A server generating a 416 (Range Not Satisfiable) response to a
    /// byte-range request SHOULD send a Content-Range header field with an
    /// unsatisfied-range value, as in the following example:
    /// > ```
    /// > Content-Range: bytes */1234`
    /// > ```
    /// > The complete-length in a 416 response indicates the current length of
    /// > the selected representation.
    ///
    /// ### References
    /// * [MDN `416 Range Not Satisfiable` HTTP status code](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/416)
    /// * [MDN `Content-Range` header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Range)
    /// * [RFC 9110](https://httpwg.org/specs/rfc9110.html#field.content-range)
    Unsatisfied {
        /// The complete length of the resource.
        complete_length: usize
    },
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

            assert!(!name.to_string_h1().split('-').any(|str| !str.is_empty() && str.chars().nth(0).unwrap().is_ascii_lowercase()),
                "HTTP/1.1 Header names should have uppercase letters");
        }
    }

    #[test]
    fn test_header_value_string_length() {
        assert_eq!(HeaderValue::StaticString("hello").string_length(), 5);
        assert_eq!(HeaderValue::String(String::new()).string_length(), 0);
        assert_eq!(HeaderValue::String(String::from("This is a line.")).string_length(), 15);
        assert_eq!(HeaderValue::ContentCoding(ContentCoding::Brotli).string_length(), 2);
        assert_eq!(HeaderValue::ContentCoding(ContentCoding::Gzip).string_length(), 4);
        assert_eq!(HeaderValue::ContentRange(ContentRangeHeaderValue::Range { start: 99, end: 4783, complete_length: None }).string_length(), "bytes 99-4783/*".len());
        assert_eq!(HeaderValue::ContentRange(ContentRangeHeaderValue::Range { start: 0, end: 4, complete_length: Some(5) }).string_length(), "bytes 0-4/5".len());
        assert_eq!(HeaderValue::ContentRange(ContentRangeHeaderValue::Range { start: 0, end: 4, complete_length: Some(60) }).string_length(), "bytes 0-4/60".len());
        assert_eq!(HeaderValue::ContentRange(ContentRangeHeaderValue::Unsatisfied { complete_length: 10 }).string_length(), "bytes */10".len());
        assert_eq!(HeaderValue::MediaType(MediaType::HTML).string_length(), MediaType::HTML.as_str().len());
        assert_eq!(HeaderValue::SecFetchDest(SecFetchDest::Document).string_length(), "document".len());
        assert_eq!(HeaderValue::Size(100).string_length(), 3);
    }
}
