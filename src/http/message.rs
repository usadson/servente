// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{borrow::Cow, time::SystemTime};

use phf::phf_map;
use unicase::UniCase;

use crate::resources::MediaType;

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
    AltSvc,
    Authorization,
    CacheControl,
    CacheStatus,
    Close,
    Connection,
    Cookie,
    ContentEncoding,
    ContentLanguage,
    ContentLength,
    ContentRange,
    ContentSecurityPolicy,
    ContentSecurityPolicyReportOnly,
    ContentType,
    CrossOriginResourcePolicy,
    Date,
    DNT,
    EarlyData,
    ETag,
    ExpectCT,
    Forwarded,
    Host,
    IfMatch,
    IfModifiedSince,
    IfNoneMatch,
    IfRange,
    IfUnmodifiedSince,
    LastModified,
    Link,
    Location,
    ProxyStatus,
    Origin,
    Pragma,
    Purpose,
    Range,
    Referer,
    ReferrerPolicy,
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
    XContentTypeOptions,
    XForwaredFor,
    XFrameOptions,
    XRequestedWith,
    XXSSProtection,
}

static STRING_TO_HEADER_NAME_MAP: phf::Map<UniCase<&'static str>, HeaderName> = phf_map!(
    UniCase::ascii("accept") => HeaderName::Accept,
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
    UniCase::ascii("alt-svc") => HeaderName::AltSvc,
    UniCase::ascii("authorization") => HeaderName::Authorization,
    UniCase::ascii("cache-control") => HeaderName::CacheControl,
    UniCase::ascii("cache-status") => HeaderName::CacheStatus,
    UniCase::ascii("close") => HeaderName::Close,
    UniCase::ascii("connection") => HeaderName::Connection,
    UniCase::ascii("cookie") => HeaderName::Cookie,
    UniCase::ascii("content-encoding") => HeaderName::ContentEncoding,
    UniCase::ascii("content-length") => HeaderName::ContentLength,
    UniCase::ascii("content-language") => HeaderName::ContentLanguage,
    UniCase::ascii("content-range") => HeaderName::ContentRange,
    UniCase::ascii("content-security-policy") => HeaderName::ContentSecurityPolicy,
    UniCase::ascii("content-security-policy-report-only") => HeaderName::ContentSecurityPolicyReportOnly,
    UniCase::ascii("content-type") => HeaderName::ContentType,
    UniCase::ascii("cross-origin-resource-policy") => HeaderName::CrossOriginResourcePolicy,
    UniCase::ascii("date") => HeaderName::Date,
    UniCase::ascii("dnt") => HeaderName::DNT,
    UniCase::ascii("early-data") => HeaderName::EarlyData,
    UniCase::ascii("etag") => HeaderName::ETag,
    UniCase::ascii("expect-ct") => HeaderName::ExpectCT,
    UniCase::ascii("forwarded") => HeaderName::Forwarded,
    UniCase::ascii("host") => HeaderName::Host,
    UniCase::ascii("if-match") => HeaderName::IfMatch,
    UniCase::ascii("if-modified-since") => HeaderName::IfModifiedSince,
    UniCase::ascii("if-none-match") => HeaderName::IfNoneMatch,
    UniCase::ascii("if-range") => HeaderName::IfRange,
    UniCase::ascii("if-unmodified-since") => HeaderName::IfUnmodifiedSince,
    UniCase::ascii("last-modified") => HeaderName::LastModified,
    UniCase::ascii("link") => HeaderName::Link,
    UniCase::ascii("location") => HeaderName::Location,
    UniCase::ascii("origin") => HeaderName::Origin,
    UniCase::ascii("pragma") => HeaderName::Pragma,
    UniCase::ascii("purpose") => HeaderName::Purpose,
    UniCase::ascii("proxy-status") => HeaderName::ProxyStatus,
    UniCase::ascii("range") => HeaderName::Range,
    UniCase::ascii("referer") => HeaderName::Referer,
    UniCase::ascii("referrer-policy") => HeaderName::ReferrerPolicy,
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
    UniCase::ascii("x-content-type-options") => HeaderName::XContentTypeOptions,
    UniCase::ascii("x-forwarded-for") => HeaderName::XForwaredFor,
    UniCase::ascii("x-frame-options") => HeaderName::XFrameOptions,
    UniCase::ascii("x-requested-with") => HeaderName::XRequestedWith,
    UniCase::ascii("x-xss-protection") => HeaderName::XXSSProtection,
);

impl HeaderName {
    pub fn from_str(string: String) -> HeaderName {
        match STRING_TO_HEADER_NAME_MAP.get(&UniCase::ascii(&string)) {
            Some(header_name) => header_name.clone(),
            None => {
                let mut string = string;

                string.make_ascii_lowercase();
                HeaderName::Other(string)
            }
        }
    }

