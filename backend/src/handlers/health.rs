use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::state::AppState;

/// Liveness: プロセスが生きているか
pub async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

/// Readiness: 依存先 (DB) に到達できるか
pub async fn readyz(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match &state.db {
        Some(pool) => match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => (StatusCode::OK, Json(json!({ "status": "ready" }))),
            Err(e) => {
                tracing::warn!(error = ?e, "readiness check failed");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({ "status": "unavailable" })),
                )
            }
        },
        None => (StatusCode::OK, Json(json!({ "status": "ready" }))),
    }
}
