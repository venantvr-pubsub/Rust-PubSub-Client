use rust_pubsub_client::{ClientConfig, PubSubClient};
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = ClientConfig::new(
        "http://localhost:5000",
        "rust_example_consumer",
        vec!["test_topic".to_string(), "data_updates".to_string()],
    )
    .with_idempotence(true, 1000)
    .with_handler_workers(5);

    let client = Arc::new(PubSubClient::new(config));

    client.register_handler("test_topic", |msg| async move {
        println!("Received on test_topic: {:?}", msg);
    });

    client.register_handler("data_updates", |msg| async move {
        println!("Received on data_updates: {:?}", msg);
    });

    let client_clone = client.clone();
    tokio::spawn(async move {
        if let Err(e) = client_clone.start().await {
            eprintln!("Client error: {}", e);
        }
    });

    sleep(Duration::from_secs(2)).await;

    client
        .publish(
            "test_topic",
            json!({"message": "Hello from Rust!"}),
            "rust_example_producer",
            None,
        )
        .await?;

    sleep(Duration::from_secs(5)).await;

    client.stop().await?;

    Ok(())
}
