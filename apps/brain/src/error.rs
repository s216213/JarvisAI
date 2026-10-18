use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("rate limited")]
    RateLimited,
    #[error("providers exhausted")]
    ProvidersExhausted { retry_after_secs: u64 },
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error(transparent)]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized", self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", self.to_string()),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found", self.to_string()),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request", self.to_string()),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict", self.to_string()),
            AppError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate_limited", self.to_string()),
            AppError::ProvidersExhausted { retry_after_secs } => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({
                        "code": "providers_exhausted",
                        "message": "All LLM providers are temporarily unavailable",
                        "retry_after_secs": retry_after_secs
                    })),
                )
                    .into_response();
            }
            AppError::Db(_) | AppError::Anyhow(_) | AppError::Jwt(_) => {
                tracing::error!(error = %self, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "internal server error".to_string(),
                )
            }
        };

        (status, Json(json!({ "code": code, "message": message }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
