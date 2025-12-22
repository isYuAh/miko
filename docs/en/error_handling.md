# Error Handling

Miko provides a unified error handling mechanism, allowing you to gracefully handle various error situations and return
consistent JSON-formatted error responses.

## Core Types

### AppError

The unified error type provided by the framework, covering common HTTP error scenarios:

```rust
pub enum AppError {
    // Client Errors (4xx)
    BadRequest(String),           // 400
    Unauthorized(String),          // 401
    Forbidden(String),             // 403
    NotFound(String),              // 404
    Conflict(String),              // 409
    UnprocessableEntity(String),   // 422
    TooManyRequests(String),       // 429

    // Server Errors (5xx)
    InternalServerError(String),   // 500
    BadGateway(String),            // 502
    ServiceUnavailable(String),    // 503
    GatewayTimeout(String),        // 504

    // Specific Error Types
    JsonParseError(serde_json::Error),
    UrlEncodedParseError(serde_urlencoded::de::Error),
    ValidationError(Vec<ValidationErrorDetail>),
    DatabaseError(String),
    IoError(std::io::Error),
    Timeout(String),
    ExternalServiceError { service: String, message: String },

    // Custom Error
    Custom {
        status: StatusCode,
        error_code: String,
        message: String,
        details: Option<serde_json::Value>,
    },
}
```

### AppResult

A type alias to simplify error handling:

```rust
pub type AppResult<T> = Result<T, AppError>;
```

### ErrorResponse

The unified error response format:

```rust
{
"status": 404,
"error": "NOT_FOUND",
"message": "User 123 not found",
"details": null,
"trace_id": "req-abc-123",
"timestamp": 1234567890
}
```

## Basic Usage

### Returning Errors

Return `AppResult` in your Handler:

```rust
use miko::{*, macros::*, AppResult, AppError};

#[get("/users/{id}")]
async fn get_user(#[path] id: u32) -> AppResult<Json<User>> {
    let user = db.find_user(id)
        .ok_or(AppError::NotFound(format!("User {} not found", id)))?;

    Ok(Json(user))
}
```

### Using the `?` Operator

```rust
#[post("/users")]
async fn create_user(Json(data): Json<CreateUser>) -> AppResult<Json<User>> {
    // Automatic conversion of various errors
    let user = db.create_user(data)?;  // std::io::Error -> AppError
    Ok(Json(user))
}
```

## Common Error Scenarios

### 404 Not Found

Resource does not exist:

```rust
#[get("/users/{id}")]
async fn get_user(#[path] id: u32) -> AppResult<Json<User>> {
    let user = db.find_user(id)
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;
    Ok(Json(user))
}
```

Response example:

```json
{
  "status": 404,
  "error": "NOT_FOUND",
  "message": "User 123 not found",
  "timestamp": 1234567890
}
```

### 400 Bad Request

Invalid request parameters:

```rust
#[post("/users")]
async fn create_user(Json(data): Json<CreateUser>) -> AppResult<Json<User>> {
    if data.email.is_empty() {
        return Err(AppError::BadRequest("Email is required".into()));
    }

    if !data.email.contains('@') {
        return Err(AppError::BadRequest("Invalid email format".into()));
    }

    Ok(Json(db.create_user(data)?))
}
```

### 401 Unauthorized

Authentication failed:

```rust
#[get("/profile")]
async fn get_profile(headers: HeaderMap) -> AppResult<Json<Profile>> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing auth token".into()))?;

    let user = verify_token(token)
        .ok_or_else(|| AppError::Unauthorized("Invalid or expired token".into()))?;

    Ok(Json(get_profile_for_user(user)))
}
```

### 403 Forbidden

Authenticated but no permission:

```rust
#[delete("/posts/{id}")]
async fn delete_post(
    #[path] id: u32,
    user: AuthUser,  // Custom extractor
) -> AppResult<StatusCode> {
    let post = db.find_post(id)
        .ok_or(AppError::NotFound("Post not found".into()))?;

    if post.author_id != user.id && !user.is_admin {
        return Err(AppError::Forbidden("You don't have permission to delete this post".into()));
    }

    db.delete_post(id)?;
    Ok(StatusCode::NO_CONTENT)
}
```

