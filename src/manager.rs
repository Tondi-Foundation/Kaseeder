use crate::types::{NetAddress, CrawlerStats};
use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use std::net::IpAddr;

/// 地址管理器配置常量
const PEERS_FILENAME: &str = "peers.json";
const DEFAULT_STALE_GOOD_TIMEOUT: Duration = Duration::from_secs(60 * 60); // 1小时，与Go版本一致
const DEFAULT_STALE_BAD_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60); // 2小时，与Go版本一致
const PRUNE_EXPIRE_TIMEOUT: Duration = Duration::from_secs(8 * 60 * 60); // 8小时，与Go版本一致
const PRUNE_ADDRESS_INTERVAL: Duration = Duration::from_secs(60 * 60); // 1小时
const DUMP_ADDRESS_INTERVAL: Duration = Duration::from_secs(10 * 60); // 10分钟
const DEFAULT_MAX_ADDRESSES: usize = 2000;

/// 节点状态
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
            last_success: UNIX_EPOCH, // 从未成功连接
            user_agent: None,
            subnetwork_id: None,
            services: 0,
        }
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.address.ip, self.address.port)
    }
}

/// 地址管理器，对应 Go 版本的 Manager
pub struct AddressManager {
    nodes: DashMap<String, Node>,
    peers_file: String,
    quit_tx: mpsc::Sender<()>,
    stats: Arc<CrawlerStats>,
}

