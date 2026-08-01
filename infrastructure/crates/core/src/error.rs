use std::fmt;

/// Unified error model produced at the application boundary.
///
/// Domains map their typed errors into an [`ApiError`] via [`IntoApiError`].
/// The presentation layer (HTTP) renders it as an RFC 9457 problem document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    /// HTTP status code.
    pub status: u16,
    /// RFC 9457 `type` URI (e.g. `about:blank` for generic errors).
    pub type_url: String,
    /// Human-readable summary (RFC 9457 `title`).
    pub title: String,
    /// Human-readable explanation (RFC 9457 `detail`).
    pub detail: String,
    /// Correlation id propagated from the request, for problem documents.
    pub correlation_id: Option<String>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.title, self.detail)
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    pub fn new(
        status: u16,
        type_url: impl Into<String>,
        title: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            status,
            type_url: type_url.into(),
            title: title.into(),
            detail: detail.into(),
            correlation_id: None,
        }
    }

    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self::new(400, "about:blank", "Bad Request", detail)
    }

    pub fn unauthorized(detail: impl Into<String>) -> Self {
        Self::new(401, "about:blank", "Unauthorized", detail)
    }

    pub fn forbidden(detail: impl Into<String>) -> Self {
        Self::new(403, "about:blank", "Forbidden", detail)
    }

    pub fn not_found(detail: impl Into<String>) -> Self {
        Self::new(404, "about:blank", "Not Found", detail)
    }

    pub fn conflict(detail: impl Into<String>) -> Self {
        Self::new(409, "about:blank", "Conflict", detail)
    }

    pub fn unprocessable(detail: impl Into<String>) -> Self {
        Self::new(422, "about:blank", "Unprocessable Entity", detail)
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        Self::new(500, "about:blank", "Internal Server Error", detail)
    }

    /// Attaches a correlation id so the problem document can reference logs.
    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }
}

/// Conversion of domain/application errors into the unified API error model.
pub trait IntoApiError {
    fn to_api_error(&self) -> ApiError;
}
