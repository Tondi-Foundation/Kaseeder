use crate::errors::Result;
use crate::logging::{HealthStatus, LoggingStats};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tracing::{error, info};

/// System monitor
pub struct SystemMonitor {
    start_time: SystemTime,
    health_status: Arc<Mutex<HealthStatus>>,
    logging_stats: Arc<Mutex<LoggingStats>>,
    performance_metrics: Arc<Mutex<PerformanceMetrics>>,
}

/// Performance metrics
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct PerformanceMetrics {
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub network_connections: u32,
    pub dns_queries_per_second: f64,
    pub grpc_requests_per_second: f64,
    pub peer_connections: u32,
    pub avg_response_time_ms: f64,
    pub last_updated: Option<SystemTime>,
}

/// System status report
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatusReport {
    pub uptime_seconds: u64,
    pub health: HealthStatus,
    pub performance: PerformanceMetrics,
    pub logging_stats: LoggingStats,
    pub timestamp: SystemTime,
}

impl SystemMonitor {
    /// Create a new system monitor
    pub fn new() -> Self {
        Self {
            start_time: SystemTime::now(),
            health_status: Arc::new(Mutex::new(HealthStatus::new())),
            logging_stats: Arc::new(Mutex::new(LoggingStats::default())),
            performance_metrics: Arc::new(Mutex::new(PerformanceMetrics::default())),
        }
    }

    /// Start monitoring
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Starting system monitoring");

        let health_status = self.health_status.clone();
        let performance_metrics = self.performance_metrics.clone();

        // Start periodic health checks
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;

                if let Err(e) =
                    Self::perform_health_check(health_status.clone(), performance_metrics.clone())
                        .await
                {
                    error!("Health check failed: {}", e);
                }
            }
        });

        // Start performance metrics collection
        let performance_metrics = self.performance_metrics.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;

                if let Err(e) = Self::collect_performance_metrics(performance_metrics.clone()).await
                {
                    error!("Performance metrics collection failed: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Perform health check
    async fn perform_health_check(
        health_status: Arc<Mutex<HealthStatus>>,
        performance_metrics: Arc<Mutex<PerformanceMetrics>>,
    ) -> Result<()> {
        let mut health = health_status.lock().await;
        let metrics = performance_metrics.lock().await;

        // Clear old issues
        health.clear_issues();

        // Check CPU usage
        if metrics.cpu_usage > 80.0 {
            health.add_issue("High CPU usage detected".to_string());
        } else if metrics.cpu_usage > 60.0 {
            health.add_issue("Elevated CPU usage detected".to_string());
        }

        // Check memory usage
        if metrics.memory_usage > 1024 * 1024 * 1024 { // 1GB
            health.add_issue("High memory usage detected".to_string());
        }

        // Check response time
        if metrics.avg_response_time_ms > 1000.0 {
            health.add_issue("High response time detected".to_string());
        } else if metrics.avg_response_time_ms > 500.0 {
            health.add_issue("Elevated response time detected".to_string());
        }

        // Log health status
        info!(
            "Health check completed. Issues: {}, Uptime: {}s",
            health.issues.len(),
            health.uptime_seconds
        );

        Ok(())
    }

    /// Collect performance metrics
    async fn collect_performance_metrics(
        performance_metrics: Arc<Mutex<PerformanceMetrics>>,
    ) -> Result<()> {
        let mut metrics = performance_metrics.lock().await;

        // Simplified performance metrics collection (should use system API in practice)
        metrics.cpu_usage = Self::get_cpu_usage().await?;
        metrics.memory_usage = Self::get_memory_usage().await?;
        metrics.network_connections = Self::get_network_connections().await?;
        metrics.last_updated = Some(SystemTime::now());

        Ok(())
    }

    /// Get CPU usage
    async fn get_cpu_usage() -> Result<f64> {
        // Simplified implementation, should read /proc/stat or use system API in practice
        Ok(rand::random::<f64>() * 50.0) // Simulate 0-50% CPU usage
    }

    /// Get memory usage
    async fn get_memory_usage() -> Result<u64> {
        // Simplified implementation, should read /proc/meminfo or use system API in practice
        Ok(1024 * 1024 * 512) // Simulate 512MB memory usage
    }

    /// Get network connection count
    async fn get_network_connections() -> Result<u32> {
        // Simplified implementation, should read /proc/net/tcp or use system API in practice
        Ok(rand::random::<u32>() % 100)
    }

    /// Update DNS query statistics
    pub async fn record_dns_query(&self, response_time: Duration) {
        let mut metrics = self.performance_metrics.lock().await;

        // Simplified moving average calculation
        let response_time_ms = response_time.as_millis() as f64;
        if metrics.avg_response_time_ms == 0.0 {
            metrics.avg_response_time_ms = response_time_ms;
        } else {
            metrics.avg_response_time_ms =
                (metrics.avg_response_time_ms * 0.9) + (response_time_ms * 0.1);
        }

        // Update QPS (simplified implementation)
        metrics.dns_queries_per_second = metrics.dns_queries_per_second * 0.9 + 0.1;
    }

    /// Update gRPC request statistics
    pub async fn record_grpc_request(&self, response_time: Duration) {
        let mut metrics = self.performance_metrics.lock().await;

        let response_time_ms = response_time.as_millis() as f64;
        if metrics.avg_response_time_ms == 0.0 {
            metrics.avg_response_time_ms = response_time_ms;
        } else {
            metrics.avg_response_time_ms =
                (metrics.avg_response_time_ms * 0.9) + (response_time_ms * 0.1);
        }

        metrics.grpc_requests_per_second = metrics.grpc_requests_per_second * 0.9 + 0.1;
    }

    /// Get system status report
    pub async fn get_status_report(&self) -> SystemStatusReport {
        let uptime = self.start_time.elapsed().unwrap_or_default();
        let health = self.health_status.lock().await.clone();
        let performance = self.performance_metrics.lock().await.clone();
        let logging_stats = {
            let guard = self.logging_stats.lock().await;
            let stats = LoggingStats {
                total_logs: guard.total_logs,
                error_logs: guard.error_logs,
                warning_logs: guard.warning_logs,
                info_logs: guard.info_logs,
                debug_logs: guard.debug_logs,
                trace_logs: guard.trace_logs,
                last_log_time: guard.last_log_time,
                log_rate_per_minute: guard.log_rate_per_minute,
            };
            stats
        };

        SystemStatusReport {
            uptime_seconds: uptime.as_secs(),
            health,
            performance,
            logging_stats,
            timestamp: SystemTime::now(),
        }
    }

    /// Record log statistics
    pub async fn record_log(&self, level: &tracing::Level) {
        let mut stats = self.logging_stats.lock().await;
        stats.increment_log(*level);
    }

    /// Get health status
    pub async fn is_healthy(&self) -> bool {
        let health = self.health_status.lock().await;
        health.is_healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_monitor_creation() {
        let monitor = SystemMonitor::new();
        assert!(monitor.is_healthy().await);
    }

    #[tokio::test]
    async fn test_status_report() {
        let monitor = SystemMonitor::new();
        let report = monitor.get_status_report().await;
        assert!(report.uptime_seconds < 1); // Should be very short for newly created
    }

    #[tokio::test]
    async fn test_dns_query_recording() {
        let monitor = SystemMonitor::new();
        monitor.record_dns_query(Duration::from_millis(100)).await;

        let metrics = monitor.performance_metrics.lock().await;
        assert!(metrics.avg_response_time_ms > 0.0);
    }
}
