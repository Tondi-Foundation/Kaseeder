use crate::config::Config;
use crate::manager::AddressManager;
use crate::netadapter::NetworkAdapter;
use crate::kaspa_protocol::{KaspaProtocolHandler, create_consensus_config};
use crate::types::{NetAddress, CrawlerStats};
use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

pub struct Crawler {
    address_manager: Arc<AddressManager>,
    network_adapter: Arc<NetworkAdapter>,
    kaspa_protocol_handler: Arc<KaspaProtocolHandler>,
    config: Config,
    stats: CrawlerStats,
}

impl Crawler {
    pub fn new(
        address_manager: Arc<AddressManager>,
        network_adapter: Arc<NetworkAdapter>,
        config: Config,
    ) -> Self {
        // 创建Kaspa共识配置
        let consensus_config = create_consensus_config(config.testnet, config.net_suffix);
        let kaspa_protocol_handler = Arc::new(KaspaProtocolHandler::new(consensus_config));
        
        Self {
            address_manager,
            network_adapter,
            kaspa_protocol_handler,
            config,
            stats: CrawlerStats::default(),
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting network crawler with {} threads", self.config.threads);
        
        // 初始化已知节点
        self.initialize_known_peers().await?;
        
        // 启动多个爬取线程
        let mut handles = Vec::new();
        
        for thread_id in 0..self.config.threads {
            let address_manager = self.address_manager.clone();
            let network_adapter = self.network_adapter.clone();
            let kaspa_protocol_handler = self.kaspa_protocol_handler.clone();
            let config = self.config.clone();
            
            let handle = tokio::spawn(async move {
                Self::crawler_thread(
                    thread_id, 
                    address_manager, 
                    network_adapter, 
                    kaspa_protocol_handler,
                    config
                ).await
            });
            
            handles.push(handle);
        }
        
        // 启动清理任务
        let address_manager = self.address_manager.clone();
        let cleanup_handle = tokio::spawn(async move {
            Self::cleanup_task(address_manager).await
        });
        
        // 等待所有任务完成
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Crawler thread error: {}", e);
            }
        }
        
        if let Err(e) = cleanup_handle.await {
            error!("Cleanup task error: {}", e);
        }
        
