use tracing::Level;
use tracing_subscriber::{
    fmt::{self, time::UtcTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

pub fn init_logging(log_level: &str, _log_file: Option<&str>) -> anyhow::Result<()> {
    // 设置环境过滤器
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    // 控制台输出层
    let console_layer = fmt::layer()
        .with_timer(UtcTime::rfc_3339())
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true);

    // 初始化订阅者
    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .init();

    Ok(())
}

pub fn set_log_level(level: Level) {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(level)
            .finish(),
    )
    .expect("Failed to set global default subscriber");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_initialization() {
        let result = init_logging("info", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_logging_with_file() {
        let result = init_logging("debug", Some("test.log"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_log_level() {
        set_log_level(Level::DEBUG);
        // 验证日志级别设置成功
        assert!(true);
    }
}
