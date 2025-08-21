use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use std::str::FromStr;

// 使用rusty-kaspa中的NetAddress类型
pub use kaspa_utils::networking::NetAddress;
pub use kaspa_utils::networking::IpAddress;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub address: NetAddress,
    pub user_agent: String,
    pub protocol_version: u32,
    pub subnetwork_id: Option<String>,
    pub last_connection: SystemTime,
}

impl NodeInfo {
    pub fn new(address: NetAddress, user_agent: String, protocol_version: u32) -> Self {
        Self {
            address,
            user_agent,
            protocol_version,
            subnetwork_id: None,
            last_connection: SystemTime::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMessage {
    pub protocol_version: u32,
    pub user_agent: String,
    pub timestamp: u64,
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressesMessage {
    pub addresses: Vec<NetAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestAddressesMessage {
    pub include_all_subnetworks: bool,
    pub subnetwork_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    pub command: String,
    pub payload: Vec<u8>,
}

impl NetworkMessage {
    pub fn new(command: &str, payload: Vec<u8>) -> Self {
        Self {
            command: command.to_string(),
            payload,
        }
    }

    pub fn version(version: &VersionMessage) -> Self {
        let payload = bincode::serialize(version).unwrap_or_default();
        Self::new("version", payload)
    }

    pub fn request_addresses(request: &RequestAddressesMessage) -> Self {
        let payload = bincode::serialize(request).unwrap_or_default();
        Self::new("getaddr", payload)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub name: String,
    pub record_type: DnsRecordType,
    pub ttl: u32,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DnsRecordType {
    A,
    AAAA,
    TXT,
}

impl DnsRecordType {
    pub fn to_u16(&self) -> u16 {
        match self {
            DnsRecordType::A => 1,
            DnsRecordType::AAAA => 28,
            DnsRecordType::TXT => 16,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlerStats {
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub failed_attempts: usize,
    pub successful_connections: usize,
    pub last_crawl: Option<SystemTime>,
    pub crawl_duration: Option<Duration>,
}

impl Default for CrawlerStats {
    fn default() -> Self {
        Self {
            total_nodes: 0,
            active_nodes: 0,
            failed_attempts: 0,
            successful_connections: 0,
            last_crawl: None,
            crawl_duration: None,
        }
    }
}

// 为NetAddress提供扩展功能的包装器
#[derive(Debug, Clone)]
pub struct NetAddressExt(pub NetAddress);

impl NetAddressExt {
    pub fn from_string(addr: &str) -> Option<NetAddress> {
        addr.parse::<NetAddress>().ok()
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}", self.0.ip, self.0.port)
    }

    pub fn is_recently_seen(&self, last_seen: SystemTime, threshold: Duration) -> bool {
        if let Ok(elapsed) = SystemTime::now().duration_since(last_seen) {
            elapsed < threshold
        } else {
            false
        }
    }

    pub fn is_good(&self, attempts: u32, successes: u32) -> bool {
        successes > 0 && attempts < 10
    }

    pub fn should_retry(&self, last_attempt: Option<SystemTime>, min_interval: Duration) -> bool {
        if let Some(last_attempt) = last_attempt {
            if let Ok(elapsed) = SystemTime::now().duration_since(last_attempt) {
                return elapsed >= min_interval;
            }
        }
        true
    }
}

// 为NetAddress提供便捷方法
impl NetAddress {
    pub fn from_string(addr: &str) -> Option<Self> {
        addr.parse::<NetAddress>().ok()
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }

    pub fn is_recently_seen(&self, last_seen: SystemTime, threshold: Duration) -> bool {
        if let Ok(elapsed) = SystemTime::now().duration_since(last_seen) {
            elapsed < threshold
        } else {
            false
        }
    }

    pub fn is_good(&self, attempts: u32, successes: u32) -> bool {
        successes > 0 && attempts < 10
    }

    pub fn should_retry(&self, last_attempt: Option<SystemTime>, min_interval: Duration) -> bool {
        if let Some(last_attempt) = last_attempt {
            if let Ok(elapsed) = SystemTime::now().duration_since(last_attempt) {
                return elapsed >= min_interval;
            }
        }
        true
    }
}
