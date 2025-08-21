use crate::types::{CrawlerStats, NetAddress};
use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Address manager configuration constants
const PEERS_FILENAME: &str = "peers.json";
const DEFAULT_STALE_GOOD_TIMEOUT: Duration = Duration::from_secs(60 * 60); // 1 hour, same as Go version
const NEW_NODE_POLL_INTERVAL: Duration = Duration::from_secs(30 * 60); // New node poll interval: 30 minutes
const PRUNE_EXPIRE_TIMEOUT: Duration = Duration::from_secs(8 * 60 * 60); // 8 hours, same as Go version
const PRUNE_ADDRESS_INTERVAL: Duration = Duration::from_secs(60 * 60); // 1 hour
const DUMP_ADDRESS_INTERVAL: Duration = Duration::from_secs(10 * 60); // 10 minutes
const DEFAULT_MAX_ADDRESSES: usize = 2000;

/// Node status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub address: NetAddress,
    pub last_seen: SystemTime,
    pub last_attempt: SystemTime,
    pub last_success: SystemTime,
    pub user_agent: Option<String>,
    pub subnetwork_id: Option<String>,
    pub services: u64,
}

impl Node {
    pub fn new(address: NetAddress) -> Self {
        let now = SystemTime::now();
        Self {
            address,
            last_seen: now,
            last_attempt: now,
            last_success: UNIX_EPOCH, // Never successfully connected
            user_agent: None,
            subnetwork_id: None,
            services: 0,
        }
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.address.ip, self.address.port)
    }
}

/// Address manager, corresponding to Go version's Manager
pub struct AddressManager {
    nodes: DashMap<String, Node>,
    peers_file: String,
    quit_tx: mpsc::Sender<()>,
    stats: Arc<CrawlerStats>,
}

impl AddressManager {
    /// Create a new address manager
    pub fn new(app_dir: &str) -> Result<Self> {
        let peers_file = std::path::Path::new(app_dir).join(PEERS_FILENAME);
        let peers_file = peers_file.to_string_lossy().to_string();

        // Ensure the directory exists
        if let Some(parent_dir) = std::path::Path::new(&peers_file).parent() {
            std::fs::create_dir_all(parent_dir)?;
        }

        let (quit_tx, _quit_rx) = mpsc::channel(1);

        let manager = Self {
            nodes: DashMap::new(),
            peers_file,
            quit_tx,
            stats: Arc::new(CrawlerStats::default()),
        };

        // Load saved nodes
        manager.deserialize_peers()?;

        Ok(manager)
    }

    /// Start the address manager (call this after creation to start background tasks)
    pub fn start(&self) {
        // Start address processing coroutine
        let manager_clone = self.clone();
        tokio::spawn(async move {
            manager_clone.address_handler().await;
        });
    }

    /// Add address list, return the number of new addresses added
    pub fn add_addresses(
        &self,
        addresses: Vec<NetAddress>,
        _default_port: u16,
        accept_unroutable: bool,
    ) -> usize {
        let mut count = 0;

        for address in addresses {
            // Check port and routability
            if address.port == 0 || (!accept_unroutable && !self.is_routable(&address)) {
                continue;
            }

            let addr_str = format!("{}:{}", address.ip, address.port);

            if let Some(mut node) = self.nodes.get_mut(&addr_str) {
                // Update the last access time of the existing node
                node.last_seen = SystemTime::now();
            } else {
                // Create a new node
                let node = Node::new(address);
                self.nodes.insert(addr_str, node);
                count += 1;
            }
        }

        count
    }

    /// Get addresses that need to be retested
    pub fn addresses(&self, threads: u8) -> Vec<NetAddress> {
        let mut addresses = Vec::new();
        let max_count = threads as usize * 3;
        let mut count = 0;

        for entry in self.nodes.iter() {
            if count >= max_count {
                break;
            }

            let node = entry.value();
            
                // First process new nodes (nodes that have never successfully connected)
            if node.last_success.eq(&UNIX_EPOCH) {
                addresses.push(node.address.clone());
                count += 1;
                continue;
            }
            
            // Then process expired nodes
            if self.is_stale(node) {
                addresses.push(node.address.clone());
                count += 1;
            }
        }

        addresses
    }

