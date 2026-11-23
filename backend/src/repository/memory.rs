use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use super::TodoRepository;
use crate::{
    domain::{NewTodo, Todo, TodoPatch},
    error::AppResult,
};

/// テスト・ローカル検証用のインメモリ実装。
#[derive(Default)]
pub struct InMemoryTodoRepository {
    items: Mutex<HashMap<Uuid, Todo>>,
}

#[async_trait]
impl TodoRepository for InMemoryTodoRepository {
    async fn list(&self, limit: u32, offset: u32) -> AppResult<Vec<Todo>> {
        let mut v: Vec<Todo> = self.items.lock().unwrap().values().cloned().collect();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.cmp(&b.id)));
        Ok(v.into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect())
    }

    async fn get(&self, id: Uuid) -> AppResult<Option<Todo>> {
        Ok(self.items.lock().unwrap().get(&id).cloned())
    }

    async fn create(&self, input: NewTodo) -> AppResult<Todo> {
        let now = Utc::now();
        let todo = Todo {
            id: Uuid::new_v4(),
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
