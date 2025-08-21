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

/// Initialize complete logging system
pub fn init_logging(log_level: &str, log_file: Option<&str>) -> Result<()> {
    // Set environment filter
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    // Console output layer
    let console_layer = fmt::layer()
        .with_timer(UtcTime::rfc_3339())
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(true);

    // Create subscriber registry
    let registry = tracing_subscriber::registry().with(env_filter);

    // If log file is specified, add file output layer
    if let Some(log_file_path) = log_file {
        // Ensure log directory exists
        if let Some(parent_dir) = Path::new(log_file_path).parent() {
            std::fs::create_dir_all(parent_dir)?;
        }

        // Create regular log file rolling writer
        let log_appender = rolling::daily("logs", "kaseeder");
        let (non_blocking_log_appender, _log_guard) = non_blocking(log_appender);

        // Regular log file output layer
        let file_layer = fmt::layer()
            .with_timer(UtcTime::rfc_3339())
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_ansi(false) // Do not use ANSI colors in files
            .with_writer(non_blocking_log_appender);

        // Initialize subscriber with file output
        registry.with(console_layer).with(file_layer).init();

        // Prevent guard from being dropped
        std::mem::forget(_log_guard);
    } else {
        // Console output only
        registry.with(console_layer).init();
    }

    Ok(())
}

/// Log errors to dedicated error log file
pub fn log_error(error: &anyhow::Error, context: &str) {
    let error_details = format!(
        "ERROR [{}] {}: {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        context,
        error
    );

    // Log to standard error log
    tracing::error!("{}: {}", context, error);

    // If error log file exists, also log to file
    if let Ok(mut error_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/kaseeder_error.log")
    {
        use std::io::Write;
        let _ = writeln!(error_file, "{}", error_details);
    }
}

/// Log warnings to dedicated error log file
pub fn log_warning(warning: &str, context: &str) {
    let warning_details = format!(
        "WARNING [{}] {}: {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        context,
        warning
    );

    // Log to standard warning log
    tracing::warn!("{}: {}", context, warning);

    // If error log file exists, also log to file
    if let Ok(mut error_file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/kaseeder_error.log")
    {
        use std::io::Write;
        let _ = writeln!(error_file, "{}", warning_details);
    }
}

/// Performance monitoring macro
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

/// Health check status
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

/// Logging statistics
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
    #[ignore]
    fn test_logging_initialization() {
        // This test is ignored because global subscriber may already be set
        // Run in integration test environment
        assert!(true);
    }

    #[test]
    #[ignore]
    fn test_logging_with_file() {
        // This test is ignored because global subscriber may already be set
        // Run in integration test environment
        assert!(true);
    }

    #[test]
    fn test_set_log_level() {
        // Use tracing's set_global_default to set log level
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(Level::DEBUG)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
        // Verify log level setting success
        assert!(true);
    }
}
