use anyhow::Result;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{CpuExt, System, SystemExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

/// Performance profiling server
pub struct ProfilingServer {
    port: u16,
    stats: Arc<Mutex<ProfilingStats>>,
    is_running: Arc<Mutex<bool>>,
}

/// Performance statistics
#[derive(Debug, Default)]
pub struct ProfilingStats {
    pub start_time: Option<Instant>,
    pub request_count: u64,
    pub error_count: u64,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
    pub active_connections: u32,
    pub custom_metrics: HashMap<String, f64>,
}

impl ProfilingServer {
    /// Create a new performance profiling server
    pub fn new(port: u16) -> Self {
        Self {
            port,
            stats: Arc::new(Mutex::new(ProfilingStats::default())),
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the performance profiling server
    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.lock().await;
        if *is_running {
            warn!("Profiling server is already running");
            return Ok(());
        }

        *is_running = true;
        drop(is_running);

        let port = self.port;
        let stats = self.stats.clone();
        let is_running = self.is_running.clone();

        // Start the performance profiling server
        tokio::spawn(async move {
            if let Err(e) = Self::run_server(port, stats, is_running).await {
                error!("Profiling server error: {}", e);
            }
        });

        info!("Profiling server started on port {}", self.port);
        Ok(())
    }

    /// Run the performance profiling server
    async fn run_server(
        port: u16,
        stats: Arc<Mutex<ProfilingStats>>,
        is_running: Arc<Mutex<bool>>,
    ) -> Result<()> {
        let addr = format!("0.0.0.0:{}", port).parse::<SocketAddr>()?;
        let listener = TcpListener::bind(addr).await?;

        info!("Profiling server listening on {}", addr);

        // Initialize statistics
        {
            let mut stats_guard = stats.lock().await;
            stats_guard.start_time = Some(Instant::now());
        }

        // Start statistics update task
        let stats_clone = stats.clone();
        tokio::spawn(async move {
            Self::update_stats_periodically(stats_clone).await;
        });

        loop {
            // Check if should stop
            {
                let running = is_running.lock().await;
                if !*running {
                    break;
                }
            }

            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((socket, addr)) => {
                            let stats = stats.clone();
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(socket, addr, stats).await {
                                    error!("Connection handling error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Accept error: {}", e);
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Periodically check stop signal
                }
            }
        }

        info!("Profiling server stopped");
        Ok(())
    }

    /// Handle connection
    async fn handle_connection(
        mut socket: tokio::net::TcpStream,
        addr: SocketAddr,
        stats: Arc<Mutex<ProfilingStats>>,
    ) -> Result<()> {
        // Update active connection count
        {
            let mut stats_guard = stats.lock().await;
            stats_guard.active_connections += 1;
            stats_guard.request_count += 1;
        }

        // Simple HTTP response
        let response = Self::generate_profiling_response(&stats).await;

        if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut socket, response.as_bytes()).await
        {
            error!("Failed to write response to {}: {}", addr, e);

            // Update error count
            let mut stats_guard = stats.lock().await;
            stats_guard.error_count += 1;
        }

        // Update active connection count
        {
            let mut stats_guard = stats.lock().await;
            stats_guard.active_connections = stats_guard.active_connections.saturating_sub(1);
        }

        Ok(())
    }

    /// Generate performance profiling response
    async fn generate_profiling_response(stats: &Arc<Mutex<ProfilingStats>>) -> String {
        let stats_guard = stats.lock().await;

        let uptime = stats_guard
            .start_time
            .map(|start| {
                let duration = start.elapsed();
                format!("{}s", duration.as_secs())
            })
            .unwrap_or_else(|| "unknown".to_string());

        let html = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <title>DNSSeeder Profiling</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .metric {{ margin: 10px 0; padding: 10px; background: #f5f5f5; border-radius: 5px; }}
        .metric h3 {{ margin: 0 0 10px 0; color: #333; }}
        .metric .value {{ font-size: 18px; font-weight: bold; color: #007acc; }}
        .metric .description {{ color: #666; font-size: 14px; }}
    </style>
</head>
<body>
    <h1>DNSSeeder Performance Metrics</h1>
    
    <div class="metric">
        <h3>Uptime</h3>
        <div class="value">{}</div>
        <div class="description">Server running time</div>
    </div>
    
    <div class="metric">
        <h3>Total Requests</h3>
        <div class="value">{}</div>
        <div class="description">Total profiling requests received</div>
    </div>
    
    <div class="metric">
        <h3>Active Connections</h3>
        <div class="value">{}</div>
        <div class="description">Current active connections</div>
    </div>
    
    <div class="metric">
        <h3>Memory Usage</h3>
        <div class="value">{:.2} MB</div>
        <div class="description">Current memory usage</div>
    </div>
    
    <div class="metric">
        <h3>Error Rate</h3>
        <div class="value">{:.2}%</div>
        <div class="description">Error rate based on total requests</div>
    </div>
    
    <div class="metric">
        <h3>Custom Metrics</h3>
        <div class="value">{}</div>
        <div class="description">Number of custom metrics tracked</div>
    </div>
    
    <p><em>Last updated: {}</em></p>
</body>
</html>
"#,
            uptime,
            stats_guard.request_count,
            stats_guard.active_connections,
            stats_guard.memory_usage_bytes as f64 / 1024.0 / 1024.0,
            if stats_guard.request_count > 0 {
                (stats_guard.error_count as f64 / stats_guard.request_count as f64) * 100.0
            } else {
                0.0
            },
            stats_guard.custom_metrics.len(),
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
            html.len(),
            html
        )
    }

    /// Periodically update statistics
    async fn update_stats_periodically(stats: Arc<Mutex<ProfilingStats>>) {
        let mut interval = tokio::time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;

            let mut stats_guard = stats.lock().await;

            // Update memory usage
            let mut system = System::new_all();
            stats_guard.memory_usage_bytes = system.used_memory();

            // Update CPU usage
            system.refresh_cpu();
            let cpu_usage: f64 =
                system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() as f64;
            stats_guard.cpu_usage_percent = cpu_usage / system.cpus().len() as f64;
        }
    }

    /// Add custom metric
    pub async fn add_custom_metric(&self, name: String, value: f64) {
        let mut stats = self.stats.lock().await;
        stats.custom_metrics.insert(name, value);
    }

    /// Get statistics
    pub async fn get_stats(&self) -> ProfilingStats {
        self.stats.lock().await.clone()
    }

    /// Stop the performance profiling server
    pub async fn stop(&self) -> Result<()> {
        let mut is_running = self.is_running.lock().await;
        *is_running = false;
        info!("Profiling server stop signal sent");
        Ok(())
    }
}

impl Clone for ProfilingServer {
    fn clone(&self) -> Self {
        Self {
            port: self.port,
            stats: self.stats.clone(),
            is_running: self.is_running.clone(),
        }
    }
}

impl ProfilingStats {
    /// Clone statistics
    pub fn clone(&self) -> Self {
        Self {
            start_time: self.start_time,
            request_count: self.request_count,
            error_count: self.error_count,
            memory_usage_bytes: self.memory_usage_bytes,
            cpu_usage_percent: self.cpu_usage_percent,
            active_connections: self.active_connections,
            custom_metrics: self.custom_metrics.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_profiling_server_creation() {
        let server = ProfilingServer::new(8080);
        assert_eq!(server.port, 8080);
    }

    #[tokio::test]
    async fn test_custom_metrics() {
        let server = ProfilingServer::new(8081);
        server
            .add_custom_metric("test_metric".to_string(), 42.0)
            .await;

        let stats = server.get_stats().await;
        assert_eq!(stats.custom_metrics.get("test_metric"), Some(&42.0));
    }
}
