//! Settings API module
//!
//! Implements REST API endpoints for system settings management.

use std::sync::Arc;

use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::web::ApiError;

/// Application state for settings API
#[derive(Clone)]
pub struct SettingsState {
    pub db: Arc<Database>,
}

/// System settings response
#[derive(Debug, Serialize)]
pub struct SystemSettings {
    /// Disabled record types (e.g., ["AAAA"] to disable IPv6)
    pub disabled_record_types: Vec<String>,
    /// Alert settings
    pub alert_enabled: bool,
    pub alert_webhook_url: Option<String>,
    pub alert_latency_threshold_ms: i64,
}

/// Update settings request
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    /// Disabled record types
    pub disabled_record_types: Option<Vec<String>>,
    /// Alert settings
    pub alert_enabled: Option<bool>,
    pub alert_webhook_url: Option<String>,
    pub alert_latency_threshold_ms: Option<i64>,
}

/// Config key for disabled record types
const CONFIG_KEY_DISABLED_RECORD_TYPES: &str = "disabled_record_types";

/// Get current system settings
///
/// GET /api/settings
pub async fn get_settings(
    State(state): State<SettingsState>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.system_config();

    let disabled_record_types = repo.get(CONFIG_KEY_DISABLED_RECORD_TYPES).await
        .map_err(|e| ApiError {
            code: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to get settings: {}", e),
            details: None,
        })?
        .map(|v| serde_json::from_str::<Vec<String>>(&v).unwrap_or_default())
        .unwrap_or_default();

    let alert_enabled = repo.get("alert_enabled").await
        .unwrap_or(None)
        .unwrap_or_default() == "true";

    let alert_webhook_url = repo.get("alert_webhook_url").await
        .unwrap_or(None);

    let alert_latency_threshold_ms = repo.get("alert_latency_threshold_ms").await
        .unwrap_or(None)
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);

    Ok(Json(SystemSettings {
        disabled_record_types,
        alert_enabled,
        alert_webhook_url,
        alert_latency_threshold_ms,
    }))
}

/// Update system settings
///
/// PUT /api/settings
pub async fn update_settings(
    State(state): State<SettingsState>,
    Json(request): Json<UpdateSettingsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.system_config();

    if let Some(disabled_types) = request.disabled_record_types {
        // Validate record types
        let valid_types = ["A", "AAAA", "CNAME", "MX", "TXT", "PTR", "NS", "SOA", "SRV"];
        for t in &disabled_types {
            let upper = t.to_uppercase();
            if !valid_types.contains(&upper.as_str()) {
                return Err(ApiError {
                    code: "BAD_REQUEST".to_string(),
                    message: format!("Invalid record type: {}", t),
                    details: None,
                });
            }
        }

        let value = serde_json::to_string(&disabled_types).map_err(|e| ApiError {
            code: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to serialize settings: {}", e),
            details: None,
        })?;

        repo.set(CONFIG_KEY_DISABLED_RECORD_TYPES, &value).await.map_err(|e| ApiError {
            code: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to save settings: {}", e),
            details: None,
        })?;
    }

    if let Some(enabled) = request.alert_enabled {
        repo.set("alert_enabled", if enabled { "true" } else { "false" }).await.map_err(|e| ApiError {
            code: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to save alert settings: {}", e),
            details: None,
        })?;
    }

    if let Some(url) = request.alert_webhook_url {
        repo.set("alert_webhook_url", &url).await.map_err(|e| ApiError {
            code: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to save alert settings: {}", e),
            details: None,
        })?;
    }

    if let Some(threshold) = request.alert_latency_threshold_ms {
        repo.set("alert_latency_threshold_ms", &threshold.to_string()).await.map_err(|e| ApiError {
            code: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to save alert settings: {}", e),
            details: None,
        })?;
    }

    // Return updated settings
    get_settings(State(state)).await
}

/// Build the settings API router
pub fn settings_router(state: SettingsState) -> axum::Router {
    use axum::routing::get;

    axum::Router::new()
        .route("/", get(get_settings).put(update_settings))
        .route("/test-alert", axum::routing::post(test_alert))
        .with_state(state)
}

/// Send a test alert
///
/// POST /api/settings/test-alert
async fn test_alert(
    State(state): State<SettingsState>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.system_config();
    
    let webhook = repo.get("alert_webhook_url").await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to get webhook URL: {}", e),
        details: None,
    })?;

    if let Some(url) = webhook {
        if url.is_empty() {
             return Err(ApiError {
                code: "BAD_REQUEST".to_string(),
                message: "Webhook URL is not configured".to_string(),
                details: None,
            });
        }
        
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "text": "🔔 **Test Alert**\n\nThis is a test notification from FluxDNS.",
            "content": "Test notification from FluxDNS"
        });

        client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ApiError {
                code: "INTERNAL_ERROR".to_string(),
                message: format!("Failed to send test alert: {}", e),
                details: None,
            })?;
            
        Ok(Json(serde_json::json!({ "status": "ok" })))
    } else {
        Err(ApiError {
            code: "BAD_REQUEST".to_string(),
            message: "Webhook URL is not configured".to_string(),
            details: None,
        })
    }
}
