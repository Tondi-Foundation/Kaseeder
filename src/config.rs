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
            NetworkParams::Testnet {
                suffix: self.net_suffix,
                default_port: 16110,
            }
        } else {
            NetworkParams::Mainnet {
                default_port: 16111,
            }
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
