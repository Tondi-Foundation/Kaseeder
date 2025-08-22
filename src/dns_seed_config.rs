use once_cell::sync::Lazy;
use std::collections::HashMap;

/// DNS seed server configuration
#[derive(Debug, Clone)]
pub struct DnsSeedConfig {
    /// Mainnet DNS seed servers
    pub mainnet_servers: Vec<String>,
    /// Testnet DNS seed servers
    pub testnet_servers: HashMap<u16, Vec<String>>,
}

impl DnsSeedConfig {
    /// Get default configuration
    pub fn default() -> Self {
        Self {
            mainnet_servers: vec![
                // Kaspa Official DNS Seed Servers
                "seeder.kaspad.net".to_string(),
                "seeder.kaspanet.org".to_string(),
                "seeder1.kaspad.net".to_string(),
                "seeder2.kaspad.net".to_string(),
                "seeder3.kaspad.net".to_string(),
                "seeder4.kaspad.net".to_string(),
                // Kaspa Community DNS Seed Servers
                "kaspadns.kaspacalc.net".to_string(),
                "n-mainnet.kaspa.ws".to_string(),
            ],
            testnet_servers: {
                let mut map = HashMap::new();
                map.insert(
                    10,
                    vec![
                        "seed.testnet.kaspa.org".to_string(),
                        "seed1-testnet.kaspad.net".to_string(),
                    ],
                );
                map.insert(
                    11,
                    vec![
                        "seed.testnet.kaspa.org".to_string(),
                        "seed1-testnet.kaspad.net".to_string(),
                    ],
                );
                map
            },
        }
    }

    /// Get mainnet DNS seed servers
    pub fn get_mainnet_servers(&self) -> &[String] {
        &self.mainnet_servers
    }

    /// Get testnet DNS seed servers
    pub fn get_testnet_servers(&self, suffix: u16) -> Option<&[String]> {
        self.testnet_servers.get(&suffix).map(|v| &**v)
    }

    /// Add mainnet DNS seed server
    pub fn add_mainnet_server(&mut self, server: String) {
        if !self.mainnet_servers.contains(&server) {
            self.mainnet_servers.push(server);
        }
    }

    /// Add testnet DNS seed server
    pub fn add_testnet_server(&mut self, suffix: u16, server: String) {
        let servers = self.testnet_servers.entry(suffix).or_insert_with(Vec::new);
        if !servers.contains(&server) {
            servers.push(server);
        }
    }

    /// Remove mainnet DNS seed server
    pub fn remove_mainnet_server(&mut self, server: &str) {
        self.mainnet_servers.retain(|s| s != server);
    }

    /// Remove testnet DNS seed server
    pub fn remove_testnet_server(&mut self, suffix: u16, server: &str) {
        if let Some(servers) = self.testnet_servers.get_mut(&suffix) {
            servers.retain(|s| s != server);
        }
    }
}

// Global DNS seed configuration instance
pub static DNS_SEED_CONFIG: Lazy<DnsSeedConfig> = Lazy::new(DnsSeedConfig::default);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_seed_config() {
        let config = DnsSeedConfig::default();

        // Test mainnet servers
        assert!(!config.get_mainnet_servers().is_empty());
        assert!(
            config
                .get_mainnet_servers()
                .contains(&"seeder1.kaspad.net".to_string())
        );

        // Test testnet servers
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

        // Add server
        config.add_mainnet_server("test.example.com".to_string());
        assert_eq!(config.get_mainnet_servers().len(), original_count + 1);
        assert!(
            config
                .get_mainnet_servers()
                .contains(&"test.example.com".to_string())
        );

        // Remove server
        config.remove_mainnet_server("test.example.com");
        assert_eq!(config.get_mainnet_servers().len(), original_count);
        assert!(
            !config
                .get_mainnet_servers()
                .contains(&"test.example.com".to_string())
        );
    }
}
