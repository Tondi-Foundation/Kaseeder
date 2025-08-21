use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// 网络参数枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// 配置文件结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub host: Option<String>,
    pub nameserver: Option<String>,
    pub listen: Option<String>,
    pub grpc_listen: Option<String>,
    pub app_dir: Option<String>,
    pub seeder: Option<String>,
    pub known_peers: Option<String>,
    pub threads: Option<u8>,
    pub min_proto_ver: Option<u16>,
    pub min_ua_ver: Option<String>,
    pub testnet: Option<bool>,
    pub net_suffix: Option<u16>,
    pub log_level: Option<String>,
    pub nologfiles: Option<bool>,
    pub error_log_file: Option<String>,
    pub profile: Option<String>,
}

/// 应用程序配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// DNS 服务器主机名
    pub host: String,
    /// DNS 服务器名称服务器
    pub nameserver: String,
    /// DNS 服务器监听地址
    pub listen: String,
    /// gRPC 服务器监听地址
    pub grpc_listen: String,
    /// 应用程序数据目录
    pub app_dir: String,
    /// 种子节点地址
    pub seeder: Option<String>,
    /// 已知的对等节点地址（逗号分隔）
    pub known_peers: Option<String>,
    /// 爬虫线程数
    pub threads: u8,
    /// 最小协议版本
    pub min_proto_ver: u16,
    /// 最小用户代理版本
    pub min_ua_ver: Option<String>,
    /// 是否为测试网
    pub testnet: bool,
    /// 测试网后缀
    pub net_suffix: u16,
    /// 日志级别
    pub log_level: String,
    /// 是否禁用日志文件
    pub nologfiles: bool,
    /// 错误日志文件路径
    pub error_log_file: Option<String>,
    /// 性能分析端口
    pub profile: Option<String>,
}

impl Config {
    /// 创建新的配置实例
    pub fn new() -> Self {
        Self {
            host: "seed.kaspa.org".to_string(),
            nameserver: "ns1.kaspa.org".to_string(),
            listen: "0.0.0.0:53".to_string(),
            grpc_listen: "0.0.0.0:50051".to_string(),
            app_dir: "./data".to_string(),
            seeder: None,
            known_peers: None,
            threads: 8,
            min_proto_ver: 0,
            min_ua_ver: None,
            testnet: false,
            net_suffix: 0,
            log_level: "info".to_string(),
            nologfiles: false,
            error_log_file: Some("logs/dnsseeder_error.log".to_string()),
            profile: None,
        }
    }

    /// 从配置文件加载配置
    pub fn load_from_file(config_path: &str) -> Result<Self> {
        let config_path = Path::new(config_path);

        if !config_path.exists() {
            return Err(anyhow::anyhow!(
                "Configuration file not found: {}",
                config_path.display()
            ));
        }

        info!("Loading configuration from: {}", config_path.display());

        let config_content = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        let config_file: ConfigFile = toml::from_str(&config_content)
            .with_context(|| "Failed to parse config file as TOML")?;

        // 从配置文件创建配置实例
        let mut config = Self::new();

        // 应用配置文件中的值（如果存在）
        if let Some(host) = config_file.host {
            config.host = host;
        }
        if let Some(nameserver) = config_file.nameserver {
            config.nameserver = nameserver;
        }
        if let Some(listen) = config_file.listen {
            config.listen = listen;
        }
        if let Some(grpc_listen) = config_file.grpc_listen {
            config.grpc_listen = grpc_listen;
        }
        if let Some(app_dir) = config_file.app_dir {
            config.app_dir = app_dir;
        }
        if let Some(seeder) = config_file.seeder {
            config.seeder = Some(seeder);
        }
        if let Some(known_peers) = config_file.known_peers {
            config.known_peers = Some(known_peers);
        }
        if let Some(threads) = config_file.threads {
            config.threads = threads;
        }
        if let Some(min_proto_ver) = config_file.min_proto_ver {
            config.min_proto_ver = min_proto_ver;
        }
        if let Some(min_ua_ver) = config_file.min_ua_ver {
            config.min_ua_ver = Some(min_ua_ver);
        }
        if let Some(testnet) = config_file.testnet {
            config.testnet = testnet;
        }
        if let Some(net_suffix) = config_file.net_suffix {
            config.net_suffix = net_suffix;
        }
        if let Some(log_level) = config_file.log_level {
            config.log_level = log_level;
        }
        if let Some(nologfiles) = config_file.nologfiles {
            config.nologfiles = nologfiles;
        }
        if let Some(error_log_file) = config_file.error_log_file {
            config.error_log_file = Some(error_log_file);
        }
        if let Some(profile) = config_file.profile {
            config.profile = Some(profile);
        }

        info!("Configuration loaded successfully from file");
        Ok(config)
    }

    /// 尝试从默认位置加载配置文件
    pub fn try_load_default() -> Result<Self> {
        let default_paths = [
            "./dnsseeder.conf",
            "./config/dnsseeder.conf",
            "~/.dnsseeder/dnsseeder.conf",
            "/etc/dnsseeder/dnsseeder.conf",
        ];

        for path in &default_paths {
            let expanded_path = if path.starts_with("~/") {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
                home.join(&path[2..])
            } else {
                path.to_string().into()
            };

            if expanded_path.exists() {
                return Self::load_from_file(expanded_path.to_str().unwrap());
            }
        }

        warn!("No configuration file found, using default configuration");
        Ok(Self::new())
    }

