use clap::Parser;
use kaseeder::config::{CliOverrides, Config};
use kaseeder::crawler::Crawler;
use kaseeder::dns::DnsServer;
use kaseeder::errors::{KaseederError, Result};
use kaseeder::grpc::GrpcServer;
use kaseeder::kaspa_protocol::create_consensus_config;
use kaseeder::logging::LoggingConfig;
use kaseeder::manager::AddressManager;
use kaseeder::profiling::ProfilingServer;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;
use tracing::{error, info};

#[derive(Parser, Clone)]
#[command(name = "kaseeder", about = "Kaspa DNS Seeder")]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Diagnose connection to specific peer address (e.g., 192.168.1.1:16110)
    #[arg(short, long)]
    diagnose: Option<String>,

    /// Hostname for DNS server
    #[arg(long)]
    host: Option<String>,

    /// Nameserver for DNS server
    #[arg(long)]
    nameserver: Option<String>,

    /// Listen address for DNS server
    #[arg(long)]
    listen: Option<String>,

    /// gRPC listen address
    #[arg(long)]
    grpc_listen: Option<String>,

    /// Application directory for data storage
    #[arg(long)]
    app_dir: Option<String>,

    /// Seed node address (IP:port or just IP)
    #[arg(long)]
    seeder: Option<String>,

    /// Known peer addresses (comma-separated)
    #[arg(long)]
    known_peers: Option<String>,

    /// Number of crawler threads
    #[arg(long)]
    threads: Option<u8>,

    /// Minimum protocol version
    #[arg(long)]
    min_proto_ver: Option<u16>,

    /// Minimum user agent version
    #[arg(long)]
    min_ua_ver: Option<String>,

    /// Testnet mode
    #[arg(long)]
    testnet: Option<bool>,

    /// Network suffix for testnet
    #[arg(long)]
    net_suffix: Option<u16>,

    /// Log level
    #[arg(long)]
    log_level: Option<String>,

    /// Disable log files
    #[arg(long)]
    nologfiles: Option<bool>,

    /// Profile port
    #[arg(long)]
    profile: Option<String>,
}

