CREATE TABLE IF NOT EXISTS level_locks
(
    level_id   BIGINT PRIMARY KEY,
    locked_by  BIGINT                 NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    locked_at  TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    reason     TEXT      DEFAULT NULL
);