        Ok(())
    }

    async fn initialize_known_peers(&mut self) -> Result<()> {
        if let Some(ref known_peers) = self.config.known_peers {
            let addresses: Vec<NetAddress> = known_peers
                .split(',')
                .filter_map(|peer| NetAddress::from_str(peer.trim()).ok())
                .collect();
            
            if !addresses.is_empty() {
                let added = self.address_manager.add_addresses(addresses.clone());
                info!("Added {} known peers", added);
                
                // 立即尝试连接已知节点
                for address in addresses {
                    self.address_manager.mark_attempt(&address);
                    if let Err(e) = self.kaspa_protocol_handler.poll_node(&address).await {
                        warn!("Failed to poll known peer {}: {}", address.to_string(), e);
                    } else {
                        self.address_manager.mark_success(&address, None, None);
                    }
                }
            }
        }
        
        // 如果有种子节点，也添加到地址管理器
        if let Some(ref seeder) = self.config.seeder {
            if let Ok(address) = NetAddress::from_str(seeder) {
                self.address_manager.add_address(address.clone());
                self.address_manager.mark_attempt(&address);
                
                if let Err(e) = self.kaspa_protocol_handler.poll_node(&address).await {
                    warn!("Failed to poll seeder {}: {}", address.to_string(), e);
                } else {
                    self.address_manager.mark_success(&address, None, None);
                }
            }
        }
        
        Ok(())
    }

    async fn crawler_thread(
        thread_id: u8,
        address_manager: Arc<AddressManager>,
        network_adapter: Arc<NetworkAdapter>,
        kaspa_protocol_handler: Arc<KaspaProtocolHandler>,
        config: Config,
    ) -> Result<()> {
        info!("Crawler thread {} started", thread_id);
        
        let mut interval = interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            // 获取要轮询的地址
            let addresses = address_manager.get_addresses();
            if addresses.is_empty() {
                debug!("Thread {}: No addresses to poll", thread_id);
                continue;
            }
            
            // 选择要轮询的地址
            let addresses_to_poll: Vec<NetAddress> = addresses
                .into_iter()
                .filter(|addr| {
                    let key = addr.to_string();
                    if let Some(entry) = address_manager.get_address_entry(&key) {
                        entry.should_retry(Duration::from_secs(300)) // 5分钟间隔
                    } else {
                        true
                    }
                })
                .take(10) // 每次最多10个
                .collect();
            
            if addresses_to_poll.is_empty() {
                debug!("Thread {}: No addresses ready for polling", thread_id);
                continue;
            }
            
            info!("Thread {}: Polling {} addresses", thread_id, addresses_to_poll.len());
            
            // 并发轮询地址
            let mut handles = Vec::new();
            for address in addresses_to_poll {
                let address_manager = address_manager.clone();
                let kaspa_protocol_handler = kaspa_protocol_handler.clone();
                let config = config.clone();
                
                let handle = tokio::spawn(async move {
                    Self::poll_single_peer(address, address_manager, kaspa_protocol_handler, config).await
                });
                
                handles.push(handle);
            }
            
            // 等待所有轮询完成
            for handle in handles {
                if let Err(e) = handle.await {
                    error!("Thread {}: Polling task error: {}", thread_id, e);
                }
            }
        }
    }

    async fn poll_single_peer(
        address: NetAddress,
        address_manager: Arc<AddressManager>,
        kaspa_protocol_handler: Arc<KaspaProtocolHandler>,
        config: Config,
    ) -> Result<()> {
        address_manager.mark_attempt(&address);
        
        match kaspa_protocol_handler.poll_node(&address).await {
            Ok(new_addresses) => {
                // 轮询成功，添加新地址
                if !new_addresses.is_empty() {
                    let added = address_manager.add_addresses(new_addresses);
                    debug!("Peer {} sent {} new addresses", address.to_string(), added);
                }
                
                // 标记为成功
                address_manager.mark_success(&address, None, None);
            }
            Err(e) => {
                debug!("Failed to poll peer {}: {}", address.to_string(), e);
                
                // 如果失败次数过多，考虑移除地址
                let key = address.to_string();
                if let Some(entry) = address_manager.get_address_entry(&key) {
                    let addr_entry = entry.value();
                    if addr_entry.attempts > 5 && addr_entry.successes == 0 {
                        warn!("Removing persistently failing peer: {}", address.to_string());
                        address_manager.remove_address(&address);
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn cleanup_task(address_manager: Arc<AddressManager>) -> Result<()> {
        let mut interval = interval(Duration::from_secs(3600)); // 每小时清理一次
        
        loop {
            interval.tick().await;
            
            info!("Running cleanup task");
            
            // 清理过期地址
            address_manager.cleanup_stale_addresses(Duration::from_secs(86400 * 7)); // 7天
            
            // 更新统计信息
            let stats = address_manager.get_stats();
            address_manager.update_stats(stats.clone());
            
            info!(
                "Cleanup complete. Total: {}, Active: {}",
                stats.total_nodes,
                stats.active_nodes
            );
        }
    }

    pub fn get_stats(&self) -> &CrawlerStats {
        &self.stats
    }
}

pub async fn start_crawler(
    address_manager: Arc<AddressManager>,
    network_adapter: Arc<NetworkAdapter>,
    config: Config,
) -> Result<()> {
    let mut crawler = Crawler::new(address_manager, network_adapter, config);
    crawler.start().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NetAddress;
    use std::net::IpAddr;

    #[tokio::test]
    async fn test_crawler_creation() {
        let config = Config::new();
        let address_manager = Arc::new(AddressManager::new("test").unwrap());
        let network_adapter = Arc::new(NetworkAdapter::new(&config).unwrap());
        
        let crawler = Crawler::new(address_manager, network_adapter, config);
        assert_eq!(crawler.config.threads, 8);
    }

    #[test]
    fn test_address_retry_logic() {
        let address = NetAddress::new(IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)).into(), 8080);
        
        // 新地址应该可以重试
        assert!(address.should_retry(None, Duration::from_secs(300)));
        
        // 标记尝试后，短时间内不应该重试
        let now = SystemTime::now();
        assert!(!address.should_retry(Some(now), Duration::from_secs(300)));
    }
}
