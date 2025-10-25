use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub url: String,
    pub consumer: String,
    pub topics: Vec<String>,
    pub enable_idempotence: bool,
    pub idempotence_max_size: usize,
    pub reconnection: bool,
    pub reconnection_attempts: u32,
    pub reconnection_delay: Duration,
    pub reconnection_delay_max: Duration,
    pub handler_workers: usize,
}

impl ClientConfig {
    pub fn new(url: impl Into<String>, consumer: impl Into<String>, topics: Vec<String>) -> Self {
        let reconnection = std::env::var("PUBSUB_RECONNECTION_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .to_lowercase()
            == "true";

        let reconnection_attempts = std::env::var("PUBSUB_RECONNECTION_ATTEMPTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let reconnection_delay = std::env::var("PUBSUB_RECONNECTION_DELAY_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_millis(2000));

        let reconnection_delay_max = std::env::var("PUBSUB_RECONNECTION_DELAY_MAX_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_millis(10000));

        Self {
            url: url.into().trim_end_matches('/').to_string(),
            consumer: consumer.into(),
            topics,
            enable_idempotence: false,
            idempotence_max_size: 1000,
            reconnection,
            reconnection_attempts,
            reconnection_delay,
            reconnection_delay_max,
            handler_workers: 10,
        }
    }

    pub fn with_idempotence(mut self, enabled: bool, max_size: usize) -> Self {
        self.enable_idempotence = enabled;
        self.idempotence_max_size = max_size;
        self
    }

    pub fn with_handler_workers(mut self, workers: usize) -> Self {
        self.handler_workers = workers;
        self
    }
}
