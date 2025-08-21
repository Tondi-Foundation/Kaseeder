use std::collections::HashMap;
use once_cell::sync::Lazy;

/// DNS种子服务器配置
#[derive(Debug, Clone)]
pub struct DnsSeedConfig {
    /// 主网DNS种子服务器
    pub mainnet_servers: Vec<String>,
    /// 测试网DNS种子服务器
    pub testnet_servers: HashMap<u16, Vec<String>>,
}

impl DnsSeedConfig {
    /// 获取默认配置
    pub fn default() -> Self {
        Self {
                    mainnet_servers: vec![
            "seeder1.kaspad.net".to_string(),
            "seeder2.kaspad.net".to_string(),
            "seeder3.kaspad.net".to_string(),
            "seeder4.kaspad.net".to_string(),
            "kaspadns.kaspacalc.net".to_string(),
            "n-mainnet.kaspa.ws".to_string(),
            "dnsseeder-kaspa-mainnet.x-con.at".to_string(),
        ],
            testnet_servers: {
                let mut map = HashMap::new();
                map.insert(10, vec![
                    "seed.testnet.kaspa.org".to_string(),
                    "seed1-testnet.kaspad.net".to_string(),
                ]);
                map.insert(11, vec![
                    "seed.testnet.kaspa.org".to_string(),
                    "seed1-testnet.kaspad.net".to_string(),
                ]);
                map
            },
        }
    }

    /// 获取主网DNS种子服务器
    pub fn get_mainnet_servers(&self) -> &[String] {
        &self.mainnet_servers
    }

    /// 获取测试网DNS种子服务器
    pub fn get_testnet_servers(&self, suffix: u16) -> Option<&[String]> {
        self.testnet_servers.get(&suffix).map(|v| &**v)
    }

    /// 添加主网DNS种子服务器
    pub fn add_mainnet_server(&mut self, server: String) {
        if !self.mainnet_servers.contains(&server) {
            self.mainnet_servers.push(server);
        }
    }

    /// 添加测试网DNS种子服务器
    pub fn add_testnet_server(&mut self, suffix: u16, server: String) {
        let servers = self.testnet_servers.entry(suffix).or_insert_with(Vec::new);
        if !servers.contains(&server) {
            servers.push(server);
        }
    }

    /// 移除主网DNS种子服务器
    pub fn remove_mainnet_server(&mut self, server: &str) {
        self.mainnet_servers.retain(|s| s != server);
    }

    /// 移除测试网DNS种子服务器
    pub fn remove_testnet_server(&mut self, suffix: u16, server: &str) {
        if let Some(servers) = self.testnet_servers.get_mut(&suffix) {
            servers.retain(|s| s != server);
        }
    }
}

// 全局DNS种子配置实例
pub static DNS_SEED_CONFIG: Lazy<DnsSeedConfig> = Lazy::new(DnsSeedConfig::default);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_seed_config() {
        let config = DnsSeedConfig::default();
        
        // 测试主网服务器
        assert!(!config.get_mainnet_servers().is_empty());
        assert!(config.get_mainnet_servers().contains(&"seeder1.kaspad.net".to_string()));
        
        // 测试测试网服务器
        let testnet_10 = config.get_testnet_servers(10);
        assert!(testnet_10.is_some());
        assert!(!testnet_10.unwrap().is_empty());
        
        let testnet_11 = config.get_testnet_servers(11);
        assert!(testnet_11.is_some());
        assert!(!testnet_11.unwrap().is_empty());
    }

    #[test]
    fn test_add_remove_servers() {
        let mut config = DnsSeedConfig::default();
        let original_count = config.get_mainnet_servers().len();
        
        // 添加服务器
        config.add_mainnet_server("test.example.com".to_string());
        assert_eq!(config.get_mainnet_servers().len(), original_count + 1);
        assert!(config.get_mainnet_servers().contains(&"test.example.com".to_string()));
        
        // 移除服务器
        config.remove_mainnet_server("test.example.com");
        assert_eq!(config.get_mainnet_servers().len(), original_count);
        assert!(!config.get_mainnet_servers().contains(&"test.example.com".to_string()));
    }
}
