use crate::logging::{HealthStatus, LoggingStats};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tracing::{error, info};

/// 系统监控器
pub struct SystemMonitor {
    start_time: SystemTime,
    health_status: Arc<Mutex<HealthStatus>>,
    logging_stats: Arc<Mutex<LoggingStats>>,
    performance_metrics: Arc<Mutex<PerformanceMetrics>>,
}

/// 性能指标
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

/// 系统状态报告
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatusReport {
    pub uptime_seconds: u64,
    pub health: HealthStatus,
    pub performance: PerformanceMetrics,
    pub logging_stats: LoggingStats,
    pub timestamp: SystemTime,
}

impl SystemMonitor {
    /// 创建新的系统监控器
    pub fn new() -> Self {
        Self {
            start_time: SystemTime::now(),
            health_status: Arc::new(Mutex::new(HealthStatus::new())),
            logging_stats: Arc::new(Mutex::new(LoggingStats::default())),
            performance_metrics: Arc::new(Mutex::new(PerformanceMetrics::default())),
        }
    }

    /// 启动监控
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Starting system monitoring");

        let health_status = self.health_status.clone();
        let performance_metrics = self.performance_metrics.clone();

        // 启动定期健康检查
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

        // 启动性能指标收集
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

    /// 执行健康检查
    async fn perform_health_check(
        health_status: Arc<Mutex<HealthStatus>>,
        performance_metrics: Arc<Mutex<PerformanceMetrics>>,
    ) -> Result<()> {
        let mut health = health_status.lock().await;
        let metrics = performance_metrics.lock().await;

        // 清除旧的问题
        health.clear_issues();

        // 检查CPU使用率
        if metrics.cpu_usage > 90.0 {
            health.add_error("High CPU usage detected".to_string());
        } else if metrics.cpu_usage > 70.0 {
            health.add_warning("Elevated CPU usage detected".to_string());
        }

        // 检查内存使用
        if metrics.memory_usage > 1024 * 1024 * 1024 {
            // 1GB
            health.add_warning("High memory usage detected".to_string());
        }

        // 检查响应时间
        if metrics.avg_response_time_ms > 5000.0 {
            health.add_error("High response time detected".to_string());
        } else if metrics.avg_response_time_ms > 2000.0 {
            health.add_warning("Elevated response time detected".to_string());
        }

        info!(
            "Health check completed: healthy={}, errors={}, warnings={}",
            health.is_healthy,
            health.errors.len(),
            health.warnings.len()
        );

        Ok(())
    }

    /// 收集性能指标
    async fn collect_performance_metrics(
        performance_metrics: Arc<Mutex<PerformanceMetrics>>,
    ) -> Result<()> {
        let mut metrics = performance_metrics.lock().await;

        // 简化的性能指标收集（实际应该使用系统API）
        metrics.cpu_usage = Self::get_cpu_usage().await?;
        metrics.memory_usage = Self::get_memory_usage().await?;
        metrics.network_connections = Self::get_network_connections().await?;
        metrics.last_updated = Some(SystemTime::now());

        Ok(())
    }

    /// 获取CPU使用率
    async fn get_cpu_usage() -> Result<f64> {
        // 简化实现，实际应该读取/proc/stat或使用系统API
        Ok(rand::random::<f64>() * 50.0) // 模拟0-50%的CPU使用率
    }

    /// 获取内存使用量
    async fn get_memory_usage() -> Result<u64> {
        // 简化实现，实际应该读取/proc/meminfo或使用系统API
        Ok(1024 * 1024 * 512) // 模拟512MB内存使用
    }

    /// 获取网络连接数
    async fn get_network_connections() -> Result<u32> {
        // 简化实现，实际应该读取/proc/net/tcp或使用系统API
        Ok(rand::random::<u32>() % 100)
    }

    /// 更新DNS查询统计
    pub async fn record_dns_query(&self, response_time: Duration) {
        let mut metrics = self.performance_metrics.lock().await;

        // 简化的移动平均计算
        let response_time_ms = response_time.as_millis() as f64;
        if metrics.avg_response_time_ms == 0.0 {
            metrics.avg_response_time_ms = response_time_ms;
        } else {
            metrics.avg_response_time_ms =
                (metrics.avg_response_time_ms * 0.9) + (response_time_ms * 0.1);
        }

        // 更新QPS（简化实现）
        metrics.dns_queries_per_second = metrics.dns_queries_per_second * 0.9 + 0.1;
    }

    /// 更新gRPC请求统计
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

    /// 获取系统状态报告
    pub async fn get_status_report(&self) -> SystemStatusReport {
        let uptime = self.start_time.elapsed().unwrap_or_default();
        let health = self.health_status.lock().await.clone();
        let performance = self.performance_metrics.lock().await.clone();
        let logging_stats = {
            let guard = self.logging_stats.lock().await;
            guard.clone()
        };

        SystemStatusReport {
            uptime_seconds: uptime.as_secs(),
            health,
            performance,
            logging_stats,
            timestamp: SystemTime::now(),
        }
    }

    /// 记录日志统计
    pub async fn record_log(&self, level: &tracing::Level) {
        let mut stats = self.logging_stats.lock().await;
        stats.record_log(level);
    }

    /// 获取健康状态
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
        assert!(report.uptime_seconds < 1); // 刚创建的应该很短
    }

    #[tokio::test]
    async fn test_dns_query_recording() {
        let monitor = SystemMonitor::new();
        monitor.record_dns_query(Duration::from_millis(100)).await;

        let metrics = monitor.performance_metrics.lock().await;
        assert!(metrics.avg_response_time_ms > 0.0);
    }
}
