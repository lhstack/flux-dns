//! Query Logs API module
//!
//! Implements REST API endpoints for DNS query log viewing.
//!
//! # Requirements
//!
//! - 4.5: Provide query log viewing functionality

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::db::{Database, PaginatedResult, QueryLog, QueryLogFilter, QueryStats};
use crate::web::ApiError;

/// Application state for logs API
#[derive(Clone)]
pub struct LogsState {
    pub db: Arc<Database>,
}

/// Query parameters for log listing
#[derive(Debug, Clone, Deserialize)]
pub struct LogsQueryParams {
    pub query_name: Option<String>,
    pub query_type: Option<String>,
    pub client_ip: Option<String>,
    pub cache_hit: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl From<LogsQueryParams> for QueryLogFilter {
    fn from(params: LogsQueryParams) -> Self {
        Self {
            query_name: params.query_name,
            query_type: params.query_type,
            client_ip: params.client_ip,
            cache_hit: params.cache_hit,
            limit: params.limit,
            offset: params.offset,
        }
    }
}

/// Paginated logs response
#[derive(Debug, Serialize)]
pub struct LogsListResponse {
    pub data: Vec<QueryLog>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

impl From<PaginatedResult<QueryLog>> for LogsListResponse {
    fn from(result: PaginatedResult<QueryLog>) -> Self {
        let has_more = result.offset + (result.items.len() as i64) < result.total;
        Self {
            data: result.items,
            total: result.total,
            limit: result.limit,
            offset: result.offset,
            has_more,
        }
    }
}

/// Query statistics response
#[derive(Debug, Serialize)]
pub struct QueryStatsResponse {
    pub total_queries: i64,
    pub cache_hits: i64,
    pub queries_today: i64,
    pub cache_hit_rate: f64,
}

impl From<QueryStats> for QueryStatsResponse {
    fn from(stats: QueryStats) -> Self {
        let cache_hit_rate = if stats.total_queries > 0 {
            stats.cache_hits as f64 / stats.total_queries as f64
        } else {
            0.0
        };
        Self {
            total_queries: stats.total_queries,
            cache_hits: stats.cache_hits,
            queries_today: stats.queries_today,
            cache_hit_rate,
        }
    }
}

/// List query logs with pagination and filtering
///
/// GET /api/logs
pub async fn list_logs(
    State(state): State<LogsState>,
    Query(params): Query<LogsQueryParams>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.query_logs();
    let filter = QueryLogFilter::from(params);

    let result = repo.list(filter).await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to list query logs: {}", e),
        details: None,
    })?;

    Ok(Json(LogsListResponse::from(result)))
}

/// Get query statistics
///
/// GET /api/logs/stats
pub async fn get_stats(
    State(state): State<LogsState>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.query_logs();

    let stats = repo.get_stats().await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to get query statistics: {}", e),
        details: None,
    })?;

    Ok(Json(QueryStatsResponse::from(stats)))
}

/// Delete old query logs
///
/// DELETE /api/logs/cleanup
#[derive(Debug, Clone, Deserialize)]
pub struct CleanupParams {
    #[serde(default = "default_retention_days")]
    pub days: i64,
}

fn default_retention_days() -> i64 {
    30
}

pub async fn cleanup_logs(
    State(state): State<LogsState>,
    Query(params): Query<CleanupParams>,
) -> Result<impl IntoResponse, ApiError> {
    if params.days < 1 {
        return Err(ApiError {
            code: "BAD_REQUEST".to_string(),
            message: "Days must be at least 1".to_string(),
            details: None,
        });
    }

    let repo = state.db.query_logs();

    let deleted = repo.delete_old(params.days).await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to cleanup query logs: {}", e),
        details: None,
    })?;

    Ok(Json(serde_json::json!({
        "message": format!("Deleted {} old log entries", deleted),
        "deleted_count": deleted
    })))
}

/// Build the logs API router
pub fn logs_router(state: LogsState) -> axum::Router {
    use axum::routing::{delete, get};

    axum::Router::new()
        .route("/", get(list_logs))
        .route("/stats", get(get_stats))
        .route("/cleanup", delete(cleanup_logs))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logs_query_params_to_filter() {
        let params = LogsQueryParams {
            query_name: Some("example.com".to_string()),
            query_type: Some("A".to_string()),
            client_ip: None,
            cache_hit: Some(true),
            limit: Some(50),
            offset: Some(0),
        };
        let filter = QueryLogFilter::from(params);
        assert_eq!(filter.query_name, Some("example.com".to_string()));
        assert_eq!(filter.query_type, Some("A".to_string()));
        assert_eq!(filter.cache_hit, Some(true));
        assert_eq!(filter.limit, Some(50));
        assert_eq!(filter.offset, Some(0));
    }

    #[test]
    fn test_query_stats_response_from() {
        let stats = QueryStats {
            total_queries: 1000,
            cache_hits: 750,
            queries_today: 100,
        };
        let response = QueryStatsResponse::from(stats);
        assert_eq!(response.total_queries, 1000);
        assert_eq!(response.cache_hits, 750);
        assert_eq!(response.queries_today, 100);
        assert!((response.cache_hit_rate - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_query_stats_response_zero_queries() {
        let stats = QueryStats {
            total_queries: 0,
            cache_hits: 0,
            queries_today: 0,
        };
        let response = QueryStatsResponse::from(stats);
        assert_eq!(response.cache_hit_rate, 0.0);
    }

    #[test]
    fn test_logs_list_response_has_more() {
        use chrono::Utc;
        
        // Create a dummy QueryLog for testing
        let dummy_log = QueryLog {
            id: 1,
            query_name: "example.com".to_string(),
            query_type: "A".to_string(),
            client_ip: "127.0.0.1".to_string(),
            response_code: Some("NOERROR".to_string()),
            response_time: Some(10),
            cache_hit: false,
            upstream_used: None,
            created_at: Utc::now(),
        };

        // Case 1: 50 items returned, total is 100, so has_more should be true
        let result = PaginatedResult {
            items: vec![dummy_log.clone(); 50],
            total: 100,
            limit: 50,
            offset: 0,
        };
        let response = LogsListResponse::from(result);
        assert!(response.has_more);

        // Case 2: 50 items returned, total is 50, so has_more should be false
        let result = PaginatedResult {
            items: vec![dummy_log; 50],
            total: 50,
            limit: 50,
            offset: 0,
        };
        let response = LogsListResponse::from(result);
        assert!(!response.has_more);
    }
}
