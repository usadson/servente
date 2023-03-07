// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::borrow::Cow;

use phf::phf_map;
use unicase::UniCase;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HttpVersion {
    Http09,
    Http10,
    Http11,
    Http2,
    Http3,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u16)]
pub enum StatusCode {
    Other(u16),

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
    pub fn to_string<'a>(&self) -> Cow<'a, str> {
        Cow::Borrowed(match self {
            StatusCode::Other(code) => return Cow::Owned(format!("{} Some Status", code)),

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
    AccessControlAllowOrigin,
    Age,
    AltSvc,
    CacheControl,
    CacheStatus,
    Close,
    Connection,
    Cookie,
    ContentEncoding,
    ContentLength,
    ContentSecurityPolicy,
    ContentSecurityPolicyReportOnly,
    ContentType,
    Date,
    DNT,
    ETag,
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
    Referer,
    ReferrerPolicy,
    SecChUa,
    SecChUaMobile,
    SecChUaPlatform,
    SecFetchDest,
    SecFetchMode,
    SecFetchSite,
    SecFetchUser,
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
    Trailer,
    TransferEncoding,
    Upgrade,
    UpgradeInsecureRequests,
    UserAgent,
    Vary,
    Via,
    XContentTypeOptions,
    XFrameOptions,
    XRequestedWith,
    XXSSProtection,
}

static STRING_TO_HEADER_NAME_MAP: phf::Map<UniCase<&'static str>, HeaderName> = phf_map!(
    UniCase::ascii("accept") => HeaderName::Accept,
    UniCase::ascii("accept-encoding") => HeaderName::AcceptEncoding,
    UniCase::ascii("accept-language") => HeaderName::AcceptLanguage,
    UniCase::ascii("accept-ranges") => HeaderName::AcceptRanges,
    UniCase::ascii("access-control-allow-origin") => HeaderName::AccessControlAllowOrigin,
    UniCase::ascii("age") => HeaderName::Age,
    UniCase::ascii("alt-svc") => HeaderName::AltSvc,
    UniCase::ascii("cache-control") => HeaderName::CacheControl,
    UniCase::ascii("cache-status") => HeaderName::CacheStatus,
    UniCase::ascii("close") => HeaderName::Close,
    UniCase::ascii("connection") => HeaderName::Connection,
    UniCase::ascii("cookie") => HeaderName::Cookie,
    UniCase::ascii("content-encoding") => HeaderName::ContentEncoding,
    UniCase::ascii("content-length") => HeaderName::ContentLength,
    UniCase::ascii("content-security-policy") => HeaderName::ContentSecurityPolicy,
    UniCase::ascii("content-security-policy-report-only") => HeaderName::ContentSecurityPolicyReportOnly,
    UniCase::ascii("content-type") => HeaderName::ContentType,
    UniCase::ascii("date") => HeaderName::Date,
    UniCase::ascii("dnt") => HeaderName::DNT,
    UniCase::ascii("etag") => HeaderName::ETag,
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
    UniCase::ascii("proxy-status") => HeaderName::ProxyStatus,
    UniCase::ascii("referer") => HeaderName::Referer,
    UniCase::ascii("referrer-policy") => HeaderName::ReferrerPolicy,
    UniCase::ascii("sec-ch-ua") => HeaderName::SecChUa,
    UniCase::ascii("sec-ch-ua-mobile") => HeaderName::SecChUaMobile,
    UniCase::ascii("sec-ch-ua-platform") => HeaderName::SecChUaPlatform,
    UniCase::ascii("sec-fetch-dest") => HeaderName::SecFetchDest,
    UniCase::ascii("sec-fetch-mode") => HeaderName::SecFetchMode,
    UniCase::ascii("sec-fetch-site") => HeaderName::SecFetchSite,
    UniCase::ascii("sec-fetch-user") => HeaderName::SecFetchUser,
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
    UniCase::ascii("trailer") => HeaderName::Trailer,
    UniCase::ascii("transfer-encoding") => HeaderName::TransferEncoding,
    UniCase::ascii("upgrade") => HeaderName::Upgrade,
    UniCase::ascii("upgrade-insecure-requests") => HeaderName::UpgradeInsecureRequests,
    UniCase::ascii("user-agent") => HeaderName::UserAgent,
    UniCase::ascii("vary") => HeaderName::Vary,
    UniCase::ascii("via") => HeaderName::Via,
    UniCase::ascii("x-content-type-options") => HeaderName::XContentTypeOptions,
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
            HeaderName::AccessControlAllowOrigin => "Access-Control-Allow-Origin",
            HeaderName::Age => "Age",
            HeaderName::AltSvc => "Alt-Svc",
            HeaderName::CacheControl => "Cache-Control",
            HeaderName::CacheStatus => "Cache-Status",
            HeaderName::Close => "Close",
            HeaderName::Connection => "Connection",
            HeaderName::Cookie => "Cookie",
            HeaderName::ContentEncoding => "Content-Encoding",
            HeaderName::ContentLength => "Content-Length",
            HeaderName::ContentSecurityPolicy => "Content-Security-Policy",
            HeaderName::ContentSecurityPolicyReportOnly => "Content-Security-Policy-Report-Only",
            HeaderName::ContentType => "Content-Type",
            HeaderName::Date => "Date",
            HeaderName::DNT => "DNT",
            HeaderName::ETag => "ETag",
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
            HeaderName::Referer => "Referer",
            HeaderName::ReferrerPolicy => "Referrer-Policy",
            HeaderName::SecChUa => "Sec-Ch-Ua",
            HeaderName::SecChUaMobile => "Sec-Ch-Ua-Mobile",
            HeaderName::SecChUaPlatform => "Sec-Ch-Ua-Platform",
            HeaderName::SecFetchDest => "Sec-Fetch-Dest",
            HeaderName::SecFetchMode => "Sec-Fetch-Mode",
            HeaderName::SecFetchSite => "Sec-Fetch-Site",
            HeaderName::SecFetchUser => "Sec-Fetch-User",
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
            HeaderName::Trailer => "Trailer",
            HeaderName::TransferEncoding => "Transfer-Encoding",
            HeaderName::Upgrade => "Upgrade",
            HeaderName::UpgradeInsecureRequests => "Upgrade-Insecure-Requests",
            HeaderName::UserAgent => "User-Agent",
            HeaderName::Vary => "Vary",
            HeaderName::Via => "Via",
            HeaderName::XContentTypeOptions => "X-Content-Type-Options",
            HeaderName::XFrameOptions => "X-Frame-Options",
            HeaderName::XRequestedWith => "X-Requested-With",
            HeaderName::XXSSProtection => "X-XSS-Protection",
        }
    }
}

#[derive(Clone, Debug)]
pub struct HeaderMap {
    headers: Vec<(HeaderName, String)>,
}

impl HeaderMap {
    pub fn new() -> HeaderMap {
        HeaderMap { headers: Vec::new() }
    }

    pub fn new_with_vec(headers: Vec<(HeaderName, String)>) -> HeaderMap {
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

    pub fn get(&self, header_name: &HeaderName) -> Option<&str> {
        for (name, value) in &self.headers {
            if name == header_name {
                return Some(value);
            }
        }

        None
    }

    pub fn set(&mut self, header_name: HeaderName, value: String) {
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

    pub fn iter(&self) -> impl Iterator<Item = (&HeaderName, &str)> {
        self.headers.iter().map(|(name, value)| (name, value.as_str()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    Origin(String),
    Absolute(String),
    Authority(String),
    Asterisk,
}

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub target: RequestTarget,
    pub version: HttpVersion,
    pub headers: HeaderMap,
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
        headers.set(HeaderName::ContentType, "text/plain; charset=utf-8".to_owned());
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