impl AddressManager {
    /// 创建新的地址管理器
    pub fn new(app_dir: &str) -> Result<Self> {
        let peers_file = std::path::Path::new(app_dir).join(PEERS_FILENAME);
        let peers_file = peers_file.to_string_lossy().to_string();
        
        let (quit_tx, _quit_rx) = mpsc::channel(1);
        
        let manager = Self {
            nodes: DashMap::new(),
            peers_file,
            quit_tx,
            stats: Arc::new(CrawlerStats::default()),
        };

        // 加载已保存的节点
        manager.deserialize_peers()?;
        
        // 启动地址处理协程
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            manager_clone.address_handler().await;
        });

        Ok(manager)
    }

    /// 添加地址列表，返回新添加的地址数量
    pub fn add_addresses(&self, addresses: Vec<NetAddress>, _default_port: u16, accept_unroutable: bool) -> usize {
        let mut count = 0;
        
        for address in addresses {
            // 检查端口和可路由性
            if address.port == 0 || (!accept_unroutable && !self.is_routable(&address)) {
                continue;
            }
            
            let addr_str = format!("{}:{}", address.ip, address.port);
            
            if let Some(mut node) = self.nodes.get_mut(&addr_str) {
                // 更新现有节点的最后访问时间
                node.last_seen = SystemTime::now();
            } else {
                // 创建新节点
                let node = Node::new(address);
                self.nodes.insert(addr_str, node);
                count += 1;
            }
        }
        
        count
    }

    /// 获取需要重新测试的地址
    pub fn addresses(&self, threads: u8) -> Vec<NetAddress> {
        let mut addresses = Vec::new();
        let max_count = threads as usize * 3;
        let mut count = 0;
        
        for entry in self.nodes.iter() {
            if count >= max_count {
                break;
            }
            
            let node = entry.value();
            if self.is_stale(node) {
                addresses.push(node.address.clone());
                count += 1;
            }
        }
        
        addresses
    }

    /// 获取地址总数
    pub fn address_count(&self) -> usize {
        self.nodes.len()
    }

    /// 获取所有节点（用于统计）
    pub fn get_all_nodes(&self) -> Vec<Node> {
        self.nodes.iter().map(|entry| entry.value().clone()).collect()
    }

    /// 获取好的地址列表，根据 DNS 查询类型过滤
    pub fn good_addresses(
        &self,
        qtype: u16,
        include_all_subnetworks: bool,
        subnetwork_id: Option<&str>,
    ) -> Vec<NetAddress> {
        let mut addresses = Vec::new();
        let mut count = 0;
        
        // 只支持 A 和 AAAA 记录
        if qtype != 1 && qtype != 28 { // 1=A, 28=AAAA
            return addresses;
        }
        
        for entry in self.nodes.iter() {
            if count >= DEFAULT_MAX_ADDRESSES {
                break;
            }
            
            let node = entry.value();
            
            // 检查子网络
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
            
            // 检查 IP 类型
            let is_ipv4 = node.address.ip.is_ipv4();
            if (qtype == 1 && !is_ipv4) || (qtype == 28 && is_ipv4) {
                continue;
            }
            
            // 检查节点状态
            if !self.is_good(node) {
                continue;
            }
            
            addresses.push(node.address.clone());
            count += 1;
        }
        
        addresses
    }

    /// 更新连接尝试时间
    pub fn attempt(&self, address: &NetAddress) {
        let addr_str = format!("{}:{}", address.ip, address.port);
        
        if let Some(mut node) = self.nodes.get_mut(&addr_str) {
            node.last_attempt = SystemTime::now();
        }
    }

    /// 更新成功连接信息
    pub fn good(&self, address: &NetAddress, user_agent: Option<&str>, subnetwork_id: Option<&str>) {
        let addr_str = format!("{}:{}", address.ip, address.port);
        
        if let Some(mut node) = self.nodes.get_mut(&addr_str) {
            node.user_agent = user_agent.map(|s| s.to_string());
            node.subnetwork_id = subnetwork_id.map(|s| s.to_string());
            node.last_success = SystemTime::now();
        }
    }

    /// 地址处理协程
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

    /// 清理过期和坏的地址
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
        
        // 移除过期的节点
        for key in to_remove {
            self.nodes.remove(&key);
        }
        
        let total = self.nodes.len();
        debug!("Pruned {} addresses. {} left.", pruned, total);
        info!("Known nodes: Good:{} [4:{}, 6:{}] Stale:{} Bad:{}", good, ipv4, ipv6, stale, bad);
    }

    /// 保存地址到文件
    fn save_peers(&self) {
        let nodes: Vec<_> = self.nodes.iter().map(|entry| {
            (entry.key().clone(), entry.value().clone())
        }).collect();
        
        // 创建临时文件
        let tmp_file = format!("{}.new", self.peers_file);
        
        if let Err(e) = std::fs::write(&tmp_file, serde_json::to_string(&nodes).unwrap_or_default()) {
            error!("Failed to write temporary file {}: {}", tmp_file, e);
            return;
        }
        
        // 原子性地重命名文件
        if let Err(e) = std::fs::rename(&tmp_file, &self.peers_file) {
            error!("Failed to rename {} to {}: {}", tmp_file, self.peers_file, e);
            if let Err(e) = std::fs::remove_file(&tmp_file) {
                error!("Failed to remove temporary file {}: {}", tmp_file, e);
            }
        }
    }

    /// 从文件加载地址
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

    /// 检查节点是否过期
    fn is_expired(&self, node: &Node, now: SystemTime) -> bool {
        let last_seen_elapsed = now.duration_since(node.last_seen).unwrap_or_default();
        
        last_seen_elapsed > PRUNE_EXPIRE_TIMEOUT
    }

    /// 检查节点是否良好
    fn is_good(&self, node: &Node) -> bool {
        let now = SystemTime::now();
        let last_success_elapsed = now.duration_since(node.last_success).unwrap_or_default();
        
        last_success_elapsed < DEFAULT_STALE_GOOD_TIMEOUT
    }

    /// 检查节点是否过期
    fn is_stale(&self, node: &Node) -> bool {
        let now = SystemTime::now();
        let last_attempt_elapsed = now.duration_since(node.last_attempt).unwrap_or_default();
        let last_success_elapsed = now.duration_since(node.last_success).unwrap_or_default();
        
        (!node.last_success.eq(&UNIX_EPOCH) && last_attempt_elapsed > DEFAULT_STALE_GOOD_TIMEOUT) ||
        last_attempt_elapsed > DEFAULT_STALE_BAD_TIMEOUT
    }

    /// 检查地址是否可路由
    /// 参考Go版本的addressmanager.IsRoutable逻辑
    fn is_routable(&self, address: &NetAddress) -> bool {
        // 检查端口
        if address.port == 0 {
            return false;
        }
        
        match address.ip {
            IpAddr::V4(ipv4) => {
                // IPv4地址可路由性检查
                !ipv4.is_private() &&           // 不是私有网络 (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
                !ipv4.is_loopback() &&          // 不是回环地址 (127.0.0.0/8)
                !ipv4.is_unspecified() &&       // 不是未指定地址 (0.0.0.0)
                !ipv4.is_multicast() &&         // 不是多播地址 (224.0.0.0/4)
                !ipv4.is_broadcast() &&         // 不是广播地址 (255.255.255.255)
                !ipv4.is_link_local() &&        // 不是链路本地地址 (169.254.0.0/16)
                // 检查特定的保留地址范围
                !(ipv4.octets() == [192, 0, 2, 0] ||     // 192.0.2.0/24 (TEST-NET-1)
                  ipv4.octets() == [198, 51, 100, 0] ||  // 198.51.100.0/24 (TEST-NET-2)
                  ipv4.octets() == [203, 0, 113, 0] ||  // 203.0.113.0/24 (TEST-NET-3)
                  (ipv4.octets()[0] == 198 && ipv4.octets()[1] == 18) || // 198.18.0.0/15 (Benchmarking)
                  ipv4.octets() == [0, 0, 0, 0] ||      // 0.0.0.0
                  ipv4.octets() == [255, 255, 255, 255]) // 255.255.255.255
            }
            IpAddr::V6(ipv6) => {
                // IPv6地址可路由性检查
                !ipv6.is_loopback() &&          // 不是回环地址 (::1)
                !ipv6.is_unspecified() &&       // 不是未指定地址 (::)
                !ipv6.is_multicast() &&         // 不是多播地址 (ff00::/8)
                !ipv6.is_unique_local() &&      // 不是唯一本地地址 (fc00::/7)
                !ipv6.is_unicast_link_local() && // 不是链路本地地址 (fe80::/10)
                // 检查特定的保留地址范围
                !(ipv6.segments() == [0x2001, 0xdb8, 0, 0, 0, 0, 0, 0] || // 2001:db8::/32 (Documentation)
                  ipv6.segments() == [0x2001, 0x2, 0, 0, 0, 0, 0, 0] ||    // 2001:2::/48 (Benchmarking)
                  ipv6.segments() == [0, 0, 0, 0, 0, 0, 0, 0] ||           // :: (Unspecified)
                  ipv6.segments() == [0, 0, 0, 0, 0, 0, 0, 1])             // ::1 (Loopback)
            }
        }
    }

    /// 关闭地址管理器
    pub async fn shutdown(&self) {
        let _ = self.quit_tx.send(()).await;
    }

    /// 获取统计信息
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
        // 确保在退出时保存地址
        self.save_peers();
    }
}
