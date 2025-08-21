use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// Network parameters enum
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

/// Configuration file structure
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

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// DNS server hostname
    pub host: String,
    /// DNS server nameserver
    pub nameserver: String,
    /// DNS server listen address
    pub listen: String,
    /// gRPC server listen address
    pub grpc_listen: String,
    /// Application data directory
    pub app_dir: String,
    /// Seed node address
    pub seeder: Option<String>,
    /// Known peer addresses (comma-separated)
    pub known_peers: Option<String>,
    /// Crawler thread count
    pub threads: u8,
    /// Minimum protocol version
    pub min_proto_ver: u16,
    /// Minimum user agent version
    pub min_ua_ver: Option<String>,
    /// Whether it is a testnet
    pub testnet: bool,
    /// Testnet suffix
    pub net_suffix: u16,
    /// Log level
    pub log_level: String,
    /// Whether to disable log files
    pub nologfiles: bool,
    /// Error log file path
    pub error_log_file: Option<String>,
    /// Performance analysis port
    pub profile: Option<String>,
}

impl Config {
    /// Create a new configuration instance
    pub fn new() -> Self {
        Self {
            host: "seed.kaspa.org".to_string(),
            nameserver: "ns1.kaspa.org".to_string(),
            listen: "0.0.0.0:5354".to_string(), // Changed from 53 to 5354 for non-privileged access
            grpc_listen: "0.0.0.0:3737".to_string(), // Changed from 50051 to 3737 for consistency
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
            error_log_file: Some("logs/kaseeder_error.log".to_string()),
            profile: None,
        }
    }

    /// Load configuration from a configuration file
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

        // Create a configuration instance from the configuration file
        let mut config = Self::new();

        // Apply the values from the configuration file (if they exist)
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

