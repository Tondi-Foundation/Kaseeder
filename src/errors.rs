use thiserror::Error;

/// Application error types
#[derive(Error, Debug)]
pub enum KaseederError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("DNS error: {0}")]
    Dns(String),

    #[error("gRPC error: {0}")]
    Grpc(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Service error: {0}")]
    Service(String),

    #[error("Address manager error: {0}")]
    AddressManager(String),

    #[error("Crawler error: {0}")]
    Crawler(String),
}

/// Result type for the application
pub type Result<T> = std::result::Result<T, KaseederError>;

impl From<anyhow::Error> for KaseederError {
    fn from(err: anyhow::Error) -> Self {
        KaseederError::Service(err.to_string())
    }
}

impl From<toml::de::Error> for KaseederError {
    fn from(err: toml::de::Error) -> Self {
        KaseederError::Serialization(format!("TOML deserialization error: {}", err))
    }
}

impl From<toml::ser::Error> for KaseederError {
    fn from(err: toml::ser::Error) -> Self {
        KaseederError::Serialization(format!("TOML serialization error: {}", err))
    }
}

impl From<serde_json::Error> for KaseederError {
    fn from(err: serde_json::Error) -> Self {
        KaseederError::Serialization(format!("JSON error: {}", err))
    }
}

impl From<tonic::transport::Error> for KaseederError {
    fn from(err: tonic::transport::Error) -> Self {
        KaseederError::Grpc(format!("gRPC transport error: {}", err))
    }
}

impl From<tonic::Status> for KaseederError {
    fn from(err: tonic::Status) -> Self {
        KaseederError::Grpc(format!("gRPC status error: {}", err))
    }
}
