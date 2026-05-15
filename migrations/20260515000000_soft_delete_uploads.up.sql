ALTER TABLE uploads
ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP DEFAULT NULL;

CREATE INDEX IF NOT EXISTS uploads_active_idx
ON uploads (level_id, accepted)
WHERE deleted_at IS NULL AND accepted = TRUE;
