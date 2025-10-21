use serde::{Deserialize, Serialize};

/// 标准错误响应结构
///
/// 所有的错误都会被转换为这个统一的格式，方便客户端解析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// HTTP 状态码
    pub status: u16,

    /// 错误类型/代码（大写下划线格式，如 VALIDATION_ERROR）
    pub error: String,

    /// 人类可读的错误消息
    pub message: String,

    /// 详细错误信息（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,

    /// 请求追踪 ID（可选，用于日志关联）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,

    /// 错误发生时间戳（Unix 时间戳，秒）
    pub timestamp: u64,
}

/// 验证错误详情
///
/// 用于 ValidationError 类型，描述具体哪个字段验证失败
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationErrorDetail {
    /// 字段名称
    pub field: String,

    /// 错误消息
    pub message: String,

    /// 错误代码（如 REQUIRED, INVALID_FORMAT, MIN_LENGTH 等）
    pub code: String,
}

impl ValidationErrorDetail {
    /// 创建新的验证错误详情
    pub fn new(
        field: impl Into<String>,
        message: impl Into<String>,
        code: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: code.into(),
        }
    }

    /// 必填字段错误
    pub fn required(field: impl Into<String>) -> Self {
        let field = field.into();
        Self {
            field: field.clone(),
            message: format!("{} is required", field),
            code: "REQUIRED".to_string(),
        }
    }

    /// 格式错误
    pub fn invalid_format(field: impl Into<String>, expected: &str) -> Self {
        let field = field.into();
        Self {
            field: field.clone(),
            message: format!("{} has invalid format, expected: {}", field, expected),
            code: "INVALID_FORMAT".to_string(),
        }
    }

    /// 长度错误
    pub fn invalid_length(field: impl Into<String>, min: usize, max: usize) -> Self {
        let field = field.into();
        Self {
            field: field.clone(),
            message: format!("{} length must be between {} and {}", field, min, max),
            code: "INVALID_LENGTH".to_string(),
        }
    }

    /// 最小值错误
    pub fn min_value(field: impl Into<String>, min: impl std::fmt::Display) -> Self {
        let field = field.into();
        Self {
            field: field.clone(),
            message: format!("{} must be at least {}", field, min),
            code: "MIN_VALUE".to_string(),
        }
    }

    /// 最大值错误
    pub fn max_value(field: impl Into<String>, max: impl std::fmt::Display) -> Self {
        let field = field.into();
        Self {
            field: field.clone(),
            message: format!("{} must be at most {}", field, max),
            code: "MAX_VALUE".to_string(),
        }
    }
}
