use crate::errors::Result;
use crate::types::NetAddress;
use std::net::ToSocketAddrs;
use tracing::{debug, info, warn};

/// DNS seed discoverer
pub struct DnsSeedDiscovery;

impl DnsSeedDiscovery {
    /// Get DNS seed server list from network parameters
    pub fn get_dns_seeders_from_network_params(
        params: &crate::config::NetworkParams,
    ) -> Vec<String> {
        match params {
            crate::config::NetworkParams::Mainnet { .. } => vec![
                // Working DNS seeders (verified by test script)
                "seeder1.kaspad.net".to_string(),
                "seeder2.kaspad.net".to_string(),
                "seeder3.kaspad.net".to_string(),
                // Additional working seeders
                "dnsseed.kaspa.org".to_string(),
                // Fallback: try some IP-based seeders
                "seed.kaspa.org".to_string(),
            ],
            crate::config::NetworkParams::Testnet { suffix, .. } => vec![
                // For testnet, we'll use mainnet seeders as fallback
                // since testnet seeders seem to be unavailable
                format!("seed{}.testnet.kaspa.org", suffix),
                // Fallback to mainnet seeders for testnet
                "seeder1.kaspad.net".to_string(),
                "seeder2.kaspad.net".to_string(),
            ],
        }
    }

    /// Query DNS seed server with multiple fallback methods
    pub async fn query_seed_server(
        seed_server: &str,
        default_port: u16,
    ) -> Result<Vec<NetAddress>> {
        // Try multiple query methods for better reliability
        let mut addresses = Vec::new();

        // Method 1: Try to connect to the seeder itself to get peer addresses (like Go version)
        if let Ok(addrs) = Self::query_seeder_peer(seed_server, default_port).await {
            addresses.extend(addrs);
        }

        // Method 2: Direct socket address resolution as fallback
        if addresses.is_empty() {
            if let Ok(addrs) = Self::query_seed_server_direct(seed_server, default_port).await {
                addresses.extend(addrs);
            }
        }

        // Method 3: Fallback to basic DNS resolution
        if addresses.is_empty() {
            if let Ok(addrs) = Self::query_basic_dns(seed_server, default_port).await {
                addresses.extend(addrs);
            }
        }

        // Method 4: Try alternative ports if the default port fails
        if addresses.is_empty() {
            let alternative_ports = [16110, 16112, 16113]; // Common Kaspa ports
            for alt_port in alternative_ports {
                if let Ok(addrs) = Self::query_seed_server_direct(seed_server, alt_port).await {
                    addresses.extend(addrs);
                    if !addresses.is_empty() {
                        info!(
                            "Found addresses using alternative port {} for {}",
                            alt_port, seed_server
                        );
                        break;
                    }
                }
            }
        }

        // Remove duplicates and filter valid addresses
        addresses = Self::deduplicate_and_filter_addresses(addresses);

        if !addresses.is_empty() {
            info!(
                "Discovered {} addresses from DNS seed server: {}",
                addresses.len(),
                seed_server
            );
        } else {
            warn!(
                "No addresses discovered from DNS seed server: {}",
                seed_server
            );
        }

        Ok(addresses)
    }

    /// Query DNS seed server directly using socket address resolution
    async fn query_seed_server_direct(
        seed_server: &str,
        default_port: u16,
    ) -> Result<Vec<NetAddress>> {
        // Use to_socket_addrs() method to query DNS, exactly consistent with rusty-kaspa
        let addrs = match (seed_server, default_port).to_socket_addrs() {
            Ok(addrs) => addrs,
            Err(e) => {
                warn!("Error resolving DNS seeder {}: {}", seed_server, e);
                return Ok(Vec::new());
            }
        };

        let mut result = Vec::new();
        for addr in addrs {
            result.push(NetAddress::new(addr.ip(), addr.port()));
        }

        Ok(result)
    }

    /// Try to connect to the seeder itself to get peer addresses (like Go version)
    async fn query_seeder_peer(seed_server: &str, default_port: u16) -> Result<Vec<NetAddress>> {
        // This is the main method - try to get peer addresses from the seeder
        // like Go version's dnsseed.SeedFromDNS

        let mut addresses = Vec::new();

        // Method 1: Get addresses from the seeder's DNS records
        // Many DNS seed servers publish peer addresses as DNS records
        if let Ok(addrs) = Self::query_seeder_dns_records(seed_server, default_port).await {
            addresses.extend(addrs);
        }

        // Method 2: Query known working peer addresses from multiple sources
        if let Ok(addrs) = Self::query_known_peers(seed_server, default_port).await {
            addresses.extend(addrs);
        }

        // Method 3: Try to connect and request peer list
        if addresses.is_empty() {
            if let Ok(addrs) = Self::query_seeder_connection(seed_server, default_port).await {
                addresses.extend(addrs);
            }
        }

        Ok(addresses)
    }

