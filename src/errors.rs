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

    #[error("Invalid address format: {0}")]
    InvalidAddress(String),

    #[error("Invalid port number: {0}")]
    InvalidPort(u16),

    #[error("Invalid IP address: {0}")]
    InvalidIp(String),

    #[error("Invalid configuration value: {field} = {value}, expected: {expected}")]
    InvalidConfigValue {
        field: String,
        value: String,
        expected: String,
    },

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),
}

/// Result type for the application
pub type Result<T> = std::result::Result<T, KaseederError>;

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

impl From<std::net::AddrParseError> for KaseederError {
    fn from(err: std::net::AddrParseError) -> Self {
        KaseederError::InvalidAddress(format!("Address parse error: {}", err))
    }
}

impl From<std::num::ParseIntError> for KaseederError {
    fn from(err: std::num::ParseIntError) -> Self {
        KaseederError::Validation(format!("Number parse error: {}", err))
    }
}

impl From<uuid::Error> for KaseederError {
    fn from(err: uuid::Error) -> Self {
        KaseederError::Validation(format!("UUID error: {}", err))
    }
}

impl From<chrono::ParseError> for KaseederError {
    fn from(err: chrono::ParseError) -> Self {
        KaseederError::Validation(format!("Date/time parse error: {}", err))
    }
}

impl From<sled::Error> for KaseederError {
    fn from(err: sled::Error) -> Self {
        KaseederError::Database(format!("Sled database error: {}", err))
    }
}

impl From<reqwest::Error> for KaseederError {
    fn from(err: reqwest::Error) -> Self {
        KaseederError::Network(format!("HTTP request error: {}", err))
    }
}

impl From<tokio::time::error::Elapsed> for KaseederError {
    fn from(_err: tokio::time::error::Elapsed) -> Self {
        KaseederError::Timeout("Operation timed out".to_string())
    }
}

impl From<tokio::sync::AcquireError> for KaseederError {
    fn from(err: tokio::sync::AcquireError) -> Self {
        KaseederError::ResourceExhausted(format!("Failed to acquire semaphore: {}", err))
    }
}

impl From<trust_dns_proto::error::ProtoError> for KaseederError {
    fn from(err: trust_dns_proto::error::ProtoError) -> Self {
        KaseederError::Protocol(format!("DNS protocol error: {}", err))
    }
}

impl From<kaspa_p2p_lib::common::ProtocolError> for KaseederError {
    fn from(err: kaspa_p2p_lib::common::ProtocolError) -> Self {
        KaseederError::Protocol(format!("Kaspa protocol error: {}", err))
    }
}

impl From<tracing_subscriber::filter::ParseError> for KaseederError {
    fn from(err: tracing_subscriber::filter::ParseError) -> Self {
        KaseederError::Config(format!("Log level parse error: {}", err))
    }
}
