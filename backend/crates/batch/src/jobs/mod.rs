mod archive_completed;
mod seed;

pub use archive_completed::ArchiveCompletedJob;
pub use seed::SeedJob;

use crate::runner::Job;

/// 履歴に保存された `job_name` と `params` からジョブを復元する (再開用)。
/// 新しいジョブを追加したらここに登録する。
pub fn build(name: &str, params: serde_json::Value) -> anyhow::Result<Box<dyn Job>> {
    Ok(match name {
        SeedJob::NAME => Box::new(serde_json::from_value::<SeedJob>(params)?),
        ArchiveCompletedJob::NAME => {
            Box::new(serde_json::from_value::<ArchiveCompletedJob>(params)?)
        }
        other => anyhow::bail!("unknown job: {other}"),
    })
}
