/// 统一错误处理模块
///
/// 提供框架级别的统一错误类型、错误响应格式和错误处理机制
pub mod app_error;
pub mod error_response;
pub mod result;

pub use app_error::{AppError, get_trace_id};
pub use error_response::{ErrorResponse, ValidationErrorDetail};
pub use result::AppResult;
