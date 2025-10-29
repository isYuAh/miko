use super::AppError;

/// 框架标准 Result 类型
///
/// 用于所有可能返回错误的操作，提供统一的错误处理体验
///
/// # 示例
///
/// ```no_run
/// use miko::error::{AppError, AppResult};
/// use miko::extractor::Json;
/// use serde::{Deserialize, Serialize};
///
/// // Define request and response structures
/// #[derive(Deserialize)]
/// struct UserData {
///     email: String,
/// }
///
/// #[derive(Serialize)]
/// struct User {
///     id: u64,
///     email: String,
/// }
///
/// async fn create_user(Json(data): Json<UserData>) -> AppResult<Json<User>> {
///     // Validation
///     if data.email.is_empty() {
///         return Err(AppError::BadRequest("Email is required".to_string()));
///     }
///
///     // Business logic...
///     let user = User { id: 1, email: data.email.clone() };
///     Ok(Json(user))
/// }
/// ```
pub type AppResult<T> = Result<T, AppError>;
