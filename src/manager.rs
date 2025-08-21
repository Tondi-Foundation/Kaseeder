use crate::types::{NetAddress, NodeInfo, CrawlerStats};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json;
use sled;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

// 使用rusty-kaspa中的地址管理器
use kaspa_addressmanager::AddressManager as KaspaAddressManager;

// 地址条目，包含状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressEntry {
    pub address: NetAddress,
    pub last_seen: SystemTime,
    pub last_success: Option<SystemTime>,
    pub last_attempt: Option<SystemTime>,
    pub attempts: u32,
    pub successes: u32,
    pub user_agent: Option<String>,
    pub protocol_version: Option<u32>,
}

impl AddressEntry {
    pub fn new(address: NetAddress) -> Self {
        Self {
            address,
            last_seen: SystemTime::now(),
            last_success: None,
            last_attempt: None,
            attempts: 0,
            successes: 0,
            user_agent: None,
            protocol_version: None,
        }
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.address.ip, self.address.port)
    }

    pub fn mark_attempt(&mut self) {
        self.last_attempt = Some(SystemTime::now());
        self.attempts += 1;
    }

    pub fn mark_success(&mut self) {
        self.last_success = Some(SystemTime::now());
        self.successes += 1;
        self.last_seen = SystemTime::now();
    }

    pub fn is_good(&self) -> bool {
        self.successes > 0 && self.attempts < 10
    }

    pub fn should_retry(&self, min_interval: Duration) -> bool {
        if let Some(last_attempt) = self.last_attempt {
            if let Ok(elapsed) = SystemTime::now().duration_since(last_attempt) {
                return elapsed >= min_interval;
            }
        }
        true
    }
}

pub struct AddressManager {
    addresses: DashMap<String, AddressEntry>,
    nodes: DashMap<String, NodeInfo>,
    stats: Arc<RwLock<CrawlerStats>>,
    db: sled::Db,
    // 使用rusty-kaspa的地址管理器
    kaspa_address_manager: Option<Arc<KaspaAddressManager>>,
}

impl AddressManager {
    pub fn new(app_dir: &str) -> anyhow::Result<Self> {
        let db_path = Path::new(app_dir).join("addresses.db");
        std::fs::create_dir_all(&db_path.parent().unwrap_or(Path::new(".")))?;
        
        let db = sled::open(&db_path)?;
        
        let manager = Self {
            addresses: DashMap::new(),
            nodes: DashMap::new(),
            stats: Arc::new(RwLock::new(CrawlerStats::default())),
            db,
            kaspa_address_manager: None,
        };
        
        // 从数据库加载现有地址
        manager.load_addresses()?;
        
        Ok(manager)
    }

    pub fn add_address(&self, address: NetAddress) -> bool {
        let key = address.to_string();
        let is_new = !self.addresses.contains_key(&key);
        
        if is_new {
            let entry = AddressEntry::new(address);
            self.addresses.insert(key.clone(), entry);
            self.save_address(&key, &self.addresses.get(&key).unwrap().value());
            debug!("Added new address: {}", key);
        }
        
        is_new
    }

    pub fn add_addresses(&self, addresses: Vec<NetAddress>) -> usize {
        let mut added = 0;
        for address in addresses {
            if self.add_address(address) {
                added += 1;
            }
        }
        
        if added > 0 {
            info!("Added {} new addresses", added);
        }
        
        added
    }

    pub fn get_addresses(&self) -> Vec<NetAddress> {
        self.addresses
            .iter()
            .map(|entry| entry.value().address.clone())
            .collect()
    }

