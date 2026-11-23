mod memory;
mod mysql;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{NewTodo, Todo, TodoPatch},
    error::AppResult,
};

pub use memory::InMemoryTodoRepository;
pub use mysql::MySqlTodoRepository;

/// 永続化の抽象。ハンドラはこの trait にのみ依存する。
#[async_trait]
pub trait TodoRepository: Send + Sync + 'static {
    async fn list(&self, limit: u32, offset: u32) -> AppResult<Vec<Todo>>;
    async fn get(&self, id: Uuid) -> AppResult<Option<Todo>>;
    async fn create(&self, input: NewTodo) -> AppResult<Todo>;
    async fn update(&self, id: Uuid, patch: TodoPatch) -> AppResult<Option<Todo>>;
    async fn delete(&self, id: Uuid) -> AppResult<bool>;
}
