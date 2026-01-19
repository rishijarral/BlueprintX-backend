//! Request ID middleware for request tracing

#![allow(dead_code)]

use axum::http::HeaderName;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

/// Header name for request ID
pub const X_REQUEST_ID: &str = "x-request-id";

/// Creates a layered middleware that:
/// 1. Sets a request ID if not present (using UUID v4)
/// 2. Propagates the request ID to the response
pub fn request_id_layer() -> (SetRequestIdLayer<MakeRequestUuid>, PropagateRequestIdLayer) {
    let header_name = HeaderName::from_static(X_REQUEST_ID);

    (
        SetRequestIdLayer::new(header_name.clone(), MakeRequestUuid),
        PropagateRequestIdLayer::new(header_name),
    )
}

/// Extension trait for extracting request ID from headers
pub trait RequestIdExt {
    fn request_id(&self) -> Option<&str>;
}

impl RequestIdExt for axum::http::HeaderMap {
    fn request_id(&self) -> Option<&str> {
        self.get(X_REQUEST_ID)?.to_str().ok()
    }
}
