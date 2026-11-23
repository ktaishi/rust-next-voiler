use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, MySqlPool};
use uuid::Uuid;

use super::TodoRepository;
use crate::{
    domain::{NewTodo, Todo, TodoPatch},
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

#[derive(FromRow)]
struct TodoRow {
    id: String,
    title: String,
    completed: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<TodoRow> for Todo {
    type Error = AppError;
    fn try_from(r: TodoRow) -> Result<Self, Self::Error> {
        Ok(Todo {
            id: Uuid::parse_str(&r.id).map_err(|e| AppError::Internal(e.into()))?,
            title: r.title,
            completed: r.completed,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

const SELECT_COLUMNS: &str = "SELECT id, title, completed, created_at, updated_at FROM todos";

#[async_trait]
impl TodoRepository for MySqlTodoRepository {
    async fn list(&self, limit: u32, offset: u32) -> AppResult<Vec<Todo>> {
        let rows: Vec<TodoRow> = sqlx::query_as(&format!(
            "{SELECT_COLUMNS} ORDER BY created_at DESC, id LIMIT ? OFFSET ?"
        ))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Todo::try_from).collect()
    }

    async fn get(&self, id: Uuid) -> AppResult<Option<Todo>> {
        let row: Option<TodoRow> = sqlx::query_as(&format!("{SELECT_COLUMNS} WHERE id = ?"))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(Todo::try_from).transpose()
    }

    async fn create(&self, input: NewTodo) -> AppResult<Todo> {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO todos (id, title) VALUES (?, ?)")
            .bind(id.to_string())
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
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        // MySQL は値が変わらない UPDATE で affected_rows=0 を返すため、存在確認は SELECT で行う
        self.get(id).await
    }

    async fn delete(&self, id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM todos WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
