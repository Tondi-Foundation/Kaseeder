use crate::checkversion::VersionChecker;
use crate::config::Config;
use crate::constants::MAX_CONCURRENT_POLLS;
use crate::dns_seed_discovery::DnsSeedDiscovery;
use crate::errors::{KaseederError, Result};
use crate::manager::AddressManager;
use crate::netadapter::DnsseedNetAdapter;
use crate::types::NetAddress;
use kaspa_consensus_core::config::Config as ConsensusConfig;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, Semaphore};
use tracing::{debug, error, info, warn};

/// Performance-optimized crawler manager
pub struct Crawler {
    address_manager: Arc<AddressManager>,
    net_adapters: Vec<Arc<DnsseedNetAdapter>>,
    config: Arc<Config>,
    quit_tx: mpsc::Sender<()>,
    // Concurrent control
    semaphore: Arc<Semaphore>,
    // Performance statistics
    stats: Arc<Mutex<CrawlerPerformanceStats>>,
}

/// Crawler performance statistics
#[derive(Debug, Default)]
pub struct CrawlerPerformanceStats {
    pub total_polls: u64,
    pub successful_polls: u64,
    pub failed_polls: u64,
    pub total_addresses_found: u64,
    pub average_poll_time_ms: f64,
    pub last_poll_batch_size: usize,
    pub memory_usage_bytes: u64,
}

