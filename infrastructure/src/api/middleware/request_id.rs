//! Request-id correlation: every request gets a unique id, echoed into
//! responses and logs so a single request can be traced end to end.

use axum::http::header::HeaderName;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

/// Header carrying the request id.
pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Generates a fresh uuid request id on every request.
pub fn assign() -> SetRequestIdLayer<MakeRequestUuid> {
    SetRequestIdLayer::new(REQUEST_ID_HEADER, MakeRequestUuid)
}

/// Copies the request id into the response header for log correlation.
pub fn propagate() -> PropagateRequestIdLayer {
    PropagateRequestIdLayer::new(REQUEST_ID_HEADER)
}
