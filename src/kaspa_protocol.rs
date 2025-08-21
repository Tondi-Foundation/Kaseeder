use kaspa_consensus_core::config::{params::Params, Config as ConsensusConfig};
use kaspa_consensus_core::network::{NetworkId, NetworkType};
use std::sync::Arc;

/// Create consensus configuration
pub fn create_consensus_config(testnet: bool, net_suffix: u16) -> Arc<ConsensusConfig> {
    // Create correct network ID based on network type and suffix
    let network_id = if testnet {
        if net_suffix == 0 {
            // Default testnet (testnet-10)
            NetworkId::with_suffix(NetworkType::Testnet, 10)
        } else if net_suffix == 11 {
            // testnet-11 is supported
            NetworkId::with_suffix(NetworkType::Testnet, 11)
        } else {
            // Other testnet suffixes
            NetworkId::with_suffix(NetworkType::Testnet, net_suffix as u32)
        }
    } else {
        NetworkId::new(NetworkType::Mainnet)
    };

    // Create parameters from network ID
    let params = Params::from(network_id);

    // Create consensus configuration
    let config = ConsensusConfig::new(params);

    Arc::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_config_creation() {
        // Test mainnet configuration
        let mainnet_config = create_consensus_config(false, 0);
        let mainnet_name = mainnet_config.params.network_name();
        println!("Mainnet network name: '{}'", mainnet_name);
        assert!(mainnet_name == "kaspa-mainnet");

        // Test testnet configuration
        let testnet_config = create_consensus_config(true, 0);
        let testnet_name = testnet_config.params.network_name();
        println!("Testnet network name: '{}'", testnet_name);
        assert!(testnet_name == "kaspa-testnet-10");

        // Test specific testnet configuration - use supported testnet suffix
        let testnet11_config = create_consensus_config(true, 10);
        let testnet11_name = testnet11_config.params.network_name();
        println!("Testnet10 network name: '{}'", testnet11_name);
        assert!(testnet11_name == "kaspa-testnet-10");
    }
}
