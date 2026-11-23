mod mysql;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{NewTodo, Page, Todo, TodoPatch},
    error::AppResult,
};

pub use mysql::MySqlTodoRepository;

/// 永続化の抽象。ハンドラはこの trait にのみ依存する。
/// テスト用のインメモリ実装は `tests/common/` にある (本番バイナリには含めない)。
#[async_trait]
pub trait TodoRepository: Send + Sync + 'static {
    /// ID (= 作成順) の降順。`cursor` より古いものを最大 `limit` 件返す。
    async fn list(&self, limit: u32, cursor: Option<Uuid>) -> AppResult<Page<Todo>>;
    async fn get(&self, id: Uuid) -> AppResult<Option<Todo>>;
    async fn create(&self, input: NewTodo) -> AppResult<Todo>;
    async fn update(&self, id: Uuid, patch: TodoPatch) -> AppResult<Option<Todo>>;
    async fn delete(&self, id: Uuid) -> AppResult<bool>;
}
