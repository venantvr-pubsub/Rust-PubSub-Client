use rust_pubsub_client::{ClientConfig, PubSubClient};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_client_creation() {
    let config = ClientConfig::new(
        "http://localhost:5000",
        "test_consumer",
        vec!["test_topic".to_string()],
    );
    let client = PubSubClient::new(config);
    assert!(!client.is_running());
}

#[tokio::test]
async fn test_handler_registration() {
    let config = ClientConfig::new(
        "http://localhost:5000",
        "test_consumer",
        vec!["test_topic".to_string()],
    );
    let client = PubSubClient::new(config);

    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    client.register_handler("test_topic", move |msg| {
        let received = received_clone.clone();
        async move {
            received.lock().await.push(msg);
        }
    });
}

#[tokio::test]
async fn test_idempotence_filter() {
    let config = ClientConfig::new("http://localhost:5000", "test_consumer", vec![])
        .with_idempotence(true, 100);
    let client = PubSubClient::new(config);

    assert!(!client.is_running());
}

#[tokio::test]
async fn test_config_builder() {
    let config = ClientConfig::new(
        "http://localhost:5000",
        "test_consumer",
        vec!["topic1".to_string(), "topic2".to_string()],
    )
    .with_idempotence(true, 500)
    .with_handler_workers(20);

    assert_eq!(config.consumer, "test_consumer");
    assert_eq!(config.topics.len(), 2);
    assert!(config.enable_idempotence);
    assert_eq!(config.idempotence_max_size, 500);
    assert_eq!(config.handler_workers, 20);
}

#[tokio::test]
async fn test_message_creation() {
    use rust_pubsub_client::PubSubMessage;

    let msg = PubSubMessage::new(
        "test_topic",
        json!({"key": "value"}),
        "test_producer",
        Some("msg_123".to_string()),
    );

    assert_eq!(msg.topic, "test_topic");
    assert_eq!(msg.message_id, "msg_123");
    assert_eq!(msg.producer, "test_producer");
}

#[tokio::test]
async fn test_wildcard_handler() {
    let config = ClientConfig::new("http://localhost:5000", "test_consumer", vec![]);
    let client = PubSubClient::new(config);

    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    client.register_handler("*", move |msg| {
        let received = received_clone.clone();
        async move {
            received.lock().await.push(msg);
        }
    });
}
