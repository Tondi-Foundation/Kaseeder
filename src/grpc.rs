use crate::manager::AddressManager;
use crate::types::NetAddress;
use anyhow::Result;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tonic::{transport::Server, Request, Response, Status};
use tracing::info;

// Include generated protobuf code
pub mod dnsseeder {
    tonic::include_proto!("dnsseeder");
}

use dnsseeder::{
    dns_seeder_service_server::{
        DnsSeederService as DnsSeederServiceTrait, DnsSeederServiceServer,
    },
    health_check_response::Status as HealthStatus,
    GetAddressStatsRequest, GetAddressStatsResponse, GetAddressesRequest, GetAddressesResponse,
    GetStatsRequest, GetStatsResponse, HealthCheckRequest, HealthCheckResponse,
};

/// gRPC server structure
pub struct GrpcServer {
    address_manager: Arc<AddressManager>,
}

impl GrpcServer {
    /// Create a new gRPC server
    pub fn new(address_manager: Arc<AddressManager>) -> Self {
        Self { address_manager }
    }

    /// Start the gRPC server
    pub async fn start(&self, listen_addr: &str) -> Result<()> {
        let addr: std::net::SocketAddr = listen_addr.parse()?;
        info!("Starting gRPC server on {}", addr);

        let service = DnsSeederServiceImpl::new(self.address_manager.clone());
        let server = DnsSeederServiceServer::new(service);

        Server::builder()
            .add_service(server)
            .serve(addr)
            .await
            .map_err(|e| anyhow::anyhow!("gRPC server error: {}", e))?;

        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> serde_json::Value {
        let stats = self.address_manager.get_stats();

        serde_json::json!({
            "total_nodes": stats.total_nodes.load(std::sync::atomic::Ordering::Relaxed),
            "active_nodes": stats.active_nodes.load(std::sync::atomic::Ordering::Relaxed),
            "failed_connections": stats.failed_connections.load(std::sync::atomic::Ordering::Relaxed),
            "successful_connections": stats.successful_connections.load(std::sync::atomic::Ordering::Relaxed),
            "last_update": stats.last_update.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
        })
    }

    /// Get address list
    pub fn get_addresses(&self, limit: usize) -> Vec<NetAddress> {
        // Get all types of addresses
        let mut addresses = Vec::new();

        // A record addresses
        let a_addresses = self.address_manager.good_addresses(1, true, None);
        addresses.extend_from_slice(&a_addresses);

        // AAAA record addresses
        let aaaa_addresses = self.address_manager.good_addresses(28, true, None);
        addresses.extend_from_slice(&aaaa_addresses);

        // Limit quantity
        addresses.truncate(limit);

        addresses
    }

    /// Get address statistics
    pub fn get_address_stats(&self) -> serde_json::Value {
        let total = self.address_manager.address_count();

        // Count IPv4 and IPv6 address quantities
        let mut ipv4_count = 0;
        let mut ipv6_count = 0;

        for node in self.address_manager.get_all_nodes() {
            if node.address.ip.is_ipv4() {
                ipv4_count += 1;
            } else {
                ipv6_count += 1;
            }
        }

        serde_json::json!({
            "total_addresses": total,
            "ipv4_addresses": ipv4_count,
            "ipv6_addresses": ipv6_count,
            "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
        })
    }
}

/// gRPC service implementation
pub struct DnsSeederServiceImpl {
    address_manager: Arc<AddressManager>,
    start_time: SystemTime,
}

impl DnsSeederServiceImpl {
    pub fn new(address_manager: Arc<AddressManager>) -> Self {
        Self {
            address_manager,
            start_time: SystemTime::now(),
        }
    }
}

#[tonic::async_trait]
impl DnsSeederServiceTrait for DnsSeederServiceImpl {
    async fn get_addresses(
        &self,
        request: Request<GetAddressesRequest>,
    ) -> Result<Response<GetAddressesResponse>, Status> {
        let req = request.into_inner();
        let limit = if req.limit == 0 {
            100
        } else {
            req.limit as usize
        };

        info!(
            "gRPC GetAddresses request: limit={}, ipv4={}, ipv6={}",
            req.limit, req.include_ipv4, req.include_ipv6
        );

        let mut addresses = Vec::new();

        // Get IPv4 addresses
        if req.include_ipv4 {
            let ipv4_addresses = self.address_manager.good_addresses(
                1,
                true,
                if req.subnetwork_id.is_empty() {
                    None
                } else {
                    Some(&req.subnetwork_id)
                },
            );
            for addr in ipv4_addresses {
                if addr.ip.is_ipv4() && addresses.len() < limit {
                    addresses.push(dnsseeder::NetAddress {
                        ip: addr.ip.to_string(),
                        port: addr.port as u32,
                        last_seen: 0,               // TODO: Implement timestamp
                        user_agent: "".to_string(), // TODO: Implement user agent
                        protocol_version: 0,        // TODO: Implement protocol version
                    });
                }
            }
        }

        // Get IPv6 addresses
        if req.include_ipv6 {
            let ipv6_addresses = self.address_manager.good_addresses(
                28,
                true,
                if req.subnetwork_id.is_empty() {
                    None
                } else {
                    Some(&req.subnetwork_id)
                },
            );
            for addr in ipv6_addresses {
                if addr.ip.is_ipv6() && addresses.len() < limit {
                    addresses.push(dnsseeder::NetAddress {
                        ip: addr.ip.to_string(),
                        port: addr.port as u32,
                        last_seen: 0,               // TODO: Implement timestamp
                        user_agent: "".to_string(), // TODO: Implement user agent
                        protocol_version: 0,        // TODO: Implement protocol version
                    });
                }
            }
        }

        let response = GetAddressesResponse {
            addresses,
            total_count: self.address_manager.address_count() as u64,
        };

        Ok(Response::new(response))
    }

