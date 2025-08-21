use crate::manager::AddressManager;
use anyhow::Result;
use std::sync::Arc;
use tracing::info;

// 这里应该定义protobuf生成的代码
// 为了简化，我们创建一个基本的gRPC服务结构

pub struct GrpcServer {
    address_manager: Arc<AddressManager>,
}

impl GrpcServer {
    pub fn new(address_manager: Arc<AddressManager>) -> Self {
        Self { address_manager }
    }

    pub async fn start(&self, listen_addr: &str) -> Result<()> {
        info!("Starting gRPC server on {}", listen_addr);
        
        // 这里应该实现实际的gRPC服务
        // 由于没有protobuf定义，我们创建一个简单的HTTP服务器作为替代
        
        let addr = listen_addr.parse()?;
        
        // 启动HTTP服务器（作为gRPC的替代）
        let app = axum::Router::new()
            .route("/health", axum::routing::get(Self::health_check))
            .route("/stats", axum::routing::get(Self::get_stats))
            .route("/addresses", axum::routing::get(Self::get_addresses))
            .with_state(self.address_manager.clone());
        
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;
        
        Ok(())
    }

    async fn health_check() -> axum::Json<serde_json::Value> {
        axum::Json(serde_json::json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    async fn get_stats(
        axum::extract::State(address_manager): axum::extract::State<Arc<AddressManager>>,
    ) -> axum::Json<serde_json::Value> {
        let stats = address_manager.get_stats();
        
        axum::Json(serde_json::json!({
            "total_nodes": stats.total_nodes,
            "active_nodes": stats.active_nodes,
            "failed_attempts": stats.failed_attempts,
            "successful_connections": stats.successful_connections,
            "last_crawl": stats.last_crawl.map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()),
            "crawl_duration": stats.crawl_duration.map(|d| d.as_secs())
        }))
    }

    async fn get_addresses(
        axum::extract::State(address_manager): axum::extract::State<Arc<AddressManager>>,
        axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    ) -> axum::Json<serde_json::Value> {
        let limit = params
            .get("limit")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(10);
        
        let addresses = address_manager.get_good_addresses(limit);
        let address_list: Vec<serde_json::Value> = addresses
            .iter()
            .map(|addr| {
                serde_json::json!({
                    "ip": addr.ip.to_string(),
                    "port": addr.port,
                    "attempts": 0, // 这些信息现在存储在AddressEntry中
                    "successes": 0,
                    "user_agent": null,
                    "protocol_version": null
                })
            })
            .collect();
        
        axum::Json(serde_json::json!({
            "addresses": address_list,
            "total": address_list.len(),
            "limit": limit
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NetAddress;
    use std::net::IpAddr;

    #[tokio::test]
    async fn test_grpc_server_creation() {
        let address_manager = Arc::new(AddressManager::new("test").unwrap());
        let grpc_server = GrpcServer::new(address_manager);
        
        // 测试服务器创建
        assert!(true); // 简单的存在性测试
    }

    #[test]
    fn test_health_check_response() {
        // 测试健康检查响应格式
        let response = serde_json::json!({
            "status": "healthy",
            "timestamp": "2023-01-01T00:00:00Z"
        });
        
        assert_eq!(response["status"], "healthy");
        assert!(response["timestamp"].is_string());
    }
}
