use axum::response::{IntoResponse, Response};
use axum::Json;
use eventquest_domain::{ApiErrorBody, ErrorCode};
use uuid::Uuid;

/// The single error type every axum handler in this service returns.
/// Converts to the standardized `{ code, message, requestId }` body from
/// spec §20 via `IntoResponse` — handlers never build raw error JSON.
#[derive(Debug)]
pub struct ApiError {
    code: ErrorCode,
    message: Option<String>,
    request_id: Uuid,
}

impl ApiError {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            message: None,
            request_id: Uuid::new_v4(),
        }
    }

    pub fn with_message(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: Some(message.into()),
            request_id: Uuid::new_v4(),
        }
    }

    // Not called yet — no handler currently correlates an error with a
    // request id generated earlier in the same request (e.g. from a
    // tracing span). Kept for that future use rather than removed, since
    // `ApiError` is meant to be the one place every error response is built.
    #[allow(dead_code)]
    pub fn with_request_id(mut self, request_id: Uuid) -> Self {
        self.request_id = request_id;
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.code.http_status();
        let body = match self.message {
            Some(message) => ApiErrorBody::with_message(self.code, self.request_id, message),
            None => ApiErrorBody::new(self.code, self.request_id),
        };

        if status.is_server_error() {
            tracing::error!(
                code = ?self.code,
                request_id = %body.request_id,
                message = %body.message,
                "internal error"
            );
        }

        (status, Json(body)).into_response()
    }
}

/// Anything unexpected (DB errors, Redis errors, etc.) becomes
/// `INTERNAL_ERROR` with a generic message — spec §20 requires errors to
/// never expose internal details.
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!(error = %err, "database error");
        ApiError::new(ErrorCode::InternalError)
    }
}

impl From<redis::RedisError> for ApiError {
    fn from(err: redis::RedisError) -> Self {
        tracing::error!(error = %err, "redis error");
        ApiError::new(ErrorCode::InternalError)
    }
}
