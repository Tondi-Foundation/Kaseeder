use crate::errors::{KaseederError, Result};
use crate::types::NetAddress;
use std::net::{IpAddr, Ipv6Addr, SocketAddr, UdpSocket};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tracing::{debug, info, warn};
use trust_dns_proto::op::{Message, MessageType, OpCode, ResponseCode};
use trust_dns_proto::rr::{Name, RData, Record, RecordType};
use trust_dns_proto::serialize::binary::{BinEncodable, BinEncoder};

/// DNS server structure
pub struct DnsServer {
    hostname: String,
    nameserver: String,
    listen: String,
    address_manager: Arc<dyn AddressManager>,
}

impl DnsServer {
    /// Create a new DNS server
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

    /// Start the DNS server
    pub async fn start(&self) -> Result<()> {
        info!("Starting DNS server on {}", self.listen);

        let socket = UdpSocket::bind(&self.listen)?;
        socket.set_read_timeout(Some(Duration::from_secs(1)))?;

        let mut buffer = [0u8; 512];

        loop {
            match socket.recv_from(&mut buffer) {
                Ok((len, src_addr)) => {
                    let request_data = &buffer[..len];

                    // Handle DNS request
                    if let Ok(response_data) =
                        self.handle_dns_request(request_data, &src_addr).await
                    {
                        if let Err(e) = socket.send_to(&response_data, src_addr) {
                            warn!("Failed to send DNS response: {}", e);
                        }
                    }
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        // Timeout, continue loop
                        continue;
                    }
                    warn!("DNS server error: {}", e);
                }
            }
        }
    }

    /// Handle DNS request
    async fn handle_dns_request(
        &self,
        request_data: &[u8],
        src_addr: &SocketAddr,
    ) -> Result<Vec<u8>> {
        let request = Message::from_vec(request_data)?;

        if request.header().message_type() != MessageType::Query {
            return Err(KaseederError::Dns("Not a query message".to_string()));
        }

        if request.header().op_code() != OpCode::Query {
            return Err(KaseederError::Dns("Not a standard query".to_string()));
        }

        let _queries = request.query();
        let query = request
            .query()
            .ok_or_else(|| KaseederError::Dns("No query in DNS request".to_string()))?;

        let domain_name = query.name();
        let query_type = query.query_type();

        debug!(
            "DNS query from {}: {} type {}",
            src_addr, domain_name, query_type
        );

        // Check if domain belongs to us
        if domain_name.to_string() != self.hostname {
            return Err(KaseederError::Dns("Domain not served by this server".to_string()));
        }

        // Create response
        let mut response = Message::new();
        response.set_id(request.header().id());
        response.set_message_type(MessageType::Response);
        response.set_op_code(OpCode::Query);
        response.set_response_code(ResponseCode::NoError);
        response.set_authoritative(true);
        response.set_recursion_desired(false);
        response.set_recursion_available(false);

        // Add query
        response.add_query(query.clone());

        // Handle based on query type
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
                // Unsupported query type
                response.set_response_code(ResponseCode::ServFail);
            }
        }

        // Serialize response
        let mut buffer = Vec::new();
        let mut encoder = BinEncoder::new(&mut buffer);
        response.emit(&mut encoder)?;

        Ok(buffer)
    }

    /// Handle A record query
    async fn handle_a_query(&self, response: &mut Message, domain_name: &Name) -> Result<()> {
        let addresses = self
            .address_manager
            .get_good_addresses(
                1,    // A record type
                true, // Include all subnetworks
                None, // Subnetwork ID
            )
            .await;

        info!(
            "A query for {}: returning {} IPv4 addresses",
            domain_name,
            addresses.len()
        );

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

    /// Handle AAAA record query
    async fn handle_aaaa_query(&self, response: &mut Message, domain_name: &Name) -> Result<()> {
        let addresses = self
            .address_manager
            .get_good_addresses(
                28,   // AAAA record type
                true, // Include all subnetworks
                None, // Subnetwork ID
            )
            .await;

        info!(
            "AAAA query for {}: returning {} IPv6 addresses",
            domain_name,
            addresses.len()
        );

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

        // If no IPv6 addresses, add a placeholder (refer to Go version implementation)
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

    /// Handle NS record query
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

    /// Check if domain belongs to us
    fn is_our_domain(&self, domain_name: &Name) -> bool {
        let hostname = Name::from_str(&self.hostname).unwrap_or_default();
        // Check if domain ends with our hostname
        domain_name
            .iter()
            .rev()
            .zip(hostname.iter().rev())
            .all(|(a, b)| a == b)
    }
}

/// Address manager trait, used for abstracting address management
#[async_trait]
pub trait AddressManager: Send + Sync {
    async fn get_good_addresses(
        &self,
        qtype: u16,
        include_all_subnetworks: bool,
        subnetwork_id: Option<&str>,
    ) -> Vec<NetAddress>;
}

/// Implement trait for our address manager
#[async_trait]
impl AddressManager for crate::manager::AddressManager {
    async fn get_good_addresses(
        &self,
        qtype: u16,
        include_all_subnetworks: bool,
        subnetwork_id: Option<&str>,
    ) -> Vec<NetAddress> {
        self.good_addresses(qtype, include_all_subnetworks, subnetwork_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_dns_record_creation() {
        // Create a mock address manager
        let address_manager = Arc::new(MockAddressManager);
        let dns_server = DnsServer::new(
            "seed.example.com".to_string(),
            "ns.example.com".to_string(),
            "127.0.0.1:5354".to_string(),
            address_manager.clone(),
        );

        // Test DNS server creation success
        assert_eq!(dns_server.hostname, "seed.example.com");
        assert_eq!(dns_server.nameserver, "ns.example.com");
        assert_eq!(dns_server.listen, "127.0.0.1:5354");
    }

    // Mock address manager for testing
    struct MockAddressManager;

    #[async_trait]
    impl AddressManager for MockAddressManager {
        async fn get_good_addresses(
            &self,
            _qtype: u16,
            _include_all_subnetworks: bool,
            _subnetwork_id: Option<&str>,
        ) -> Vec<NetAddress> {
            vec![]
        }
    }
}
