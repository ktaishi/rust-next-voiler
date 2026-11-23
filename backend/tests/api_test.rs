//! HTTP 層のテスト (インメモリリポジトリを使用、DB 不要)。

use std::sync::Arc;

use api::{repository::InMemoryTodoRepository, routes, state::AppState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

fn app() -> Router {
    let state = AppState {
        todos: Arc::new(InMemoryTodoRepository::default()),
        db: None,
    };
    routes::router(state, &["http://localhost:3000".into()])
}

async fn send(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(b) => {
            req = req.header("content-type", "application/json");
            Body::from(b.to_string())
        }
        None => Body::empty(),
    };
    let res = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, json)
}

#[tokio::test]
async fn healthz_returns_ok() {
    let (status, body) = send(&app(), "GET", "/healthz", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn todo_crud_flow() {
    let app = app();

    let (s, created) = send(
        &app,
        "POST",
        "/api/v1/todos",
        Some(json!({ "title": " buy milk " })),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(created["title"], "buy milk");
    assert_eq!(created["completed"], false);
    let id = created["id"].as_str().unwrap().to_string();

    let (s, list) = send(&app, "GET", "/api/v1/todos", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    let (s, updated) = send(
        &app,
        "PATCH",
        &format!("/api/v1/todos/{id}"),
        Some(json!({ "completed": true })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(updated["completed"], true);

    let (s, _) = send(&app, "DELETE", &format!("/api/v1/todos/{id}"), None).await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    let (s, err) = send(&app, "GET", &format!("/api/v1/todos/{id}"), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(err["code"], "NOT_FOUND");
}

#[tokio::test]
async fn create_rejects_blank_title() {
    let (s, err) = send(
        &app(),
        "POST",
        "/api/v1/todos",
        Some(json!({ "title": "  " })),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(err["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn invalid_uuid_is_bad_request() {
    let (s, err) = send(&app(), "GET", "/api/v1/todos/not-a-uuid", None).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(err["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn malformed_json_is_bad_request() {
    let app = app();
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/todos")
        .header("content-type", "application/json")
        .body(Body::from("{broken"))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
