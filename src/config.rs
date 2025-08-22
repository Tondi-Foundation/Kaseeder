use crate::errors::{KaseederError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::{IpAddr, SocketAddr};
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

/// Configuration file structure - aligned with Go version
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
    // Additional fields from Go version
    pub peers: Option<String>, // Alias for known_peers
    pub default_seeder: Option<String>, // Alias for seeder
}

/// Application configuration - aligned with Go version
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
    /// Known peer addresses (comma-separated list)
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
    /// Create a new configuration instance - aligned with Go version defaults
    pub fn new() -> Self {
        Self {
            host: "seed.kaspa.org".to_string(),
            nameserver: "ns1.kaspa.org".to_string(),
            listen: "127.0.0.1:5354".to_string(), // Changed to match Go version default
            grpc_listen: "127.0.0.1:3737".to_string(), // Changed to match Go version default
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

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate hostname
        if self.host.is_empty() {
            return Err(KaseederError::InvalidConfigValue {
                field: "host".to_string(),
                value: self.host.clone(),
                expected: "non-empty hostname".to_string(),
            });
        }

        // Validate nameserver
        if self.nameserver.is_empty() {
            return Err(KaseederError::InvalidConfigValue {
                field: "nameserver".to_string(),
                value: self.nameserver.clone(),
                expected: "non-empty nameserver".to_string(),
            });
        }

        // Validate listen address
        self.validate_socket_addr(&self.listen, "listen")?;

        // Validate gRPC listen address
        self.validate_socket_addr(&self.grpc_listen, "grpc_listen")?;

        // Validate thread count (aligned with Go version: 1-32)
        if self.threads == 0 || self.threads > 32 {
            return Err(KaseederError::InvalidConfigValue {
                field: "threads".to_string(),
                value: self.threads.to_string(),
                expected: "1-32".to_string(),
            });
        }

        // Validate protocol version
        if self.min_proto_ver > 65535 {
            return Err(KaseederError::InvalidConfigValue {
                field: "min_proto_ver".to_string(),
                value: self.min_proto_ver.to_string(),
                expected: "0-65535".to_string(),
            });
        }

        // Validate testnet suffix (aligned with Go version: only support testnet-11)
        if self.testnet && self.net_suffix != 0 {
            if self.net_suffix != 11 {
                return Err(KaseederError::InvalidConfigValue {
                    field: "net_suffix".to_string(),
                    value: self.net_suffix.to_string(),
                    expected: "only testnet-11 (suffix 11) is supported".to_string(),
                });
            }
        }

        // Validate log level
        self.validate_log_level(&self.log_level)?;

        // Validate app directory
        self.validate_directory(&self.app_dir)?;

        // Validate seeder address if provided
        if let Some(ref seeder) = self.seeder {
            self.validate_address(seeder, "seeder")?;
        }

        // Validate known peers if provided
        if let Some(ref peers) = self.known_peers {
            self.validate_peer_list(peers)?;
        }

        // Validate profile port if provided (aligned with Go version: 1024-65535)
        if let Some(ref profile) = self.profile {
            self.validate_profile_port(profile, "profile")?;
        }

        Ok(())
    }

    /// Validate socket address format
    fn validate_socket_addr(&self, addr: &str, field: &str) -> Result<()> {
        addr.parse::<SocketAddr>().map_err(|_| {
            KaseederError::InvalidConfigValue {
                field: field.to_string(),
                value: addr.to_string(),
                expected: "valid socket address (IP:port)".to_string(),
            }
        })?;
        Ok(())
    }

    /// Validate address format (IP:port or just IP)
    fn validate_address(&self, addr: &str, field: &str) -> Result<()> {
        // First try to parse as IP address (IPv4 or IPv6)
        if let Ok(_) = addr.parse::<IpAddr>() {
            return Ok(());
        }
        
        // If that fails, check if it's IP:port format
        if addr.contains(':') {
            // Try to parse as socket address
            if let Ok(_) = addr.parse::<SocketAddr>() {
                return Ok(());
            }
            
            // If socket address parsing fails, try to parse as hostname:port
            let parts: Vec<&str> = addr.split(':').collect();
            if parts.len() == 2 {
                let hostname = parts[0];
                let port = parts[1];
                
                // Validate port
                self.validate_port(port, field)?;
                
                // For hostname validation, we'll be lenient and accept any non-empty string
                if !hostname.is_empty() {
                    return Ok(());
                }
            }
            
            return Err(KaseederError::InvalidConfigValue {
                field: field.to_string(),
                value: addr.to_string(),
                expected: "valid address format (IP:port or hostname:port)".to_string(),
            });
        } else {
            // Just hostname format (no port) - only accept if it looks like a valid hostname
            // Basic hostname validation: must contain at least one dot and valid characters
            if !addr.is_empty() && 
               addr.contains('.') && 
               addr.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') &&
               !addr.starts_with('.') && 
               !addr.ends_with('.') {
                return Ok(());
            }
            
            return Err(KaseederError::InvalidConfigValue {
                field: field.to_string(),
                value: addr.to_string(),
                expected: "valid IP address or hostname".to_string(),
            });
        }
    }

    /// Validate port number
    fn validate_port(&self, port: &str, field: &str) -> Result<()> {
        let port_num: u16 = port.parse().map_err(|_| {
            KaseederError::InvalidConfigValue {
                field: field.to_string(),
                value: port.to_string(),
                expected: "valid port number (1-65535)".to_string(),
            }
        })?;

        if port_num == 0 {
            return Err(KaseederError::InvalidConfigValue {
                field: field.to_string(),
                value: port.to_string(),
                expected: "non-zero port number".to_string(),
            });
        }

        Ok(())
    }

    /// Validate profile port (aligned with Go version: 1024-65535)
    fn validate_profile_port(&self, port: &str, field: &str) -> Result<()> {
        let port_num: u16 = port.parse().map_err(|_| {
            KaseederError::InvalidConfigValue {
                field: field.to_string(),
                value: port.to_string(),
                expected: "valid port number (1024-65535)".to_string(),
            }
        })?;

        if port_num < 1024 || port_num > 65535 {
            return Err(KaseederError::InvalidConfigValue {
                field: field.to_string(),
                value: port.to_string(),
                expected: "port number between 1024 and 65535".to_string(),
            });
        }

        Ok(())
    }

    /// Validate log level
    fn validate_log_level(&self, level: &str) -> Result<()> {
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&level.to_lowercase().as_str()) {
            return Err(KaseederError::InvalidConfigValue {
                field: "log_level".to_string(),
                value: level.to_string(),
                expected: format!("one of: {}", valid_levels.join(", ")),
            });
        }
        Ok(())
    }

    /// Validate directory path
    fn validate_directory(&self, dir: &str) -> Result<()> {
        let path = Path::new(dir);
        if path.exists() && !path.is_dir() {
            return Err(KaseederError::InvalidConfigValue {
                field: "app_dir".to_string(),
                value: dir.to_string(),
                expected: "valid directory path".to_string(),
            });
        }
        Ok(())
    }

    /// Validate peer list format
    fn validate_peer_list(&self, peers: &str) -> Result<()> {
        for peer in peers.split(',') {
            let peer = peer.trim();
            if !peer.is_empty() {
                self.validate_address(peer, "known_peers")?;
            }
        }
        Ok(())
    }

    /// Load configuration from file with validation
    pub fn load_from_file(path: &str) -> Result<Self> {
        let config_file = Self::load_config_file(path)?;
        let mut config = Self::new();
        
        // Apply file configuration
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
        
        // Handle aliases from Go version
        if let Some(seeder) = config_file.seeder.or(config_file.default_seeder) {
            config.seeder = Some(seeder);
        }
        if let Some(known_peers) = config_file.known_peers.or(config_file.peers) {
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

        // Validate the final configuration
        config.validate()?;
        
        Ok(config)
    }

    /// Load configuration file
    fn load_config_file(path: &str) -> Result<ConfigFile> {
        if !Path::new(path).exists() {
            return Err(KaseederError::FileNotFound(path.to_string()));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| KaseederError::Io(e))?;

        let config: ConfigFile = toml::from_str(&content)
            .map_err(|e| KaseederError::Serialization(format!("TOML parse error: {}", e)))?;

        Ok(config)
    }

    /// Create configuration with CLI overrides
    pub fn with_cli_overrides(mut self, overrides: CliOverrides) -> Result<Self> {
        if let Some(host) = overrides.host {
            self.host = host;
        }
        if let Some(nameserver) = overrides.nameserver {
            self.nameserver = nameserver;
        }
        if let Some(listen) = overrides.listen {
            self.listen = listen;
        }
        if let Some(grpc_listen) = overrides.grpc_listen {
            self.grpc_listen = grpc_listen;
        }
        if let Some(app_dir) = overrides.app_dir {
            self.app_dir = app_dir;
        }
        if let Some(seeder) = overrides.seeder {
            self.seeder = Some(seeder);
        }
        if let Some(known_peers) = overrides.known_peers {
            self.known_peers = Some(known_peers);
        }
        if let Some(threads) = overrides.threads {
            self.threads = threads;
        }
        if let Some(min_proto_ver) = overrides.min_proto_ver {
            self.min_proto_ver = min_proto_ver;
        }
        if let Some(min_ua_ver) = overrides.min_ua_ver {
            self.min_ua_ver = Some(min_ua_ver);
        }
        if let Some(testnet) = overrides.testnet {
            self.testnet = testnet;
        }
        if let Some(net_suffix) = overrides.net_suffix {
            self.net_suffix = net_suffix;
        }
        if let Some(log_level) = overrides.log_level {
            self.log_level = log_level;
        }
        if let Some(nologfiles) = overrides.nologfiles {
            self.nologfiles = nologfiles;
        }
        if let Some(profile) = overrides.profile {
            self.profile = Some(profile);
        }

        // Re-validate after applying overrides
        self.validate()?;
        
        Ok(self)
    }

    /// Get network parameters - aligned with Go version
    pub fn network_params(&self) -> NetworkParams {
        if self.testnet {
            NetworkParams::Testnet {
                suffix: self.net_suffix,
                default_port: if self.net_suffix == 11 { 16311 } else { 16211 }, // Aligned with Go version
            }
        } else {
            NetworkParams::Mainnet {
                default_port: 16111, // Default mainnet port
            }
        }
    }

    /// Get default port for the network
    pub fn default_port(&self) -> u16 {
        self.network_params().default_port()
    }

    /// Get network name - aligned with Go version
    pub fn network_name(&self) -> String {
        if self.testnet {
            if self.net_suffix == 11 {
                "kaspa-testnet-11".to_string() // Aligned with Go version
            } else {
                "kaspa-testnet".to_string()
            }
        } else {
            "kaspa-mainnet".to_string()
        }
    }

    /// Save the configuration to a file
    pub fn save_to_file(&self, config_path: &str) -> Result<()> {
        let config_path = Path::new(config_path);

        // Ensure the parent directory exists
        if let Some(parent) = config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| KaseederError::Io(e))?;
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
            peers: None, // Don't save aliases
            default_seeder: None,
        };

        let toml_content = toml::to_string_pretty(&config_file)
            .map_err(|e| KaseederError::Serialization(format!("TOML serialization error: {}", e)))?;

        fs::write(config_path, toml_content)
            .map_err(|e| KaseederError::Io(e))?;

        info!("Configuration saved to: {}", config_path.display());
        Ok(())
    }

    /// Create a default configuration file
    pub fn create_default_config(config_path: &str) -> Result<()> {
        let default_config = Self::new();
        default_config.save_to_file(config_path)
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
                    .ok_or_else(|| KaseederError::Config("Could not determine home directory".to_string()))?;
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

    /// Display the configuration information
    pub fn display(&self) {
        info!("Configuration:");
        info!("  Host: {}", self.host);
        info!("  Nameserver: {}", self.nameserver);
        info!("  Listen: {}", self.listen);
        info!("  gRPC Listen: {}", self.grpc_listen);
        info!("  App Directory: {}", self.app_dir);
        info!("  Threads: {}", self.threads);
        if let Some(ref peers) = self.known_peers {
            info!("  Known Peers: {}", peers);
        }
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
        assert_eq!(config.listen, "127.0.0.1:5354");
        assert_eq!(config.grpc_listen, "127.0.0.1:3737");
    }

    #[test]
    fn test_network_params() {
        let config = Config::new();
        let params = config.network_params();
        assert_eq!(params.default_port(), 16111);

        let mut testnet_config = Config::new();
        testnet_config.testnet = true;
        testnet_config.net_suffix = 10;
        let testnet_params = testnet_config.network_params();
        assert_eq!(testnet_params.default_port(), 16211);
    }

    #[test]
    fn test_network_name() {
        let config = Config::new();
        assert_eq!(config.network_name(), "kaspa-mainnet");

        let mut testnet_config = Config::new();
        testnet_config.testnet = true;
        testnet_config.net_suffix = 11;
        assert_eq!(testnet_config.network_name(), "kaspa-testnet-11");
    }

    #[test]
    fn test_cli_overrides() {
        let config = Config::new();
        let overrides = CliOverrides {
            host: Some("test.kaspa.org".to_string()),
            threads: Some(16),
            testnet: Some(true),
            ..Default::default()
        };

        let modified_config = config.with_cli_overrides(overrides).unwrap();
        assert_eq!(modified_config.host, "test.kaspa.org");
        assert_eq!(modified_config.threads, 16);
        assert!(modified_config.testnet);
    }

    #[test]
    fn test_config_validation() {
        let config = Config::new();
        assert!(config.validate().is_ok());

        let mut invalid_config = Config::new();
        invalid_config.threads = 0;
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = Config::new();
        invalid_config.listen = "invalid-address".to_string();
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = Config::new();
        invalid_config.log_level = "invalid-level".to_string();
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
    fn test_address_validation() {
        let config = Config::new();
        
        // Valid addresses
        assert!(config.validate_address("127.0.0.1", "test").is_ok());
        assert!(config.validate_address("127.0.0.1:8080", "test").is_ok());
        assert!(config.validate_address("::1", "test").is_ok());
        assert!(config.validate_address("[::1]:8080", "test").is_ok());
        
        // Invalid addresses
        assert!(config.validate_address("invalid-ip", "test").is_err());
        assert!(config.validate_address("127.0.0.1:invalid-port", "test").is_err());
    }

    #[test]
    fn test_port_validation() {
        let config = Config::new();
        
        // Valid ports
        assert!(config.validate_port("8080", "test").is_ok());
        assert!(config.validate_port("1", "test").is_ok());
        assert!(config.validate_port("65535", "test").is_ok());
        
        // Invalid ports
        assert!(config.validate_port("0", "test").is_err());
        assert!(config.validate_port("invalid", "test").is_err());
        assert!(config.validate_port("70000", "test").is_err());
    }

    #[test]
    fn test_log_level_validation() {
        let config = Config::new();
        
        // Valid log levels
        assert!(config.validate_log_level("trace").is_ok());
        assert!(config.validate_log_level("debug").is_ok());
        assert!(config.validate_log_level("info").is_ok());
        assert!(config.validate_log_level("warn").is_ok());
        assert!(config.validate_log_level("error").is_ok());
        
        // Invalid log levels
        assert!(config.validate_log_level("invalid").is_err());
        assert!(config.validate_log_level("").is_err());
    }
}
