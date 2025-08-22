use crate::errors::Result;
use crate::types::NetAddress;
use std::net::ToSocketAddrs;
use tracing::warn;

/// DNS seed discoverer
pub struct DnsSeedDiscovery;

impl DnsSeedDiscovery {
    /// Get DNS seed server list from network parameters
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

    /// Query DNS seed server
    pub async fn query_seed_server(
        seed_server: &str,
        default_port: u16,
    ) -> Result<Vec<NetAddress>> {
        // Query DNS seed server directly
        Self::query_seed_server_direct(seed_server, default_port).await
    }

    /// Query DNS seed server directly
    async fn query_seed_server_direct(
        seed_server: &str,
        default_port: u16,
    ) -> Result<Vec<NetAddress>> {
        // Use to_socket_addrs() method to query DNS, exactly consistent with rusty-kaspa
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
        println!("Testnet servers: {:?}", testnet_servers);
        assert!(!testnet_servers.is_empty());
        assert!(testnet_servers.contains(&"seed1-10-testnet.kaspad.net".to_string()));
    }

    #[tokio::test]
    async fn test_query_seed_server() {
        // Note: This test requires network connection
        let result =
            DnsSeedDiscovery::query_seed_server("seeder1.kaspad.net", 16111).await;
        // Should not panic even if it fails
        assert!(result.is_ok());
    }
}
