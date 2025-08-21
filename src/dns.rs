use crate::types::{DnsRecord, NetAddress, NetAddressExt};
use anyhow::Result;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use trust_dns_proto::op::{Message, MessageType, OpCode, ResponseCode};
use trust_dns_proto::rr::{DNSClass, Name, RData, Record, RecordType};
use trust_dns_proto::serialize::binary::{BinEncoder, BinEncodable};
use async_trait::async_trait;

/// DNS 服务器结构
pub struct DnsServer {
    hostname: String,
    nameserver: String,
    listen: String,
    address_manager: Arc<dyn AddressManager>,
}

impl DnsServer {
    /// 创建新的 DNS 服务器
    pub fn new(
        hostname: String,
        nameserver: String,
        listen: String,
        address_manager: Arc<dyn AddressManager>,
    ) -> Self {
        Self {
            hostname,
            nameserver,
            listen,
            address_manager,
        }
    }

    /// 启动 DNS 服务器
    pub async fn start(&self) -> Result<()> {
        info!("Starting DNS server on {}", self.listen);
        
        let socket = UdpSocket::bind(&self.listen)?;
        socket.set_read_timeout(Some(Duration::from_secs(1)))?;
        
        let mut buffer = [0u8; 512];
        
        loop {
            match socket.recv_from(&mut buffer) {
                Ok((len, src_addr)) => {
                    let request_data = &buffer[..len];
                    
                    // 处理 DNS 请求
                    if let Ok(response_data) = self.handle_dns_request(request_data, &src_addr).await {
                        if let Err(e) = socket.send_to(&response_data, src_addr) {
                            warn!("Failed to send DNS response: {}", e);
                        }
                    }
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        // 超时，继续循环
                        continue;
                    }
                    warn!("DNS server error: {}", e);
                }
            }
        }
    }

    /// 处理 DNS 请求
    async fn handle_dns_request(&self, request_data: &[u8], src_addr: &SocketAddr) -> Result<Vec<u8>> {
        let request = Message::from_vec(request_data)?;
        
        if request.header().message_type() != MessageType::Query {
            return Err(anyhow::anyhow!("Not a query message"));
        }
        
        if request.header().op_code() != OpCode::Query {
            return Err(anyhow::anyhow!("Not a standard query"));
        }
        
        let queries = request.query();
        let query = request.query().ok_or_else(|| {
            anyhow::anyhow!("No query in DNS request")
        })?;
        
        let domain_name = query.name();
        let query_type = query.query_type();
        
        debug!("DNS query from {}: {} type {}", src_addr, domain_name, query_type);
        
        // 检查域名是否属于我们
        if !self.is_our_domain(domain_name) {
            return Err(anyhow::anyhow!("Domain not served by this server"));
        }
        
        // 创建响应
        let mut response = Message::new();
        response.set_id(request.header().id());
        response.set_message_type(MessageType::Response);
        response.set_op_code(OpCode::Query);
        response.set_response_code(ResponseCode::NoError);
        response.set_authoritative(true);
        response.set_recursion_desired(false);
        response.set_recursion_available(false);
        
        // 添加查询
        response.add_query(query.clone());
        
        // 根据查询类型处理
        match query_type {
            RecordType::A => {
                self.handle_a_query(&mut response, domain_name).await?;
            }
            RecordType::AAAA => {
                self.handle_aaaa_query(&mut response, domain_name).await?;
            }
            RecordType::NS => {
                self.handle_ns_query(&mut response, domain_name).await?;
            }
            _ => {
                // 不支持的查询类型
                response.set_response_code(ResponseCode::ServFail);
            }
        }
        
        // 序列化响应
        let mut buffer = Vec::new();
        let mut encoder = BinEncoder::new(&mut buffer);
        response.emit(&mut encoder)?;
        
        Ok(buffer)
    }

    /// 处理 A 记录查询
    async fn handle_a_query(&self, response: &mut Message, domain_name: &Name) -> Result<()> {
        let addresses = self.address_manager.get_good_addresses(
            1, // A 记录类型
            true, // 包含所有子网络
            None, // 子网络 ID
        ).await;
        
        info!("A query for {}: returning {} IPv4 addresses", domain_name, addresses.len());
        
        for address in addresses.iter().take(8) {
            if let IpAddr::V4(ipv4) = address.ip {
                let record = Record::from_rdata(
                    domain_name.clone(),
                    30, // TTL
                    RData::A(trust_dns_proto::rr::rdata::A(ipv4)),
                );
                response.add_answer(record);
            }
        }
        
        Ok(())
    }

    /// 处理 AAAA 记录查询
    async fn handle_aaaa_query(&self, response: &mut Message, domain_name: &Name) -> Result<()> {
        let addresses = self.address_manager.get_good_addresses(
            28, // AAAA 记录类型
            true, // 包含所有子网络
            None, // 子网络 ID
        ).await;
        
        info!("AAAA query for {}: returning {} IPv6 addresses", domain_name, addresses.len());
        
        for address in addresses.iter().take(8) {
            if let IpAddr::V6(ipv6) = address.ip {
                let record = Record::from_rdata(
                    domain_name.clone(),
                    30, // TTL
                    RData::AAAA(trust_dns_proto::rr::rdata::AAAA(ipv6)),
                );
                response.add_answer(record);
            }
        }
        
        // 如果没有 IPv6 地址，添加一个占位符（参考 Go 版本的实现）
        if addresses.is_empty() {
            let placeholder_ip = Ipv6Addr::new(0x100, 0, 0, 0, 0, 0, 0, 0);
            let record = Record::from_rdata(
                domain_name.clone(),
                30, // TTL
                RData::AAAA(trust_dns_proto::rr::rdata::AAAA(placeholder_ip)),
            );
            response.add_answer(record);
        }
        
        Ok(())
    }

    /// 处理 NS 记录查询
    async fn handle_ns_query(&self, response: &mut Message, domain_name: &Name) -> Result<()> {
        let nameserver_name = Name::from_str(&self.nameserver)?;
        let record = Record::from_rdata(
            domain_name.clone(),
            86400, // TTL
            RData::NS(trust_dns_proto::rr::rdata::NS(nameserver_name)),
        );
        response.add_answer(record);
        
        Ok(())
    }

    /// 检查域名是否属于我们
    fn is_our_domain(&self, domain_name: &Name) -> bool {
        let hostname = Name::from_str(&self.hostname).unwrap_or_default();
        // 检查域名是否以我们的主机名结尾
        domain_name.iter().rev().zip(hostname.iter().rev()).all(|(a, b)| a == b)
    }
}

/// 地址管理器 trait，用于抽象地址管理
#[async_trait]
pub trait AddressManager: Send + Sync {
    async fn get_good_addresses(&self, qtype: u16, include_all_subnetworks: bool, subnetwork_id: Option<&str>) -> Vec<NetAddress>;
}

/// 为我们的地址管理器实现 trait
#[async_trait]
impl AddressManager for crate::manager::AddressManager {
    async fn get_good_addresses(&self, qtype: u16, include_all_subnetworks: bool, subnetwork_id: Option<&str>) -> Vec<NetAddress> {
        self.good_addresses(qtype, include_all_subnetworks, subnetwork_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NetAddress;
    use std::net::IpAddr;

    #[test]
    fn test_dns_record_creation() {
        let address_manager = Arc::new(AddressManager::new("test").unwrap());
        let dns_server = DnsServer::new(
            address_manager,
            "seed.example.com".to_string(),
            "ns.example.com".to_string(),
            "127.0.0.1:5354".to_string(),
        );

        let records = dns_server.get_dns_records(&address_manager);
        assert!(!records.is_empty());
    }
}