impl Crawler {
    /// Create a new crawler instance
    pub fn new(
        address_manager: Arc<AddressManager>,
        consensus_config: Arc<ConsensusConfig>,
        config: Arc<Config>,
    ) -> Result<Self> {
        let mut net_adapters = Vec::new();

        // Create network adapter for each thread
        for _ in 0..config.threads {
            let adapter = DnsseedNetAdapter::new(consensus_config.clone())?;
            net_adapters.push(Arc::new(adapter));
        }

        let (quit_tx, _quit_rx) = mpsc::channel(1);

        // Create semaphore to control concurrency
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_POLLS));

        Ok(Self {
            address_manager,
            net_adapters,
            config,
            quit_tx,
            semaphore,
            stats: Arc::new(Mutex::new(CrawlerPerformanceStats::default())),
        })
    }

    /// Start crawler
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting crawler with {} threads", self.config.threads);

        // Initialize known peers
        self.initialize_known_peers().await?;

        // Start main crawl loop
        self.creep_loop().await?;

        Ok(())
    }

    /// Initialize known peers - aligned with Go version logic
    async fn initialize_known_peers(&self) -> Result<()> {
        if let Some(ref known_peers) = self.config.known_peers {
            info!("Processing {} known peers", known_peers.split(',').count());
            
            let peers: Vec<NetAddress> = known_peers
                .split(',')
                .filter_map(|peer_str| {
                    let parts: Vec<&str> = peer_str.split(':').collect();
                    if parts.len() != 2 {
                        warn!("Invalid peer address format: {}", peer_str);
                        return None;
                    }

                    let ip = parts[0].parse().ok()?;
                    let port = parts[1].parse().ok()?;

                    Some(NetAddress::new(ip, port))
                })
                .collect();

            if !peers.is_empty() {
                let added = self.address_manager.add_addresses(
                    peers.clone(),
                    self.config.network_params().default_port(),
                    false, // Do not accept unroutable addresses
                );

                info!("Adding {} known peers to address manager", peers.len());

                // Mark known nodes as good (like Go version)
                for peer in peers {
                    info!("Marking peer {}:{} as good", peer.ip, peer.port);
                    self.address_manager.attempt(&peer);
                    self.address_manager.good(&peer, None, None);
                }
                
                info!("Address manager now has {} total nodes", self.address_manager.address_count());
                info!("Added {} known peers", added);
            }
        }

        Ok(())
    }

    /// Main crawl loop - aligned with Go version logic
    async fn creep_loop(&mut self) -> Result<()> {
        let mut batch_tasks = Vec::new();

        loop {
            // Get addresses to poll like Go version
            let peers = self.address_manager.addresses(self.config.threads);
            info!("Main loop: Addresses() returned {} peers, total nodes: {}", peers.len(), self.address_manager.address_count());



            // Only try DNS seeding if we have no addresses at all
            if peers.is_empty() && self.address_manager.address_count() == 0 {
                info!("No addresses at all, trying DNS seeding...");
                self.seed_from_dns().await?;
                let peers_after_dns = self.address_manager.addresses(self.config.threads);
                info!("After DNS seeding: Addresses() returned {} peers", peers_after_dns.len());
                
                // If still no peers, sleep and retry
                if peers_after_dns.is_empty() {
                    info!("No addresses discovered - waiting 30 seconds before retry");
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    continue;
                }
            } else if peers.is_empty() {
                // If we have nodes but none are stale, wait longer before retrying
                info!("No stale addresses available - waiting 60 seconds before retry");
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }

            // Process peers (like Go version)
            info!("Processing {} peers for polling", peers.len());
            
            // Process peers in parallel with optimized network adapter selection
            for (i, addr) in peers.iter().enumerate() {
                let permit = self.semaphore.clone().acquire_owned().await?;
                // Use round-robin distribution for better load balancing
                let net_adapter = self.net_adapters[i % self.net_adapters.len()].clone();
                let address = addr.clone();
                let address_manager = self.address_manager.clone();
                let config = self.config.clone();

                let task = tokio::spawn(async move {
                    let result = Self::poll_single_peer(
                        net_adapter,
                        address,
                        address_manager,
                        config,
                    )
                    .await;

                    // Automatically release semaphore permit
                    drop(permit);
                    result
                });

                batch_tasks.push(task);
            }

            // Wait for all tasks to complete
            let results = futures::future::join_all(batch_tasks.drain(..)).await;
            
            for result in results {
                match result {
                    Ok(Err(e)) => {
                        debug!("{}", e);
                    }
                    Err(e) => {
                        error!("Task join failed: {}", e);
                    }
                    _ => {}
                }
            }
        }
    }

    /// Discover nodes from DNS seed servers - aligned with Go version dnsseed.SeedFromDNS
    async fn seed_from_dns(&self) -> Result<()> {
        let network_params = self.config.network_params();
        let seed_servers = DnsSeedDiscovery::get_dns_seeders_from_network_params(&network_params);
        let mut discovered_addresses = Vec::new();

        // Query each DNS seed server (like Go version)
        for seed_server in seed_servers {
            match DnsSeedDiscovery::query_seed_server(&seed_server, network_params.default_port())
                .await
            {
                Ok(addresses) => {
                    if !addresses.is_empty() {
                        info!(
                            "DNS seeding found {} addresses from {}",
                            addresses.len(),
                            seed_server
                        );
                        discovered_addresses.extend(addresses);
                    }
                }
                Err(e) => {
                    warn!("Failed to query DNS seed server {}: {}", seed_server, e);
                }
            }
        }

        // Add discovered addresses (like Go version)
        if !discovered_addresses.is_empty() {
            info!("DNS seeding found {} addresses", discovered_addresses.len());
            self.address_manager.add_addresses(
                discovered_addresses,
                network_params.default_port(),
                true, // Accept any addresses from DNS seeding
            );
        }

        Ok(())
    }



    /// Poll a single node with intelligent connection tracking
    async fn poll_single_peer(
        net_adapter: Arc<DnsseedNetAdapter>,
        address: NetAddress,
        address_manager: Arc<AddressManager>,
        config: Arc<Config>,
    ) -> Result<()> {
        // Mark attempt to connect
        address_manager.attempt(&address);

        let peer_address = format!("{}:{}", address.ip, address.port);
        debug!("Polling peer {}", peer_address);

        // Connect to node and get addresses
        let connection_result = net_adapter
            .connect_and_get_addresses(&peer_address)
            .await;

        match connection_result {
            Ok((version_msg, addresses)) => {
                // Record successful connection
                address_manager.record_connection_result(&address, true, None);
                
                // Check protocol version
                if let Err(e) = VersionChecker::check_protocol_version(
                    version_msg.protocol_version,
                    config.min_proto_ver,
                ) {
                    let error_msg = format!("Protocol version validation failed: {}", e);
                    address_manager.record_connection_result(&address, false, Some(error_msg.clone()));
                    return Err(KaseederError::Validation(format!(
                        "Peer {} protocol version validation failed: {}",
                        peer_address,
                        e
                    )));
                }

                // Check user agent version
                if let Some(ref min_ua_ver) = config.min_ua_ver {
                    if let Err(e) = VersionChecker::check_version(min_ua_ver, &version_msg.user_agent) {
                        let error_msg = format!("User agent validation failed: {}", e);
                        address_manager.record_connection_result(&address, false, Some(error_msg.clone()));
                        return Err(KaseederError::Validation(format!(
                            "Peer {} user agent validation failed: {}",
                            peer_address,
                            e
                        )));
                    }
                }

                // Add received addresses
                let added = address_manager.add_addresses(
                    addresses.clone(),
                    config.network_params().default_port(),
                    false, // Do not accept unroutable addresses
                );

                info!(
                    "✅ Peer {} ({}) sent {} addresses, {} new",
                    peer_address,
                    version_msg.user_agent,
                    addresses.len(),
                    added
                );

                // Mark node as good
                address_manager.good(&address, Some(&version_msg.user_agent), None);

                Ok(())
            }
            Err(e) => {
                // Record failed connection with error details
                let error_msg = e.to_string();
                address_manager.record_connection_result(&address, false, Some(error_msg.clone()));
                
                // Classify error type for different handling
                let classified_error = if error_msg.contains("Unimplemented") {
                    "Unsupported protocol"
                } else if error_msg.contains("transport error") {
                    "Network unreachable"
                } else if error_msg.contains("timeout") {
                    "Connection timeout"
                } else {
                    "Connection failed"
                };
                
                debug!("❌ {} - {}: {}", classified_error, peer_address, error_msg);
                
                Err(KaseederError::ConnectionFailed(format!("Could not connect to {}: {}", peer_address, e)))
            }
        }
    }

    /// Shutdown crawler
    pub async fn shutdown(&self) {
        let _ = self.quit_tx.send(()).await;
    }
}

