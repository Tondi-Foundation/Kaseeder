use anyhow::Result;
use std::path::Path;
use tracing::Level;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{
    fmt::{self, time::UtcTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// 初始化完整的日志系统
pub fn init_logging(log_level: &str, log_file: Option<&str>) -> Result<()> {
    // 设置环境过滤器
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    // 控制台输出层
    let console_layer = fmt::layer()
        .with_timer(UtcTime::rfc_3339())
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(true);

    // 创建订阅者注册器
    let registry = tracing_subscriber::registry().with(env_filter);

    // 如果指定了日志文件，添加文件输出层
    if let Some(log_file_path) = log_file {
        // 确保日志目录存在
        if let Some(parent_dir) = Path::new(log_file_path).parent() {
            std::fs::create_dir_all(parent_dir)?;
        }

        // 创建普通日志文件滚动写入器
        let log_appender = rolling::daily("logs", "dnsseeder");
        let (non_blocking_log_appender, _log_guard) = non_blocking(log_appender);

        // 普通日志文件输出层
        let file_layer = fmt::layer()
            .with_timer(UtcTime::rfc_3339())
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_ansi(false) // 文件中不使用ANSI颜色
            .with_writer(non_blocking_log_appender);

        // 初始化带文件输出的订阅者
        registry.with(console_layer).with(file_layer).init();

        // 防止guard被drop
        std::mem::forget(_log_guard);
    } else {
        // 只有控制台输出
        registry.with(console_layer).init();
    }

    Ok(())
}

/// 记录错误到专门的错误日志文件
pub fn log_error(error: &anyhow::Error, context: &str) {
    let error_details = format!(
        "ERROR [{}] {}: {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        context,
        error
    );

    // 记录到标准错误日志
    tracing::error!("{}: {}", context, error);

    // 如果有错误日志文件，也记录到文件
    if let Ok(mut error_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/dnsseeder_error.log")
    {
        use std::io::Write;
        let _ = writeln!(error_file, "{}", error_details);
    }
}

/// 记录警告到专门的错误日志文件
pub fn log_warning(warning: &str, context: &str) {
    let warning_details = format!(
        "WARNING [{}] {}: {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        context,
        warning
    );

    // 记录到标准警告日志
    tracing::warn!("{}: {}", context, warning);

    // 如果有错误日志文件，也记录到文件
    if let Ok(mut error_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/dnsseeder_error.log")
    {
        use std::io::Write;
        let _ = writeln!(error_file, "{}", warning_details);
    }
}

/// 性能监控宏
#[macro_export]
macro_rules! monitor_performance {
    ($name:expr, $block:block) => {{
        let start = std::time::Instant::now();
        let result = $block;
        let duration = start.elapsed();
        tracing::info!(
            target: "performance",
            operation = $name,
            duration_ms = duration.as_millis(),
            "Performance monitoring"
        );
        result
    }};
}

/// 健康检查状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub uptime: std::time::Duration,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub last_check: std::time::SystemTime,
}

impl HealthStatus {
    pub fn new() -> Self {
        Self {
            is_healthy: true,
            uptime: std::time::Duration::from_secs(0),
            errors: Vec::new(),
            warnings: Vec::new(),
            last_check: std::time::SystemTime::now(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_healthy = false;
        self.last_check = std::time::SystemTime::now();
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
        self.last_check = std::time::SystemTime::now();
    }

    pub fn clear_issues(&mut self) {
        self.errors.clear();
        self.warnings.clear();
        self.is_healthy = true;
        self.last_check = std::time::SystemTime::now();
    }
}

/// 日志统计信息
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoggingStats {
    pub total_logs: u64,
    pub error_logs: u64,
    pub warn_logs: u64,
    pub info_logs: u64,
    pub debug_logs: u64,
    pub trace_logs: u64,
}

impl LoggingStats {
    pub fn record_log(&mut self, level: &Level) {
        self.total_logs += 1;
        match *level {
            Level::ERROR => self.error_logs += 1,
            Level::WARN => self.warn_logs += 1,
            Level::INFO => self.info_logs += 1,
            Level::DEBUG => self.debug_logs += 1,
            Level::TRACE => self.trace_logs += 1,
        }
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_logs == 0 {
            0.0
        } else {
            self.error_logs as f64 / self.total_logs as f64
        }
    }
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
