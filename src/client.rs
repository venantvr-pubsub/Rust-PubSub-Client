use crate::{
    config::ClientConfig, error::Result, idempotence::IdempotenceFilter, message::PubSubMessage,
    Error,
};
use dashmap::DashMap;
use futures::future::BoxFuture;
use parking_lot::RwLock;
use reqwest::Client as HttpClient;
use rust_socketio::{
    asynchronous::{Client as SocketClient, ClientBuilder},
    Payload,
};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

type HandlerFn = Arc<dyn Fn(serde_json::Value) -> BoxFuture<'static, ()> + Send + Sync>;

struct HandlerInfo {
    handler: HandlerFn,
    handler_name: String,
}

pub struct PubSubClient {
    config: ClientConfig,
    socket_client: Arc<RwLock<Option<SocketClient>>>,
    handlers: Arc<DashMap<String, HandlerInfo>>,
    idempotence_filter: Option<Arc<IdempotenceFilter>>,
    http_client: HttpClient,
    running: Arc<RwLock<bool>>,
    worker_semaphore: Arc<Semaphore>,
}

impl PubSubClient {
    pub fn new(config: ClientConfig) -> Self {
        let idempotence_filter = if config.enable_idempotence {
            Some(Arc::new(IdempotenceFilter::new(
                config.idempotence_max_size,
            )))
        } else {
            None
        };

        Self {
            worker_semaphore: Arc::new(Semaphore::new(config.handler_workers)),
            config,
            socket_client: Arc::new(RwLock::new(None)),
            handlers: Arc::new(DashMap::new()),
            idempotence_filter,
            http_client: HttpClient::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub fn register_handler<F, Fut>(&self, topic: impl Into<String>, handler: F)
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let topic = topic.into();
        let handler_name = format!("Handler_{}", topic);

        let handler_fn: HandlerFn = Arc::new(move |value| Box::pin(handler(value)));

        self.handlers.insert(
            topic.clone(),
            HandlerInfo {
                handler: handler_fn,
                handler_name: handler_name.clone(),
            },
        );

        debug!("Registered handler '{}' for topic '{}'", handler_name, topic);
    }

    pub async fn start(&self) -> Result<()> {
        info!(
            "Starting client {} with topics {:?}",
            self.config.consumer, self.config.topics
        );

        *self.running.write() = true;

        let socket_client = self.build_socket_client().await?;

        *self.socket_client.write() = Some(socket_client);

        Ok(())
    }

    async fn build_socket_client(&self) -> Result<SocketClient> {
        let consumer = self.config.consumer.clone();
        let topics = self.config.topics.clone();
        let handlers = self.handlers.clone();
        let idempotence_filter = self.idempotence_filter.clone();
        let http_client = self.http_client.clone();
        let url = self.config.url.clone();
        let socket_client_arc = self.socket_client.clone();
        let worker_semaphore = self.worker_semaphore.clone();

        let consumer_connect = consumer.clone();
        let on_connect = move |_payload: Payload, socket: SocketClient| -> BoxFuture<'static, ()> {
            let consumer = consumer_connect.clone();
            let topics = topics.clone();
            Box::pin(async move {
                info!("[{}] Connected to server", consumer);
                let subscribe_data = json!({
                    "consumer": consumer,
                    "topics": topics
                });

                if let Err(e) = socket.emit("subscribe", subscribe_data).await {
                    error!("Failed to emit subscribe: {}", e);
                }
            })
        };

        let consumer_disconnect = consumer.clone();
        let on_disconnect = move |_payload: Payload, _socket: SocketClient| -> BoxFuture<'static, ()> {
            let consumer = consumer_disconnect.clone();
            Box::pin(async move {
                warn!(
                    "[{}] Disconnected from server. Reconnection will be attempted automatically.",
                    consumer
                );
            })
        };

        let on_message = move |payload: Payload, _socket: SocketClient| -> BoxFuture<'static, ()> {
            let handlers = handlers.clone();
            let idempotence_filter = idempotence_filter.clone();
            let http_client = http_client.clone();
            let url = url.clone();
            let consumer = consumer.clone();
            let socket_client_arc = socket_client_arc.clone();
            let worker_semaphore = worker_semaphore.clone();