    /// Try to load the configuration file from the default location
    pub fn try_load_default() -> Result<Self> {
        let default_paths = [
            "./kaseeder.conf",
            "./config/kaseeder.conf",
            "~/.kaseeder/kaseeder.conf",
            "/etc/kaseeder/kaseeder.conf",
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

    /// Save the configuration to a file
    pub fn save_to_file(&self, config_path: &str) -> Result<()> {
        let config_path = Path::new(config_path);

        // Ensure the parent directory exists
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

    /// Create a default configuration file
    pub fn create_default_config(config_path: &str) -> Result<()> {
        let default_config = Self::new();
        default_config.save_to_file(config_path)
    }

    /// Get network parameters
    pub fn get_network_params(&self) -> NetworkParams {
        if self.testnet {
            // Determine the port based on net_suffix
            let default_port = if self.net_suffix == 11 {
                16311 // Special port for testnet-11
            } else {
                16110 // Default port for other testnets
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

    /// Get network name
    pub fn get_network_name(&self) -> String {
        if self.testnet {
            "testnet".to_string()
        } else {
            "mainnet".to_string()
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        use crate::constants::*;

        // Validate the port number
        if let Some(port_str) = self.listen.split(':').last() {
            if let Ok(port) = port_str.parse::<u16>() {
                if !is_valid_port(port) {
                    return Err(anyhow::anyhow!(
                        "Invalid listen port: {} (must be between {} and {})",
                        port, MIN_PORT, MAX_PORT
                    ));
                }
            } else {
                return Err(anyhow::anyhow!("Invalid listen port: {}", port_str));
            }
        }

        if let Some(port_str) = self.grpc_listen.split(':').last() {
            if let Ok(port) = port_str.parse::<u16>() {
                if !is_valid_port(port) {
                    return Err(anyhow::anyhow!(
                        "Invalid gRPC listen port: {} (must be between {} and {})",
                        port, MIN_PORT, MAX_PORT
                    ));
                }
            } else {
                return Err(anyhow::anyhow!("Invalid gRPC listen port: {}", port_str));
            }
        }

        // Validate the thread count
        if !is_valid_thread_count(self.threads) {
            return Err(anyhow::anyhow!(
                "Invalid thread count: {} (must be between {} and {})",
                self.threads, MIN_THREADS, MAX_THREADS
            ));
        }

        // Validate the network suffix
        if self.testnet && !is_valid_network_suffix(self.net_suffix) {
            return Err(anyhow::anyhow!(
                "Invalid network suffix: {} (must be between 0 and {})",
                self.net_suffix, MAX_NETWORK_SUFFIX
            ));
        }

        // Validate protocol version
        if !is_valid_protocol_version(self.min_proto_ver) {
            return Err(anyhow::anyhow!(
                "Invalid protocol version: {} (must be between {} and {})",
                self.min_proto_ver, MIN_PROTOCOL_VERSION, MAX_PROTOCOL_VERSION
            ));
        }

        Ok(())
    }

    /// Display the configuration information
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

    /// Apply command line overrides to configuration
    pub fn apply_cli_overrides(&mut self, cli: &CliOverrides) {
        if let Some(host) = &cli.host {
            self.host = host.clone();
        }
        if let Some(nameserver) = &cli.nameserver {
            self.nameserver = nameserver.clone();
        }
        if let Some(listen) = &cli.listen {
            self.listen = listen.clone();
        }
        if let Some(grpc_listen) = &cli.grpc_listen {
            self.grpc_listen = grpc_listen.clone();
        }
        if let Some(app_dir) = &cli.app_dir {
            self.app_dir = app_dir.clone();
        }
        if let Some(seeder) = &cli.seeder {
            self.seeder = Some(seeder.clone());
        }
        if let Some(known_peers) = &cli.known_peers {
            self.known_peers = Some(known_peers.clone());
        }
        if let Some(threads) = cli.threads {
            self.threads = threads;
        }
        if let Some(min_proto_ver) = cli.min_proto_ver {
            self.min_proto_ver = min_proto_ver;
        }
        if let Some(min_ua_ver) = &cli.min_ua_ver {
            self.min_ua_ver = Some(min_ua_ver.clone());
        }
        if let Some(testnet) = cli.testnet {
            self.testnet = testnet;
        }
        if let Some(net_suffix) = cli.net_suffix {
            self.net_suffix = net_suffix;
        }
        if let Some(log_level) = &cli.log_level {
            self.log_level = log_level.clone();
        }
        if let Some(nologfiles) = cli.nologfiles {
            self.nologfiles = nologfiles;
        }
        if let Some(profile) = &cli.profile {
            self.profile = Some(profile.clone());
        }
    }
}

/// Command line overrides structure
#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
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
    pub profile: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::tempdir;

    #[test]
    fn test_config_creation() {
        let config = Config::new();
        assert_eq!(config.host, "seed.kaspa.org");
        assert_eq!(config.threads, 8);
        assert!(!config.testnet);
        assert_eq!(config.listen, "0.0.0.0:5354");
        assert_eq!(config.grpc_listen, "0.0.0.0:3737");
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

        // Create a default configuration
        Config::create_default_config(config_path.to_str().unwrap())?;
        assert!(config_path.exists());

        // Load the configuration
        let loaded_config = Config::load_from_file(config_path.to_str().unwrap())?;
        assert_eq!(loaded_config.host, "seed.kaspa.org");

        // Save the configuration
        let mut modified_config = loaded_config;
        modified_config.host = "test.kaspa.org".to_string();
        modified_config.save_to_file(config_path.to_str().unwrap())?;

        // Verify that the modifications have been saved
        let reloaded_config = Config::load_from_file(config_path.to_str().unwrap())?;
        assert_eq!(reloaded_config.host, "test.kaspa.org");

        Ok(())
    }

    #[test]
    fn test_cli_overrides() {
        let mut config = Config::new();
        let overrides = CliOverrides {
            host: Some("test.kaspa.org".to_string()),
            threads: Some(16),
            testnet: Some(true),
            ..Default::default()
        };

        config.apply_cli_overrides(&overrides);
        assert_eq!(config.host, "test.kaspa.org");
        assert_eq!(config.threads, 16);
        assert!(config.testnet);
    }
}
