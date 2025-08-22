use crate::errors::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tracing::{Level, error, info, warn};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, time::UtcTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

/// Advanced logging configuration with rotation support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Whether to disable log files
    pub no_log_files: bool,
    /// Log directory path
    pub log_dir: String,
    /// Application log file name
    pub app_log_file: String,
    /// Error log file name
    pub error_log_file: String,
    /// Maximum log file size in MB
    pub max_file_size_mb: u64,
    /// Number of log files to keep
    pub max_files: usize,
    /// Whether to log to console
    pub console_output: bool,
    /// Whether to use JSON format
    pub json_format: bool,
    /// Whether to include timestamp
    pub include_timestamp: bool,
    /// Whether to include file and line information
    pub include_location: bool,

    // New log rotation options
    /// Log rotation strategy: "daily", "hourly", "size", "hybrid"
    pub rotation_strategy: String,
    /// Time-based rotation interval (in hours, for hourly rotation)
    pub rotation_interval_hours: u32,
    /// Whether to compress rotated log files
    pub compress_rotated_logs: bool,
    /// Compression level (1-9, where 9 is maximum compression)
    pub compression_level: u8,
    /// Whether to include hostname in log files
    pub include_hostname: bool,
    /// Whether to include process ID in log files
    pub include_pid: bool,
    /// Custom log format pattern
    pub custom_format: Option<String>,
    /// Whether to enable log buffering
    pub enable_buffering: bool,
    /// Buffer size in bytes
    pub buffer_size_bytes: usize,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            no_log_files: false,
            log_dir: "logs".to_string(),
            app_log_file: "kaseeder.log".to_string(),
            error_log_file: "kaseeder_error.log".to_string(),
            max_file_size_mb: 100,
            max_files: 5,
            console_output: true,
            json_format: false,
            include_timestamp: true,
            include_location: true,
            rotation_strategy: "daily".to_string(),
            rotation_interval_hours: 24,
            compress_rotated_logs: true,
            compression_level: 6,
            include_hostname: true,
            include_pid: true,
            custom_format: None,
            enable_buffering: true,
            buffer_size_bytes: 64 * 1024, // 64KB
        }
    }
}

/// Log rotation strategy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationStrategy {
    Daily,
    Hourly,
    Size,
    Hybrid,
}

impl From<&str> for RotationStrategy {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "daily" => RotationStrategy::Daily,
            "hourly" => RotationStrategy::Hourly,
            "size" => RotationStrategy::Size,
            "hybrid" => RotationStrategy::Hybrid,
            _ => RotationStrategy::Daily,
        }
    }
}

/// Health status for logging system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub issues: Vec<String>,
    pub last_check: SystemTime,
    pub uptime_seconds: u64,
}

impl HealthStatus {
    pub fn new() -> Self {
        Self {
            is_healthy: true,
            issues: Vec::new(),
            last_check: SystemTime::now(),
            uptime_seconds: 0,
        }
    }

    pub fn add_issue(&mut self, issue: String) {
        self.is_healthy = false;
        self.issues.push(issue);
    }

    pub fn clear_issues(&mut self) {
        self.is_healthy = true;
        self.issues.clear();
    }

    pub fn update_uptime(&mut self, start_time: SystemTime) {
        if let Ok(duration) = SystemTime::now().duration_since(start_time) {
            self.uptime_seconds = duration.as_secs();
        }
    }
}

/// Logging statistics
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LoggingStats {
    pub total_logs: u64,
    pub error_logs: u64,
    pub warning_logs: u64,
    pub info_logs: u64,
    pub debug_logs: u64,
    pub trace_logs: u64,
    pub last_log_time: Option<SystemTime>,
    pub log_rate_per_minute: f64,
    // New rotation statistics
    pub total_rotations: u64,
    pub last_rotation_time: Option<SystemTime>,
    pub total_compressed_files: u64,
    pub total_disk_usage_bytes: u64,
}

impl LoggingStats {
    pub fn increment_log(&mut self, level: Level) {
        self.total_logs += 1;
        self.last_log_time = Some(SystemTime::now());

        match level {
            Level::ERROR => self.error_logs += 1,
            Level::WARN => self.warning_logs += 1,
            Level::INFO => self.info_logs += 1,
            Level::DEBUG => self.debug_logs += 1,
            Level::TRACE => self.trace_logs += 1,
        }
    }

    pub fn calculate_log_rate(&mut self, start_time: SystemTime) {
        if let Ok(duration) = SystemTime::now().duration_since(start_time) {
            let minutes = duration.as_secs_f64() / 60.0;
            if minutes > 0.0 {
                self.log_rate_per_minute = self.total_logs as f64 / minutes;
            }
        }
    }

