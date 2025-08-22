use crate::errors::{KaseederError, Result};
use crate::manager::AddressManager;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use tracing::{info, warn};
use trust_dns_proto::op::{Message, MessageType, OpCode, ResponseCode};
use trust_dns_proto::rr::{Name, RData, Record, RecordType};
use trust_dns_proto::serialize::binary::{BinEncodable, BinEncoder};

/// DNS server implementation
pub struct DnsServer {
    hostname: String,
    nameserver: String,
    listen: String,
    address_manager: Arc<AddressManager>,
}

impl DnsServer {
    /// Create a new DNS server
    pub fn new(
        hostname: String,
        nameserver: String,
        listen: String,
        address_manager: Arc<AddressManager>,
    ) -> Self {
        // Ensure hostname and nameserver end with dot (like Go version)
        let hostname = if !hostname.ends_with('.') {
            format!("{}.", hostname)
        } else {
            hostname
        };

        let nameserver = if !nameserver.ends_with('.') {
            format!("{}.", nameserver)
        } else {
            nameserver
        };

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

        // Parse listen address
        let socket_addr: SocketAddr = self
            .listen
            .parse()
            .map_err(|_| KaseederError::Dns(format!("Invalid listen address: {}", self.listen)))?;

        // Use tokio async UDP socket
        let socket = if socket_addr.is_ipv4() {
            tokio::net::UdpSocket::bind(&self.listen).await?
        } else {
            // If IPv6 address provided, force IPv4 binding on the same port
            let ipv4_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), socket_addr.port());
            tokio::net::UdpSocket::bind(&ipv4_addr).await?
        };

        // Verify binding success (like Go version)
        let actual_addr = socket.local_addr()?;
        info!("DNS server actually bound to: {}", actual_addr);
        info!("DNS server successfully bound to {}", self.listen);
        info!("DNS server is now listening for requests");

        let mut buffer = [0u8; 512];
        let socket = Arc::new(socket);

        loop {
            let socket = socket.clone();
            match socket.recv_from(&mut buffer).await {
                Ok((len, src_addr)) => {
                    let request_data = buffer[..len].to_vec(); // Clone the data

                    // Handle DNS request asynchronously (like Go version)
                    let address_manager = self.address_manager.clone();
                    let hostname = self.hostname.clone();
                    let nameserver = self.nameserver.clone();
                    let socket_clone = socket.clone();

                    tokio::spawn(async move {
                        if let Ok(response_data) = Self::handle_dns_request_static(
                            &request_data,
                            &src_addr,
                            &address_manager,
                            &hostname,
                            &nameserver,
                        )
                        .await
                        {
                            info!(
                                "Attempting to send {} bytes to {}",
                                response_data.len(),
                                src_addr
                            );
                            match socket_clone.send_to(&response_data, src_addr).await {
                                Ok(sent) => {
                                    info!("Successfully sent {} bytes to {}", sent, src_addr);
                                }
                                Err(e) => {
                                    warn!("Failed to send DNS response to {}: {}", src_addr, e);
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    warn!("DNS server error: {}", e);
                }
            }
        }
    }

    /// Handle DNS request (static method for async spawn)
    async fn handle_dns_request_static(
        request_data: &[u8],
        src_addr: &SocketAddr,
        address_manager: &Arc<AddressManager>,
        hostname: &str,
        nameserver: &str,
    ) -> Result<Vec<u8>> {
        // Parse DNS message
        let request = match Message::from_vec(request_data) {
            Ok(msg) => msg,
            Err(e) => {
                warn!("{}: invalid DNS message: {}", src_addr, e);
                return Err(KaseederError::Dns(format!("Invalid DNS message: {}", e)));
            }
        };

        // Validate message type
        if request.header().message_type() != MessageType::Query {
            warn!("{}: not a query message", src_addr);
            return Err(KaseederError::Dns("Not a query message".to_string()));
        }

        if request.header().op_code() != OpCode::Query {
            warn!("{}: not a standard query", src_addr);
            return Err(KaseederError::Dns("Not a standard query".to_string()));
        }

        // Get the first query from the message (like Go version)
        let query = match request.query() {
            Some(q) => q,
            None => {
                warn!("{}: no query in DNS request", src_addr);
                return Err(KaseederError::Dns("No query in DNS request".to_string()));
            }
        };

        let domain_name = query.name();
        let query_type = query.query_type();

        info!("{}: query {} for {}", src_addr, query_type, domain_name);

        // Validate domain name (like Go version)
        if !Self::is_our_domain(domain_name, hostname) {
            warn!("{}: invalid name: {}", src_addr, domain_name);
            return Err(KaseederError::Dns(format!("Invalid name: {}", domain_name)));
        }

        // Extract subnetwork ID (like Go version)
        let (subnetwork_id, include_all_subnetworks) =
            Self::extract_subnetwork_id(domain_name, hostname)?;

        info!(
            "{}: query {} for subnetwork ID {:?}, include_all: {}",
            src_addr, query_type, subnetwork_id, include_all_subnetworks
        );

        // Build DNS response (like Go version)
        let response_data = Self::build_dns_response(
            &request,
            domain_name,
            query_type,
            include_all_subnetworks,
            subnetwork_id.as_deref(),
            nameserver,
            address_manager,
        )
        .await?;

        Ok(response_data)
    }

    /// Check if domain is our domain (like Go version)
    fn is_our_domain(domain_name: &Name, hostname: &str) -> bool {
        let domain_str = domain_name.to_string();
        domain_str.ends_with(hostname)
    }

    /// Extract subnetwork ID from domain name (like Go version)
    fn extract_subnetwork_id(domain_name: &Name, hostname: &str) -> Result<(Option<String>, bool)> {
        let domain_str = domain_name.to_string();

        // If it's our exact hostname, include all subnetworks
        if domain_str == hostname {
            return Ok((None, true));
        }

        // Check for subnetwork prefix (like Go version)
        let labels: Vec<&str> = domain_str.split('.').collect();
        if !labels.is_empty() && labels[0].starts_with('n') {
            let subnetwork_id = labels[0][1..].to_string();
            if !subnetwork_id.is_empty() {
                return Ok((Some(subnetwork_id), false));
            }
        }

        // Default: include all subnetworks
        Ok((None, true))
    }

    /// Build DNS response (like Go version)
    async fn build_dns_response(
        request: &Message,
        domain_name: &Name,
        query_type: RecordType,
        include_all_subnetworks: bool,
        subnetwork_id: Option<&str>,
        nameserver: &str,
        address_manager: &Arc<AddressManager>,
    ) -> Result<Vec<u8>> {
        // Create response message
        let mut response = Message::new();
        response.set_id(request.header().id());
        response.set_message_type(MessageType::Response);
        response.set_op_code(OpCode::Query);
        response.set_response_code(ResponseCode::NoError);
        response.set_authoritative(true);
        response.set_recursion_desired(false);
        response.set_recursion_available(false);

        // Add query
        if let Some(query) = request.query() {
            response.add_query(query.clone());
        }

        // Handle based on query type (like Go version)
        match query_type {
            RecordType::A => {
                Self::handle_a_query(
                    &mut response,
                    domain_name,
                    include_all_subnetworks,
                    subnetwork_id,
                    nameserver,
                    address_manager,
                )
                .await?;
            }
            RecordType::AAAA => {
                Self::handle_aaaa_query(
                    &mut response,
                    domain_name,
                    include_all_subnetworks,
                    subnetwork_id,
                    nameserver,
                    address_manager,
                )
                .await?;
            }
            RecordType::NS => {
                Self::handle_ns_query(&mut response, domain_name, nameserver).await?;
            }
            _ => {
                // Unsupported query type
                response.set_response_code(ResponseCode::ServFail);
            }
        }

        // Serialize response (like Go version)
        let mut buffer = Vec::new();
        let mut encoder = BinEncoder::new(&mut buffer);
        response.emit(&mut encoder)?;

        info!(
            "Response serialized: {} bytes, {} answers, {} authorities",
            buffer.len(),
            response.answers().len(),
            response.name_servers().len()
        );

        Ok(buffer)
    }

    /// Handle A record query (like Go version)
    async fn handle_a_query(
        response: &mut Message,
        domain_name: &Name,
        include_all_subnetworks: bool,
        subnetwork_id: Option<&str>,
        nameserver: &str,
        address_manager: &Arc<AddressManager>,
    ) -> Result<()> {
        let addresses = address_manager.good_addresses(
            1, // A record type
            include_all_subnetworks,
            subnetwork_id,
        );

        info!("Sending {} IPv4 addresses", addresses.len());

        // Add authority record (like Go version)
        let authority_name = Name::from_str(nameserver)?;
        let authority_record = Record::from_rdata(
            domain_name.clone(),
            86400, // TTL
            RData::NS(trust_dns_proto::rr::rdata::NS(authority_name)),
        );
        response.add_name_server(authority_record);

        // Add A records
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

    /// Handle AAAA record query (like Go version)
    async fn handle_aaaa_query(
        response: &mut Message,
        domain_name: &Name,
        include_all_subnetworks: bool,
        subnetwork_id: Option<&str>,
        nameserver: &str,
        address_manager: &Arc<AddressManager>,
    ) -> Result<()> {
        let addresses = address_manager.good_addresses(
            28, // AAAA record type
            include_all_subnetworks,
            subnetwork_id,
        );

        info!("Sending {} IPv6 addresses", addresses.len());

        // Add authority record (like Go version)
        let authority_name = Name::from_str(nameserver)?;
        let authority_record = Record::from_rdata(
            domain_name.clone(),
            86400, // TTL
            RData::NS(trust_dns_proto::rr::rdata::NS(authority_name)),
        );
        response.add_name_server(authority_record);

        // Add AAAA records
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

        // If no IPv6 addresses, add a placeholder (like Go version)
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

    /// Handle NS record query (like Go version)
    async fn handle_ns_query(
        response: &mut Message,
        domain_name: &Name,
        nameserver: &str,
    ) -> Result<()> {
        let ns_name = Name::from_str(nameserver)?;
        let record = Record::from_rdata(
            domain_name.clone(),
            86400, // TTL
            RData::NS(trust_dns_proto::rr::rdata::NS(ns_name)),
        );
        response.add_answer(record);

        Ok(())
    }
}
