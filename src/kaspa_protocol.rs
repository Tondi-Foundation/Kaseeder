use kaspa_consensus_core::config::{params::Params, Config as ConsensusConfig};
use kaspa_consensus_core::network::{NetworkId, NetworkType};
use std::sync::Arc;

/// 创建共识配置
pub fn create_consensus_config(testnet: bool, net_suffix: u16) -> Arc<ConsensusConfig> {
    // 根据网络类型和后缀创建正确的网络ID
    let network_id = if testnet {
        if net_suffix == 0 {
            // 默认testnet (testnet-10)
            NetworkId::with_suffix(NetworkType::Testnet, 10)
        } else if net_suffix == 11 {
            // testnet-11 是支持的
            NetworkId::with_suffix(NetworkType::Testnet, 11)
        } else {
            // 其他测试网后缀
            NetworkId::with_suffix(NetworkType::Testnet, net_suffix as u32)
        }
    } else {
        NetworkId::new(NetworkType::Mainnet)
    };

    // 从网络ID创建参数
    let params = Params::from(network_id);

    // 创建共识配置
    let config = ConsensusConfig::new(params);

    Arc::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_config_creation() {
        // 测试主网配置
        let mainnet_config = create_consensus_config(false, 0);
        assert!(mainnet_config.params().network_id().is_mainnet());

        // 测试测试网配置
        let testnet_config = create_consensus_config(true, 0);
        assert!(testnet_config.params().network_id().is_testnet());

        // 测试特定测试网配置
        let testnet11_config = create_consensus_config(true, 11);
        assert!(testnet11_config.params().network_id().is_testnet());
    }
}
