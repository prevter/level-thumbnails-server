WITH ranked_pending_uploads AS (
    SELECT
        id,
        ROW_NUMBER() OVER (
            PARTITION BY user_id, level_id
            ORDER BY upload_time DESC, id DESC
        ) AS row_number
    FROM uploads
    WHERE accepted = FALSE AND accepted_time IS NULL
)
DELETE FROM uploads
WHERE id IN (
    SELECT id
    FROM ranked_pending_uploads
    WHERE row_number > 1
);

CREATE UNIQUE INDEX IF NOT EXISTS uploads_active_pending_unique_idx
ON uploads (user_id, level_id)
WHERE accepted = FALSE AND accepted_time IS NULL;
