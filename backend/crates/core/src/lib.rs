//! API とバッチで共有するコア。HTTP (axum) には依存しない。

pub mod db;
pub mod domain;
pub mod error;
pub mod repository;
pub mod telemetry;

pub use error::{AppError, AppResult};
