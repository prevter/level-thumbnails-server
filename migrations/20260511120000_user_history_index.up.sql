CREATE INDEX IF NOT EXISTS uploads_user_time_idx
ON uploads (user_id, upload_time DESC, id DESC);

