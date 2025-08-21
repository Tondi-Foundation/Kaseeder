//! Application constants and configuration limits

use std::time::Duration;

// Network Configuration
pub const DEFAULT_DNS_PORT: u16 = 5354;
pub const DEFAULT_GRPC_PORT: u16 = 3737;
pub const DEFAULT_PROFILE_PORT: u16 = 8080;

// Port Ranges
pub const MIN_PORT: u16 = 1024;  // Avoid privileged ports
pub const MAX_PORT: u16 = 65535;

// Thread Configuration
pub const MIN_THREADS: u8 = 1;
pub const MAX_THREADS: u8 = 64;
pub const DEFAULT_THREADS: u8 = 8;

// Network Suffix Limits
pub const MAX_NETWORK_SUFFIX: u16 = 99;

// Protocol Configuration
pub const MIN_PROTOCOL_VERSION: u16 = 0;
pub const MAX_PROTOCOL_VERSION: u16 = 65535;

// Timeout Configuration
pub const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(60);
pub const DEFAULT_WRITE_TIMEOUT: Duration = Duration::from_secs(60);

// Crawler Configuration
pub const MAX_CONCURRENT_POLLS: usize = 100;
pub const CRAWLER_SLEEP_INTERVAL: Duration = Duration::from_secs(10);
pub const MAX_ADDRESSES_PER_BATCH: usize = 1000;

// Address Manager Configuration
pub const DEFAULT_MAX_ADDRESSES: usize = 2000;
pub const MAX_ADDRESSES: usize = 10000;
pub const PEER_CLEANUP_INTERVAL: Duration = Duration::from_secs(3600);  // 1 hour
pub const ADDRESS_EXPIRY_TIMEOUT: Duration = Duration::from_secs(86400);  // 24 hours

// DNS Configuration
pub const MAX_DNS_RECORDS: usize = 100;
pub const DNS_TTL: u32 = 300;  // 5 minutes
pub const DNS_CACHE_SIZE: usize = 1000;

// gRPC Configuration
pub const MAX_GRPC_CONNECTIONS: usize = 100;
pub const GRPC_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);
pub const GRPC_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(10);

// Logging Configuration
pub const MAX_LOG_FILE_SIZE: u64 = 100 * 1024 * 1024;  // 100 MB
pub const MAX_LOG_FILES: usize = 10;
pub const LOG_ROTATION_INTERVAL: Duration = Duration::from_secs(86400);  // 24 hours

// Health Check Configuration
pub const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);
pub const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(10);
pub const HEALTH_CHECK_RETRIES: u32 = 3;

// Performance Monitoring
pub const METRICS_COLLECTION_INTERVAL: Duration = Duration::from_secs(60);
pub const MAX_METRICS_HISTORY: usize = 1000;

// Validation Functions
pub fn is_valid_port(port: u16) -> bool {
    port >= MIN_PORT && port <= MAX_PORT
}

pub fn is_valid_thread_count(threads: u8) -> bool {
    threads >= MIN_THREADS && threads <= MAX_THREADS
}

pub fn is_valid_network_suffix(suffix: u16) -> bool {
    suffix <= MAX_NETWORK_SUFFIX
}

pub fn is_valid_protocol_version(version: u16) -> bool {
    version >= MIN_PROTOCOL_VERSION && version <= MAX_PROTOCOL_VERSION
}

pub fn is_valid_max_addresses(count: usize) -> bool {
    count > 0 && count <= MAX_ADDRESSES
}