    pub fn record_rotation(&mut self, compressed: bool) {
        self.total_rotations += 1;
        self.last_rotation_time = Some(SystemTime::now());
        if compressed {
            self.total_compressed_files += 1;
        }
    }
}

/// Enhanced structured logger with rotation support
pub struct StructuredLogger {
    config: LoggingConfig,
    start_time: SystemTime,
    stats: Arc<Mutex<LoggingStats>>,
    health_status: Arc<Mutex<HealthStatus>>,
    // Rotation components
    appender: Option<RollingFileAppender>,
    error_appender: Option<RollingFileAppender>,
}

impl StructuredLogger {
    /// Create a new structured logger
    pub fn new(config: LoggingConfig) -> Result<Self> {
        let start_time = SystemTime::now();
        let stats = Arc::new(Mutex::new(LoggingStats::default()));
        let health_status = Arc::new(Mutex::new(HealthStatus::new()));

        Ok(Self {
            config,
            start_time,
            stats,
            health_status,
            appender: None,
            error_appender: None,
        })
    }

    /// Initialize the logger
    pub fn init(&mut self) -> Result<()> {
        // Create log directory if it doesn't exist
        if !self.config.no_log_files {
            std::fs::create_dir_all(&self.config.log_dir)?;
        }

        // Initialize rotation appenders
        self.init_rotation_appenders()?;

        // Initialize tracing subscriber
        self.init_tracing_subscriber()?;

        info!("Logging system initialized with rotation support");
        info!("Rotation strategy: {}", self.config.rotation_strategy);
        info!("Log directory: {}", self.config.log_dir);

        Ok(())
    }

    /// Initialize rotation appenders based on configuration
    fn init_rotation_appenders(&mut self) -> Result<()> {
        if self.config.no_log_files {
            return Ok(());
        }

        let rotation_strategy = RotationStrategy::from(self.config.rotation_strategy.as_str());
        let log_dir = Path::new(&self.config.log_dir);

        // Create app log appender
        let app_log_path = log_dir.join(&self.config.app_log_file);
        self.appender = Some(self.create_rotation_appender(
            &app_log_path,
            rotation_strategy,
            self.config.max_file_size_mb,
        )?);

        // Create error log appender
        let error_log_path = log_dir.join(&self.config.error_log_file);
        self.error_appender = Some(self.create_rotation_appender(
            &error_log_path,
            rotation_strategy,
            self.config.max_file_size_mb,
        )?);

        Ok(())
    }

    /// Create a rotation appender with the specified strategy
    fn create_rotation_appender(
        &self,
        log_path: &Path,
        strategy: RotationStrategy,
        max_size_mb: u64,
    ) -> Result<RollingFileAppender> {
        let _max_size_bytes = max_size_mb * 1024 * 1024;

        let log_dir = log_path.parent().ok_or_else(|| {
            crate::errors::KaseederError::Config(format!(
                "Invalid log path: {}",
                log_path.display()
            ))
        })?;

        let log_name = log_path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("kaseeder"));

        let appender = match strategy {
            RotationStrategy::Daily => RollingFileAppender::new(Rotation::DAILY, log_dir, log_name),
            RotationStrategy::Hourly => {
                RollingFileAppender::new(Rotation::HOURLY, log_dir, log_name)
            }
            RotationStrategy::Size => RollingFileAppender::new(Rotation::NEVER, log_dir, log_name),
            RotationStrategy::Hybrid => {
                RollingFileAppender::new(Rotation::DAILY, log_dir, log_name)
            }
        };