impl Clone for Crawler {
    fn clone(&self) -> Self {
        Self {
            address_manager: self.address_manager.clone(),
            net_adapters: self.net_adapters.clone(),
            config: self.config.clone(),
            quit_tx: self.quit_tx.clone(),
            semaphore: self.semaphore.clone(),
            stats: self.stats.clone(),
        }
    }
}

impl Crawler {
    /// Get performance statistics
    pub async fn get_performance_stats(&self) -> CrawlerPerformanceStats {
        let stats = self.stats.lock().await;
        CrawlerPerformanceStats {
            total_polls: stats.total_polls,
            successful_polls: stats.successful_polls,
            failed_polls: stats.failed_polls,
            total_addresses_found: stats.total_addresses_found,
            average_poll_time_ms: stats.average_poll_time_ms,
            last_poll_batch_size: stats.last_poll_batch_size,
            memory_usage_bytes: Self::estimate_memory_usage(),
        }
    }

    /// Estimate memory usage
    fn estimate_memory_usage() -> u64 {
        // Simple memory usage estimate (should use a more precise method)
        std::process::id() as u64 * 1024 // Rough estimate
    }

    /// Reset performance statistics
    pub async fn reset_performance_stats(&self) {
        let mut stats = self.stats.lock().await;
        *stats = CrawlerPerformanceStats::default();
    }
}

/// Crawler statistics
#[derive(Debug, Clone, Default)]
pub struct CrawlerStats {
    pub total_peers_polled: u64,
    pub successful_polls: u64,
    pub failed_polls: u64,
    pub addresses_discovered: u64,
    pub last_poll_time: Option<std::time::SystemTime>,
}

impl CrawlerStats {
    pub fn new() -> Self {
        Self {
            total_peers_polled: 0,
            successful_polls: 0,
            failed_polls: 0,
            addresses_discovered: 0,
            last_poll_time: None,
        }
    }

    pub fn record_poll_success(&mut self, addresses_count: usize) {
        self.total_peers_polled += 1;
        self.successful_polls += 1;
        self.addresses_discovered += addresses_count as u64;
        self.last_poll_time = Some(std::time::SystemTime::now());
    }

    pub fn record_poll_failure(&mut self) {
        self.total_peers_polled += 1;
        self.failed_polls += 1;
        self.last_poll_time = Some(std::time::SystemTime::now());
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_peers_polled == 0 {
            0.0
        } else {
            self.successful_polls as f64 / self.total_peers_polled as f64
        }
    }
}
