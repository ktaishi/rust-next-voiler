use std::{collections::BTreeMap, sync::Mutex};

use app_core::{
    domain::{NewTodo, Page, Todo, TodoPatch},
    repository::TodoRepository,
    AppResult,
};
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

/// HTTP 層テスト用のインメモリ実装。UUID v7 の順序 = 作成順を BTreeMap で再現する。
#[derive(Default)]
pub struct InMemoryTodoRepository {
    items: Mutex<BTreeMap<Uuid, Todo>>,
}

#[async_trait]
impl TodoRepository for InMemoryTodoRepository {
    async fn list(&self, limit: u32, cursor: Option<Uuid>) -> AppResult<Page<Todo>> {
        let items = self.items.lock().unwrap();
        let rows: Vec<Todo> = items
            .values()
            .rev()
            .filter(|t| cursor.is_none_or(|c| t.id < c))
            .take(limit as usize + 1)
            .cloned()
            .collect();
        Ok(Page::from_overfetch(rows, limit as usize, |t| t.id))
    }

    async fn get(&self, id: Uuid) -> AppResult<Option<Todo>> {
        Ok(self.items.lock().unwrap().get(&id).cloned())
    }

    async fn create(&self, input: NewTodo) -> AppResult<Todo> {
        let now = Utc::now();
        let todo = Todo {
            id: Uuid::now_v7(),
            title: input.title,
            completed: false,
            created_at: now,
            updated_at: now,
        };
        self.items.lock().unwrap().insert(todo.id, todo.clone());
        Ok(todo)
    }

    async fn update(&self, id: Uuid, patch: TodoPatch) -> AppResult<Option<Todo>> {
        let mut items = self.items.lock().unwrap();
        Ok(items.get_mut(&id).map(|t| {
            if let Some(title) = patch.title {
                t.title = title;
            }
            if let Some(c) = patch.completed {
                t.completed = c;
            }
            t.updated_at = Utc::now();
            t.clone()
        }))
    }

    async fn delete(&self, id: Uuid) -> AppResult<bool> {
        Ok(self.items.lock().unwrap().remove(&id).is_some())
    }
}
