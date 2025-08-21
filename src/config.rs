use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub app_dir: String,
    pub known_peers: Option<String>,
    pub host: String,
    pub listen: String,
    pub nameserver: String,
    pub seeder: Option<String>,
    pub profile: Option<String>,
    pub grpc_listen: String,
    pub min_proto_ver: u8,
    pub min_ua_ver: Option<String>,
    pub net_suffix: u16,
    pub threads: u8,
    pub testnet: bool,
}

impl Config {
    pub fn new() -> Self {
        Self {
            app_dir: "~/.dnsseeder".to_string(),
            known_peers: None,
            host: String::new(),
            listen: "127.0.0.1:5354".to_string(),
            nameserver: String::new(),
            seeder: None,
            profile: None,
            grpc_listen: "127.0.0.1:3737".to_string(),
            min_proto_ver: 0,
            min_ua_ver: None,
            net_suffix: 0,
            threads: 8,
            testnet: false,
        }
    }

    pub fn get_app_dir(&self) -> PathBuf {
        let mut path = PathBuf::from(&self.app_dir);
        if path.starts_with("~") {
            if let Some(home) = dirs::home_dir() {
                path = home.join(path.strip_prefix("~").unwrap_or(&path));
            }
        }
        path
    }

    pub fn get_network_params(&self) -> NetworkParams {
        if self.testnet {
            // 根据 net_suffix 决定端口，参考 Go 版本的逻辑
            let default_port = if self.net_suffix == 11 {
                16311 // testnet-11 的特殊端口
            } else {
                16110 // 其他测试网的默认端口
            };
            
            NetworkParams::Testnet {
                suffix: self.net_suffix,
                default_port,
            }
        } else {
            NetworkParams::Mainnet {
                default_port: 16111,
            }
        }
    }

    /// 获取网络名称，用于应用目录命名空间
    pub fn get_network_name(&self) -> String {
        if self.testnet {
            if self.net_suffix == 11 {
                "kaspa-testnet-11".to_string()
            } else if self.net_suffix == 0 {
                "kaspa-testnet-10".to_string()  // 默认 testnet
            } else {
                format!("kaspa-testnet-{}", self.net_suffix)
            }
        } else {
            "kaspa-mainnet".to_string()
        }
    }
}

#[derive(Debug, Clone)]
pub enum NetworkParams {
    Mainnet { default_port: u16 },
    Testnet { suffix: u16, default_port: u16 },
}

impl NetworkParams {
    pub fn default_port(&self) -> u16 {
        match self {
            NetworkParams::Mainnet { default_port } => *default_port,
            NetworkParams::Testnet { default_port, .. } => *default_port,
        }
    }
}
