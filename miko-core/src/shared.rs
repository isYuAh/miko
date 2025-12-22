use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::{Request, Response};
use std::fmt;

pub struct MikoError(pub Box<dyn std::error::Error + Send + Sync + 'static>);

impl fmt::Debug for MikoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for MikoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for MikoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for MikoError {
    fn from(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        MikoError(err)
    }
}

impl From<std::convert::Infallible> for MikoError {
    fn from(err: std::convert::Infallible) -> Self {
        match err {}
    }
}

impl From<hyper::Error> for MikoError {
    fn from(err: hyper::Error) -> Self {
        MikoError(Box::new(err))
    }
}

impl From<std::io::Error> for MikoError {
    fn from(err: std::io::Error) -> Self {
        MikoError(Box::new(err))
    }
}

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub enum HTTPStatusCode {
    OK = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,
    BadRequest = 400,
    Unauthorized = 401,
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
    PayloadTooLarge = 413,
    URITooLong = 414,
    UnsupportedMediaType = 415,
    RangeNotSatisfiable = 416,
    ExpectationFailed = 417,
    ImATeapot = 418,
    MisdirectedRequest = 421,
    UnprocessableEntity = 422,
    Locked = 423,
    FailedDependency = 424,
    TooEarly = 425,
    UpgradeRequired = 426,
    PreconditionRequired = 428,
    TooManyRequests = 429,
    RequestHeaderFieldsTooLarge = 431,
    UnavailableForLegalReasons = 451,
    InternalServerError = 500,
}

pub type RespBody = BoxBody<Bytes, MikoError>;
pub type ReqBody = BoxBody<Bytes, MikoError>;
pub type Resp = Response<RespBody>;
pub type Req = Request<ReqBody>;
