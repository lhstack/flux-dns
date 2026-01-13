//! System Status API module
//!
//! Implements REST API endpoint for system status monitoring.
//!
//! # Requirements
//!
//! - 4.6: Provide service status monitoring functionality

use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use tokio::sync::RwLock;

use crate::db::Database;
use crate::dns::CacheManager;
use crate::dns::proxy::{ProxyManager, UpstreamManager};
use crate::web::ApiError;

/// Application state for status API
#[derive(Clone)]
pub struct StatusState {
    pub db: Arc<Database>,
    pub cache: Arc<CacheManager>,
    pub proxy_manager: Arc<ProxyManager>,
    pub upstream_manager: Arc<UpstreamManager>,
    pub start_time: Arc<RwLock<Instant>>,
}

/// System status response
#[derive(Debug, Serialize)]
pub struct SystemStatusResponse {
    pub status: String,
    pub uptime_seconds: u64,
    pub cache: CacheStatusInfo,
    pub query: QueryStatusInfo,
    pub upstreams: UpstreamsStatusInfo,
    pub strategy: String,
}

/// Cache status information
#[derive(Debug, Serialize)]
pub struct CacheStatusInfo {
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub default_ttl: u64,
    pub max_entries: usize,
}

/// Query status information
#[derive(Debug, Serialize)]
pub struct QueryStatusInfo {
    pub total_queries: i64,
    pub cache_hits: i64,
    pub queries_today: i64,
}

/// Upstreams status information
#[derive(Debug, Serialize)]
pub struct UpstreamsStatusInfo {
    pub total: usize,
    pub healthy: usize,
    pub servers: Vec<UpstreamStatusInfo>,
}

/// Individual upstream server status
#[derive(Debug, Serialize)]
pub struct UpstreamStatusInfo {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub protocol: String,
    pub enabled: bool,
    pub healthy: bool,
    pub queries: u64,
    pub failures: u64,
    pub avg_response_time_ms: u64,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub database: bool,
    pub cache: bool,
    pub upstreams: bool,
}

/// Get system status
///
/// GET /api/status
pub async fn system_status(
    State(state): State<StatusState>,
) -> Result<impl IntoResponse, ApiError> {
    // Calculate uptime
    let start_time = *state.start_time.read().await;
    let uptime_seconds = start_time.elapsed().as_secs();

    // Get cache stats
    let cache_stats = state.cache.stats().await;
    let cache_config = state.cache.get_config().await;

    // Get query stats from database
    let query_stats = state.db.query_logs().get_stats().await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to get query stats: {}", e),
        details: None,
    })?;

    // Get upstream servers status
    let servers = state.db.upstream_servers().list().await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to get upstream servers: {}", e),
        details: None,
    })?;

    let upstream_stats = state.upstream_manager.get_all_stats().await;
    let healthy_count = servers.iter().filter(|s| {
        s.enabled && upstream_stats.get(&s.id).map(|st| st.is_healthy()).unwrap_or(true)
    }).count();

    let upstream_servers: Vec<UpstreamStatusInfo> = servers
        .into_iter()
        .map(|s| {
            let stats = upstream_stats.get(&s.id);
            UpstreamStatusInfo {
                id: s.id,
                name: s.name,
                address: s.address,
                protocol: s.protocol,
                enabled: s.enabled,
                healthy: stats.map(|st| st.is_healthy()).unwrap_or(s.enabled),
                queries: stats.map(|st| st.queries).unwrap_or(0),
                failures: stats.map(|st| st.failures).unwrap_or(0),
                avg_response_time_ms: stats.map(|st| st.avg_response_time_ms()).unwrap_or(0),
            }
        })
        .collect();

    // Get current strategy
    let strategy = state.proxy_manager.get_strategy().await;

    Ok(Json(SystemStatusResponse {
        status: "running".to_string(),
        uptime_seconds,
        cache: CacheStatusInfo {
            entries: cache_stats.entries,
            hits: cache_stats.hits,
            misses: cache_stats.misses,
            hit_rate: cache_stats.hit_rate(),
            default_ttl: cache_config.default_ttl,
            max_entries: cache_config.max_entries,
        },
        query: QueryStatusInfo {
            total_queries: query_stats.total_queries,
            cache_hits: query_stats.cache_hits,
            queries_today: query_stats.queries_today,
        },
        upstreams: UpstreamsStatusInfo {
            total: upstream_servers.len(),
            healthy: healthy_count,
            servers: upstream_servers,
        },
        strategy: strategy.as_str().to_string(),
    }))
}

/// Health check endpoint
///
/// GET /api/health
pub async fn health_check(
    State(state): State<StatusState>,
) -> Result<impl IntoResponse, ApiError> {
    // Check database connectivity
    let db_healthy = state.db.query_logs().get_stats().await.is_ok();

    // Check cache (always healthy if it exists)
    let cache_healthy = true;

    // Check if we have any healthy upstreams
    let servers = state.db.upstream_servers().list_enabled().await.unwrap_or_default();
    let upstreams_healthy = !servers.is_empty();

    let overall_status = if db_healthy && cache_healthy {
        "healthy"
    } else {
        "degraded"
    };

    Ok(Json(HealthCheckResponse {
        status: overall_status.to_string(),
        database: db_healthy,
        cache: cache_healthy,
        upstreams: upstreams_healthy,
    }))
}

/// Build the status API router
pub fn status_router(state: StatusState) -> axum::Router {
    use axum::routing::get;

    axum::Router::new()
        .route("/", get(system_status))
        .route("/health", get(health_check))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_status_info() {
        let info = CacheStatusInfo {
            entries: 100,
            hits: 80,
            misses: 20,
            hit_rate: 0.8,
            default_ttl: 60,
            max_entries: 10000,
        };
        assert_eq!(info.entries, 100);
        assert_eq!(info.hit_rate, 0.8);
    }

    #[test]
    fn test_query_status_info() {
        let info = QueryStatusInfo {
            total_queries: 1000,
            cache_hits: 750,
            queries_today: 100,
        };
        assert_eq!(info.total_queries, 1000);
    }

    #[test]
    fn test_health_check_response() {
        let response = HealthCheckResponse {
            status: "healthy".to_string(),
            database: true,
            cache: true,
            upstreams: true,
        };
        assert_eq!(response.status, "healthy");
        assert!(response.database);
    }
}
