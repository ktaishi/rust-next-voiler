use std::sync::Arc;

use sqlx::MySqlPool;

use app_core::repository::TodoRepository;

#[derive(Clone)]
pub struct AppState {
    pub todos: Arc<dyn TodoRepository>,
    /// readiness チェック用。テストでインメモリ実装を使う場合は `None`。
    pub db: Option<MySqlPool>,
}
