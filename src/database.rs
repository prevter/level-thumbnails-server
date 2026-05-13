use std::collections::HashMap;
use sqlx::postgres::PgPoolOptions;
use sqlx::{FromRow, Postgres, QueryBuilder};
use std::path::Path;

use chrono::{Datelike, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::util::VersionInfo;

fn serialize_discord_snowflake<S>(value: &Option<i64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(id) => serializer.serialize_some(&id.to_string()),
        None => serializer.serialize_none(),
    }
}

fn default_min_supported_client() -> VersionInfo {
    VersionInfo::from_str("v2.1.0").expect("Invalid default version")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub pause_submissions: bool,
    #[serde(default = "default_min_supported_client")]
    pub min_supported_client: VersionInfo,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pause_submissions: false,
            min_supported_client: default_min_supported_client(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: Arc<sqlx::Pool<Postgres>>,
    pub settings: Arc<tokio::sync::RwLock<Settings>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum Role {
    User,      // regular user
    Verified,  // verified users can upload thumbnails without approval
    Moderator, // moderators can approve or reject uploads
    Admin,     // admins can manage users and uploads
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Verified => write!(f, "verified"),
            Role::Moderator => write!(f, "moderator"),
            Role::Admin => write!(f, "admin"),
        }
    }
}

#[derive(Debug, FromRow, Serialize)]
pub struct User {
    pub id: i64,
    pub account_id: i64,
    pub username: String,
    pub role: Role,
    #[serde(serialize_with = "serialize_discord_snowflake")]
    pub discord_id: Option<i64>,
}

