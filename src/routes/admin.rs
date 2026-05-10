use crate::{database, util};
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use serde::Deserialize;
use crate::util::VersionInfo;

pub async fn admin_middleware(
    headers: &HeaderMap,
    db: &database::AppState,
) -> Result<database::User, Response> {
    match util::auth_middleware(headers, db).await {
        Ok(user) => {
            if user.role == database::Role::Admin {
                Ok(user)
            } else {
                Err(util::str_response(StatusCode::FORBIDDEN, "Admin privileges required"))
            }
        }
        Err(resp) => Err(resp),
    }
}

pub async fn get_settings(headers: HeaderMap, State(db): State<database::AppState>) -> Response {
    match admin_middleware(&headers, &db).await {
        Ok(_) => util::response(
            StatusCode::OK,
            serde_json::to_value(&*db.settings.read().await).unwrap(),
        ),
        Err(resp) => resp,
    }
}

#[derive(Deserialize, Debug)]
pub struct UpdateSettingsPayload {
    pub pause_submissions: bool,
    pub min_supported_client: String,
}

pub async fn update_settings(
    headers: HeaderMap,
    State(db): State<database::AppState>,
    Json(payload): Json<UpdateSettingsPayload>,
) -> Response {
    match admin_middleware(&headers, &db).await {
        Ok(_) => {
            {
                let mut settings = db.settings.write().await;
                settings.pause_submissions = payload.pause_submissions;
                settings.min_supported_client = match VersionInfo::from_str(&payload.min_supported_client) {
                    Some(version) => version,
                    None => {
                        return util::str_response(
                            StatusCode::BAD_REQUEST,
                            "Invalid version format for min_supported_client",
                        );
                    }
                };
            }

            match db.save_settings().await {
                Ok(_) => util::str_response(StatusCode::OK, "Settings updated successfully"),
                Err(e) => util::str_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Failed to save settings: {}", e),
                ),
            }
        }
        Err(resp) => resp,
    }
}
