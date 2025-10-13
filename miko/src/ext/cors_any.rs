use tower_http::cors::CorsLayer;

pub fn cors_any() -> CorsLayer {
    CorsLayer::permissive()
}