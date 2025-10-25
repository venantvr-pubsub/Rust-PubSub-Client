use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Socket.IO error: {0}")]
    SocketIo(String),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Client not connected")]
    NotConnected,

    #[error("Handler error: {0}")]
    Handler(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, Error>;
