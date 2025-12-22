use super::error_response::{ErrorResponse, ValidationErrorDetail};
use crate::http::response::into_response::IntoResponse;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Response, StatusCode};
use miko_core::Resp;
use serde_json::json;
use std::cell::RefCell;
use std::convert::Infallible;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    /// 用于存储当前请求的 trace_id
    static TRACE_ID: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// 设置当前请求的 trace_id
///
/// 通常在中间件或请求处理开始时调用
///
/// # Example
/// ```no_run
/// use miko::error::set_trace_id;
///
/// // 在中间件中设置
/// set_trace_id(Some("req-12345".to_string()));
/// ```
pub fn set_trace_id(trace_id: Option<String>) {
    TRACE_ID.with(|id| {
        *id.borrow_mut() = trace_id;
    });
}

/// 获取当前请求的 trace_id
pub fn get_trace_id() -> Option<String> {
    TRACE_ID.with(|id| id.borrow().clone())
}

/// 清除当前请求的 trace_id
///
/// 通常在请求处理结束时调用
pub fn clear_trace_id() {
    TRACE_ID.with(|id| {
        *id.borrow_mut() = None;
    });
}

/// 框架统一错误类型
///
/// 所有错误都会被转换为 HTTP 响应，提供一致的错误处理体验
#[derive(Debug)]
pub enum AppError {
    // ============ 客户端错误 (4xx) ============
    /// 400 Bad Request - 请求格式错误或参数不合法
    BadRequest(String),

    /// 401 Unauthorized - 未认证
    Unauthorized(String),

    /// 403 Forbidden - 已认证但无权限
    Forbidden(String),

    /// 404 Not Found - 资源不存在
    NotFound(String),

    /// 409 Conflict - 资源冲突（如重复创建）
    Conflict(String),

    /// 422 Unprocessable Entity - 验证失败
    UnprocessableEntity(String),

    /// 429 Too Many Requests - 请求过于频繁
    TooManyRequests(String),

    // ============ 服务器错误 (5xx) ============
    /// 500 Internal Server Error - 内部错误
    InternalServerError(String),

    /// 502 Bad Gateway - 网关错误
    BadGateway(String),

    /// 503 Service Unavailable - 服务不可用
    ServiceUnavailable(String),

    /// 504 Gateway Timeout - 网关超时
    GatewayTimeout(String),

    // ============ 具体错误类型 ============
    /// JSON 解析错误
    JsonParseError(serde_json::Error),

    /// URL 编码解析错误
    UrlEncodedParseError(serde_urlencoded::de::Error),

    /// Multipart 解析错误
    MultipartParseError(String),

    /// 验证错误（包含详细字段错误）
    ValidationError(Vec<ValidationErrorDetail>),

    /// 数据库错误
    DatabaseError(String),

    /// IO 错误
    IoError(std::io::Error),

    /// 超时错误
    Timeout(String),

    /// 第三方服务错误
    ExternalServiceError { service: String, message: String },

    // ============ 自定义错误 ============
    /// 自定义错误，允许完全控制状态码和响应内容
    Custom {
        status: StatusCode,
        error_code: String,
        message: String,
        details: Option<serde_json::Value>,
    },
}

impl AppError {
    /// 创建自定义错误
    pub fn custom(
        status: StatusCode,
        error_code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Custom {
            status,
            error_code: error_code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// 创建带详细信息的自定义错误
    pub fn custom_with_details(
        status: StatusCode,
        error_code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self::Custom {
            status,
            error_code: error_code.into(),
            message: message.into(),
            details: Some(details),
        }
    }

    /// 获取 HTTP 状态码
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_)
            | Self::JsonParseError(_)
            | Self::UrlEncodedParseError(_)
            | Self::MultipartParseError(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::UnprocessableEntity(_) | Self::ValidationError(_) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::InternalServerError(_)
            | Self::DatabaseError(_)
            | Self::IoError(_)
            | Self::ExternalServiceError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BadGateway(_) => StatusCode::BAD_GATEWAY,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::GatewayTimeout(_) | Self::Timeout(_) => StatusCode::GATEWAY_TIMEOUT,
            Self::Custom { status, .. } => *status,
        }
    }

    /// 获取错误代码
    pub fn error_code(&self) -> String {
        match self {
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::Unauthorized(_) => "UNAUTHORIZED",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Conflict(_) => "CONFLICT",
            Self::UnprocessableEntity(_) => "UNPROCESSABLE_ENTITY",
            Self::TooManyRequests(_) => "TOO_MANY_REQUESTS",
            Self::InternalServerError(_) => "INTERNAL_SERVER_ERROR",
            Self::BadGateway(_) => "BAD_GATEWAY",
            Self::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
            Self::GatewayTimeout(_) => "GATEWAY_TIMEOUT",
            Self::Timeout(_) => "TIMEOUT",
            Self::JsonParseError(_) => "JSON_PARSE_ERROR",
            Self::UrlEncodedParseError(_) => "URL_ENCODED_PARSE_ERROR",
            Self::MultipartParseError(_) => "MULTIPART_PARSE_ERROR",
            Self::ValidationError(_) => "VALIDATION_ERROR",
            Self::DatabaseError(_) => "DATABASE_ERROR",
            Self::IoError(_) => "IO_ERROR",
            Self::ExternalServiceError { .. } => "EXTERNAL_SERVICE_ERROR",
            Self::Custom { error_code, .. } => return error_code.clone(),
        }
        .to_string()
    }

