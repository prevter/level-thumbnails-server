use crate::{database, util};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

pub async fn get_stats(State(db): State<database::AppState>) -> Response {
    let latest_snapshot = match db.get_recent_stats_snapshots(1).await {
        Ok(mut snapshots) => snapshots.pop(),
        Err(e) => {
            return util::str_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to fetch stats snapshot: {}", e),
            )
        }
    };

    let total_levels = match db.get_total_level_count().await {
        Ok(value) => value,
        Err(e) => {
            return util::str_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to fetch total level count: {}", e),
            )
        }
    };

    let current_pending_uploads = match db.get_current_pending_upload_count().await {
        Ok(value) => value,
        Err(e) => {
            return util::str_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to fetch pending upload count: {}", e),
            )
        }
    };

    let snapshot = latest_snapshot;

    util::response(
        StatusCode::OK,
        serde_json::json!({
            "status": StatusCode::OK.as_u16(),
            "data": {
                "storage": snapshot.as_ref().map(|s| s.storage_bytes).unwrap_or(0),
                "thumbnails": snapshot.as_ref().map(|s| s.thumbnails_count).unwrap_or(0),
                "users_per_month": snapshot.as_ref().and_then(|s| s.users_per_month).unwrap_or(0),
                "users_total": snapshot.as_ref().map(|s| s.users_total).unwrap_or(0),
                "uploads_total": snapshot.as_ref().map(|s| s.uploads_total).unwrap_or(0),
                "pending_uploads_total": snapshot.as_ref().map(|s| s.pending_uploads_total).unwrap_or(0),
                "accepted_uploads_total": snapshot.as_ref().map(|s| s.accepted_uploads_total).unwrap_or(0),
                "total_levels": total_levels,
                "current_pending_uploads": current_pending_uploads,
            }
        }),
    )
}

#[derive(Debug, Default, Deserialize)]
pub struct StatsHistoryQueryParams {
    pub limit: Option<i64>,
}

pub async fn get_stats_history(
    Query(params): Query<StatsHistoryQueryParams>,
    State(db): State<database::AppState>,
) -> Response {
    let limit = params.limit.unwrap_or(72).clamp(1, 720);

    match db.get_recent_stats_snapshots_ascending(limit).await {
        Ok(history) => util::response(
            StatusCode::OK,
            serde_json::json!({
                "status": StatusCode::OK.as_u16(),
                "data": history,
            }),
        ),
        Err(e) => util::str_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to fetch stats history: {}", e),
        ),
    }
}


