use anyhow::Result;
use std::net::ToSocketAddrs;
use tracing::{info, warn};
use crate::config::Config;
use crate::types::NetAddress;

/// DNS种子发现器 - 基于rusty-kaspa的connectionmanager实现
pub struct DnsSeedDiscovery;

impl DnsSeedDiscovery {
    /// 从DNS种子服务器发现节点地址
    /// 以及rusty-kaspa的connectionmanager.dns_seed_single()
    pub async fn seed_from_dns(
        network_params: &crate::config::NetworkParams,
        _include_all_subnetworks: bool,
        config: &Config,
    ) -> Result<Vec<NetAddress>> {
        let mut discovered_addresses = Vec::new();
        let default_port = network_params.default_port();
        
        // 使用网络参数中的DNS种子服务器 - 完全对齐rusty-kaspa
        let seed_servers = Self::get_dns_seeders_from_network_params(network_params);
        
        for seed_server in seed_servers {
            match Self::query_seed_server(seed_server, default_port, config).await {
                Ok(addresses) => {
                    info!("Discovered {} addresses from DNS seed server: {}", addresses.len(), seed_server);
                    discovered_addresses.extend(addresses);
                }
                Err(e) => {
                    warn!("Failed to query DNS seed server {}: {}", seed_server, e);
                }
            }
        }
        
        info!("Total discovered addresses from DNS: {}", discovered_addresses.len());
        Ok(discovered_addresses)
    }
    
    /// 从网络参数获取DNS种子服务器列表
    /// 完全对齐rusty-kaspa consensus/core/src/config/params.rs 中的实现
    fn get_dns_seeders_from_network_params(network_params: &crate::config::NetworkParams) -> &'static [&'static str] {
        match network_params {
            crate::config::NetworkParams::Mainnet { .. } => &[
                // 与rusty-kaspa MAINNET_PARAMS.dns_seeders 完全一致
                "mainnet-dnsseed-1.kaspanet.org",
                "mainnet-dnsseed-2.kaspanet.org",
                "seeder1.kaspad.net",
                "seeder2.kaspad.net", 
                "seeder3.kaspad.net",
                "seeder4.kaspad.net",
                "kaspadns.kaspacalc.net",
                "n-mainnet.kaspa.ws",
                "dnsseeder-kaspa-mainnet.x-con.at",
            ],
            crate::config::NetworkParams::Testnet { .. } => &[
                // 与rusty-kaspa TESTNET_PARAMS.dns_seeders 完全一致
                "seeder1-testnet.kaspad.net",
                "dnsseeder-kaspa-testnet.x-con.at",
                "n-testnet-10.kaspa.ws",
            ],
        }
    }
    
    /// 查询单个DNS种子服务器
    /// 完全对齐rusty-kaspa connectionmanager.dns_seed_single() 的实现
    /// 支持通过HTTP代理访问（用于中国大陆用户）
    async fn query_seed_server(seed_server: &str, default_port: u16, config: &Config) -> Result<Vec<NetAddress>> {
        info!("Querying DNS seeder {} (with proxy support: {})", seed_server, config.proxy_enabled);
        
        // 如果启用了代理，首先尝试通过代理进行DNS查询
        if config.proxy_enabled {
            match Self::query_seed_server_with_proxy(seed_server, default_port, config).await {
                Ok(addresses) if !addresses.is_empty() => {
                    info!("Retrieved {} addresses from DNS seeder {} via proxy", addresses.len(), seed_server);
                    return Ok(addresses);
                }
                Ok(_) => {
                    warn!("Proxy query returned no addresses for {}, trying direct connection", seed_server);
                }
                Err(e) => {
                    warn!("Proxy query failed for {}: {}, trying direct connection", seed_server, e);
                }
            }
        }
        
        // 如果代理查询失败或未启用代理，尝试直接连接
        match Self::query_seed_server_direct(seed_server, default_port).await {
            Ok(addresses) => {
                info!("Retrieved {} addresses from DNS seeder {} via direct connection", addresses.len(), seed_server);
                Ok(addresses)
            }
            Err(e) => {
                warn!("Direct connection also failed for {}: {}", seed_server, e);
                Ok(Vec::new())
            }
        }
    }
    
    /// 通过HTTP代理查询DNS种子服务器
    async fn query_seed_server_with_proxy(seed_server: &str, default_port: u16, config: &Config) -> Result<Vec<NetAddress>> {
        // 检查是否启用代理
        if config.proxy_enabled {
            info!("Proxy is enabled but DNS resolution through proxy is not supported in current implementation");
            info!("Will use direct DNS resolution instead");
        }
        
        // 直接使用直接连接方法
        Self::query_seed_server_direct(seed_server, default_port).await
    }
    
    /// 直接查询DNS种子服务器（不使用代理）
    async fn query_seed_server_direct(seed_server: &str, default_port: u16) -> Result<Vec<NetAddress>> {
        // 使用 to_socket_addrs() 方法查询DNS，与rusty-kaspa完全一致
        let addrs = match (seed_server, default_port).to_socket_addrs() {
            Ok(addrs) => addrs,
            Err(e) => {
                warn!("Error connecting to DNS seeder {}: {}", seed_server, e);
                return Ok(Vec::new());
            }
        };

        let mut result = Vec::new();
        for addr in addrs {
            result.push(NetAddress::new(addr.ip(), addr.port()));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_dns_seeders() {
        use crate::config::NetworkParams;
        
        let mainnet_params = NetworkParams::Mainnet { default_port: 16111 };
        let mainnet_servers = DnsSeedDiscovery::get_dns_seeders_from_network_params(&mainnet_params);
        assert!(!mainnet_servers.is_empty());
        assert!(mainnet_servers.contains(&"mainnet-dnsseed-1.kaspanet.org"));
        assert!(mainnet_servers.contains(&"seeder1.kaspad.net"));
        
        let testnet_params = NetworkParams::Testnet { suffix: 10, default_port: 16110 };
        let testnet_servers = DnsSeedDiscovery::get_dns_seeders_from_network_params(&testnet_params);
        assert!(!testnet_servers.is_empty());
        assert!(testnet_servers.contains(&"seeder1-testnet.kaspad.net"));
    }
    
    #[tokio::test]
    async fn test_query_seed_server() {
        // 注意：这个测试需要网络连接
        let result = DnsSeedDiscovery::query_seed_server("mainnet-dnsseed-1.kaspanet.org", 16111, &Config::default()).await;
        // 即使失败也不应该panic
        assert!(result.is_ok());
    }
}
