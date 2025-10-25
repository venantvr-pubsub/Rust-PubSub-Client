use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubSubMessage {
    pub topic: String,
    pub message_id: String,
    pub message: serde_json::Value,
    pub producer: String,
}

impl PubSubMessage {
    pub fn new(
        topic: impl Into<String>,
        message: serde_json::Value,
        producer: impl Into<String>,
        message_id: Option<String>,
    ) -> Self {
        Self {
            topic: topic.into(),
            message_id: message_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            message,
            producer: producer.into(),
        }
    }

    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("topic".to_string(), serde_json::json!(self.topic));
        map.insert("message_id".to_string(), serde_json::json!(self.message_id));
        map.insert("message".to_string(), self.message.clone());
        map.insert("producer".to_string(), serde_json::json!(self.producer));
        map
    }
}
