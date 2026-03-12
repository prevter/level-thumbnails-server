CREATE INDEX IF NOT EXISTS uploads_accepted_level_time_idx
ON uploads (level_id, upload_time DESC)
WHERE accepted = TRUE;

CREATE INDEX IF NOT EXISTS uploads_pending_queue_idx
ON uploads (upload_time ASC, id ASC)
WHERE accepted = FALSE AND accepted_time IS NULL;

CREATE INDEX IF NOT EXISTS uploads_pending_user_queue_idx
ON uploads (user_id, upload_time ASC, id ASC)
WHERE accepted = FALSE AND accepted_time IS NULL;

CREATE INDEX IF NOT EXISTS uploads_pending_level_queue_idx
ON uploads (level_id, upload_time ASC, id ASC)
WHERE accepted = FALSE AND accepted_time IS NULL;

CREATE INDEX IF NOT EXISTS users_account_id_idx
ON users (account_id);

CREATE INDEX IF NOT EXISTS users_discord_id_idx
ON users (discord_id)
WHERE discord_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS stats_snapshots
(
    id                     BIGSERIAL PRIMARY KEY,
    captured_at            TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    storage_bytes          BIGINT NOT NULL,
    thumbnails_count       BIGINT NOT NULL,
    users_per_month        BIGINT DEFAULT NULL,
    users_total            BIGINT NOT NULL,
    uploads_total          BIGINT NOT NULL,
    pending_uploads_total  BIGINT NOT NULL,
    accepted_uploads_total BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS stats_snapshots_captured_at_idx
ON stats_snapshots (captured_at DESC);