    /// Get the total number of addresses
    pub fn address_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get all nodes (for statistics)
    pub fn get_all_nodes(&self) -> Vec<Node> {
        self.nodes
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get good address list, filtered by DNS query type
    pub fn good_addresses(
        &self,
        qtype: u16,
        include_all_subnetworks: bool,
        subnetwork_id: Option<&str>,
    ) -> Vec<NetAddress> {
        let mut addresses = Vec::new();
        let mut count = 0;

        // Only support A and AAAA records
        if qtype != 1 && qtype != 28 {
            // 1=A, 28=AAAA
            return addresses;
        }

        for entry in self.nodes.iter() {
            if count >= DEFAULT_MAX_ADDRESSES {
                break;
            }

            let node = entry.value();

            // Check subnet
            if !include_all_subnetworks {
                if let Some(ref expected_id) = subnetwork_id {
                    if let Some(ref node_id) = node.subnetwork_id {
                        if expected_id != node_id {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
            }

            // Check IP type
            let is_ipv4 = node.address.ip.is_ipv4();
            if (qtype == 1 && !is_ipv4) || (qtype == 28 && is_ipv4) {
                continue;
            }

            // Check node status
            if !self.is_good(node) {
                continue;
            }

            addresses.push(node.address.clone());
            count += 1;
        }

        addresses
    }

    /// Update connection attempt time
    pub fn attempt(&self, address: &NetAddress) {
        let addr_str = format!("{}:{}", address.ip, address.port);

        if let Some(mut node) = self.nodes.get_mut(&addr_str) {
            node.last_attempt = SystemTime::now();
        }
    }

    /// Update successful connection information
    pub fn good(
        &self,
        address: &NetAddress,
        user_agent: Option<&str>,
        subnetwork_id: Option<&str>,
    ) {
        let addr_str = format!("{}:{}", address.ip, address.port);

        if let Some(mut node) = self.nodes.get_mut(&addr_str) {
            node.user_agent = user_agent.map(|s| s.to_string());
            node.subnetwork_id = subnetwork_id.map(|s| s.to_string());
            node.last_success = SystemTime::now();
        }
    }

    /// Address processing coroutine
    async fn address_handler(&self) {
        let mut prune_ticker = tokio::time::interval(PRUNE_ADDRESS_INTERVAL);
        let mut dump_ticker = tokio::time::interval(DUMP_ADDRESS_INTERVAL);

        loop {
            tokio::select! {
                _ = prune_ticker.tick() => {
                    self.prune_peers();
                }
                _ = dump_ticker.tick() => {
                    self.save_peers();
                }
            }
        }
    }

        /// Clean up expired and bad addresses
    fn prune_peers(&self) {
        let mut pruned = 0;
        let mut good = 0;
        let mut stale = 0;
        let mut bad = 0;
        let mut ipv4 = 0;
        let mut ipv6 = 0;

        let now = SystemTime::now();
        let mut to_remove = Vec::new();

        for entry in self.nodes.iter() {
            let node = entry.value();

            if self.is_expired(node, now) {
                to_remove.push(entry.key().clone());
                pruned += 1;
            } else if self.is_good(node) {
                good += 1;
                if node.address.ip.is_ipv4() {
                    ipv4 += 1;
                } else {
                    ipv6 += 1;
                }
            } else if self.is_stale(node) {
                stale += 1;
            } else {
                bad += 1;
            }
        }

        // Remove expired nodes
        for key in to_remove {
            self.nodes.remove(&key);
        }

        let total = self.nodes.len();
        debug!("Pruned {} addresses. {} left.", pruned, total);
        info!(
            "Known nodes: Good:{} [4:{}, 6:{}] Stale:{} Bad:{}",
            good, ipv4, ipv6, stale, bad
        );
    }

    /// Save addresses to file
    fn save_peers(&self) {
        let nodes: Vec<_> = self
            .nodes
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        // Ensure the directory exists before writing files
        if let Some(parent_dir) = std::path::Path::new(&self.peers_file).parent() {
            if let Err(e) = std::fs::create_dir_all(parent_dir) {
                error!("Failed to create directory {}: {}", parent_dir.display(), e);
                return;
            }
        }

        // Create temporary file
        let tmp_file = format!("{}.new", self.peers_file);

        if let Err(e) = std::fs::write(&tmp_file, serde_json::to_string(&nodes).unwrap_or_default())
        {
            error!("Failed to write temporary file {}: {}", tmp_file, e);
            return;
        }

        // Atomically rename file
        if let Err(e) = std::fs::rename(&tmp_file, &self.peers_file) {
            error!(
                "Failed to rename {} to {}: {}",
                tmp_file, self.peers_file, e
            );
            if let Err(e) = std::fs::remove_file(&tmp_file) {
                error!("Failed to remove temporary file {}: {}", tmp_file, e);
            }
        }
    }

    /// Load addresses from file
    fn deserialize_peers(&self) -> Result<()> {
        if !std::path::Path::new(&self.peers_file).exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.peers_file)?;
        let nodes: Vec<(String, Node)> = serde_json::from_str(&content)?;

        let nodes_count = nodes.len();
        for (key, node) in nodes {
            self.nodes.insert(key, node);
        }

        info!("{} nodes loaded", nodes_count);
        Ok(())
    }

    /// Check if node is expired
    fn is_expired(&self, node: &Node, now: SystemTime) -> bool {
        let last_seen_elapsed = now.duration_since(node.last_seen).unwrap_or_default();

        last_seen_elapsed > PRUNE_EXPIRE_TIMEOUT
    }

    /// Check if node is good
    fn is_good(&self, node: &Node) -> bool {
        let now = SystemTime::now();
        let last_success_elapsed = now.duration_since(node.last_success).unwrap_or_default();

        last_success_elapsed < DEFAULT_STALE_GOOD_TIMEOUT
    }

    /// Check if node is stale
    fn is_stale(&self, node: &Node) -> bool {
        let now = SystemTime::now();
        let last_attempt_elapsed = now.duration_since(node.last_attempt).unwrap_or_default();
        let _last_success_elapsed = now.duration_since(node.last_success).unwrap_or_default();

        // For nodes that have never successfully connected (new nodes), if they have never been attempted or the attempt time exceeds a short threshold, it is considered expired
        if node.last_success.eq(&UNIX_EPOCH) {
            // New node: If it has never been attempted or the attempt time exceeds the new node poll interval, it is considered expired
            // However, if this is the first attempt (last_attempt == last_seen), it is immediately considered expired
            if node.last_attempt == node.last_seen {
                return true; // New node is immediately considered expired, can be polled
            }
            return last_attempt_elapsed > NEW_NODE_POLL_INTERVAL;
        }

        // For nodes that have successfully connected, use the original logic
        last_attempt_elapsed > DEFAULT_STALE_GOOD_TIMEOUT
    }

    /// Check if address is routable
    /// Reference Go version's addressmanager.IsRoutable logic
    fn is_routable(&self, address: &NetAddress) -> bool {
        // Check port
        if address.port == 0 {
            return false;
        }

        match address.ip {
            IpAddr::V4(ipv4) => {
                // IPv4 address routability check
                !ipv4.is_private() &&           // Not private network (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
                !ipv4.is_loopback() &&          // Not loopback address (127.0.0.0/8)
                !ipv4.is_unspecified() &&       // Not unspecified address (0.0.0.0)
                !ipv4.is_multicast() &&         // Not multicast address (224.0.0.0/4)
                !ipv4.is_broadcast() &&         // Not broadcast address (255.255.255.255)
                !ipv4.is_link_local() &&        // Not link local address (169.254.0.0/16)
                // Check specific reserved address ranges
                !(ipv4.octets() == [192, 0, 2, 0] ||     // 192.0.2.0/24 (TEST-NET-1)
                  ipv4.octets() == [198, 51, 100, 0] ||  // 198.51.100.0/24 (TEST-NET-2)
                  ipv4.octets() == [203, 0, 113, 0] ||  // 203.0.113.0/24 (TEST-NET-3)
                  (ipv4.octets()[0] == 198 && ipv4.octets()[1] == 18) || // 198.18.0.0/15 (Benchmarking)
                  ipv4.octets() == [0, 0, 0, 0] ||      // 0.0.0.0
                  ipv4.octets() == [255, 255, 255, 255]) // 255.255.255.255
            }
            IpAddr::V6(ipv6) => {
                // IPv6 address routability check
                !ipv6.is_loopback() &&          // Not loopback address (::1)
                !ipv6.is_unspecified() &&       // Not unspecified address (::)
                !ipv6.is_multicast() &&         // Not multicast address (ff00::/8)
                !ipv6.is_unique_local() &&      // Not unique local address (fc00::/7)
                !ipv6.is_unicast_link_local() && // Not unicast link local address (fe80::/10)
                // Check specific reserved address ranges
                !(ipv6.segments() == [0x2001, 0xdb8, 0, 0, 0, 0, 0, 0] || // 2001:db8::/32 (Documentation)
                  ipv6.segments() == [0x2001, 0x2, 0, 0, 0, 0, 0, 0] ||    // 2001:2::/48 (Benchmarking)
                  ipv6.segments() == [0, 0, 0, 0, 0, 0, 0, 0] ||           // :: (Unspecified)
                  ipv6.segments() == [0, 0, 0, 0, 0, 0, 0, 1]) // ::1 (Loopback)
            }
        }
    }

    /// Shutdown address manager
    pub async fn shutdown(&self) {
        let _ = self.quit_tx.send(()).await;
    }

    /// Get statistics
    pub fn get_stats(&self) -> Arc<CrawlerStats> {
        self.stats.clone()
    }
}

impl Clone for AddressManager {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            peers_file: self.peers_file.clone(),
            quit_tx: self.quit_tx.clone(),
            stats: Arc::clone(&self.stats),
        }
    }
}

impl Drop for AddressManager {
    fn drop(&mut self) {
        // Ensure addresses are saved when exiting
        self.save_peers();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_address_manager_creates_directory() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let test_app_dir = temp_dir.path().join("test_app");
        let test_app_dir_str = test_app_dir.to_string_lossy().to_string();

        // Ensure the directory doesn't exist initially
        assert!(!test_app_dir.exists());

        // Create address manager - this should create the directory
        let manager = AddressManager::new(&test_app_dir_str).unwrap();

        // Verify the directory was created
        assert!(test_app_dir.exists());

        // Verify the peers file path is correct
        let expected_peers_file = test_app_dir.join("peers.json");
        assert_eq!(manager.peers_file, expected_peers_file.to_string_lossy());

        // Test saving peers (this should not fail due to directory issues)
        manager.save_peers();

        // Verify the peers file was created
        assert!(expected_peers_file.exists());
    }

    #[test]
    fn test_save_peers_creates_parent_directory() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let test_app_dir = temp_dir.path().join("nested").join("deep").join("app");
        let test_app_dir_str = test_app_dir.to_string_lossy().to_string();

        // Ensure the nested directory doesn't exist initially
        assert!(!test_app_dir.exists());

        // Create address manager - this should create the nested directory
        let manager = AddressManager::new(&test_app_dir_str).unwrap();

        // Verify the nested directory was created
        assert!(test_app_dir.exists());

        // Test saving peers - this should create the directory structure
        manager.save_peers();

        // Verify the peers file was created in the nested directory
        let expected_peers_file = test_app_dir.join("peers.json");
        assert!(expected_peers_file.exists());
    }
}
