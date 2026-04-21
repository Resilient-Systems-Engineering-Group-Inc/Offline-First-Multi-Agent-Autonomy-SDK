//! Rate limiting middleware for API protection.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use warp::Filter;
use tracing::warn;

/// Rate limit configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub window_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 100,
            burst_size: 20,
            window_seconds: 60,
        }
    }
}

/// Rate limiter state.
#[derive(Clone)]
pub struct RateLimiter {
    configs: HashMap<String, RateLimitConfig>,
    request_counts: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            request_counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add rate limit configuration for an endpoint.
    pub fn with_endpoint(mut self, endpoint: &str, config: RateLimitConfig) -> Self {
        self.configs.insert(endpoint.to_string(), config);
        self
    }

    /// Check if request is allowed.
    pub async fn is_allowed(&self, client_id: &str, endpoint: &str) -> bool {
        let config = self.configs
            .get(endpoint)
            .unwrap_or(&RateLimitConfig::default());

        let mut counts = self.request_counts.lock().await;
        let now = Instant::now();
        let window_start = now - Duration::from_secs(config.window_seconds);

        // Get or create request log
        let entry = counts
            .entry(client_id.to_string())
            .or_insert_with(Vec::new);

        // Remove old requests outside the window
        entry.retain(|&time| time > window_start);

        // Check if under limit
        if entry.len() as u32 < config.requests_per_minute {
            entry.push(now);
            true
        } else {
            warn!("Rate limit exceeded for client: {}", client_id);
            false
        }
    }

    /// Get remaining requests for a client.
    pub async fn remaining(&self, client_id: &str, endpoint: &str) -> u32 {
        let config = self.configs
            .get(endpoint)
            .unwrap_or(&RateLimitConfig::default());

        let counts = self.request_counts.lock().await;
        
        if let Some(entry) = counts.get(client_id) {
            let now = Instant::now();
            let window_start = now - Duration::from_secs(config.window_seconds);
            let valid_requests: usize = entry.iter()
                .filter(|&&time| time > window_start)
                .count();
            
            config.requests_per_minute.saturating_sub(valid_requests as u32)
        } else {
            config.requests_per_minute
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Create rate limit filter.
pub fn rate_limit(
    limiter: Arc<RateLimiter>,
    endpoint: &'static str,
) -> impl Filter<Extract = (), Error = warp::Rejection> + Clone {
    warp::header::<String>("X-Client-ID")
        .or_else(|_| async { Ok::<(String,), warp::Rejection>((String::from("anonymous"),)) })
        .and_then(move |client_id: String| {
            let limiter = limiter.clone();
            
            async move {
                if limiter.is_allowed(&client_id, endpoint).await {
                    Ok(())
                } else {
                    Err(warp::reject::custom(RateLimitExceeded))
                }
            }
        })
}

/// Rate limit exceeded error.
#[derive(Debug)]
pub struct RateLimitExceeded;

impl warp::reject::Reject for RateLimitExceeded {}

/// Handle rate limit rejection.
pub async fn handle_rate_limit_rejection(
    err: warp::Rejection,
) -> Result<impl warp::Reply, warp::Rejection> {
    if err.find::<RateLimitExceeded>().is_some() {
        let status = warp::http::StatusCode::TOO_MANY_REQUESTS;
        Ok(warp::reply::with_status(
            serde_json::json!({
                "error": "Rate limit exceeded",
                "message": "Too many requests. Please try again later."
            }),
            status,
        ))
    } else {
        Err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new()
            .with_endpoint("/api/tasks", RateLimitConfig {
                requests_per_minute: 10,
                burst_size: 5,
                window_seconds: 60,
            });

        // Should allow first 10 requests
        for _ in 0..10 {
            assert!(limiter.is_allowed("client-1", "/api/tasks").await);
        }

        // 11th request should be denied
        assert!(!limiter.is_allowed("client-1", "/api/tasks").await);

        // Different client should be allowed
        assert!(limiter.is_allowed("client-2", "/api/tasks").await);
    }

    #[tokio::test]
    async fn test_remaining_requests() {
        let limiter = RateLimiter::new()
            .with_endpoint("/api/tasks", RateLimitConfig {
                requests_per_minute: 100,
                burst_size: 20,
                window_seconds: 60,
            });

        // Make 10 requests
        for _ in 0..10 {
            limiter.is_allowed("client-1", "/api/tasks").await;
        }

        // Should have 90 remaining
        let remaining = limiter.remaining("client-1", "/api/tasks").await;
        assert_eq!(remaining, 90);
    }
}