### 409 Conflict

Resource conflict:

```rust
#[post("/users")]
async fn create_user(Json(data): Json<CreateUser>) -> AppResult<Json<User>> {
    if db.email_exists(&data.email) {
        return Err(AppError::Conflict("Email already exists".into()));
    }

    if db.username_exists(&data.username) {
        return Err(AppError::Conflict("Username already taken".into()));
    }

    Ok(Json(db.create_user(data)?))
}
```

### 422 Validation Error

Data validation failed:

> We recommend using `ValidatedJson` for automatic validation. See [Data Validation](data_validation.md) for details.

```rust
use miko::ValidationErrorDetail;

#[post("/users")]
async fn create_user(Json(data): Json<CreateUser>) -> AppResult<Json<User>> {
    let mut errors = Vec::new();

    if data.name.len() < 3 {
        errors.push(ValidationErrorDetail::invalid_length("name", 3, 50));
    }

    if !data.email.contains('@') {
        errors.push(ValidationErrorDetail::invalid_format("email", "valid email address"));
    }

    if data.age < 18 {
        errors.push(ValidationErrorDetail::min_value("age", 18));
    }

    if !errors.is_empty() {
        return Err(AppError::ValidationError(errors));
    }

    Ok(Json(db.create_user(data)?))
}
```

Response example:

```json
{
  "status": 422,
  "error": "VALIDATION_ERROR",
  "message": "Request validation failed",
  "details": {
    "fields": [
      {
        "field": "name",
        "message": "name length must be between 3 and 50",
        "code": "VALIDATION_FAILED"
      },
      {
        "field": "email",
        "message": "email has invalid format, expected: valid email address",
        "code": "VALIDATION_FAILED"
      }
    ]
  },
  "timestamp": 1234567890
}
```

### 500 Internal Server Error

Server internal error:

```rust
#[get("/users")]
async fn list_users() -> AppResult<Json<Vec<User>>> {
    let users = db.query_users()
        .map_err(|e| AppError::DatabaseError(format!("Failed to query users: {}", e)))?;

    Ok(Json(users))
}
```

## Automatic Error Conversion

The framework implements the `From` trait for common error types:

```rust
// std::io::Error
let data = tokio::fs::read("file.txt").await?;  // Automatic conversion

// serde_json::Error
let value: MyData = serde_json::from_str( & json) ?;  // Automatic conversion

// multer::Error (File upload)
while let Some(field) = multipart.next_field().await? {  // Automatic conversion
// ...
}
```

Supported automatic conversions:

| Error Type                    | Converted To                                                |
|-------------------------------|-------------------------------------------------------------|
| `std::io::Error`              | `AppError::IoError`                                         |
| `serde_json::Error`           | `AppError::JsonParseError`                                  |
| `serde_urlencoded::de::Error` | `AppError::UrlEncodedParseError`                            |
| `multer::Error`               | `AppError::MultipartParseError`                             |
| `anyhow::Error`               | `AppError::InternalServerError`                             |
| `garde::Report`               | `AppError::ValidationError` (requires `validation` feature) |

## Custom Errors

### Using the Custom Variant

Completely customize error responses:

```rust
use hyper::StatusCode;
use serde_json::json;

#[get("/custom-error")]
async fn handler() -> AppResult<()> {
    Err(AppError::custom(
        StatusCode::PAYMENT_REQUIRED,
        "PAYMENT_REQUIRED",
        "Please upgrade to premium"
    ))
}

// With detailed information
#[get("/rate-limit")]
async fn rate_limited() -> AppResult<()> {
    Err(AppError::custom_with_details(
        StatusCode::TOO_MANY_REQUESTS,
        "RATE_LIMIT_EXCEEDED",
        "Too many requests",
        json!({
            "limit": 100,
            "remaining": 0,
            "reset_at": 1234567890
        })
    ))
}
```

