-- 主キーは UUID v7 (時刻順) を BINARY(16) で保持する。
-- ランダムな UUID v4 / CHAR(36) は InnoDB のクラスタ化インデックスでページ分割を起こし、
-- 大量データで挿入性能とインデックスサイズが悪化するため使わない。
CREATE TABLE IF NOT EXISTS todos (
    id          BINARY(16)   NOT NULL PRIMARY KEY,
    title       VARCHAR(255) NOT NULL,
    completed   BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