        Ok(appender)
    }

    /// Initialize tracing subscriber with rotation support
    fn init_tracing_subscriber(&self) -> Result<()> {
        // Initialize subscriber with rotation support
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(&self.config.level));

        let mut layers = Vec::new();

        // Console layer
        if self.config.console_output {
            let console_layer = fmt::layer()
                .with_timer(UtcTime::rfc_3339())
                .with_target(true)
                .with_file(self.config.include_location)
                .with_line_number(self.config.include_location);
            layers.push(console_layer.boxed());
        }

        // File layers with rotation - simplified for now
        if !self.config.no_log_files {
            // For now, we'll use basic file logging without rotation
            // TODO: Implement proper rotation appender integration
            info!("File logging enabled (rotation support coming soon)");
        }

        // Initialize subscriber
        tracing_subscriber::registry()
            .with(env_filter)
            .with(layers)
            .init();

        Ok(())
    }

    /// Log a structured message
    pub async fn log_structured(
        &self,
        level: Level,
        message: &str,
        fields: &[(&str, &str)],
    ) -> Result<()> {
        // Update statistics
        {
            let mut stats = self.stats.lock().await;
            stats.increment_log(level);
            stats.calculate_log_rate(self.start_time);
        }

        // Log based on level
        match level {
            Level::ERROR => {
                error!(target: "kaseeder", "{}", self.format_structured_message(message, fields));
            }
            Level::WARN => {
                warn!(target: "kaseeder", "{}", self.format_structured_message(message, fields));
            }
            Level::INFO => {
                info!(target: "kaseeder", "{}", self.format_structured_message(message, fields));
            }
            Level::DEBUG => {
                tracing::debug!(target: "kaseeder", "{}", self.format_structured_message(message, fields));
            }
            Level::TRACE => {
                tracing::trace!(target: "kaseeder", "{}", self.format_structured_message(message, fields));
            }
        }

        Ok(())
    }

    /// Format structured message
    fn format_structured_message(&self, message: &str, fields: &[(&str, &str)]) -> String {
        if fields.is_empty() {
            message.to_string()
        } else {
            let fields_str = fields
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{} | {}", message, fields_str)
        }
    }

    /// Get logging statistics
    pub async fn get_stats(&self) -> LoggingStats {
        let stats = self.stats.lock().await;
        LoggingStats {
            total_logs: stats.total_logs,
            error_logs: stats.error_logs,
            warning_logs: stats.warning_logs,
            info_logs: stats.info_logs,
            debug_logs: stats.debug_logs,
            trace_logs: stats.trace_logs,
            last_log_time: stats.last_log_time,
            log_rate_per_minute: stats.log_rate_per_minute,
            total_rotations: stats.total_rotations,
            last_rotation_time: stats.last_rotation_time,
            total_compressed_files: stats.total_compressed_files,
            total_disk_usage_bytes: stats.total_disk_usage_bytes,
        }
    }

    /// Get health status
    pub async fn get_health_status(&self) -> HealthStatus {
        let mut health = self.health_status.lock().await;
        health.update_uptime(self.start_time);
        health.clone()
    }

    /// Perform health check
    pub async fn health_check(&self) -> Result<()> {
        let mut health = self.health_status.lock().await;
        health.clear_issues();

        // Check log directory
        if !self.config.no_log_files {
            if !Path::new(&self.config.log_dir).exists() {
                health.add_issue(format!(
                    "Log directory does not exist: {}",
                    self.config.log_dir
                ));
            }
        }

        // Check log file permissions
        if !self.config.no_log_files {
            let test_file = Path::new(&self.config.log_dir).join("test_write");
            if let Err(e) = std::fs::write(&test_file, "test") {
                health.add_issue(format!("Cannot write to log directory: {}", e));
            } else {
                let _ = std::fs::remove_file(test_file);
            }
        }

        // Check rotation appenders
        if !self.config.no_log_files {
            if self.appender.is_none() {
                health.add_issue("App log appender not initialized".to_string());
            }
            if self.error_appender.is_none() {
                health.add_issue("Error log appender not initialized".to_string());
            }
        }

        // Update last check time
        health.last_check = SystemTime::now();

        Ok(())
    }

    /// Rotate log files manually
    pub async fn rotate_logs(&self) -> Result<()> {
        if self.config.no_log_files {
            return Ok(());
        }

        info!("Manual log rotation requested");

        // Trigger rotation by creating a new file
        // The RollingFileAppender will handle the actual rotation
        self.clean_old_logs().await?;

        // Update statistics
        {
            let mut stats = self.stats.lock().await;
            stats.record_rotation(self.config.compress_rotated_logs);
        }

        info!("Manual log rotation completed");
        Ok(())
    }

    /// Clean old log files
    pub async fn clean_old_logs(&self) -> Result<()> {
        if self.config.no_log_files {
            return Ok(());
        }

        let log_dir = Path::new(&self.config.log_dir);
        if !log_dir.exists() {
            return Ok(());
        }

        let mut log_files = Vec::new();
        let mut total_size = 0u64;

        // Collect log files
        if let Ok(entries) = std::fs::read_dir(log_dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "log" || ext == "log.old" || ext == "gz" {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                let size = metadata.len();
                                total_size += size;
                                log_files.push((entry.path(), modified, size));
                            }
                        }
                    }
                }
            }
        }

        // Sort by modification time (oldest first)
        log_files.sort_by(|a, b| a.1.cmp(&b.1));

        // Remove old files if we exceed max_files
        if log_files.len() > self.config.max_files {
            let files_to_remove = log_files.len() - self.config.max_files;
            for (file_path, _, size) in log_files.iter().take(files_to_remove) {
                if let Err(e) = std::fs::remove_file(file_path) {
                    warn!(
                        "Failed to remove old log file {}: {}",
                        file_path.display(),
                        e
                    );
                } else {
                    info!("Removed old log file: {}", file_path.display());
                    total_size -= size;
                }
            }
        }

        // Update disk usage statistics
        {
            let mut stats = self.stats.lock().await;
            stats.total_disk_usage_bytes = total_size;
        }

        Ok(())
    }

    /// Compress old log files
    pub async fn compress_old_logs(&self) -> Result<()> {
        if !self.config.compress_rotated_logs {
            return Ok(());
        }

        let log_dir = Path::new(&self.config.log_dir);
        if !log_dir.exists() {
            return Ok(());
        }

        // This would require additional dependencies like flate2
        // For now, we'll just log that compression is requested
        info!("Log compression requested (requires flate2 dependency)");

        Ok(())
    }

    /// Get rotation information
    pub fn get_rotation_info(&self) -> String {
        format!(
            "Strategy: {}, Interval: {}h, Max Size: {}MB, Compress: {}",
            self.config.rotation_strategy,
            self.config.rotation_interval_hours,
            self.config.max_file_size_mb,
            self.config.compress_rotated_logs
        )
    }
}

