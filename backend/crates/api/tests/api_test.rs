//! HTTP 層のテスト (インメモリリポジトリを使用、DB 不要)。

use std::sync::Arc;

mod common;

use api::{routes, state::AppState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use common::InMemoryTodoRepository;
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
    assert_eq!(list["items"].as_array().unwrap().len(), 1);
    assert!(list["nextCursor"].is_null());

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

#[tokio::test]
async fn list_paginates_with_cursor_newest_first() {
    let app = app();
    for i in 0..5 {
        send(
            &app,
            "POST",
            "/api/v1/todos",
            Some(json!({ "title": format!("t{i}") })),
        )
        .await;
    }

    let (_, p1) = send(&app, "GET", "/api/v1/todos?limit=2", None).await;
    let titles = |p: &Value| -> Vec<String> {
        p["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["title"].as_str().unwrap().into())
            .collect()
    };
    assert_eq!(titles(&p1), ["t4", "t3"]);

    let cursor = p1["nextCursor"].as_str().unwrap();
    let (_, p2) = send(
        &app,
        "GET",
        &format!("/api/v1/todos?limit=2&cursor={cursor}"),
        None,
    )
    .await;
    assert_eq!(titles(&p2), ["t2", "t1"]);

    let cursor = p2["nextCursor"].as_str().unwrap();
    let (_, p3) = send(
        &app,
        "GET",
        &format!("/api/v1/todos?limit=2&cursor={cursor}"),
        None,
    )
    .await;
    assert_eq!(titles(&p3), ["t0"]);
    assert!(p3["nextCursor"].is_null());
}

#[tokio::test]
async fn invalid_cursor_is_bad_request() {
    let (s, err) = send(&app(), "GET", "/api/v1/todos?cursor=xyz", None).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(err["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn healthcheck_client_accepts_running_server() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = axum::serve(listener, app());
    tokio::spawn(async move { server.await });

    let target = addr.to_string();
    api::healthcheck::check(&target, std::time::Duration::from_secs(3))
        .await
        .unwrap();

    // 何も listen していないポートは unhealthy
    let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead = closed.local_addr().unwrap().to_string();
    drop(closed);
    assert!(
        api::healthcheck::check(&dead, std::time::Duration::from_secs(1))
            .await
            .is_err()
    );
}