    /// 获取错误消息
    pub fn message(&self) -> String {
        match self {
            Self::BadRequest(msg)
            | Self::Unauthorized(msg)
            | Self::Forbidden(msg)
            | Self::NotFound(msg)
            | Self::Conflict(msg)
            | Self::UnprocessableEntity(msg)
            | Self::TooManyRequests(msg)
            | Self::InternalServerError(msg)
            | Self::BadGateway(msg)
            | Self::ServiceUnavailable(msg)
            | Self::GatewayTimeout(msg)
            | Self::Timeout(msg)
            | Self::DatabaseError(msg)
            | Self::MultipartParseError(msg) => msg.clone(),
            Self::JsonParseError(e) => format!("Invalid JSON: {}", e),
            Self::UrlEncodedParseError(e) => format!("Invalid URL encoding: {}", e),
            Self::ValidationError(_) => "Request validation failed".to_string(),
            Self::IoError(e) => format!("IO error: {}", e),
            Self::ExternalServiceError { service, message } => {
                format!("External service '{}' error: {}", service, message)
            }
            Self::Custom { message, .. } => message.clone(),
        }
    }

    /// 获取错误详细信息
    pub fn details(&self) -> Option<serde_json::Value> {
        match self {
            Self::ValidationError(errors) => Some(json!({
                "fields": errors
            })),
            Self::ExternalServiceError { service, .. } => Some(json!({
                "service": service
            })),
            Self::Custom { details, .. } => details.clone(),
            _ => None,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error_code(), self.message())
    }
}

impl std::error::Error for AppError {}

// ============ From 实现：自动转换常见错误类型 ============

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonParseError(err)
    }
}

impl From<serde_urlencoded::de::Error> for AppError {
    fn from(err: serde_urlencoded::de::Error) -> Self {
        Self::UrlEncodedParseError(err)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<multer::Error> for AppError {
    fn from(err: multer::Error) -> Self {
        Self::MultipartParseError(err.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        // 对于任何 anyhow::Error，简单地转换为 InternalServerError
        // 因为现在提取器都直接返回 AppError 的具体类型，
        // 这个转换主要用于其他地方的 anyhow 错误
        Self::InternalServerError(err.to_string())
    }
}

impl From<Infallible> for AppError {
    fn from(_: Infallible) -> Self {
        Self::InternalServerError("Infallible error occurred".to_string())
    }
}

impl From<Box<dyn std::error::Error>> for AppError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Self::InternalServerError(err.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for AppError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::InternalServerError(err.to_string())
    }
}

// ============ garde 验证库集成 ============
#[cfg(feature = "validation")]
impl From<garde::Report> for AppError {
    fn from(report: garde::Report) -> Self {
        use crate::error::ValidationErrorDetail;

        let details: Vec<ValidationErrorDetail> = report
            .iter()
            .map(|(path, error)| ValidationErrorDetail {
                field: path.to_string(),
                message: error.to_string(),
                code: "VALIDATION_FAILED".to_string(),
            })
            .collect();

        Self::ValidationError(details)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Resp {
        let status = self.status_code();
        let error_code = self.error_code();
        let message = self.message();
        let details = self.details();

        // 记录服务器内部错误（5xx）
        if status.is_server_error() {
            tracing::error!(
                error_code = %error_code,
                message = %message,
                trace_id = ?get_trace_id(),
                "Internal server error"
            );
        }

        let error_response = ErrorResponse {
            status: status.as_u16(),
            error: error_code,
            message,
            details,
            trace_id: get_trace_id(), // 从 thread_local 获取
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let body = serde_json::to_string(&error_response).unwrap_or_else(|_| {
            // 如果序列化失败，返回一个简单的错误
            r#"{"error":"SERIALIZATION_ERROR","message":"Failed to serialize error response"}"#
                .to_string()
        });

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body)).map_err(Into::into).boxed())
            .unwrap_or_else(|_| {
                // 如果构建响应失败，返回一个最简单的 500 响应
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(
                        Full::new(Bytes::from(r#"{"error":"INTERNAL_SERVER_ERROR"}"#))
                            .map_err(Into::into)
                            .boxed(),
                    )
                    .unwrap()
            })
    }
}