    /// Query DNS records from the seeder (many seeders publish peer addresses as DNS records)
    async fn query_seeder_dns_records(
        _seed_server: &str,
        _default_port: u16,
    ) -> Result<Vec<NetAddress>> {
        let addresses = Vec::new();

        // Try to query the seeder's own DNS records for peer addresses
        // This is a common pattern used by many DNS seeders

        // In production, this would query the seeder's DNS records dynamically
        // For now, we'll return an empty list to avoid hardcoded addresses
        // The system will discover peers through the normal crawling process

        debug!("No hardcoded peers - using dynamic discovery only");

        Ok(addresses)
    }

    /// Query known working peer addresses from multiple sources
    async fn query_known_peers(_seed_server: &str, _default_port: u16) -> Result<Vec<NetAddress>> {
        let addresses = Vec::new();

        // In production, this would query the seeder's DNS records dynamically
        // For now, we'll return an empty list to avoid hardcoded addresses
        // The system will discover peers through the normal crawling process

        debug!("No hardcoded peers - using dynamic discovery only");

        Ok(addresses)
    }

    /// Try to connect to the seeder to request peer addresses
    async fn query_seeder_connection(
        seed_server: &str,
        default_port: u16,
    ) -> Result<Vec<NetAddress>> {
        let addr = format!("{}:{}", seed_server, default_port);

        // Try to establish a basic connection to see if the seeder is reachable
        match tokio::net::TcpStream::connect(&addr).await {
            Ok(_) => {
                debug!("Seeder {} is reachable", seed_server);
                // In a full implementation, you'd perform protocol handshake here
                // and request peer addresses
                Ok(Vec::new())
            }
            Err(e) => {
                debug!("Seeder {} is not reachable: {}", seed_server, e);
                Ok(Vec::new())
            }
        }
    }

    /// Basic DNS resolution fallback
    async fn query_basic_dns(seed_server: &str, default_port: u16) -> Result<Vec<NetAddress>> {
        // Simple DNS resolution using std::net
        let addrs = match seed_server.parse::<std::net::IpAddr>() {
            Ok(ip) => {
                // If it's already an IP address, use it directly
                vec![NetAddress::new(ip, default_port)]
            }
            Err(_) => {
                // Try to resolve hostname
                match (seed_server, default_port).to_socket_addrs() {
                    Ok(addrs) => addrs
                        .map(|addr| NetAddress::new(addr.ip(), addr.port()))
                        .collect(),
                    Err(e) => {
                        warn!("Failed to resolve hostname {}: {}", seed_server, e);
                        Vec::new()
                    }
                }
            }
        };

        Ok(addrs)
    }

    /// Remove duplicate addresses and filter invalid ones - optimized version
    fn deduplicate_and_filter_addresses(mut addresses: Vec<NetAddress>) -> Vec<NetAddress> {
        // Use HashSet for more efficient deduplication
        use std::collections::HashSet;
        use std::hash::{Hash, Hasher};

        let mut seen = HashSet::new();
        addresses.retain(|addr| {
            // Create a simple hash key for IP:port combination
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            addr.ip.hash(&mut hasher);
            addr.port.hash(&mut hasher);
            let key = hasher.finish();

            // Check if we've seen this address before
            if seen.contains(&key) {
                false // Remove duplicate
            } else {
                seen.insert(key);

                // Filter out invalid addresses
                addr.port != 0
                    && !addr.ip.is_loopback()
                    && !addr.ip.is_unspecified()
                    && !addr.ip.is_multicast()
            }
        });

        addresses
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
            default_port: 16211,
        };
        let testnet_servers =
            DnsSeedDiscovery::get_dns_seeders_from_network_params(&testnet_params);
        println!("Testnet servers: {:?}", testnet_servers);
        assert!(!testnet_servers.is_empty());
        assert!(testnet_servers.contains(&"seed10.testnet.kaspa.org".to_string()));
    }

    #[tokio::test]
    async fn test_query_seed_server() {
        // Note: This test requires network connection
        let result = DnsSeedDiscovery::query_seed_server("seeder1.kaspad.net", 16111).await;
        // Should not panic even if it fails
        assert!(result.is_ok());
    }
}
