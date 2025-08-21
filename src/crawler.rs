use crate::checkversion::VersionChecker;
use crate::config::Config;
use crate::dns_seed_discovery::DnsSeedDiscovery;
use crate::manager::AddressManager;
use crate::netadapter::DnsseedNetAdapter;
use crate::types::NetAddress;
use anyhow::Result;
use kaspa_consensus_core::config::Config as ConsensusConfig;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, Semaphore};
use tracing::{debug, error, info, warn};

/// 爬虫配置常量

const CRAWLER_SLEEP_INTERVAL: Duration = Duration::from_secs(10);
const MAX_CONCURRENT_POLLS: usize = 100;

/// 性能优化的爬虫管理器
pub struct Crawler {
    address_manager: Arc<AddressManager>,
    net_adapters: Vec<Arc<DnsseedNetAdapter>>,
    config: Arc<Config>,
    quit_tx: mpsc::Sender<()>,
    // 并发控制
    semaphore: Arc<Semaphore>,
    // 性能统计
    stats: Arc<Mutex<CrawlerPerformanceStats>>,
}

/// 爬虫性能统计
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
    /// 创建新的爬虫实例
    pub fn new(
        address_manager: Arc<AddressManager>,
        consensus_config: Arc<ConsensusConfig>,
        config: Arc<Config>,
    ) -> Result<Self> {
        let mut net_adapters = Vec::new();

        // 为每个线程创建网络适配器
        for _ in 0..config.threads {
            let adapter = DnsseedNetAdapter::new(consensus_config.clone())?;
            net_adapters.push(Arc::new(adapter));
        }

        let (quit_tx, _quit_rx) = mpsc::channel(1);

        // 创建信号量来控制并发数量
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

    /// 启动爬虫
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting crawler with {} threads", self.config.threads);

        // 初始化已知节点
        self.initialize_known_peers().await?;

        // 启动主爬取循环
        self.creep_loop().await?;

        Ok(())
    }

    /// 初始化已知节点
    async fn initialize_known_peers(&self) -> Result<()> {
        if let Some(ref known_peers) = self.config.known_peers {
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
                    self.config.get_network_params().default_port(),
                    false, // 不接受不可路由地址
                );

                info!("Added {} known peers", added);

                // 标记已知节点为良好状态
                for peer in peers {
                    self.address_manager.attempt(&peer);
                    self.address_manager.good(&peer, None, None);
                }
            }
        }

        Ok(())
    }

    /// 主爬取循环（优化版本）
    async fn creep_loop(&mut self) -> Result<()> {
        let mut batch_tasks = Vec::new();

        loop {
            let start_time = Instant::now();

            // 获取需要轮询的地址（批量获取以减少锁竞争）
            let batch_size = (self.config.threads as usize).max(20).min(50);
            let peers = self.address_manager.addresses(batch_size as u8);

            if peers.is_empty() && self.address_manager.address_count() == 0 {
                // 如果没有地址，尝试从 DNS 发现种子节点
                self.seed_from_dns().await?;

                // 再次获取地址
                let peers = self.address_manager.addresses(batch_size as u8);
                if peers.is_empty() {
                    debug!(
                        "No addresses to poll, sleeping for {} seconds",
                        CRAWLER_SLEEP_INTERVAL.as_secs()
                    );
                    tokio::time::sleep(CRAWLER_SLEEP_INTERVAL).await;
                    continue;
                }
            }

            // 批量处理节点，使用信号量控制并发
            for (i, addr) in peers.iter().enumerate() {
                let permit = self.semaphore.clone().acquire_owned().await?;
                let net_adapter = self.net_adapters[i % self.net_adapters.len()].clone();
                let address = addr.clone();
                let address_manager = self.address_manager.clone();
                let config = self.config.clone();
                let stats = self.stats.clone();

                let task = tokio::spawn(async move {
                    let poll_start = Instant::now();
                    let result = Self::poll_single_peer_with_stats(
                        net_adapter,
                        address,
                        address_manager,
                        config,
                        stats,
                        poll_start,
                    )
                    .await;

                    // 自动释放信号量许可
                    drop(permit);
                    result
                });

                batch_tasks.push(task);
            }

            // 等待这一批任务完成，并收集结果
            let results = futures::future::join_all(batch_tasks.drain(..)).await;
            let mut successful_polls = 0;
            let mut failed_polls = 0;

            for result in results {
                match result {
                    Ok(Ok(_)) => successful_polls += 1,
                    Ok(Err(e)) => {
                        failed_polls += 1;
                        debug!("Poll failed: {}", e);
                    }
                    Err(e) => {
                        failed_polls += 1;
                        error!("Task join failed: {}", e);
                    }
                }
            }

            // 更新批处理统计
            let batch_duration = start_time.elapsed();
            let mut stats = self.stats.lock().await;
            stats.last_poll_batch_size = peers.len();
            stats.total_polls += successful_polls + failed_polls;
            stats.successful_polls += successful_polls;
            stats.failed_polls += failed_polls;

            info!(
                "Batch completed: {} peers, {} successful, {} failed, took {:?}",
                peers.len(),
                successful_polls,
                failed_polls,
                batch_duration
            );

            // 自适应休眠时间
            let sleep_time = if successful_polls > 0 {
                CRAWLER_SLEEP_INTERVAL / 2 // 成功时缩短休眠
            } else {
                CRAWLER_SLEEP_INTERVAL * 2 // 失败时延长休眠
            };

            tokio::time::sleep(sleep_time).await;
        }
    }

    /// 从DNS种子服务器发现节点
    async fn seed_from_dns(&self) -> Result<()> {
        debug!("Attempting to seed from DNS");

        let network_params = self.config.get_network_params();
        let seed_servers = DnsSeedDiscovery::get_dns_seeders_from_network_params(&network_params);
        let mut discovered_addresses = Vec::new();

        for seed_server in seed_servers {
            match DnsSeedDiscovery::query_seed_server(&seed_server, network_params.default_port())
                .await
            {
                Ok(addresses) => {
                    if !addresses.is_empty() {
                        info!(
                            "Discovered {} addresses from DNS seed server: {}",
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

        if !discovered_addresses.is_empty() {
            let added = self.address_manager.add_addresses(
                discovered_addresses.clone(),
                network_params.default_port(),
                false, // 不接受不可路由地址
            );

            info!("Added {} addresses from DNS seed discovery", added);

            // 标记发现的地址为尝试状态
            for addr in discovered_addresses {
                self.address_manager.attempt(&addr);
            }
        } else {
            debug!("No addresses discovered from DNS");
        }

        Ok(())
    }

    /// 轮询单个节点（带性能统计）
    async fn poll_single_peer_with_stats(
        net_adapter: Arc<DnsseedNetAdapter>,
        address: NetAddress,
        address_manager: Arc<AddressManager>,
        config: Arc<Config>,
        stats: Arc<Mutex<CrawlerPerformanceStats>>,
        start_time: Instant,
    ) -> Result<()> {
        let result =
            Self::poll_single_peer(net_adapter, address.clone(), address_manager, config).await;

        // 更新性能统计
        let poll_duration = start_time.elapsed();
        let mut stats = stats.lock().await;
        let duration_ms = poll_duration.as_millis() as f64;
        stats.average_poll_time_ms = if stats.total_polls == 0 {
            duration_ms
        } else {
            (stats.average_poll_time_ms * stats.total_polls as f64 + duration_ms)
                / (stats.total_polls + 1) as f64
        };

        result
    }

    /// 轮询单个节点
    async fn poll_single_peer(
        net_adapter: Arc<DnsseedNetAdapter>,
        address: NetAddress,
        address_manager: Arc<AddressManager>,
        config: Arc<Config>,
    ) -> Result<()> {
        // 标记尝试连接
        address_manager.attempt(&address);

        let peer_address = format!("{}:{}", address.ip, address.port);
        debug!("Polling peer {}", peer_address);

        // 连接到节点并获取地址
        let (version_msg, addresses) =
            net_adapter
                .connect_and_get_addresses(&peer_address)
                .await
                .map_err(|e| anyhow::anyhow!("Could not connect to {}: {}", peer_address, e))?;

        // 检查协议版本
        if let Err(e) = VersionChecker::check_protocol_version(
            version_msg.protocol_version,
            config.min_proto_ver,
        ) {
            return Err(anyhow::anyhow!(
                "Peer {} protocol version validation failed: {}",
                peer_address,
                e
            ));
        }

        // 检查用户代理版本
        if let Some(ref min_ua_ver) = config.min_ua_ver {
            if let Err(e) = VersionChecker::check_version(min_ua_ver, &version_msg.user_agent) {
                return Err(anyhow::anyhow!(
                    "Peer {} user agent validation failed: {}",
                    peer_address,
                    e
                ));
            }
        }

        // 添加收到的地址
        let added = address_manager.add_addresses(
            addresses.clone(),
            config.get_network_params().default_port(),
            false, // 不接受不可路由地址
        );

        info!(
            "Peer {} ({}) sent {} addresses, {} new",
            peer_address,
            version_msg.user_agent,
            addresses.len(),
            added
        );

        // 标记节点为良好状态
        address_manager.good(&address, Some(&version_msg.user_agent), None);

        Ok(())
    }

    /// 关闭爬虫
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
    /// 获取性能统计信息
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

    /// 估计内存使用量
    fn estimate_memory_usage() -> u64 {
        // 简单的内存使用估计（实际应该使用更精确的方法）
        std::process::id() as u64 * 1024 // 粗略估计
    }

    /// 重置性能统计
    pub async fn reset_performance_stats(&self) {
        let mut stats = self.stats.lock().await;
        *stats = CrawlerPerformanceStats::default();
    }
}

/// 爬虫统计信息
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
