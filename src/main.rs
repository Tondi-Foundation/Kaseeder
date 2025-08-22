use kaseeder::config::{Config, CliOverrides};
use kaseeder::crawler::Crawler;
use kaseeder::dns::DnsServer;
use kaseeder::errors::{KaseederError, Result};
use kaseeder::grpc::GrpcServer;
use kaseeder::kaspa_protocol::create_consensus_config;
use kaseeder::logging::LoggingConfig;
use kaseeder::manager::AddressManager;
use kaseeder::profiling::ProfilingServer;
use clap::Parser;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "kaseeder")]
#[command(about = "Kaspa DNS Seeder")]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,
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

    // Initialize logging with custom configuration
    let mut logging_config = LoggingConfig::default();
    if let Some(log_level) = &cli.log_level {
        logging_config.level = log_level.clone();
    }
    if let Some(nologfiles) = cli.nologfiles {
        logging_config.no_log_files = nologfiles;
    }

    // Initialize logging system
    kaseeder::logging::init_logging_with_config(logging_config)?;

    info!("Starting Kaspa DNS Seeder...");

    // Load configuration
    let config = if let Some(config_path) = &cli.config {
        Config::load_from_file(config_path)?
    } else {
        Config::try_load_default()?
    };

    // Apply CLI overrides
    let config = config.with_cli_overrides(cli.into())?;

    // Display configuration
    config.display();

    // Validate configuration
    config.validate()?;

    // Create consensus configuration
    let consensus_config = create_consensus_config(config.testnet, config.net_suffix);

    // Create address manager
    let address_manager = Arc::new(AddressManager::new(&config.app_dir)?);
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
        let port: u16 = profile_port.parse()
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
