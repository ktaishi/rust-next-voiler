//! 内部結合テスト: 実 MySQL に対してリポジトリを検証する。
//! 実行: DATABASE_URL=mysql://root:root@localhost:3306/app cargo test --features integration
//! `#[sqlx::test]` はテストごとに使い捨て DB を作成しマイグレーションを適用する。
#![cfg(feature = "integration")]

use api::{
    domain::{NewTodo, TodoPatch},
    repository::{MySqlTodoRepository, TodoRepository},
};
use sqlx::MySqlPool;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
async fn create_and_get(pool: MySqlPool) {
    let repo = MySqlTodoRepository::new(pool);
    let created = repo
        .create(NewTodo {
            title: "hello".into(),
        })
        .await
        .unwrap();
    let fetched = repo.get(created.id).await.unwrap().unwrap();
    assert_eq!(fetched.title, "hello");
    assert!(!fetched.completed);
}

#[sqlx::test(migrations = "./migrations")]
async fn update_partial_fields(pool: MySqlPool) {
    let repo = MySqlTodoRepository::new(pool);
    let t = repo.create(NewTodo { title: "a".into() }).await.unwrap();
    let u = repo
        .update(
            t.id,
            TodoPatch {
                title: None,
                completed: Some(true),
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(u.title, "a");
    assert!(u.completed);
    // 値が変わらない更新でも Some を返す
    let same = repo
        .update(
            t.id,
            TodoPatch {
                title: None,
                completed: Some(true),
            },
        )
        .await
        .unwrap();
    assert!(same.is_some());
}

#[sqlx::test(migrations = "./migrations")]
async fn update_and_delete_missing(pool: MySqlPool) {
    let repo = MySqlTodoRepository::new(pool);
    let id = Uuid::now_v7();
    assert!(repo
        .update(
            id,
            TodoPatch {
                title: Some("x".into()),
                completed: None
            }
        )
        .await
        .unwrap()
        .is_none());
    assert!(!repo.delete(id).await.unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn list_pagination(pool: MySqlPool) {
    let repo = MySqlTodoRepository::new(pool);
    for i in 0..5 {
        repo.create(NewTodo {
            title: format!("t{i}"),
        })
        .await
        .unwrap();
    }
    let p1 = repo.list(2, None).await.unwrap();
    assert_eq!(
        p1.items
            .iter()
            .map(|t| t.title.as_str())
            .collect::<Vec<_>>(),
        ["t4", "t3"]
    );
    let p2 = repo.list(2, p1.next_cursor).await.unwrap();
    assert_eq!(
        p2.items
            .iter()
            .map(|t| t.title.as_str())
            .collect::<Vec<_>>(),
        ["t2", "t1"]
    );
    let p3 = repo.list(2, p2.next_cursor).await.unwrap();
    assert_eq!(p3.items.len(), 1);
    assert!(p3.next_cursor.is_none());
}
