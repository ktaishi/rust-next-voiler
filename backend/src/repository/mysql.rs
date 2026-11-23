use async_trait::async_trait;
use sqlx::MySqlPool;
use uuid::Uuid;

use super::TodoRepository;
use crate::{
    domain::{NewTodo, Page, Todo, TodoPatch},
    error::{AppError, AppResult},
};

#[derive(Clone)]
pub struct MySqlTodoRepository {
    pool: MySqlPool,
}

impl MySqlTodoRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

const SELECT_COLUMNS: &str = "SELECT id, title, completed, created_at, updated_at FROM todos";

#[async_trait]
impl TodoRepository for MySqlTodoRepository {
    async fn list(&self, limit: u32, cursor: Option<Uuid>) -> AppResult<Page<Todo>> {
        // keyset ページネーション: 主キーの範囲検索なので深いページでも O(limit)。
        // OFFSET は読み飛ばす行数に比例して遅くなるため使わない。
        let fetch = i64::from(limit) + 1;
        let rows: Vec<Todo> = match cursor {
            Some(c) => {
                sqlx::query_as(&format!(
                    "{SELECT_COLUMNS} WHERE id < ? ORDER BY id DESC LIMIT ?"
                ))
                .bind(c)
                .bind(fetch)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as(&format!("{SELECT_COLUMNS} ORDER BY id DESC LIMIT ?"))
                    .bind(fetch)
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        Ok(Page::from_overfetch(rows, limit as usize, |t| t.id))
    }

    async fn get(&self, id: Uuid) -> AppResult<Option<Todo>> {
        Ok(sqlx::query_as(&format!("{SELECT_COLUMNS} WHERE id = ?"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?)
    }

    async fn create(&self, input: NewTodo) -> AppResult<Todo> {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO todos (id, title) VALUES (?, ?)")
            .bind(id)
            .bind(&input.title)
            .execute(&self.pool)
            .await?;
        self.get(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("inserted row not found")))
    }

    async fn update(&self, id: Uuid, patch: TodoPatch) -> AppResult<Option<Todo>> {
        sqlx::query(
            "UPDATE todos SET title = COALESCE(?, title), completed = COALESCE(?, completed) WHERE id = ?",
        )
        .bind(patch.title)
        .bind(patch.completed)
        .bind(id)
        .execute(&self.pool)
        .await?;
        // MySQL は値が変わらない UPDATE で affected_rows=0 を返すため、存在確認は SELECT で行う
        self.get(id).await
    }

    async fn delete(&self, id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM todos WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
