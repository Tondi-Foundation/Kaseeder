use tracing::{Level, Subscriber};
use tracing_subscriber::{
    fmt::{self, time::UtcTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

pub fn init_logging(log_level: &str, enable_file_logging: bool) -> anyhow::Result<()> {
    // 解析日志级别
    let level = match log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    // 创建环境过滤器
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("dnsseeder={}", level)));

    // 创建控制台格式化器
    let console_layer = fmt::layer()
        .with_timer(UtcTime::rfc_3339())
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true);

    // 创建文件格式化器（如果启用）
    let file_layer = if enable_file_logging {
        let file_appender = tracing_appender::rolling::daily("logs", "dnsseeder.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        
        Some(
            fmt::layer()
                .with_timer(UtcTime::rfc_3339())
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true)
                .with_writer(non_blocking)
        )
    } else {
        None
    };

    // 创建错误日志文件
    let error_file_appender = tracing_appender::rolling::daily("logs", "dnsseeder_error.log");
    let (error_non_blocking, _error_guard) = tracing_appender::non_blocking(error_file_appender);
    
    let error_layer = fmt::layer()
        .with_timer(UtcTime::rfc_3339())
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_writer(error_non_blocking)
        .with_filter(tracing::metadata::LevelFilter::ERROR);

    // 初始化订阅者
    let mut subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer);

    if let Some(file_layer) = file_layer {
        subscriber = subscriber.with(file_layer);
    }

    subscriber = subscriber.with(error_layer);

    subscriber.init();

    tracing::info!("Logging initialized with level: {}", level);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_parsing() {
        // 测试各种日志级别解析
        assert!(init_logging("info", false).is_ok());
        assert!(init_logging("debug", false).is_ok());
        assert!(init_logging("warn", false).is_ok());
        assert!(init_logging("error", false).is_ok());
        assert!(init_logging("trace", false).is_ok());
        
        // 测试无效级别（应该默认为info）
        assert!(init_logging("invalid", false).is_ok());
    }
}