            Box::pin(async move {
                let data: serde_json::Value = match payload {
                    Payload::Text(values) => {
                        if let Some(first) = values.first() {
                            first.clone()
                        } else {
                            error!("Empty text payload");
                            return;
                        }
                    },
                    #[allow(deprecated)]
                    Payload::String(s) => match serde_json::from_str(&s) {
                        Ok(v) => v,
                        Err(e) => {
                            error!("Failed to parse message: {}", e);
                            return;
                        }
                    },
                    Payload::Binary(b) => match serde_json::from_slice(&b) {
                        Ok(v) => v,
                        Err(e) => {
                            error!("Failed to parse binary message: {}", e);
                            return;
                        }
                    },
                    _ => return,
                };

                let topic = data.get("topic").and_then(|v| v.as_str());
                let message = data.get("message");
                let message_id = data.get("message_id").and_then(|v| v.as_str());
                let producer = data.get("producer").and_then(|v| v.as_str());

                if topic.is_none() || message.is_none() {
                    return;
                }

                let topic = topic.unwrap();
                let message = message.unwrap().clone();

                if let Some(filter) = &idempotence_filter {
                    if !filter.should_process(message_id) {
                        info!(
                            "[{}] Duplicate message skipped from topic [{}]: ID={:?}",
                            consumer, topic, message_id
                        );
                        return;
                    }
                }

                let handler_info = if let Some(info) = handlers.get(topic) {
                    HandlerInfo {
                        handler: info.handler.clone(),
                        handler_name: info.handler_name.clone(),
                    }
                } else if let Some(info) = handlers.get("*") {
                    let enriched_message = json!({
                        "topic": topic,
                        "message": message,
                        "producer": producer,
                        "message_id": message_id
                    });

                    let handler_name = info.handler_name.clone();
                    let handler = info.handler.clone();

                    let permit = worker_semaphore.clone().acquire_owned().await.unwrap();

                    tokio::spawn(async move {
                        handler(enriched_message).await;
                        drop(permit);
                    });

                    Self::notify_consumption_static(
                        http_client.clone(),
                        url.clone(),
                        socket_client_arc.clone(),
                        &handler_name,
                        topic,
                        message_id,
                        &message,
                    )
                    .await;
                    return;
                } else {
                    warn!("[{}] No handler for topic {}", consumer, topic);
                    return;
                };

                info!(
                    "[{}] Processing message from topic [{}]: (from {:?}, ID={:?})",
                    consumer, topic, producer, message_id
                );

                let handler_name = handler_info.handler_name.clone();
                let handler = handler_info.handler.clone();
                let message_for_handler = message.clone();

                let permit = worker_semaphore.clone().acquire_owned().await.unwrap();

                tokio::spawn(async move {
                    handler(message_for_handler).await;
                    drop(permit);
                });

                Self::notify_consumption_static(
                    http_client,
                    url,
                    socket_client_arc,
                    &handler_name,
                    topic,
                    message_id,
                    &message,
                )
                .await;
            })
        };

        let client = ClientBuilder::new(&self.config.url)
            .on("connect", on_connect)
            .on("disconnect", on_disconnect)
            .on("message", on_message)
            .on("new_message", |_payload: Payload, _socket: SocketClient| -> BoxFuture<'static, ()> { Box::pin(async {}) })
            .reconnect(self.config.reconnection)
            .reconnect_delay(
                self.config.reconnection_delay.as_millis() as u64,
                self.config.reconnection_delay_max.as_millis() as u64,
            )
            .max_reconnect_attempts(self.config.reconnection_attempts.min(255) as u8);

        let client = client.connect().await.map_err(|e| Error::SocketIo(e.to_string()))?;

        Ok(client)
    }

    async fn notify_consumption_static(
        _http_client: HttpClient,
        _url: String,
        socket_client_arc: Arc<RwLock<Option<SocketClient>>>,
        handler_name: &str,
        topic: &str,
        message_id: Option<&str>,
        message: &serde_json::Value,
    ) {
        let data = json!({
            "consumer": handler_name,
            "topic": topic,
            "message_id": message_id,
            "message": message,
        });

        let socket_opt = {
            let guard = socket_client_arc.read();
            guard.clone()
        };

        if let Some(socket) = socket_opt {
            if let Err(e) = socket.emit("consumed", data).await {
                error!("Failed to emit consumed event: {}", e);
            }
        }
    }

    pub async fn publish(
        &self,
        topic: impl Into<String>,
        message: serde_json::Value,
        producer: impl Into<String>,
        message_id: Option<String>,
    ) -> Result<()> {
        let topic = topic.into();
        let producer = producer.into();

        let msg = PubSubMessage::new(&topic, message, &producer, message_id);
        let url = format!("{}/publish", self.config.url);

        info!("[{}] Publishing to {}", self.config.consumer, topic);

        let response = self
            .http_client
            .post(&url)
            .json(&msg.to_dict())
            .send()
            .await?;

        if response.status().is_success() {
            let body = response.text().await?;
            info!("[{}] Publish response: {}", self.config.consumer, body);
        } else {
            error!(
                "[{}] Publish failed with status: {}",
                self.config.consumer,
                response.status()
            );
        }

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        info!("Stopping client {}", self.config.consumer);
        *self.running.write() = false;

        if let Some(socket) = self.socket_client.write().take() {
            socket
                .disconnect()
                .await
                .map_err(|e| Error::SocketIo(e.to_string()))?;
        }

        Ok(())
    }

    pub fn is_running(&self) -> bool {
        *self.running.read()
    }
}
