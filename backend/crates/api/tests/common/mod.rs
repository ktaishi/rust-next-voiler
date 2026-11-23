//! テスト専用のヘルパ。`tests/` 配下なので本番バイナリには含まれない。
#![allow(dead_code)]

mod memory;

pub use memory::InMemoryTodoRepository;