### Implementing From for Custom Errors

```rust
// Custom business errors
enum BusinessError {
    InsufficientBalance,
    ProductOutOfStock,
    InvalidCoupon(String),
}

impl From<BusinessError> for AppError {
    fn from(err: BusinessError) -> Self {
        match err {
            BusinessError::InsufficientBalance => {
                AppError::BadRequest("Insufficient balance".into())
            }
            BusinessError::ProductOutOfStock => {
                AppError::Conflict("Product is out of stock".into())
            }
            BusinessError::InvalidCoupon(code) => {
                AppError::BadRequest(format!("Invalid coupon code: {}", code))
            }
        }
    }
}

// Usage
#[post("/orders")]
async fn create_order(Json(data): Json<CreateOrder>) -> AppResult<Json<Order>> {
    let order = business_logic::create_order(data)?;  // BusinessError automatically converted
    Ok(Json(order))
}
```

## ValidationErrorDetail Helpers

Quickly create validation errors:

```rust
use miko::ValidationErrorDetail;

// Required field
let error = ValidationErrorDetail::required("email");

// Format error
let error = ValidationErrorDetail::invalid_format("email", "valid email");

// Length error
let error = ValidationErrorDetail::invalid_length("name", 3, 50);

// Min value
let error = ValidationErrorDetail::min_value("age", 18);

// Max value
let error = ValidationErrorDetail::max_value("age", 120);

// Custom
let error = ValidationErrorDetail::new(
"field_name",
"Custom error message",
"CUSTOM_CODE"
);
```

## Trace ID

The framework **automatically** sets a Trace ID for each request and includes it in error responses.

### Automatic Generation

Trace ID generation rules (by priority):

1. Get from `x-trace-id` request header
2. Get from `x-request-id` request header
3. Automatically generated (format: `trace-{timestamp}-{thread_id}`)

### Accessing in Code

You can get the Trace ID of the current request in a Handler:

```rust
use miko::error::get_trace_id;

#[get("/users")]
async fn list_users() -> AppResult<Json<Vec<User>>> {
    // Get current request's trace_id
    let trace_id = get_trace_id().unwrap_or_default();

    // Use for logging
    tracing::info!(trace_id = %trace_id, "Querying users");

    let users = db.query_users()?;
    Ok(Json(users))
}
```

### Included Automatically in Error Responses

All error responses automatically include the Trace ID:

```json
{
  "status": 500,
  "error": "DATABASE_ERROR",
  "message": "Query failed",
  "trace_id": "trace-123abc-ThreadId(5)",
  // Automatically added
  "timestamp": 1234567890
}
```

> **Note**:
> - The framework automatically sets and cleans up the Trace ID; you **do not need to call** `set_trace_id()` or
    `clear_trace_id()` manually.
> - If your client sends an `x-trace-id` header, the framework will use it (useful for distributed tracing).

See [Advanced Features - Trace ID](advanced_features.md#trace-id) for more details.

## Error Logging

5xx errors are automatically logged:

```rust
// Server errors are automatically logged
#[get("/crash")]
async fn crash() -> AppResult<()> {
    Err(AppError::InternalServerError("Something went wrong".into()))
    // Automatic log output: error_code=INTERNAL_SERVER_ERROR message="Something went wrong" trace_id=...
}

// 4xx client errors are not logged (considered normal business cases)
#[get("/not-found")]
async fn not_found() -> AppResult<()> {
    Err(AppError::NotFound("Resource not found".into()))
    // No log output
}
```

## Next Steps

- ✅ Learn [Data Validation](data_validation.md) for automatic input validation
- 🔍 Understand [Request Extractor](request_extractors.md) error handling
- 📤 Master various ways of [Response Handling](response_handling.md)
- 🔐 Use [Middleware](middleware_and_layers.md) to add global error handling
