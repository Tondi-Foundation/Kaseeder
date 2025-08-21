use std::time::Duration;

// ============================================================================
// Network configuration constants
// ============================================================================

/// Default DNS server port
pub const DEFAULT_DNS_PORT: u16 = 53;
/// Default gRPC server port
pub const DEFAULT_GRPC_PORT: u16 = 50051;
/// Default performance profiling port
pub const DEFAULT_PROFILE_PORT: u16 = 8080;
/// Default Kaspa node port
pub const DEFAULT_KASPA_PORT: u16 = 16111;
/// Default testnet port
pub const DEFAULT_TESTNET_PORT: u16 = 16110;
/// Testnet-11 special port
pub const TESTNET_11_PORT: u16 = 16311;

// ============================================================================
// Timeout and interval constants
// ============================================================================

/// DNS server read timeout
pub const DNS_READ_TIMEOUT: Duration = Duration::from_secs(1);
/// Crawler sleep interval
pub const CRAWLER_SLEEP_INTERVAL: Duration = Duration::from_secs(10);
/// Maximum concurrent poll count
pub const MAX_CONCURRENT_POLLS: usize = 100;
/// Batch size range
pub const MIN_BATCH_SIZE: usize = 20;
pub const MAX_BATCH_SIZE: usize = 50;
/// Connection timeout
pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
/// Address wait timeout
pub const ADDRESS_WAIT_TIMEOUT: Duration = Duration::from_secs(30);
/// Ping interval
pub const PING_INTERVAL: Duration = Duration::from_secs(60);
/// Health check interval
pub const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);
/// Performance metrics collection interval
pub const PERFORMANCE_COLLECTION_INTERVAL: Duration = Duration::from_secs(10);
/// Performance profiling update interval
pub const PROFILING_UPDATE_INTERVAL: Duration = Duration::from_secs(5);

// ============================================================================
// Address management constants
// ============================================================================

/// Node expiration timeout (good nodes)
pub const STALE_GOOD_TIMEOUT: Duration = Duration::from_secs(60 * 60); // 1 hour
/// Node expiration timeout (bad nodes)
pub const STALE_BAD_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60); // 2 hours
/// Address cleanup expiration timeout
pub const PRUNE_EXPIRE_TIMEOUT: Duration = Duration::from_secs(8 * 60 * 60); // 8 hours
/// Address cleanup interval
pub const PRUNE_ADDRESS_INTERVAL: Duration = Duration::from_secs(60 * 60); // 1 hour
/// Address dump interval
pub const DUMP_ADDRESS_INTERVAL: Duration = Duration::from_secs(10 * 60); // 10 minutes

// ============================================================================
// DNS-related constants
// ============================================================================

/// DNS buffer size
pub const DNS_BUFFER_SIZE: usize = 512;
/// Maximum DNS record count
pub const MAX_DNS_RECORDS: usize = 8;
/// DNS record TTL
pub const DNS_RECORD_TTL: u32 = 30;
/// DNS SOA record TTL
pub const DNS_SOA_TTL: u32 = 86400;
/// IPv6 placeholder address
pub const IPV6_PLACEHOLDER: [u16; 8] = [0x100, 0, 0, 0, 0, 0, 0, 0];

// ============================================================================
// Performance monitoring constants
// ============================================================================

/// CPU usage warning threshold
pub const CPU_WARNING_THRESHOLD: f64 = 90.0;
/// CPU usage notice threshold
pub const CPU_NOTICE_THRESHOLD: f64 = 70.0;
/// Memory usage warning threshold (1GB)
pub const MEMORY_WARNING_THRESHOLD: u64 = 1024 * 1024 * 1024;
/// Response time warning threshold (5 seconds)
pub const RESPONSE_TIME_WARNING_THRESHOLD: f64 = 5000.0;
/// Response time notice threshold (2 seconds)
pub const RESPONSE_TIME_NOTICE_THRESHOLD: f64 = 2000.0;

// ============================================================================
// Network adapter constants
// ============================================================================

/// Maximum retry count
pub const MAX_RETRY_COUNT: u32 = 3;
/// Base retry delay
pub const BASE_RETRY_DELAY: Duration = Duration::from_secs(1);
/// Address channel buffer size
pub const ADDRESS_CHANNEL_BUFFER: usize = 100;
/// Exponential backoff base
pub const EXPONENTIAL_BACKOFF_BASE: u32 = 2;

// ============================================================================
// Configuration validation constants
// ============================================================================

/// Maximum thread count
pub const MAX_THREADS: u8 = 64;
/// Maximum testnet suffix
pub const MAX_TESTNET_SUFFIX: u16 = 99;
/// Maximum protocol version
pub const MAX_PROTOCOL_VERSION: u32 = 100;