    pub fn to_string_h1(&self) -> &str {
        match self {
            HeaderName::Other(str) => str,

            HeaderName::Accept => "Accept",
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
            HeaderName::AltSvc => "Alt-Svc",
            HeaderName::Authorization => "Authorization",
            HeaderName::CacheControl => "Cache-Control",
            HeaderName::CacheStatus => "Cache-Status",
            HeaderName::Close => "Close",
            HeaderName::Connection => "Connection",
            HeaderName::Cookie => "Cookie",
            HeaderName::ContentEncoding => "Content-Encoding",
            HeaderName::ContentLanguage => "Content-Language",
            HeaderName::ContentLength => "Content-Length",
            HeaderName::ContentRange => "Content-Range",
            HeaderName::ContentSecurityPolicy => "Content-Security-Policy",
            HeaderName::ContentSecurityPolicyReportOnly => "Content-Security-Policy-Report-Only",
            HeaderName::ContentType => "Content-Type",
            HeaderName::CrossOriginResourcePolicy => "Cross-Origin-Resource-Policy",
            HeaderName::Date => "Date",
            HeaderName::DNT => "DNT",
            HeaderName::EarlyData => "Early-Data",
            HeaderName::ETag => "ETag",
            HeaderName::ExpectCT => "ExpectCT",
            HeaderName::Forwarded => "Forwarded",
            HeaderName::Host => "Host",
            HeaderName::IfMatch => "If-Match",
            HeaderName::IfModifiedSince => "If-Modified-Since",
            HeaderName::IfNoneMatch => "If-None-Match",
            HeaderName::IfRange => "If-Range",
            HeaderName::IfUnmodifiedSince => "If-Unmodified-Since",
            HeaderName::LastModified => "Last-Modified",
            HeaderName::Link => "Link",
            HeaderName::Location => "Location",
            HeaderName::Origin => "Origin",
            HeaderName::Pragma => "Pragma",
            HeaderName::ProxyStatus => "Proxy-Status",
            HeaderName::Purpose => "Purpose",
            HeaderName::Range => "Range",
            HeaderName::Referer => "Referer",
            HeaderName::ReferrerPolicy => "Referrer-Policy",
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
            HeaderName::XContentTypeOptions => "X-Content-Type-Options",
            HeaderName::XForwaredFor => "X-Forwarded-For",
            HeaderName::XFrameOptions => "X-Frame-Options",
            HeaderName::XRequestedWith => "X-Requested-With",
            HeaderName::XXSSProtection => "X-XSS-Protection",
        }
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
    ContentRange(ContentRangeHeaderValue),
    DateTime(SystemTime),
    MediaType(MediaType),
    SecFetchDest(SecFetchDest),
    Size(usize),
}

impl HeaderValue {
    /// Returns the value as a string, but does not convert it to a string if
    /// it is some other non-convertible type.
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

    /// Parses the value as a number.
    pub fn parse_number(&self) -> Option<usize> {
        match self {
            HeaderValue::StaticString(string) => string.parse().ok(),
            HeaderValue::String(string) => string.parse().ok(),
            HeaderValue::Size(size) => Some(*size),
            _ => None,
        }
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
            HeaderValue::String(string) => httpdate::parse_http_date(&string).map_err(|_| HeaderValueDateTimeParseError::InvalidFormat),
            HeaderValue::DateTime(date_time) => Ok(*date_time),
            _ => Err(HeaderValueDateTimeParseError::InvalidFormat),
        }
    }
}

#[derive(Clone, Debug)]
pub struct HeaderMap {
    headers: Vec<(HeaderName, HeaderValue)>,
}

impl HeaderMap {
    pub fn new() -> HeaderMap {
        HeaderMap { headers: Vec::new() }
    }

    pub fn new_with_vec(headers: Vec<(HeaderName, HeaderValue)>) -> HeaderMap {
        HeaderMap { headers }
    }

    pub fn contains(&self, header_name: &HeaderName) -> bool {
        for (name, _) in &self.headers {
            if name == header_name {
                return true;
            }
        }

        false
    }

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

//
// Header-specific methods
//
impl HeaderMap {
    pub fn sec_fetch_dest(&self) -> Option<SecFetchDest> {
        self.get(&HeaderName::SecFetchDest)
            .and_then(|value| value.as_str_no_convert())
            .and_then(|string| SecFetchDest::parse(string))
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

impl Method {
    pub fn from_str(string: String) -> Method {
        match METHOD_MAP.get(&UniCase::ascii(&string)) {
            Some(method) => method.clone(),
            None => Method::Other(string),
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
    File(tokio::fs::File),
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
            let range = if range.starts_with('-') {
                let suffix = range[1..].parse().ok()?;
                Range::Suffix { suffix }
            } else if range.ends_with('-') {
                let start = range[..range.len() - 1].parse().ok()?;
                Range::StartPointToEnd { start }
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