    pub fn get_good_addresses(&self, limit: usize) -> Vec<NetAddress> {
        let mut addresses: Vec<NetAddress> = self.addresses
            .iter()
            .filter_map(|entry| {
                let entry = entry.value();
                if entry.is_good() {
                    Some(entry.address.clone())
                } else {
                    None
                }
            })
            .collect();
        
        // 按成功率排序
        addresses.sort_by(|a, b| {
            let a_entry = self.addresses.get(&a.to_string()).unwrap();
            let b_entry = self.addresses.get(&b.to_string()).unwrap();
            
            let a_ratio = if a_entry.attempts > 0 { 
                a_entry.successes as f64 / a_entry.attempts as f64 
            } else { 
                0.0 
            };
            let b_ratio = if b_entry.attempts > 0 { 
                b_entry.successes as f64 / b_entry.attempts as f64 
            } else { 
                0.0 
            };
            b_ratio.partial_cmp(&a_ratio).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        addresses.truncate(limit);
        addresses
    }

    pub fn mark_attempt(&self, address: &NetAddress) {
        let key = address.to_string();
        if let Some(mut entry) = self.addresses.get_mut(&key) {
            entry.mark_attempt();
            self.save_address(&key, &*entry);
        }
    }

    pub fn mark_success(&self, address: &NetAddress, user_agent: Option<&str>, protocol_version: Option<u32>) {
        let key = address.to_string();
        if let Some(mut entry) = self.addresses.get_mut(&key) {
            entry.mark_success();
            if let Some(ua) = user_agent {
                entry.user_agent = Some(ua.to_string());
            }
            if let Some(ver) = protocol_version {
                entry.protocol_version = Some(ver);
            }
            self.save_address(&key, &*entry);
        }
    }

    pub fn remove_address(&self, address: &NetAddress) {
        let key = address.to_string();
        if self.addresses.remove(&key).is_some() {
            self.db.remove(key.as_bytes()).ok();
            debug!("Removed address: {}", key);
        }
    }

    pub fn cleanup_stale_addresses(&self, max_age: Duration) {
        let now = SystemTime::now();
        let stale_keys: Vec<String> = self.addresses
            .iter()
            .filter_map(|entry| {
                let entry = entry.value();
                if let Ok(age) = now.duration_since(entry.last_seen) {
                    if age > max_age && entry.successes == 0 {
                        Some(entry.key().clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        
        for key in &stale_keys {
            if let Some((_, _)) = self.addresses.remove(key) {
                self.db.remove(key.as_bytes()).ok();
            }
        }
        
        if !stale_keys.is_empty() {
            info!("Cleaned up {} stale addresses", stale_keys.len());
        }
    }

    pub fn get_stats(&self) -> CrawlerStats {
        let mut stats = CrawlerStats::default();
        stats.total_nodes = self.addresses.len();
        stats.active_nodes = self.addresses
            .iter()
            .filter(|entry| entry.value().is_good())
            .count();
        
        stats
    }

    pub fn update_stats(&self, new_stats: CrawlerStats) {
        if let Ok(mut stats) = self.stats.try_write() {
            *stats = new_stats;
        }
    }

    fn save_address(&self, key: &str, entry: &AddressEntry) {
        if let Ok(data) = serde_json::to_vec(entry) {
            if let Err(e) = self.db.insert(key.as_bytes(), data) {
                warn!("Failed to save address {}: {}", key, e);
            }
        }
    }

    fn load_addresses(&self) -> anyhow::Result<()> {
        let mut loaded = 0;
        
        for result in self.db.iter() {
            let (key, value) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            if let Ok(entry) = serde_json::from_slice::<AddressEntry>(&value) {
                self.addresses.insert(key_str, entry);
                loaded += 1;
            }
        }
        
        info!("Loaded {} addresses from database", loaded);
        Ok(())
    }

    pub fn get_address_count(&self) -> usize {
        self.addresses.len()
    }

    pub fn get_good_address_count(&self) -> usize {
        self.addresses
            .iter()
            .filter(|entry| entry.value().is_good())
            .count()
    }

    // 设置rusty-kaspa地址管理器
    pub fn set_kaspa_address_manager(&mut self, kaspa_manager: Arc<KaspaAddressManager>) {
        self.kaspa_address_manager = Some(kaspa_manager);
    }

    // 获取rusty-kaspa地址管理器
    pub fn get_kaspa_address_manager(&self) -> Option<&Arc<KaspaAddressManager>> {
        self.kaspa_address_manager.as_ref()
    }

    // 提供对addresses字段的访问
    pub fn get_address_entry(&self, key: &str) -> Option<dashmap::mapref::one::Ref<String, AddressEntry>> {
        self.addresses.get(key)
    }
}

impl Drop for AddressManager {
    fn drop(&mut self) {
        // 确保数据库正确关闭
        if let Err(e) = self.db.flush() {
            warn!("Failed to flush database: {}", e);
        }
    }
}
