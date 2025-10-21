use super::AppError;

/// 框架标准 Result 类型
///
/// 用于所有可能返回错误的操作，提供统一的错误处理体验
///
/// # 示例
///
/// ```no_run
/// use miko::error::AppResult;
/// use miko::extractor::Json;
///
/// async fn create_user(data: Json<UserData>) -> AppResult<Json<User>> {
///     // 验证
///     if data.email.is_empty() {
///         return Err(AppError::BadRequest("Email is required".to_string()));
///     }
///     
///     // 业务逻辑...
///     Ok(Json(user))
/// }
/// ```
pub type AppResult<T> = Result<T, AppError>;
