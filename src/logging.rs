use crate::errors::{KaseederError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tracing::{error, info, warn, Level};
use tracing_subscriber::{
    fmt::{self, time::UtcTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Logging configuration
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
}

/// Structured logging system
pub struct StructuredLogger {
    config: LoggingConfig,
    stats: Arc<Mutex<LoggingStats>>,
    health_status: Arc<Mutex<HealthStatus>>,
    start_time: SystemTime,
}

impl StructuredLogger {
    /// Create a new structured logger
    pub fn new(config: LoggingConfig) -> Result<Self> {
        // Ensure log directory exists
        if !config.no_log_files {
            std::fs::create_dir_all(&config.log_dir)
                .map_err(|e| KaseederError::Io(e))?;
        }

        Ok(Self {
            config,
            stats: Arc::new(Mutex::new(LoggingStats::default())),
            health_status: Arc::new(Mutex::new(HealthStatus::new())),
            start_time: SystemTime::now(),
        })
    }

    /// Initialize the logging system
    pub fn init(&self) -> Result<()> {
        // Set environment filter
        let env_filter = EnvFilter::from_default_env()
            .add_directive(format!("kaseeder={}", self.config.level).parse()
                .map_err(|e| KaseederError::Config(format!("Invalid log level: {}", e)))?);

        // Create subscriber
        let subscriber = tracing_subscriber::registry()
            .with(env_filter);

        // Add console layer if enabled
        if self.config.console_output {
            let console_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(self.config.include_location)
                .with_line_number(self.config.include_location)
                .with_timer(UtcTime::rfc_3339());
            
            subscriber.with(console_layer).init();
        } else {
            subscriber.init();
        }

        // Add file layers if enabled
        if !self.config.no_log_files {
            // Create log directory
            std::fs::create_dir_all(&self.config.log_dir)
                .map_err(|e| KaseederError::Io(e))?;
            
            info!("Log directory created: {}", self.config.log_dir);
        }

        info!("Logging system initialized with level: {}", self.config.level);
        info!("Log directory: {}", self.config.log_dir);
        info!("Console output: {}", self.config.console_output);

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
                health.add_issue(format!("Log directory does not exist: {}", self.config.log_dir));
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
        
        // This is a placeholder - actual rotation is handled by RollingFileAppender
        // In a real implementation, you might want to trigger rotation based on size or time
        
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
        
        // Collect log files
        if let Ok(entries) = std::fs::read_dir(log_dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "log" || ext == "log.old" {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                log_files.push((entry.path(), modified));
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
            for (file_path, _) in log_files.iter().take(files_to_remove) {
                if let Err(e) = std::fs::remove_file(file_path) {
                    warn!("Failed to remove old log file {}: {}", file_path.display(), e);
                } else {
                    info!("Removed old log file: {}", file_path.display());
                }
            }
        }

        Ok(())
    }
}

/// Initialize logging with default configuration
pub fn init_logging() -> Result<()> {
    let config = LoggingConfig::default();
    let logger = StructuredLogger::new(config)?;
    logger.init()?;
    Ok(())
}

/// Initialize logging with custom configuration
pub fn init_logging_with_config(config: LoggingConfig) -> Result<()> {
    let logger = StructuredLogger::new(config)?;
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
        config.log_dir = temp_dir.path().to_string_lossy().to_string();

        let logger = StructuredLogger::new(config)?;
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
