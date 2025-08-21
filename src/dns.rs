use crate::manager::AddressManager;
use crate::types::{DnsRecord, DnsRecordType};
use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use trust_dns_proto::op::{Message, MessageType, OpCode, ResponseCode};
use trust_dns_proto::rr::{Record, RecordType};
use tracing::{error, info};

pub struct DnsServer {
    address_manager: Arc<AddressManager>,
    host: String,
    nameserver: String,
    listen_addr: String,
}

impl DnsServer {
    pub fn new(
        address_manager: Arc<AddressManager>,
        host: String,
        nameserver: String,
        listen_addr: String,
    ) -> Self {
        Self {
            address_manager,
            host,
            nameserver,
            listen_addr,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let socket = UdpSocket::bind(&self.listen_addr).await?;
        info!("DNS server listening on {}", self.listen_addr);

        let mut buf = [0u8; 512];
        let address_manager = self.address_manager.clone();

        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, src)) => {
                    let address_manager = address_manager.clone();
                    let request_data = buf[..len].to_vec();
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_dns_request(request_data, src, address_manager).await {
                            error!("Error handling DNS request: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error receiving DNS request: {}", e);
                }
            }
        }
    }

    async fn handle_dns_request(
        request_data: Vec<u8>,
        src: SocketAddr,
        address_manager: Arc<AddressManager>,
    ) -> Result<()> {
        // 解析DNS请求
        let request = Message::from_vec(&request_data)?;
        
        if request.header().message_type() != MessageType::Query {
            return Ok(());
        }

        let query = request.query().ok_or_else(|| {
            anyhow::anyhow!("No query in DNS request")
        })?;

        let response = Self::create_dns_response(&request, query, &address_manager).await?;
        
        // 发送响应
        let response_data = response.to_vec()?;
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.send_to(&response_data, src).await?;
        
        Ok(())
    }

    async fn create_dns_response(
        request: &Message,
        query: &trust_dns_proto::op::Query,
        address_manager: &AddressManager,
    ) -> Result<Message> {
        let mut response = Message::new();
        response.set_id(request.header().id());
        response.set_message_type(MessageType::Response);
        response.set_op_code(OpCode::Query);
        response.set_response_code(ResponseCode::NoError);
        response.set_authoritative(true);
        response.set_recursion_available(false);
        response.set_recursion_desired(request.header().recursion_desired());
        response.set_authentic_data(false);
        response.set_checking_disabled(false);

        // 添加查询
        response.add_query(query.clone());

        // 根据查询类型添加记录
        match query.query_type() {
            RecordType::A => {
                let addresses = address_manager.get_good_addresses(8);
                for address in addresses {
                    if let std::net::IpAddr::V4(ipv4) = address.ip.0 {
                        let record = Record::from_rdata(
                            query.name().clone(),
                            300, // TTL
                            trust_dns_proto::rr::RData::A(trust_dns_proto::rr::rdata::A(ipv4)),
                        );
                        response.add_answer(record);
                    }
                }
            }
            RecordType::AAAA => {
                let addresses = address_manager.get_good_addresses(8);
                for address in addresses {
                    if let std::net::IpAddr::V6(ipv6) = address.ip.0 {
                        let record = Record::from_rdata(
                            query.name().clone(),
                            300, // TTL
                            trust_dns_proto::rr::RData::AAAA(trust_dns_proto::rr::rdata::AAAA(ipv6)),
                        );
                        response.add_answer(record);
                    }
                }
            }
            RecordType::TXT => {
                // 添加版本信息作为TXT记录
                let version_info = format!("version={}", env!("CARGO_PKG_VERSION"));
                let record = Record::from_rdata(
                    query.name().clone(),
                    300, // TTL
                    trust_dns_proto::rr::RData::TXT(
                        trust_dns_proto::rr::rdata::TXT::new(vec![version_info]),
                    ),
                );
                response.add_answer(record);
            }
            _ => {
                // 不支持的查询类型
                response.set_response_code(ResponseCode::ServFail);
            }
        }

        Ok(response)
    }

    pub fn get_dns_records(&self, address_manager: &AddressManager) -> Vec<DnsRecord> {
        let mut records = Vec::new();
        
        // 获取好的地址
        let addresses = address_manager.get_good_addresses(8);
        
        for address in addresses {
            match address.ip.0 {
                std::net::IpAddr::V4(ipv4) => {
                    records.push(DnsRecord {
                        name: self.host.clone(),
                        record_type: DnsRecordType::A,
                        ttl: 300,
                        data: ipv4.to_string(),
                    });
                }
                std::net::IpAddr::V6(ipv6) => {
                    records.push(DnsRecord {
                        name: self.host.clone(),
                        record_type: DnsRecordType::AAAA,
                        ttl: 300,
                        data: ipv6.to_string(),
                    });
                }
            }
        }

        // 添加TXT记录
        records.push(DnsRecord {
            name: self.host.clone(),
            record_type: DnsRecordType::TXT,
            ttl: 300,
            data: format!("version={}", env!("CARGO_PKG_VERSION")),
        });

        records
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
