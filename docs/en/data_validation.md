# Data Validation

> **Requires `validation` feature**

Miko provides the `ValidatedJson` extractor, which automatically validates request data and returns a unified error
response. It is built on top of the [garde](https://github.com/jprochazk/garde) validation library.

## Core Features

### `ValidatedJson` Extractor

Use `ValidatedJson` instead of `Json` to execute validation automatically:

```rust
use miko::*;
use miko::macros::*;

#[derive(Deserialize, Validate)]
struct CreateUser {
    #[garde(length(min = 3, max = 20))]
    username: String,

    #[garde(email)]
    email: String,

    #[garde(length(min = 8))]
    password: String,

    #[garde(range(min = 18, max = 120))]
    age: u8,
}

#[post("/users")]
async fn create_user(
    ValidatedJson(data): ValidatedJson<CreateUser>
) -> impl IntoResponse {
    // ✅ data is validated
    Json(json!({
        "message": "User created successfully",
        "username": data.username
    }))
}
```

**Comparison with standard `Json` extractor:**

```rust
// ❌ Using Json - No validation
#[post("/users")]
async fn create_user(Json(data): Json<CreateUser>) -> impl IntoResponse {
    // ⚠️ data may contain invalid data
    // Manual validation needed
}

// ✅ Using ValidatedJson - Automatic validation
#[post("/users")]
async fn create_user(ValidatedJson(data): ValidatedJson<CreateUser>) -> impl IntoResponse {
    // ✅ data has passed all validation rules
}
```

### Automatic Error Response

A 422 error is automatically returned when validation fails:

```json
{
  "status": 422,
  "error": "VALIDATION_ERROR",
  "message": "Request validation failed",
  "details": {
    "fields": [
      {
        "field": "username",
        "message": "username length must be between 3 and 20",
        "code": "VALIDATION_FAILED"
      },
      {
        "field": "email",
        "message": "email has invalid format, expected: valid email address",
        "code": "VALIDATION_FAILED"
      }
    ]
  },
  "trace_id": "trace-123abc",
  "timestamp": 1234567890
}
```

> The error response format is consistent with the framework's [Error Handling](error_handling.md) system.

## Common Validation Rules

### String Validation

```rust
#[derive(Deserialize, Validate)]
struct StringExample {
    // Length constraints
    #[garde(length(min = 3, max = 50))]
    username: String,

    // Email format
    #[garde(email)]
    email: String,

    // URL format
    #[garde(url)]
    website: Option<String>,

    // Regular expression
    #[garde(pattern(r"^[A-Z]{2}\d{4}$"))]
    code: String,
}
```

### Numeric Validation

```rust
#[derive(Deserialize, Validate)]
struct NumberExample {
    // Range
    #[garde(range(min = 18, max = 120))]
    age: u8,

    // Min value
    #[garde(range(min = 0.0))]
    price: f64,
}
```

### Collection Validation

```rust
#[derive(Deserialize, Validate)]
struct CollectionExample {
    // Array length
    #[garde(length(min = 1, max = 10))]
    tags: Vec<String>,

    // Validate each element in array
    #[garde(dive)]
    items: Vec<Item>,
}

#[derive(Deserialize, Validate)]
struct Item {
    #[garde(length(min = 1))]
    name: String,
}
```

### Nested Object Validation

```rust
#[derive(Deserialize, Validate)]
struct UserProfile {
    #[garde(length(min = 3))]
    username: String,

    // Validate nested object
    #[garde(dive)]
    address: Address,
}

#[derive(Deserialize, Validate)]
struct Address {
    #[garde(length(min = 1))]
    street: String,

    #[garde(length(min = 1))]
    city: String,
}
```

## Custom Validation

### Simple Custom Validation

```rust
#[derive(Deserialize, Validate)]
struct User {
    #[garde(custom(validate_username))]
    username: String,
}

fn validate_username(value: &str, _ctx: &()) -> garde::Result {
    if value.to_lowercase() == "admin" {
        return Err(garde::Error::new("Username 'admin' is unavailable"));
    }
    Ok(())
}
```

### Complex Validation Logic

```rust
#[derive(Deserialize, Validate)]
struct CreateUser {
    #[garde(length(min = 8), custom(strong_password))]
    password: String,
}

fn strong_password(value: &str, _ctx: &()) -> garde::Result {
    let has_upper = value.chars().any(|c| c.is_uppercase());
    let has_lower = value.chars().any(|c| c.is_lowercase());
    let has_digit = value.chars().any(|c| c.is_numeric());

    if !has_upper || !has_lower || !has_digit {
        return Err(garde::Error::new(
            "Password must contain uppercase, lowercase letters, and numbers"
        ));
    }
    Ok(())
}
```

## Complete Example

```rust
use miko::*;
use miko::macros::*;

#[derive(Deserialize, Validate)]
struct RegisterRequest {
    #[garde(length(min = 3, max = 20))]
    username: String,

    #[garde(email)]
    email: String,

    #[garde(length(min = 8), custom(strong_password))]
    password: String,

    #[garde(range(min = 18))]
    age: u8,
}

fn strong_password(value: &str, _ctx: &()) -> garde::Result {
    let has_upper = value.chars().any(|c| c.is_uppercase());
    let has_digit = value.chars().any(|c| c.is_numeric());

    if !has_upper || !has_digit {
        return Err(garde::Error::new("Password must contain uppercase letters and numbers"));
    }
    Ok(())
}

/// User Registration
#[post("/register")]
async fn register(
    ValidatedJson(data): ValidatedJson<RegisterRequest>
) -> impl IntoResponse {
    // ✅ data passed all validations
    Json(json!({
        "message": "Registration successful",
        "username": data.username
    }))
}

/// Update Profile (Optional fields)
#[derive(Deserialize, Validate)]
struct UpdateProfile {
    #[garde(length(min = 1, max = 50))]
    name: Option<String>,

    #[garde(length(max = 200))]
    bio: Option<String>,
}

#[put("/profile")]
async fn update_profile(
    ValidatedJson(data): ValidatedJson<UpdateProfile>
) -> impl IntoResponse {
    Json(json!({ "message": "Profile updated successfully" }))
}

#[miko]
async fn main() {
    println!("🚀 Server running on http://localhost:8080");
}
```

## OpenAPI Integration

Works with `utoipa` to display validation rules in API documentation:

```rust
use miko::*;
use miko::macros::*;

#[derive(Deserialize, Validate, ToSchema)]
struct CreateUser {
    #[garde(length(min = 3, max = 20))]
    #[schema(example = "john_doe", min_length = 3, max_length = 20)]
    username: String,

    #[garde(email)]
    #[schema(example = "john@example.com", format = "email")]
    email: String,

    #[garde(range(min = 18))]
    #[schema(example = 25, minimum = 18)]
    age: u8,
}

#[post("/users")]
#[u_tag("User")]
#[u_response(status = 201, body = User)]
#[u_response(status = 422, description = "Validation Failed")]
async fn create_user(
    ValidatedJson(data): ValidatedJson<CreateUser>
) -> impl IntoResponse {
    // ...
}
```

## More Validation Rules

Miko's validation feature is based on the [garde](https://github.com/jprochazk/garde) library and supports more rules:

- **String**: `ascii`, `alphanumeric`, `phone`, `credit_card`, `ip`, etc.
- **Numeric**: `finite`, `byte_range`, etc.
- **Collection**: `unique`, `contains`, `subset`, etc.
- **Temporal**: `past`, `future`, etc.

See [garde documentation](https://docs.rs/garde/latest/garde/) for details.

## Next Steps

- 🔍 Check [Error Handling](error_handling.md) for unified error formats.
- 📖 Learn other [Request Extractor](request_extractors.md) usages.
- 📚 Understand [OpenAPI Integration](openapi_integration.md) for generating API docs.