impl From<Cli> for CliOverrides {
    fn from(cli: Cli) -> Self {
        Self {
            host: cli.host,
            nameserver: cli.nameserver,
            listen: cli.listen,
            grpc_listen: cli.grpc_listen,
            app_dir: cli.app_dir,
            seeder: cli.seeder,
            known_peers: cli.known_peers,
            threads: cli.threads,
            min_proto_ver: cli.min_proto_ver,
            min_ua_ver: cli.min_ua_ver,
            testnet: cli.testnet,
            net_suffix: cli.net_suffix,
            log_level: cli.log_level,
            nologfiles: cli.nologfiles,
            profile: cli.profile,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Load configuration first to get logging settings
    let config = if let Some(config_path) = &cli.config {
        Config::load_from_file(config_path)?
    } else {
        Config::try_load_default()?
    };

    // Apply CLI overrides
    let config = config.with_cli_overrides(cli.clone().into())?;

    // Initialize logging with configuration
    let mut logging_config = LoggingConfig::default();

    // Apply CLI overrides to logging
    if let Some(log_level) = &cli.log_level {
        logging_config.level = log_level.clone();
    }
    if let Some(nologfiles) = cli.nologfiles {
        logging_config.no_log_files = nologfiles;
    }

    // Apply advanced logging configuration from main config
    logging_config.rotation_strategy = config.advanced_logging.rotation_strategy.clone();
    logging_config.rotation_interval_hours = config.advanced_logging.rotation_interval_hours;
    logging_config.compress_rotated_logs = config.advanced_logging.compress_rotated_logs;
    logging_config.compression_level = config.advanced_logging.compression_level;
    logging_config.include_hostname = config.advanced_logging.include_hostname;
    logging_config.include_pid = config.advanced_logging.include_pid;
    logging_config.custom_format = config.advanced_logging.custom_format.clone();
    logging_config.enable_buffering = config.advanced_logging.enable_buffering;
    logging_config.buffer_size_bytes = config.advanced_logging.buffer_size_bytes;
    logging_config.max_file_size_mb = config.advanced_logging.max_file_size_mb;
    logging_config.max_files = config.advanced_logging.max_rotated_files;

    // Initialize logging system
    kaseeder::logging::init_logging_with_config(logging_config)?;

    info!("Starting Kaspa DNS Seeder...");
    info!(
        "Log rotation: {}",
        config.advanced_logging.rotation_strategy
    );

    // Check if this is a diagnose command first
    if let Some(address) = &cli.diagnose {
        info!("Running network diagnosis for address: {}", address);

        // For diagnosis, use minimal configuration
        let consensus_config = create_consensus_config(false, 0); // Use mainnet defaults

        // Create network adapter for diagnosis
        let net_adapter = kaseeder::netadapter::DnsseedNetAdapter::new(consensus_config)?;

        // Run diagnosis
        let result = net_adapter.diagnose_connection(address).await?;

        println!("{}", result);
        return Ok(());
    }

    // Display configuration
    config.display();

    // Validate configuration
    config.validate()?;

    // Create consensus configuration
    let consensus_config = create_consensus_config(config.testnet, config.net_suffix);

    // Create address manager
    let address_manager = Arc::new(AddressManager::new(&config.app_dir, config.default_port())?);
    address_manager.start();

    // Create crawler
    let mut crawler = Crawler::new(
        address_manager.clone(),
        consensus_config,
        Arc::new(config.clone()),
    )?;

    // Create DNS server
    let dns_server = DnsServer::new(
        config.host.clone(),
        config.nameserver.clone(),
        config.listen.clone(),
        address_manager.clone(),
    );

    // Create gRPC server
    let grpc_server = GrpcServer::new(address_manager.clone());

    // Create profiling server if enabled
    let profiling_server = if let Some(ref profile_port) = config.profile {
        let port: u16 = profile_port
            .parse()
            .map_err(|_| KaseederError::InvalidConfigValue {
                field: "profile".to_string(),
                value: profile_port.clone(),
                expected: "valid port number".to_string(),
            })?;
        Some(ProfilingServer::new(port))
    } else {
        None
    };

    // Start profiling server if enabled
    if let Some(ref profiling_server) = profiling_server {
        profiling_server.start().await?;
    }

    // Create shutdown signal handler
    let shutdown_signal = Arc::new(AtomicBool::new(false));
    let shutdown_signal_clone = shutdown_signal.clone();

    // Handle shutdown signals
    tokio::spawn(async move {
        if let Ok(_) = signal::ctrl_c().await {
            info!("Received Ctrl+C, shutting down...");
            shutdown_signal_clone.store(true, Ordering::SeqCst);
        }
    });

    // Handle SIGTERM
    let shutdown_signal_clone2 = shutdown_signal.clone();
    tokio::spawn(async move {
        if let Ok(mut sigterm) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
            if let Some(()) = sigterm.recv().await {
                info!("Received SIGTERM, shutting down...");
                shutdown_signal_clone2.store(true, Ordering::SeqCst);
            }
        }
    });

    // Start services
    let dns_server = Arc::new(dns_server);
    let grpc_server = Arc::new(grpc_server);
    let grpc_listen = config.grpc_listen.clone();

    // Start DNS server
    let dns_server_clone = dns_server.clone();
    let dns_handle = tokio::spawn(async move {
        if let Err(e) = dns_server_clone.start().await {
            error!("DNS server error: {}", e);
        }
    });

    // Start gRPC server
    let grpc_server_clone = grpc_server.clone();
    let grpc_handle = tokio::spawn(async move {
        if let Err(e) = grpc_server_clone.start(&grpc_listen).await {
            error!("gRPC server error: {}", e);
        }
    });

    // Start crawler
    let crawler_handle = tokio::spawn(async move {
        if let Err(e) = crawler.start().await {
            error!("Crawler error: {}", e);
        }
    });

    // Start address manager background tasks
    let shutdown_signal_clone3 = shutdown_signal.clone();
    let address_manager_handle = tokio::spawn(async move {
        // Keep address manager running
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            if shutdown_signal_clone3.load(Ordering::SeqCst) {
                break;
            }
        }
    });

    info!("All services started successfully");
    info!("DNS server listening on {}", config.listen);
    info!("gRPC server listening on {}", config.grpc_listen);
    if let Some(ref profile_port) = config.profile {
        info!("Profiling server listening on port {}", profile_port);
    }

    // Wait for shutdown signal
    while !shutdown_signal.load(Ordering::SeqCst) {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    info!("Shutting down services...");

    // Graceful shutdown
    dns_handle.abort();
    grpc_handle.abort();
    crawler_handle.abort();
    address_manager_handle.abort();

    info!("Shutdown complete");
    Ok(())
}
