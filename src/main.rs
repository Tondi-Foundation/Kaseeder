use anyhow::Result;
use clap::Parser;
use kaseeder::config::{Config, CliOverrides};
use kaseeder::crawler::Crawler;
use kaseeder::dns::DnsServer;
use kaseeder::grpc::GrpcServer;
use kaseeder::kaspa_protocol::create_consensus_config;
use kaseeder::logging::init_logging;
use kaseeder::manager::AddressManager;
use kaseeder::profiling::ProfilingServer;
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

    // Load and configure application
    let config = load_and_configure(&cli)?;
    
    // Initialize logging
    init_logging(
        &config.log_level,
        if config.nologfiles {
            None
        } else {
            Some("kaseeder.log")
        },
    )?;

    // Display version and configuration information
    info!("Version {}", env!("CARGO_PKG_VERSION"));
    config.display();

    // Create application directory
    let app_dir = create_app_directory(&config)?;

    // Create and start services
    let services = start_services(&config, &app_dir).await?;

    // Wait for shutdown signal
    wait_for_shutdown().await?;

    // Graceful shutdown
    shutdown_services(services).await?;

    info!("All services shutdown complete");
    Ok(())
}

/// Load configuration from file and apply CLI overrides
fn load_and_configure(cli: &Cli) -> Result<Arc<Config>> {
    // Try to load configuration from file
    let mut config = if let Some(config_file) = &cli.config {
        info!("Loading configuration from file: {}", config_file);
        Config::load_from_file(config_file)?
    } else {
        info!("No config file specified, trying default locations");
        Config::try_load_default()?
    };

    // Apply command line overrides
    let overrides = CliOverrides {
        host: cli.host.clone(),
        nameserver: cli.nameserver.clone(),
        listen: cli.listen.clone(),
        grpc_listen: cli.grpc_listen.clone(),
        app_dir: cli.app_dir.clone(),
        seeder: cli.seeder.clone(),
        known_peers: cli.known_peers.clone(),
        threads: cli.threads,
        min_proto_ver: cli.min_proto_ver,
        min_ua_ver: cli.min_ua_ver.clone(),
        testnet: cli.testnet,
        net_suffix: cli.net_suffix,
        log_level: cli.log_level.clone(),
        nologfiles: cli.nologfiles,
        profile: cli.profile.clone(),
    };
    config.apply_cli_overrides(&overrides);

    // Validate configuration
    config.validate()?;

    Ok(Arc::new(config))
}

/// Create application directory with network namespace
fn create_app_directory(config: &Config) -> Result<String> {
    let network_name = config.get_network_name();
    let namespaced_app_dir = std::path::Path::new(&config.app_dir).join(network_name);
    let app_dir_str = namespaced_app_dir.to_string_lossy().to_string();

    // Ensure application directory exists
    std::fs::create_dir_all(&namespaced_app_dir)?;
    info!("Created application directory: {}", app_dir_str);

    Ok(app_dir_str)
}

/// Start all application services
async fn start_services(config: &Arc<Config>, app_dir: &str) -> Result<Services> {
    // Create address manager
    let address_manager = Arc::new(AddressManager::new(app_dir)?);
    address_manager.start();

    // Create consensus configuration
    let consensus_config = create_consensus_config(config.testnet, config.net_suffix);

    // Create crawler
    let mut crawler = Crawler::new(
        address_manager.clone(),
        consensus_config.clone(),
        config.clone(),
    )?;

    // Start performance analysis server (if enabled)
    let profiling_server = if let Some(profile_port) = &config.profile {
        let port = profile_port.parse::<u16>().unwrap_or(8080);
        let server = ProfilingServer::new(port);

        // Start performance analysis server
        let server_clone = server.clone();
        tokio::spawn(async move {
            if let Err(e) = server_clone.start().await {
                error!("Failed to start profiling server: {}", e);
            }
        });

        Some(server)
    } else {
        None
    };

    // Start DNS server
    let dns_server = DnsServer::new(
        config.host.clone(),
        config.nameserver.clone(),
        config.listen.clone(),
        address_manager.clone(),
    );
    let dns_handle = tokio::spawn(async move {
        if let Err(e) = dns_server.start().await {
            error!("DNS server error: {}", e);
        }
    });

    // Start gRPC server - clone config values to avoid lifetime issues
    let grpc_listen = config.grpc_listen.clone();
    let grpc_server = GrpcServer::new(address_manager.clone());
    let grpc_handle = tokio::spawn(async move {
        if let Err(e) = grpc_server.start(&grpc_listen).await {
            error!("gRPC server error: {}", e);
        }
    });

    // Start crawler
    let crawler_clone = crawler.clone();
    let crawler_handle = tokio::spawn(async move {
        if let Err(e) = crawler.start().await {
            error!("Crawler error: {}", e);
        }
    });

    Ok(Services {
        address_manager,
        profiling_server,
        dns_handle,
        grpc_handle,
        crawler_handle,
        crawler: crawler_clone,
    })
}

/// Services container for managing all running services
struct Services {
    address_manager: Arc<AddressManager>,
    profiling_server: Option<ProfilingServer>,
    dns_handle: tokio::task::JoinHandle<()>,
    grpc_handle: tokio::task::JoinHandle<()>,
    crawler_handle: tokio::task::JoinHandle<()>,
    crawler: Crawler,
}

/// Wait for shutdown signal
async fn wait_for_shutdown() -> Result<()> {
    info!("Waiting for interrupt signal...");
    signal::ctrl_c().await?;
    info!("Received interrupt signal, shutting down...");
    Ok(())
}

/// Gracefully shutdown all services
async fn shutdown_services(services: Services) -> Result<()> {
    let shutdown_signal = Arc::new(AtomicBool::new(true));
    shutdown_signal.store(false, Ordering::SeqCst);

    // Shutdown crawler
    services.crawler.shutdown().await;

    // Shutdown performance analysis server
    if let Some(server) = services.profiling_server {
        if let Err(e) = server.stop().await {
            error!("Failed to stop profiling server: {}", e);
        }
    }

    // Shutdown address manager
    services.address_manager.shutdown().await;

    // Wait for all services to complete
    tokio::select! {
        _ = services.dns_handle => info!("DNS server shutdown complete"),
        _ = services.grpc_handle => info!("gRPC server shutdown complete"),
        _ = services.crawler_handle => info!("Crawler shutdown complete"),
    }

    Ok(())
}
