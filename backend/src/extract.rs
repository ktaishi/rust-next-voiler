//! 入力抽出のラッパ。axum 標準の reject を `AppError` (JSON エラー形式) に統一する。

use axum::extract::{
    rejection::{JsonRejection, PathRejection, QueryRejection},
    FromRequest, FromRequestParts,
};

use crate::error::AppError;

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct Json<T>(pub T);

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(AppError))]
pub struct Path<T>(pub T);

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(AppError))]
pub struct Query<T>(pub T);

impl From<JsonRejection> for AppError {
    fn from(r: JsonRejection) -> Self {
        AppError::Validation(r.body_text())
    }
}
impl From<PathRejection> for AppError {
    fn from(r: PathRejection) -> Self {
        AppError::Validation(r.body_text())
    }
}
impl From<QueryRejection> for AppError {
    fn from(r: QueryRejection) -> Self {
        AppError::Validation(r.body_text())
    }
}