#[derive(FromRow)]
pub struct UploadInfo {
    pub account_id: i64,
    pub username: String,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct UploadExtended {
    pub level_id: i64,
    pub account_id: i64,
    pub username: String,
    pub upload_time: NaiveDateTime,
    pub first_upload_time: NaiveDateTime,
    pub accepted_time: Option<NaiveDateTime>,
    pub accepted_by: Option<i64>,
    pub accepted_by_username: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LevelLock {
    pub level_id: i64,
    pub locked_by: i64,
    pub locked_by_username: String,
    pub locked_at: NaiveDateTime,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PendingUpload {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub level_id: i64,
    pub accepted: bool,
    pub upload_time: NaiveDateTime,
    pub submission_note: Option<String>,
    pub account_id: Option<i64>,
    pub user_role: Role,

    #[sqlx(skip)]
    pub replacement: bool,
}

#[derive(Debug, Clone)]
pub struct PendingQueryOptions {
    pub page: u32,
    pub per_page: u32,
    pub level_id: Option<i64>,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub replacement_only: bool,
    pub new_only: bool,
}

#[derive(Debug, Clone)]
pub struct PendingUploadsPage {
    pub uploads: Vec<PendingUpload>,
    pub total: i64,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct UserStats {
    pub id: i64,
    pub account_id: i64,
    pub username: String,
    pub role: Role,
    pub upload_count: i64,
    pub accepted_upload_count: i64,
    pub pending_upload_count: i64,
    pub level_count: i64,
    pub accepted_level_count: i64,
    pub active_thumbnail_count: i64,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct StatsSnapshot {
    pub id: i64,
    pub captured_at: NaiveDateTime,
    pub storage_bytes: i64,
    pub thumbnails_count: i64,
    pub users_per_month: Option<i64>,
    pub users_total: i64,
    pub uploads_total: i64,
    pub pending_uploads_total: i64,
    pub accepted_uploads_total: i64,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserHistoryPoint {
    pub period: NaiveDate,
    pub upload_count: i64,
    pub accepted_upload_count: i64,
    pub pending_upload_count: i64,
    pub level_count: i64,
    pub accepted_level_count: i64,
}

fn month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).expect("invalid month start")
}

fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    let total_months = date.year() * 12 + date.month0() as i32 + months;
    let year = total_months.div_euclid(12);
    let month0 = total_months.rem_euclid(12);

    NaiveDate::from_ymd_opt(year, (month0 + 1) as u32, 1).expect("invalid shifted month")
}

impl AppState {
    pub async fn new() -> Self {
        let connection_string = dotenv::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await
            .expect("Failed to connect to the database");

        // Run migrations if needed
        sqlx::migrate!("./migrations").run(&pool).await.expect("Failed to run migrations");

        // load settings from state.json or create default
        let settings = if let Ok(settings_data) = tokio::fs::read_to_string("state.json").await {
            serde_json::from_str(&settings_data).unwrap_or_default()
        } else {
            Settings::default()
        };

        AppState {
            pool: Arc::new(pool),
            settings: Arc::new(tokio::sync::RwLock::new(settings)),
        }
    }

    pub async fn get_upload_info(&self, id: i64) -> Option<UploadInfo> {
        sqlx::query_as::<_, UploadInfo>(
            "SELECT users.account_id, users.username
                 FROM uploads
                 JOIN users ON uploads.user_id = users.id
                 WHERE uploads.level_id = $1 AND accepted = TRUE
                 ORDER BY upload_time DESC LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .ok()?
    }

    pub async fn get_upload_extended(&self, id: i64) -> Option<UploadExtended> {
        sqlx::query_as::<_, UploadExtended>(
            "SELECT 
                    uploads.level_id,
                    users.account_id,
                    users.username,
                    uploads.upload_time,
                    (
                        SELECT MIN(upload_time) FROM uploads u2
                        WHERE u2.level_id = uploads.level_id AND u2.accepted = TRUE
                    ) AS first_upload_time,
                    uploads.accepted_time,
                    accepted_by.account_id AS accepted_by,
                    accepted_by.username AS accepted_by_username
                 FROM uploads
                 JOIN users ON uploads.user_id = users.id
                 LEFT JOIN users AS accepted_by ON uploads.accepted_by = accepted_by.id
                 WHERE uploads.level_id = $1 AND accepted = TRUE
                 ORDER BY upload_time DESC LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .ok()?
    }

    pub async fn find_or_create_user(
        &self,
        account_id: i64,
        username: &str,
    ) -> Result<User, sqlx::Error> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE account_id = $1")
            .bind(account_id)
            .bind(username)
            .fetch_optional(&*self.pool)
            .await?;

        if let Some(user) = user {
            Ok(user)
        } else {
            let new_user = sqlx::query_as::<_, User>(
                "INSERT INTO users (account_id, username, role) VALUES ($1, $2, 'user') RETURNING *",
            )
            .bind(account_id)
            .bind(username)
            .fetch_one(&*self.pool)
            .await?;
            Ok(new_user)
        }
    }

    pub async fn find_or_create_user_discord(
        &self,
        discord_id: i64,
        username: &str,
    ) -> Result<User, sqlx::Error> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE discord_id = $1")
            .bind(discord_id)
            .fetch_optional(&*self.pool)
            .await?;

        if let Some(user) = user {
            Ok(user)
        } else {
            // first check if we can link to existing legacy account
            let legacy_user = sqlx::query_as::<_, User>(
                "SELECT * FROM users WHERE account_id = -1 AND username = $1 AND discord_id IS NULL",
            )
            .bind(username)
            .fetch_optional(&*self.pool)
            .await?;
            if let Some(legacy_user) = legacy_user {
                // update the legacy user with the discord_id
                sqlx::query("UPDATE users SET discord_id = $1 WHERE id = $2")
                    .bind(discord_id)
                    .bind(legacy_user.id)
                    .execute(&*self.pool)
                    .await?;
                return Ok(legacy_user);
            }
            // if no legacy user found, create a new user
            let new_user = sqlx::query_as::<_, User>(
                "INSERT INTO users (account_id, username, role, discord_id) VALUES (-1, $1, 'user', $2) RETURNING *",
            )
            .bind(username)
            .bind(discord_id)
            .fetch_one(&*self.pool)
            .await?;
            Ok(new_user)
        }
    }

    pub async fn get_user_by_id(&self, id: i64) -> Option<User> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&*self.pool)
            .await
            .ok()?
    }

    pub async fn add_upload(
        &self,
        level_id: i64,
        user_id: i64,
        image_path: &str,
        accepted: bool,
        submission_note: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        if accepted {
            sqlx::query(
                "INSERT INTO uploads (level_id, user_id, image_path, accepted, accepted_time, accepted_by, submission_note)
                 VALUES ($1, $2, $3, TRUE, NOW(), $2, $4)",
            )
            .bind(level_id)
            .bind(user_id)
            .bind(image_path)
            .bind(submission_note)
            .execute(&*self.pool)
            .await?;
        } else {
            sqlx::query(
                "INSERT INTO uploads (level_id, user_id, image_path, accepted, submission_note)
                 VALUES ($1, $2, $3, FALSE, $4)
                 ON CONFLICT (user_id, level_id)
                 WHERE accepted = FALSE AND accepted_time IS NULL
                 DO UPDATE SET
                     image_path = EXCLUDED.image_path,
                     submission_note = EXCLUDED.submission_note,
                     upload_time = NOW()",
            )
            .bind(level_id)
            .bind(user_id)
            .bind(image_path)
            .bind(submission_note)
            .execute(&*self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn is_level_locked(&self, level_id: i64) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM level_locks WHERE level_id = $1)",
        )
        .bind(level_id)
        .fetch_one(&*self.pool)
        .await?;

        Ok(exists)
    }

    pub async fn get_level_lock(&self, level_id: i64) -> Result<Option<LevelLock>, sqlx::Error> {
        sqlx::query_as::<_, LevelLock>(
            "SELECT
                level_locks.level_id,
                level_locks.locked_by,
                users.username AS locked_by_username,
                level_locks.locked_at,
                level_locks.reason
             FROM level_locks
             JOIN users ON users.id = level_locks.locked_by
             WHERE level_locks.level_id = $1",
        )
        .bind(level_id)
        .fetch_optional(&*self.pool)
        .await
    }

    pub async fn get_all_level_locks(&self) -> Result<Vec<LevelLock>, sqlx::Error> {
        sqlx::query_as::<_, LevelLock>(
            "SELECT
                level_locks.level_id,
                level_locks.locked_by,
                users.username AS locked_by_username,
                level_locks.locked_at,
                level_locks.reason
             FROM level_locks
             JOIN users ON users.id = level_locks.locked_by
             ORDER BY level_locks.locked_at DESC, level_locks.level_id DESC",
        )
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn lock_level(
        &self,
        level_id: i64,
        locked_by: i64,
        reason: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO level_locks (level_id, locked_by, reason)
             VALUES ($1, $2, $3)
             ON CONFLICT (level_id)
             DO UPDATE SET
                 locked_by = EXCLUDED.locked_by,
                 reason = EXCLUDED.reason,
                 locked_at = NOW()",
        )
        .bind(level_id)
        .bind(locked_by)
        .bind(reason)
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    pub async fn unlock_level(&self, level_id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM level_locks WHERE level_id = $1")
            .bind(level_id)
            .execute(&*self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // pub async fn get_pending_uploads(&self) -> Result<Vec<PendingUpload>, sqlx::Error> {
    //     sqlx::query_as::<_, PendingUpload>(
    //         "SELECT uploads.id, user_id, username, level_id, accepted, upload_time FROM uploads
    //          LEFT JOIN users ON users.id = user_id
    //          WHERE accepted = FALSE AND accepted_time IS NULL
    //          ORDER BY upload_time",
    //     )
    //     .fetch_all(&*self.pool)
    //     .await
    // }

    fn apply_pending_filters<'a>(
        builder: &mut QueryBuilder<'a, Postgres>,
        options: &PendingQueryOptions,
    ) {
        if let Some(level_id) = options.level_id {
            builder.push(" AND uploads.level_id = ").push_bind(level_id);
        }

        if let Some(user_id) = options.user_id {
            builder.push(" AND uploads.user_id = ").push_bind(user_id);
        }

        if let Some(ref username) = options.username {
            builder
                .push(" AND LOWER(username) LIKE LOWER(")
                .push_bind(format!("%{}%", username))
                .push(")");
        }
    }

    fn is_image_uploaded(level_id: i64) -> bool {
        let image_path = format!("thumbnails/{}.webp", level_id);
        Path::new(&image_path).exists()
    }

    pub async fn get_pending_uploads_paginated(
        &self,
        options: PendingQueryOptions,
    ) -> Result<PendingUploadsPage, sqlx::Error> {
        if options.replacement_only || options.new_only {
            let mut data_builder = QueryBuilder::new(
                "SELECT uploads.id, user_id, users.username AS username, level_id, accepted, upload_time, submission_note, users.account_id AS account_id, users.role AS user_role FROM uploads
                 LEFT JOIN users ON users.id = user_id
                 WHERE accepted = FALSE AND accepted_time IS NULL",
            );
            Self::apply_pending_filters(&mut data_builder, &options);
            data_builder.push(" ORDER BY upload_time ASC, uploads.id ASC");

            let mut all_uploads =
                data_builder.build_query_as::<PendingUpload>().fetch_all(&*self.pool).await?;

            all_uploads.retain(|upload| {
                let is_uploaded = Self::is_image_uploaded(upload.level_id);
                if options.replacement_only { is_uploaded } else { !is_uploaded }
            });

            let total = all_uploads.len() as i64;
            let per_page = options.per_page as usize;
            let offset = options.page.saturating_sub(1) as usize * per_page;

            let uploads = all_uploads.into_iter().skip(offset).take(per_page).collect();

            Ok(PendingUploadsPage { uploads, total })
        } else {
            let per_page = options.per_page as i64;
            let offset = ((options.page.saturating_sub(1)) as i64) * per_page;

            let mut data_builder = QueryBuilder::new(
                "SELECT uploads.id, user_id, users.username AS username, level_id, accepted, upload_time, submission_note, users.account_id AS account_id, users.role AS user_role FROM uploads
                 LEFT JOIN users ON users.id = user_id
                 WHERE accepted = FALSE AND accepted_time IS NULL",
            );
            Self::apply_pending_filters(&mut data_builder, &options);
            data_builder
                .push(" ORDER BY upload_time ASC, uploads.id ASC LIMIT ")
                .push_bind(per_page)
                .push(" OFFSET ")
                .push_bind(offset);

            let uploads =
                data_builder.build_query_as::<PendingUpload>().fetch_all(&*self.pool).await?;

            let mut count_builder = QueryBuilder::new(
                "SELECT COUNT(*) FROM uploads
                 LEFT JOIN users ON users.id = user_id
                 WHERE accepted = FALSE AND accepted_time IS NULL",
            );
            Self::apply_pending_filters(&mut count_builder, &options);

            let total: i64 = count_builder.build_query_scalar().fetch_one(&*self.pool).await?;

            Ok(PendingUploadsPage { uploads, total })
        }
    }

    // pub async fn get_pending_uploads_for_level(
    //     &self,
    //     level_id: i64,
    // ) -> Result<Vec<PendingUpload>, sqlx::Error> {
    //     sqlx::query_as::<_, PendingUpload>(
    //         "SELECT uploads.id, user_id, username, accepted, upload_time FROM uploads
    //               LEFT JOIN users ON users.id = user_id
    //               WHERE accepted = FALSE AND accepted_time IS NULL AND level_id = $1
    //               ORDER BY upload_time",
    //     )
    //     .bind(level_id)
    //     .fetch_all(&*self.pool)
    //     .await
    // }

    pub async fn get_pending_uploads_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<PendingUpload>, sqlx::Error> {
        sqlx::query_as::<_, PendingUpload>(
            "SELECT uploads.id, user_id, users.username AS username, level_id, accepted, upload_time, submission_note, users.account_id AS account_id, users.role AS user_role FROM uploads
              LEFT JOIN users ON users.id = user_id
              WHERE accepted = FALSE AND accepted_time IS NULL AND user_id = $1
              ORDER BY upload_time",
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_pending_upload(&self, id: i64) -> Result<PendingUpload, sqlx::Error> {
        sqlx::query_as::<_, PendingUpload>(
            "SELECT uploads.id, user_id, users.username AS username, level_id, accepted, upload_time, submission_note, users.account_id AS account_id, users.role AS user_role FROM uploads
              LEFT JOIN users ON users.id = user_id
              WHERE accepted = FALSE AND accepted_time IS NULL AND uploads.id = $1",
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await
    }

    pub async fn accept_upload(
        &self,
        id: i64,
        accepted_by: i64,
        reason: Option<String>,
        accept: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
                 "UPDATE uploads SET accepted = $1, accepted_time = NOW(), accepted_by = $2, reason = $3 WHERE id = $4",
             )
             .bind(accept)
             .bind(accepted_by)
             .bind(reason)
             .bind(id)
             .execute(&*self.pool)
             .await?;
        Ok(())
    }

    pub async fn get_user_stats(&self, id: i64) -> Option<UserStats> {
        sqlx::query_as::<_, UserStats>(
             "SELECT
                 users.id, users.account_id,
                 users.username, users.role,
                 COUNT(uploads.id) AS upload_count,
                 COUNT(DISTINCT uploads.level_id) AS level_count,
                 COUNT(uploads.id) FILTER (WHERE uploads.accepted = TRUE) AS accepted_upload_count,
                 COUNT(uploads.id) FILTER (WHERE uploads.accepted = FALSE AND uploads.accepted_time IS NULL) AS pending_upload_count,
                 COUNT(DISTINCT uploads.level_id) FILTER (WHERE uploads.accepted = TRUE) AS accepted_level_count,
                 (
                   SELECT COUNT(*)
                   FROM (
                     SELECT u.level_id
                     FROM uploads u
                     WHERE u.accepted = TRUE
                     AND u.user_id = users.id
                     AND u.upload_time = (
                       SELECT MAX(u2.upload_time)
                       FROM uploads u2
                       WHERE u2.level_id = u.level_id
                         AND u2.accepted = TRUE
                     )
                   ) active_levels
                 ) AS active_thumbnail_count
               FROM users
               LEFT JOIN uploads ON users.id = uploads.user_id
               WHERE users.id = $1
               GROUP BY users.id, users.account_id, users.username, users.role",
         )
         .bind(id)
         .fetch_optional(&*self.pool)
         .await
         .ok()?
    }

    pub async fn get_user_history(
        &self,
        id: i64,
        months: i64,
    ) -> Result<Vec<UserHistoryPoint>, sqlx::Error> {
        let months = months.clamp(1, 24) as i32;
        let current_month = month_start(Utc::now().date_naive());
        let start_month = add_months(current_month, -(months - 1));
        let end_month = add_months(current_month, 1);

        let rows = sqlx::query_as::<_, UserHistoryPoint>(
            "SELECT
                date_trunc('month', upload_time)::date AS period,
                COUNT(*) AS upload_count,
                COUNT(*) FILTER (WHERE accepted = TRUE) AS accepted_upload_count,
                COUNT(*) FILTER (WHERE accepted = FALSE AND accepted_time IS NULL) AS pending_upload_count,
                COUNT(DISTINCT level_id) AS level_count,
                COUNT(DISTINCT level_id) FILTER (WHERE accepted = TRUE) AS accepted_level_count
             FROM uploads
             WHERE user_id = $1
               AND upload_time >= $2
               AND upload_time < $3
             GROUP BY 1
             ORDER BY 1 ASC",
        )
        .bind(id)
        .bind(start_month.and_hms_opt(0, 0, 0).expect("invalid start month time"))
        .bind(end_month.and_hms_opt(0, 0, 0).expect("invalid end month time"))
        .fetch_all(&*self.pool)
        .await?;

        let mut by_period = HashMap::with_capacity(rows.len());
        for row in rows {
            by_period.insert(row.period, row);
        }

        let mut history = Vec::with_capacity(months as usize);
        let mut period = start_month;

        for _ in 0..months {
            if let Some(row) = by_period.remove(&period) {
                history.push(row);
            } else {
                history.push(UserHistoryPoint {
                    period,
                    upload_count: 0,
                    accepted_upload_count: 0,
                    pending_upload_count: 0,
                    level_count: 0,
                    accepted_level_count: 0,
                });
            }

            period = add_months(period, 1);
        }

        Ok(history)
    }

    pub async fn migrate_user_account(
        &self,
        old_account_id: i64,
        new_account_id: i64,
    ) -> Result<User, sqlx::Error> {
        sqlx::query("CALL migrate($1, $2)")
            .bind(new_account_id)
            .bind(old_account_id)
            .execute(&*self.pool)
            .await?;

        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(new_account_id)
            .fetch_one(&*self.pool)
            .await?;

        Ok(user)
    }

    pub async fn save_settings(&self) -> Result<(), std::io::Error> {
        let settings = self.settings.read().await;
        let settings_data = serde_json::to_string_pretty(&*settings)?;
        tokio::fs::write("state.json", settings_data).await
    }

    pub async fn create_stats_snapshot(
        &self,
        storage_bytes: i64,
        thumbnails_count: i64,
        users_per_month: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO stats_snapshots (
                storage_bytes,
                thumbnails_count,
                users_per_month,
                users_total,
                uploads_total,
                pending_uploads_total,
                accepted_uploads_total
            )
            VALUES (
                $1,
                $2,
                $3,
                (SELECT COUNT(*) FROM users),
                (SELECT COUNT(*) FROM uploads),
                (SELECT COUNT(*) FROM uploads WHERE accepted = FALSE AND accepted_time IS NULL),
                (SELECT COUNT(*) FROM uploads WHERE accepted = TRUE)
            )",
        )
        .bind(storage_bytes)
        .bind(thumbnails_count)
        .bind(users_per_month)
        .execute(&*self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_recent_stats_snapshots(
        &self,
        limit: i64,
    ) -> Result<Vec<StatsSnapshot>, sqlx::Error> {
        sqlx::query_as::<_, StatsSnapshot>(
            "SELECT
                id,
                captured_at,
                storage_bytes,
                thumbnails_count,
                users_per_month,
                users_total,
                uploads_total,
                pending_uploads_total,
                accepted_uploads_total
             FROM stats_snapshots
             ORDER BY captured_at DESC, id DESC
             LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_recent_stats_snapshots_ascending(
        &self,
        limit: i64,
    ) -> Result<Vec<StatsSnapshot>, sqlx::Error> {
        sqlx::query_as::<_, StatsSnapshot>(
            "SELECT id, captured_at, storage_bytes, thumbnails_count, users_per_month, users_total, uploads_total, pending_uploads_total, accepted_uploads_total
             FROM (
                 SELECT id, captured_at, storage_bytes, thumbnails_count, users_per_month, users_total, uploads_total, pending_uploads_total, accepted_uploads_total
                 FROM stats_snapshots
                 ORDER BY captured_at DESC, id DESC
                 LIMIT $1
             ) recent
             ORDER BY captured_at ASC, id ASC",
        )
        .bind(limit)
        .fetch_all(&*self.pool)
        .await
    }

    pub async fn get_total_level_count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(DISTINCT level_id) FROM uploads")
            .fetch_one(&*self.pool)
            .await
    }

    pub async fn get_current_pending_upload_count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM uploads WHERE accepted = FALSE AND accepted_time IS NULL",
        )
        .fetch_one(&*self.pool)
        .await
    }
}

pub async fn get_db() -> AppState {
    AppState::new().await
}
