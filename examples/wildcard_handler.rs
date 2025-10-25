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
        "wildcard_consumer",
        vec!["*".to_string()],
    );

    let client = Arc::new(PubSubClient::new(config));

    client.register_handler("*", |msg| async move {
        if let Some(topic) = msg.get("topic") {
            println!("Wildcard handler received message on topic: {}", topic);
            println!("Full message: {:?}", msg);
        }
    });

    let client_clone = client.clone();
    tokio::spawn(async move {
        if let Err(e) = client_clone.start().await {
            eprintln!("Client error: {}", e);
        }
    });

    sleep(Duration::from_secs(2)).await;

    for i in 0..5 {
        client
            .publish(
                format!("dynamic_topic_{}", i),
                json!({"index": i, "data": "test"}),
                "wildcard_producer",
                None,
            )
            .await?;
        sleep(Duration::from_millis(500)).await;
    }

    sleep(Duration::from_secs(3)).await;

    client.stop().await?;

    Ok(())
}
