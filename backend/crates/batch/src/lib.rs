//! バッチ処理基盤。
//!
//! - [`runner::JobRunner`]: 多重起動防止 (GET_LOCK)・実行履歴・チェックポイント・再開
//! - [`retry`]: 一時的な DB エラーの指数バックオフ再試行
//! - [`chunk`]: バルク SQL のチャンクサイズ計算
//! - [`jobs`]: 個別ジョブ。新しいジョブは [`runner::Job`] を実装して `jobs::build` に登録する

pub mod chunk;
pub mod jobs;
pub mod retry;
pub mod runner;
