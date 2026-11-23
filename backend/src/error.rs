use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// アプリケーション共通エラー。HTTP レスポンスへの変換規約はここに集約する。
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("resource not found")]
    NotFound,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::Validation(m) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", m.clone()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND", self.to_string()),
            AppError::Database(e) => {
                tracing::error!(error = ?e, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "internal server error".into(),
                )
            }
            AppError::Internal(e) => {
                tracing::error!(error = ?e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "internal server error".into(),
                )
            }
        };
        (status, Json(ErrorBody { code, message })).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