    /// 保存配置到文件
    pub fn save_to_file(&self, config_path: &str) -> Result<()> {
        let config_path = Path::new(config_path);

        // 确保父目录存在
        if let Some(parent) = config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }
        }

        let config_file = ConfigFile {
            host: Some(self.host.clone()),
            nameserver: Some(self.nameserver.clone()),
            listen: Some(self.listen.clone()),
            grpc_listen: Some(self.grpc_listen.clone()),
            app_dir: Some(self.app_dir.clone()),
            seeder: self.seeder.clone(),
            known_peers: self.known_peers.clone(),
            threads: Some(self.threads),
            min_proto_ver: Some(self.min_proto_ver),
            min_ua_ver: self.min_ua_ver.clone(),
            testnet: Some(self.testnet),
            net_suffix: Some(self.net_suffix),
            log_level: Some(self.log_level.clone()),
            nologfiles: Some(self.nologfiles),
            error_log_file: self.error_log_file.clone(),
            profile: self.profile.clone(),
        };

        let toml_content = toml::to_string_pretty(&config_file)
            .with_context(|| "Failed to serialize config to TOML")?;

        fs::write(config_path, toml_content)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

        info!("Configuration saved to: {}", config_path.display());
        Ok(())
    }

    /// 创建默认配置文件
    pub fn create_default_config(config_path: &str) -> Result<()> {
        let default_config = Self::new();
        default_config.save_to_file(config_path)
    }

    /// 获取网络参数
    pub fn get_network_params(&self) -> NetworkParams {
        if self.testnet {
            // 根据 net_suffix 决定端口
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

    /// 获取网络名称
    pub fn get_network_name(&self) -> String {
        if self.testnet {
            "testnet".to_string()
        } else {
            "mainnet".to_string()
        }
    }

    /// 验证配置
    pub fn validate(&self) -> Result<()> {
        // 验证端口号
        if let Some(port_str) = self.listen.split(':').last() {
            if let Ok(port) = port_str.parse::<u16>() {
                if port == 0 {
                    return Err(anyhow::anyhow!("Invalid listen port: 0"));
                }
            } else {
                return Err(anyhow::anyhow!("Invalid listen port: {}", port_str));
            }
        }

        if let Some(port_str) = self.grpc_listen.split(':').last() {
            if let Ok(port) = port_str.parse::<u16>() {
                if port == 0 {
                    return Err(anyhow::anyhow!("Invalid gRPC listen port: 0"));
                }
            } else {
                return Err(anyhow::anyhow!("Invalid gRPC listen port: {}", port_str));
            }
        }

        // 验证线程数
        if self.threads == 0 {
            return Err(anyhow::anyhow!("Thread count must be greater than 0"));
        }
        if self.threads > 64 {
            return Err(anyhow::anyhow!("Thread count too high: {}", self.threads));
        }

        // 验证网络后缀
        if self.testnet && self.net_suffix > 99 {
            return Err(anyhow::anyhow!(
                "Network suffix too high: {}",
                self.net_suffix
            ));
        }

        Ok(())
    }

    /// 显示配置信息
    pub fn display(&self) {
        info!("Configuration:");
        info!("  Host: {}", self.host);
        info!("  Nameserver: {}", self.nameserver);
        info!("  Listen: {}", self.listen);
        info!("  gRPC Listen: {}", self.grpc_listen);
        info!("  App Directory: {}", self.app_dir);
        info!("  Threads: {}", self.threads);
        info!("  Testnet: {}", self.testnet);
        if self.testnet {
            info!("  Network Suffix: {}", self.net_suffix);
        }
        info!("  Log Level: {}", self.log_level);
        info!("  No Log Files: {}", self.nologfiles);
        if let Some(ref error_log_file) = self.error_log_file {
            info!("  Error Log File: {}", error_log_file);
        }
        if let Some(ref profile) = self.profile {
            info!("  Profile Port: {}", profile);
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_config_creation() {
        let config = Config::new();
        assert_eq!(config.host, "seed.kaspa.org");
        assert_eq!(config.threads, 8);
        assert!(!config.testnet);
    }

    #[test]
    fn test_network_params() {
        let config = Config::new();
        let params = config.get_network_params();
        assert_eq!(params.default_port(), 16111);

        let mut testnet_config = Config::new();
        testnet_config.testnet = true;
        testnet_config.net_suffix = 10;
        let testnet_params = testnet_config.get_network_params();
        assert_eq!(testnet_params.default_port(), 16110);
    }

    #[test]
    fn test_network_name() {
        let config = Config::new();
        assert_eq!(config.get_network_name(), "mainnet");

        let mut testnet_config = Config::new();
        testnet_config.testnet = true;
        testnet_config.net_suffix = 11;
        assert_eq!(testnet_config.get_network_name(), "testnet");
    }

    #[test]
    fn test_config_validation() {
        let config = Config::new();
        assert!(config.validate().is_ok());

        let mut invalid_config = Config::new();
        invalid_config.threads = 0;
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_config_file_operations() -> Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("test.conf");

        // 创建默认配置
        Config::create_default_config(config_path.to_str().unwrap())?;
        assert!(config_path.exists());

        // 加载配置
        let loaded_config = Config::load_from_file(config_path.to_str().unwrap())?;
        assert_eq!(loaded_config.host, "seed.kaspa.org");

        // 保存配置
        let mut modified_config = loaded_config;
        modified_config.host = "test.kaspa.org".to_string();
        modified_config.save_to_file(config_path.to_str().unwrap())?;

        // 验证修改已保存
        let reloaded_config = Config::load_from_file(config_path.to_str().unwrap())?;
        assert_eq!(reloaded_config.host, "test.kaspa.org");

        Ok(())
    }
}
