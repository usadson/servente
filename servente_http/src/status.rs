// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::borrow::Cow;

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
