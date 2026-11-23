use std::{future::Future, time::Duration};

use rand::Rng;

/// 再試行ポリシー (指数バックオフ + ジッタ)。
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
        }
    }
}

impl RetryPolicy {
    /// `attempt` 回目 (0 始まり) の失敗後に待つ時間の上限。ジッタ適用前。
    pub fn backoff(&self, attempt: u32) -> Duration {
        let factor = 2u32.saturating_pow(attempt);
        self.base_delay.saturating_mul(factor).min(self.max_delay)
    }
}

/// 再試行すべき MySQL エラー番号。
/// - 1213: ER_LOCK_DEADLOCK (デッドロックで自トランザクションがロールバックされた)
/// - 1205: ER_LOCK_WAIT_TIMEOUT
/// - 2006 / 2013: サーバ切断 (MySQL server has gone away / Lost connection)
pub fn is_retryable_mysql_errno(errno: u16) -> bool {
    matches!(errno, 1213 | 1205 | 2006 | 2013)
}

/// 一時的なエラーかどうか。制約違反や構文エラーなどは再試行しても直らないので false。
pub fn is_retryable(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db) => db
            .try_downcast_ref::<sqlx::mysql::MySqlDatabaseError>()
            .is_some_and(|e| is_retryable_mysql_errno(e.number())),
        sqlx::Error::Io(_) | sqlx::Error::PoolTimedOut => true,
        _ => false,
    }
}

/// `op` を再試行付きで実行する。`op` は毎回トランザクションを張り直す単位 (冪等) にすること。
pub async fn retry<T, F, Fut>(policy: RetryPolicy, name: &str, mut op: F) -> Result<T, sqlx::Error>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, sqlx::Error>>,
{
    let mut attempt = 0;
    loop {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) if is_retryable(&e) && attempt + 1 < policy.max_attempts => {
                let cap = policy.backoff(attempt);
                // equal jitter: [cap/2, cap]
                let half = cap / 2;
                let jitter = rand::rng().random_range(0..=half.as_millis() as u64);
                let delay = half + Duration::from_millis(jitter);
                tracing::warn!(op = name, attempt, ?delay, error = %e, "retryable error, backing off");
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    // AC-8
    #[test]
    fn retryable_errors_are_classified() {
        assert!(is_retryable_mysql_errno(1213));
        assert!(is_retryable_mysql_errno(1205));
        assert!(!is_retryable_mysql_errno(1062)); // duplicate key
        assert!(is_retryable(&sqlx::Error::Io(std::io::Error::other(
            "reset"
        ))));
        assert!(is_retryable(&sqlx::Error::PoolTimedOut));
        assert!(!is_retryable(&sqlx::Error::RowNotFound));
    }

    // AC-8
    #[test]
    fn backoff_is_capped() {
        let p = RetryPolicy::default();
        assert_eq!(p.backoff(0), Duration::from_millis(100));
        assert_eq!(p.backoff(3), Duration::from_millis(800));
        assert_eq!(p.backoff(10), Duration::from_secs(5));
        assert_eq!(p.backoff(u32::MAX), Duration::from_secs(5));
    }

    fn fast() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(2),
        }
    }

    #[tokio::test]
    async fn retries_transient_errors_until_success() {
        let calls = AtomicU32::new(0);
        let r = retry(fast(), "t", || async {
            if calls.fetch_add(1, Ordering::SeqCst) < 2 {
                Err(sqlx::Error::PoolTimedOut)
            } else {
                Ok(42)
            }
        })
        .await;
        assert_eq!(r.unwrap(), 42);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn gives_up_after_max_attempts_and_does_not_retry_permanent_errors() {
        let calls = AtomicU32::new(0);
        let r: Result<(), _> = retry(fast(), "t", || async {
            calls.fetch_add(1, Ordering::SeqCst);
            Err(sqlx::Error::PoolTimedOut)
        })
        .await;
        assert!(r.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 3);

        let calls = AtomicU32::new(0);
        let r: Result<(), _> = retry(fast(), "t", || async {
            calls.fetch_add(1, Ordering::SeqCst);
            Err(sqlx::Error::RowNotFound)
        })
        .await;
        assert!(r.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
