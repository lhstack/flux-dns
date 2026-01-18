use std::sync::Arc;
use tokio::time::{interval, Duration, Instant};
use tokio::sync::Mutex;
use crate::state::AppState;
use serde_json::json;

pub struct AlertManager {
    state: Arc<AppState>,
    last_alert_time: Mutex<Option<Instant>>,
}

impl AlertManager {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            last_alert_time: Mutex::new(None),
        }
    }

    pub async fn start(self: Arc<Self>) {
        tracing::info!("AlertManager background task started");
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Check every 30s
            loop {
                interval.tick().await;
                if let Err(e) = self.check_alerts().await {
                    tracing::error!("Failed to check alerts: {}", e);
                }
            }
        });
    }

    async fn check_alerts(&self) -> anyhow::Result<()> {
        let config = self.state.db.system_config();
        
        // 1. Check if enabled
        let enabled = config.get("alert_enabled").await?.unwrap_or_default() == "true";
        if !enabled {
            return Ok(());
        }

        // 2. Cooldown check (5 minutes)
        let mut last_alert = self.last_alert_time.lock().await;
        if let Some(last) = *last_alert {
            if last.elapsed() < Duration::from_secs(300) {
                return Ok(());
            }
        }

        // 3. Get thresholds
        let webhook = config.get("alert_webhook_url").await?;
        if webhook.is_none() || webhook.as_ref().unwrap().is_empty() {
            return Ok(());
        }
        let webhook = webhook.unwrap();
        
        let latency_threshold: f64 = config.get("alert_latency_threshold_ms").await?
            .and_then(|v| v.parse().ok())
            .unwrap_or(200.0);

        // 4. Check current stats
        let stats = self.state.upstream_manager.get_all_stats().await;
        
        // Calculate weighted average latency based on query count
        let mut total_latency_product = 0.0;
        let mut total_queries_count = 0;
        
        for s in stats.values() {
             if s.queries > 0 {
                 total_latency_product += (s.queries as f64) * s.ema_response_time_ms;
                 total_queries_count += s.queries;
             }
        }
        
        let avg_latency = if total_queries_count > 0 {
            total_latency_product / (total_queries_count as f64)
        } else {
            0.0
        };
        
        if avg_latency > latency_threshold && total_queries_count > 0 {
            let message = format!(
                "🚨 **High Latency Alert**\n\nCurrent Average Latency: **{:.2}ms**\nThreshold: {}ms\n\nPlease check your upstream servers.",
                avg_latency, latency_threshold
            );
            
            self.send_alert(&webhook, &message).await?;
            *last_alert = Some(Instant::now());
        }

        Ok(())
    }

    async fn send_alert(&self, webhook: &str, message: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        // Webhook payload compatible with Slack, Discord, etc.
        let payload = json!({
            "text": message,
            "content": message 
        });

        client.post(webhook)
            .json(&payload)
            .send()
            .await?;
            
        tracing::info!("Alert sent to webhook: {}", webhook);
        Ok(())
    }
}
