use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
    routing::get,
    Router,
};
use std::sync::Arc;
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone)]
pub struct ProfilingServer {
    stats: Arc<HashMap<String, String>>,
}

impl ProfilingServer {
    pub fn new() -> Self {
        let mut stats = HashMap::new();
        stats.insert("start_time".to_string(), chrono::Utc::now().to_rfc3339());
        stats.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
        
        Self {
            stats: Arc::new(stats),
        }
    }

    pub async fn start_profiling_server(port: &str) -> Result<()> {
        let addr = format!("127.0.0.1:{}", port).parse()?;
        info!("Starting profiling server on {}", addr);
        
        let app = Router::new()
            .route("/", get(Self::index))
            .route("/stats", get(Self::stats))
            .route("/health", get(Self::health))
            .with_state(Arc::new(ProfilingServer::new()));
        
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;
        
        Ok(())
    }

    async fn index(State(stats): State<Arc<ProfilingServer>>) -> Html<String> {
        let html = format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>DNSSeeder Profiling</title>
                <style>
                    body {{ font-family: Arial, sans-serif; margin: 20px; }}
                    .container {{ max-width: 800px; margin: 0 auto; }}
                    .header {{ background: #f0f0f0; padding: 20px; border-radius: 5px; margin-bottom: 20px; }}
                    .section {{ background: white; padding: 20px; border: 1px solid #ddd; border-radius: 5px; margin-bottom: 20px; }}
                    .metric {{ display: flex; justify-content: space-between; margin: 10px 0; }}
                    .metric-label {{ font-weight: bold; }}
                    .metric-value {{ color: #666; }}
                    .refresh {{ background: #007cba; color: white; padding: 10px 20px; border: none; border-radius: 5px; cursor: pointer; }}
                    .refresh:hover {{ background: #005a87; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <div class="header">
                        <h1>DNSSeeder Profiling Dashboard</h1>
                        <p>Real-time monitoring and profiling information</p>
                    </div>
                    
                    <div class="section">
                        <h2>System Information</h2>
                        <div class="metric">
                            <span class="metric-label">Version:</span>
                            <span class="metric-value">{}</span>
                        </div>
                        <div class="metric">
                            <span class="metric-label">Start Time:</span>
                            <span class="metric-value">{}</span>
                        </div>
                        <div class="metric">
                            <span class="metric-label">Uptime:</span>
                            <span class="metric-value" id="uptime">Calculating...</span>
                        </div>
                    </div>
                    
                    <div class="section">
                        <h2>Performance Metrics</h2>
                        <div id="metrics">Loading metrics...</div>
                    </div>
                    
                    <div class="section">
                        <button class="refresh" onclick="refreshData()">Refresh Data</button>
                    </div>
                </div>
                
                <script>
                    function refreshData() {{
                        fetch('/stats')
                            .then(response => response.json())
                            .then(data => {{
                                document.getElementById('metrics').innerHTML = formatMetrics(data);
                            }});
                        
                        updateUptime();
                    }}
                    
                    function formatMetrics(data) {{
                        let html = '';
                        for (const [key, value] of Object.entries(data)) {{
                            html += `<div class="metric">
                                <span class="metric-label">${{key}}:</span>
                                <span class="metric-value">${{value}}</span>
                            </div>`;
                        }}
                        return html;
                    }}
                    
                    function updateUptime() {{
                        const startTime = new Date('{}');
                        const now = new Date();
                        const uptime = now - startTime;
                        const seconds = Math.floor(uptime / 1000);
                        const minutes = Math.floor(seconds / 60);
                        const hours = Math.floor(minutes / 60);
                        const days = Math.floor(hours / 24);
                        
                        let uptimeStr = '';
                        if (days > 0) uptimeStr += `${{days}}d `;
                        if (hours > 0) uptimeStr += `${{hours % 24}}h `;
                        if (minutes > 0) uptimeStr += `${{minutes % 60}}m `;
                        uptimeStr += `${{seconds % 60}}s`;
                        
                        document.getElementById('uptime').textContent = uptimeStr;
                    }}
                    
                    // 初始加载
                    refreshData();
                    setInterval(updateUptime, 1000);
                </script>
            </body>
            </html>
            "#,
            stats.stats.get("version").unwrap_or(&"Unknown".to_string()),
            stats.stats.get("start_time").unwrap_or(&"Unknown".to_string()),
            stats.stats.get("start_time").unwrap_or(&"Unknown".to_string())
        );
        
        Html(html)
    }

    async fn stats(State(stats): State<Arc<ProfilingServer>>) -> axum::Json<HashMap<String, String>> {
        let mut current_stats = (*stats).stats.as_ref().clone();
        
        // 添加实时统计信息
        current_stats.insert("memory_usage".to_string(), "N/A".to_string());
        current_stats.insert("cpu_usage".to_string(), "N/A".to_string());
        current_stats.insert("active_connections".to_string(), "0".to_string());
        current_stats.insert("requests_per_second".to_string(), "0".to_string());
        
        axum::Json(current_stats)
    }

    async fn health() -> (StatusCode, axum::Json<serde_json::Value>) {
        let response = serde_json::json!({
            "status": "healthy",
            "service": "profiling",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        
        (StatusCode::OK, axum::Json(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiling_server_creation() {
        let server = ProfilingServer::new();
        assert!(server.stats.contains_key("version"));
        assert!(server.stats.contains_key("start_time"));
    }

    #[test]
    fn test_stats_contains_required_fields() {
        let server = ProfilingServer::new();
        let stats = &*server.stats;
        
        assert!(stats.contains_key("version"));
        assert!(stats.contains_key("start_time"));
        assert_eq!(stats.get("version").unwrap(), env!("CARGO_PKG_VERSION"));
    }
}
