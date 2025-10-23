use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use std::convert::Infallible;

/// Helper function to create an empty boxed body
pub fn box_empty_body() -> BoxBody<Bytes, Infallible> {
    Full::new(Bytes::new()).boxed()
}
