use crate::types::NetAddress;
use anyhow::Result;
use std::net::ToSocketAddrs;
use tracing::warn;

/// DNS种子发现器
pub struct DnsSeedDiscovery;

impl DnsSeedDiscovery {
    /// 从网络参数获取DNS种子服务器列表
    pub fn get_dns_seeders_from_network_params(
        params: &crate::config::NetworkParams,
    ) -> Vec<String> {
        match params {
            crate::config::NetworkParams::Mainnet { .. } => vec![
                "seeder1.kaspad.net".to_string(),
                "seeder2.kaspad.net".to_string(),
                "seeder3.kaspad.net".to_string(),
                "seeder4.kaspad.net".to_string(),
                "kaspadns.kaspacalc.net".to_string(),
                "n-mainnet.kaspa.ws".to_string(),
                "dnsseeder-kaspa-mainnet.x-con.at".to_string(),
            ],
            crate::config::NetworkParams::Testnet { suffix, .. } => vec![
                format!("seed{}.testnet.kaspa.org", suffix),
                format!(
                    "seed1{}-testnet.kaspad.net",
                    if *suffix > 0 {
                        format!("-{}", suffix)
                    } else {
                        "".to_string()
                    }
                ),
            ],
        }
    }

    /// 查询DNS种子服务器
    pub async fn query_seed_server(
        seed_server: &str,
        default_port: u16,
    ) -> Result<Vec<NetAddress>> {
        // 直接查询DNS种子服务器
        Self::query_seed_server_direct(seed_server, default_port).await
    }

    /// 直接查询DNS种子服务器
    async fn query_seed_server_direct(
        seed_server: &str,
        default_port: u16,
    ) -> Result<Vec<NetAddress>> {
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

        let mainnet_params = NetworkParams::Mainnet {
            default_port: 16111,
        };
        let mainnet_servers =
            DnsSeedDiscovery::get_dns_seeders_from_network_params(&mainnet_params);
        assert!(!mainnet_servers.is_empty());
        assert!(mainnet_servers.contains(&"seeder1.kaspad.net".to_string()));
        assert!(mainnet_servers.contains(&"seeder1.kaspad.net".to_string()));

        let testnet_params = NetworkParams::Testnet {
            suffix: 10,
            default_port: 16110,
        };
        let testnet_servers =
            DnsSeedDiscovery::get_dns_seeders_from_network_params(&testnet_params);
        assert!(!testnet_servers.is_empty());
        assert!(testnet_servers.contains(&"seeder1-testnet.kaspad.net".to_string()));
    }

    #[tokio::test]
    async fn test_query_seed_server() {
        // 注意：这个测试需要网络连接
        let result =
            DnsSeedDiscovery::query_seed_server("seeder1.kaspad.net", 16111).await;
        // 即使失败也不应该panic
        assert!(result.is_ok());
    }
}
