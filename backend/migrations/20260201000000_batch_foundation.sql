-- 0002: バッチ処理基盤

-- アーカイブ対象 (completed = TRUE AND updated_at < ?) をインデックス範囲検索にする
CREATE INDEX idx_todos_completed_updated ON todos (completed, updated_at, id);

CREATE TABLE IF NOT EXISTS todos_archive (
    id           BINARY(16)   NOT NULL PRIMARY KEY,
    title        VARCHAR(255) NOT NULL,
    completed    BOOLEAN      NOT NULL,
    created_at   DATETIME(6)  NOT NULL,
    updated_at   DATETIME(6)  NOT NULL,
    archived_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    INDEX idx_todos_archive_archived_at (archived_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;

CREATE TABLE IF NOT EXISTS batch_jobs (
    id           BINARY(16)       NOT NULL PRIMARY KEY,
    job_name     VARCHAR(64)      NOT NULL,
    status       ENUM('running', 'succeeded', 'failed') NOT NULL,
    params       JSON             NOT NULL,
    processed    BIGINT UNSIGNED  NOT NULL DEFAULT 0,
    checkpoint   VARCHAR(255)     NULL,
    error        TEXT             NULL,
    started_at   DATETIME(6)      NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    finished_at  DATETIME(6)      NULL,
    updated_at   DATETIME(6)      NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    INDEX idx_batch_jobs_name_started (job_name, started_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