    async fn get_stats(
        &self,
        _request: Request<GetStatsRequest>,
    ) -> Result<Response<GetStatsResponse>, Status> {
        let stats = self.address_manager.get_stats();
        let uptime = self.start_time.elapsed().unwrap_or_default();

        let response = GetStatsResponse {
            total_nodes: stats.total_nodes.load(std::sync::atomic::Ordering::Relaxed),
            active_nodes: stats
                .active_nodes
                .load(std::sync::atomic::Ordering::Relaxed),
            failed_connections: stats
                .failed_connections
                .load(std::sync::atomic::Ordering::Relaxed),
            successful_connections: stats
                .successful_connections
                .load(std::sync::atomic::Ordering::Relaxed),
            last_update: stats
                .last_update
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            uptime: format!("{}s", uptime.as_secs()),
        };

        Ok(Response::new(response))
    }

    async fn get_address_stats(
        &self,
        _request: Request<GetAddressStatsRequest>,
    ) -> Result<Response<GetAddressStatsResponse>, Status> {
        let total = self.address_manager.address_count();

        // Count different types of addresses
        let mut ipv4_count = 0;
        let mut ipv6_count = 0;
        let mut good_count = 0;
        let stale_count = 0;

        for node in self.address_manager.get_all_nodes() {
            if node.address.ip.is_ipv4() {
                ipv4_count += 1;
            } else {
                ipv6_count += 1;
            }

            // TODO: Implement good/stale classification logic
            good_count += 1;
        }

        let response = GetAddressStatsResponse {
            total_addresses: total as u64,
            ipv4_addresses: ipv4_count,
            ipv6_addresses: ipv6_count,
            good_addresses: good_count,
            stale_addresses: stale_count,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        Ok(Response::new(response))
    }

    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let response = HealthCheckResponse {
            status: HealthStatus::Serving as i32,
            message: "DNS Seeder service is healthy".to_string(),
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[tokio::test]
    async fn test_grpc_server_creation() {
        let address_manager = Arc::new(AddressManager::new("./test_data").unwrap());
        let _server = GrpcServer::new(address_manager);
        assert!(true); // Verify creation success
    }

    #[tokio::test]
    async fn test_get_addresses() {
        let address_manager = Arc::new(AddressManager::new("./test_data").unwrap());
        let _server = GrpcServer::new(address_manager);

        let addresses = _server.get_addresses(10);
        assert_eq!(addresses.len(), 0); // Newly created address manager should be empty
    }
}