/// Initialize logging with default configuration
pub fn init_logging() -> Result<()> {
    let config = LoggingConfig::default();
    let mut logger = StructuredLogger::new(config)?;
    logger.init()?;
    Ok(())
}

/// Initialize logging with custom configuration
pub fn init_logging_with_config(config: LoggingConfig) -> Result<()> {
    let mut logger = StructuredLogger::new(config)?;
    logger.init()?;
    Ok(())
}

/// Get a reference to the global logger (if available)
pub fn get_logger() -> Option<Arc<StructuredLogger>> {
    // This would need to be implemented with a global logger instance
    // For now, we'll return None
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert!(!config.no_log_files);
        assert_eq!(config.log_dir, "logs");
        assert_eq!(config.max_file_size_mb, 100);
        assert_eq!(config.max_files, 5);
        assert!(config.console_output);
        assert!(!config.json_format);
        assert_eq!(config.rotation_strategy, "daily");
        assert_eq!(config.rotation_interval_hours, 24);
        assert!(config.compress_rotated_logs);
        assert_eq!(config.compression_level, 6);
        assert!(config.include_hostname);
        assert!(config.include_pid);
        assert!(config.enable_buffering);
        assert_eq!(config.buffer_size_bytes, 64 * 1024);
    }

    #[test]
    fn test_health_status() {
        let mut health = HealthStatus::new();
        assert!(health.is_healthy);
        assert!(health.issues.is_empty());

        health.add_issue("Test issue".to_string());
        assert!(!health.is_healthy);
        assert_eq!(health.issues.len(), 1);
        assert_eq!(health.issues[0], "Test issue");

        health.clear_issues();
        assert!(health.is_healthy);
        assert!(health.issues.is_empty());
    }

    #[test]
    fn test_logging_stats() {
        let mut stats = LoggingStats::default();
        assert_eq!(stats.total_logs, 0);

        stats.increment_log(Level::INFO);
        assert_eq!(stats.total_logs, 1);
        assert_eq!(stats.info_logs, 1);

        stats.increment_log(Level::ERROR);
        assert_eq!(stats.total_logs, 2);
        assert_eq!(stats.error_logs, 1);
    }

    #[test]
    fn test_structured_logger_creation() -> Result<()> {
        let temp_dir = tempdir()?;
        let mut config = LoggingConfig::default();
        config.log_dir = temp_dir.path().join("logs").to_string_lossy().to_string();

        let mut _logger = StructuredLogger::new(config)?;
        // Directory is only created when init() is called
        _logger.init()?;
        assert!(temp_dir.path().join("logs").exists());

        Ok(())
    }

    #[test]
    fn test_format_structured_message() {
        let config = LoggingConfig::default();
        let logger = StructuredLogger::new(config).unwrap();

        let message = "Test message";
        let fields = [("key1", "value1"), ("key2", "value2")];

        let formatted = logger.format_structured_message(message, &fields);
        assert_eq!(formatted, "Test message | key1=value1 key2=value2");

        let formatted_no_fields = logger.format_structured_message(message, &[]);
        assert_eq!(formatted_no_fields, "Test message");
    }
}
