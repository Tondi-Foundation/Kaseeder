use std::time::Duration;

// ============================================================================
// 网络配置常量
// ============================================================================

/// 默认DNS服务器端口
pub const DEFAULT_DNS_PORT: u16 = 53;
/// 默认gRPC服务器端口
pub const DEFAULT_GRPC_PORT: u16 = 50051;
/// 默认性能分析端口
pub const DEFAULT_PROFILE_PORT: u16 = 8080;
/// 默认Kaspa节点端口
pub const DEFAULT_KASPA_PORT: u16 = 16111;
/// 默认测试网端口
pub const DEFAULT_TESTNET_PORT: u16 = 16110;
/// 测试网-11特殊端口
pub const TESTNET_11_PORT: u16 = 16311;

// ============================================================================
// 超时和间隔常量
// ============================================================================

/// DNS服务器读取超时
pub const DNS_READ_TIMEOUT: Duration = Duration::from_secs(1);
/// 爬虫休眠间隔
pub const CRAWLER_SLEEP_INTERVAL: Duration = Duration::from_secs(10);
/// 最大并发轮询数量
pub const MAX_CONCURRENT_POLLS: usize = 100;
/// 批处理大小范围
pub const MIN_BATCH_SIZE: usize = 20;
pub const MAX_BATCH_SIZE: usize = 50;
/// 连接超时
pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
/// 地址等待超时
pub const ADDRESS_WAIT_TIMEOUT: Duration = Duration::from_secs(30);
/// Ping间隔
pub const PING_INTERVAL: Duration = Duration::from_secs(60);
/// 健康检查间隔
pub const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);
/// 性能指标收集间隔
pub const PERFORMANCE_COLLECTION_INTERVAL: Duration = Duration::from_secs(10);
/// 性能分析更新间隔
pub const PROFILING_UPDATE_INTERVAL: Duration = Duration::from_secs(5);

// ============================================================================
// 地址管理常量
// ============================================================================

/// 节点过期超时（良好节点）
pub const STALE_GOOD_TIMEOUT: Duration = Duration::from_secs(60 * 60); // 1小时
/// 节点过期超时（不良节点）
pub const STALE_BAD_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60); // 2小时
/// 地址清理过期超时
pub const PRUNE_EXPIRE_TIMEOUT: Duration = Duration::from_secs(8 * 60 * 60); // 8小时
/// 地址清理间隔
pub const PRUNE_ADDRESS_INTERVAL: Duration = Duration::from_secs(60 * 60); // 1小时
/// 地址转储间隔
pub const DUMP_ADDRESS_INTERVAL: Duration = Duration::from_secs(10 * 60); // 10分钟

// ============================================================================
// DNS相关常量
// ============================================================================

/// DNS缓冲区大小
pub const DNS_BUFFER_SIZE: usize = 512;
/// 最大DNS记录数量
pub const MAX_DNS_RECORDS: usize = 8;
/// DNS记录TTL
pub const DNS_RECORD_TTL: u32 = 30;
/// DNS SOA记录TTL
pub const DNS_SOA_TTL: u32 = 86400;
/// IPv6占位符地址
pub const IPV6_PLACEHOLDER: [u16; 8] = [0x100, 0, 0, 0, 0, 0, 0, 0];

// ============================================================================
// 性能监控常量
// ============================================================================

/// CPU使用率警告阈值
pub const CPU_WARNING_THRESHOLD: f64 = 90.0;
/// CPU使用率注意阈值
pub const CPU_NOTICE_THRESHOLD: f64 = 70.0;
/// 内存使用警告阈值（1GB）
pub const MEMORY_WARNING_THRESHOLD: u64 = 1024 * 1024 * 1024;
/// 响应时间警告阈值（5秒）
pub const RESPONSE_TIME_WARNING_THRESHOLD: f64 = 5000.0;
/// 响应时间注意阈值（2秒）
pub const RESPONSE_TIME_NOTICE_THRESHOLD: f64 = 2000.0;

// ============================================================================
// 网络适配器常量
// ============================================================================

/// 最大重试次数
pub const MAX_RETRY_COUNT: u32 = 3;
/// 基础重试延迟
pub const BASE_RETRY_DELAY: Duration = Duration::from_secs(1);
/// 地址通道缓冲区大小
pub const ADDRESS_CHANNEL_BUFFER: usize = 100;
/// 指数退避基数
pub const EXPONENTIAL_BACKOFF_BASE: u32 = 2;

// ============================================================================
// 配置验证常量
// ============================================================================

/// 最大线程数
pub const MAX_THREADS: u8 = 64;
/// 最大测试网后缀
pub const MAX_TESTNET_SUFFIX: u16 = 99;
/// 最大协议版本
pub const MAX_PROTOCOL_VERSION: u32 = 100;
