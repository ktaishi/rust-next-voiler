use app_core::AppError;
use axum::{
    extract::rejection::{JsonRejection, PathRejection, QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// HTTP 用エラー。`AppError` → HTTP ステータス/JSON の変換規約をここに集約する。
#[derive(Debug)]
pub struct ApiError(pub AppError);

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
}

impl From<AppError> for ApiError {
    fn from(e: AppError) -> Self {
        Self(e)
    }
}
impl From<JsonRejection> for ApiError {
    fn from(r: JsonRejection) -> Self {
        Self(AppError::Validation(r.body_text()))
    }
}
impl From<PathRejection> for ApiError {
    fn from(r: PathRejection) -> Self {
        Self(AppError::Validation(r.body_text()))
    }
}
impl From<QueryRejection> for ApiError {
    fn from(r: QueryRejection) -> Self {
        Self(AppError::Validation(r.body_text()))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self.0 {
            AppError::Validation(m) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", m.clone()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND", self.0.to_string()),
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
