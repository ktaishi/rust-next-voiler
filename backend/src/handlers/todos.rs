use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    domain::{NewTodo, Page, Todo, TodoPatch},
    error::{AppError, AppResult},
    extract::{Json as JsonBody, Path, Query},
    state::AppState,
};

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 100;

#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub limit: Option<u32>,
    /// 前ページの `nextCursor`
    pub cursor: Option<Uuid>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(p): Query<ListParams>,
) -> AppResult<Json<Page<Todo>>> {
    let limit = p.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    Ok(Json(state.todos.list(limit, p.cursor).await?))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> AppResult<Json<Todo>> {
    state
        .todos
        .get(id)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

pub async fn create(
    State(state): State<AppState>,
    JsonBody(input): JsonBody<NewTodo>,
) -> AppResult<(StatusCode, Json<Todo>)> {
    let todo = state.todos.create(input.validate()?).await?;
    Ok((StatusCode::CREATED, Json(todo)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    JsonBody(patch): JsonBody<TodoPatch>,
) -> AppResult<Json<Todo>> {
    state
        .todos
        .update(id, patch.validate()?)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<Uuid>) -> AppResult<StatusCode> {
    if state.todos.delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
