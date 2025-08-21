use anyhow::Result;
use clap::Parser;
use dnsseeder::config::Config;
use dnsseeder::crawler::Crawler;
use dnsseeder::dns::DnsServer;
use dnsseeder::grpc::GrpcServer;
use dnsseeder::kaspa_protocol::create_consensus_config;
use dnsseeder::logging::init_logging;
use dnsseeder::manager::AddressManager;
use dnsseeder::profiling::ProfilingServer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "dnsseeder")]
#[command(about = "Kaspa DNS Seeder")]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,
    /// Hostname for DNS server
    #[arg(long, default_value = "seed.kaspa.org")]
    host: String,

    /// Nameserver for DNS server
    #[arg(long, default_value = "ns1.kaspa.org")]
    nameserver: String,

    /// Listen address for DNS server
    #[arg(long, default_value = "0.0.0.0:53")]
    listen: String,

    /// gRPC listen address
    #[arg(long, default_value = "0.0.0.0:50051")]
    grpc_listen: String,

    /// Application directory for data storage
    #[arg(long, default_value = "./data")]
    app_dir: String,

    /// Seed node address (IP:port or just IP)
    #[arg(long)]
    seeder: Option<String>,

    /// Known peer addresses (comma-separated)
    #[arg(long)]
    known_peers: Option<String>,

    /// Number of crawler threads
    #[arg(long, default_value = "8")]
    threads: u8,

    /// Minimum protocol version
    #[arg(long, default_value = "0")]
    min_proto_ver: u16,

    /// Minimum user agent version
    #[arg(long)]
    min_ua_ver: Option<String>,

    /// Testnet mode
    #[arg(long)]
    testnet: bool,

    /// Network suffix for testnet
    #[arg(long, default_value = "0")]
    net_suffix: u16,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Disable log files
    #[arg(long)]
    nologfiles: bool,

    /// Profile port
    #[arg(long)]
    profile: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 解析命令行参数
    let cli = Cli::parse();

    // 尝试从配置文件加载配置
    let mut config = if let Some(config_file) = &cli.config {
        info!("Loading configuration from file: {}", config_file);
        Config::load_from_file(config_file)?
    } else {
        info!("No config file specified, trying default locations");
        Config::try_load_default()?
    };

    // 命令行参数覆盖配置文件
    println!("Applying command line overrides...");
    if cli.host != "seed.kaspa.org" {
        println!("  Overriding host: {} -> {}", config.host, cli.host);
        config.host = cli.host.clone();
    }
    if cli.nameserver != "ns1.kaspa.org" {
        println!(
            "  Overriding nameserver: {} -> {}",
            config.nameserver, cli.nameserver
        );
        config.nameserver = cli.nameserver.clone();
    }
    if cli.listen != "0.0.0.0:53" {
        println!("  Overriding listen: {} -> {}", config.listen, cli.listen);
        config.listen = cli.listen.clone();
    }
    if cli.grpc_listen != "0.0.0.0:50051" {
        println!(
            "  Overriding grpc_listen: {} -> {}",
            config.grpc_listen, cli.grpc_listen
        );
        config.grpc_listen = cli.grpc_listen.clone();
    }
    if cli.app_dir != "./data" {
        println!(
            "  Overriding app_dir: {} -> {}",
            config.app_dir, cli.app_dir
        );
        config.app_dir = cli.app_dir.clone();
    }
    if cli.seeder.is_some() {
        println!(
            "  Overriding seeder: {:?} -> {:?}",
            config.seeder, cli.seeder
        );
        config.seeder = cli.seeder.clone();
    }
    if cli.known_peers.is_some() {
        println!(
            "  Overriding known_peers: {:?} -> {:?}",
            config.known_peers, cli.known_peers
        );
        config.known_peers = cli.known_peers.clone();
    }
    if cli.threads != 8 {
        println!(
            "  Overriding threads: {} -> {}",
            config.threads, cli.threads
        );
        config.threads = cli.threads;
    }
    if cli.min_proto_ver != 0 {
        println!(
            "  Overriding min_proto_ver: {} -> {}",
            config.min_proto_ver, cli.min_proto_ver
        );
        config.min_proto_ver = cli.min_proto_ver;
    }
    if cli.min_ua_ver.is_some() {
        println!(
            "  Overriding min_ua_ver: {:?} -> {:?}",
            config.min_ua_ver, cli.min_ua_ver
        );
        config.min_ua_ver = cli.min_ua_ver.clone();
    }
    if cli.testnet {
        println!(
            "  Overriding testnet: {} -> {}",
            config.testnet, cli.testnet
        );
        config.testnet = cli.testnet;
    }
    if cli.net_suffix != 0 {
        println!(
            "  Overriding net_suffix: {} -> {}",
            config.net_suffix, cli.net_suffix
        );
        config.net_suffix = cli.net_suffix;
    }
    if cli.log_level != "info" {
        println!(
            "  Overriding log_level: {} -> {}",
            config.log_level, cli.log_level
        );
        config.log_level = cli.log_level.clone();
    }
    if cli.nologfiles {
        println!(
            "  Overriding nologfiles: {} -> {}",
            config.nologfiles, cli.nologfiles
        );
        config.nologfiles = cli.nologfiles;
    }
    if cli.profile.is_some() {
        println!(
            "  Overriding profile: {:?} -> {:?}",
            config.profile, cli.profile
        );
        config.profile = cli.profile.clone();
    }

    println!("Final configuration:");
    println!("  Host: {}", config.host);
    println!("  Nameserver: {}", config.nameserver);
    println!("  Listen: {}", config.listen);
    println!("  gRPC Listen: {}", config.grpc_listen);
    println!("  App Directory: {}", config.app_dir);
    println!("  Threads: {}", config.threads);
    println!("  Testnet: {}", config.testnet);
    if config.testnet {
        println!("  Network Suffix: {}", config.net_suffix);
    }
    println!("  Log Level: {}", config.log_level);
    println!("  No Log Files: {}", config.nologfiles);
    if let Some(ref profile) = config.profile {
        println!("  Profile Port: {}", profile);
    }

    // 验证配置
    config.validate()?;

    // 显示配置信息
    config.display();

    // 初始化日志
    init_logging(
        &config.log_level,
        if config.nologfiles {
            None
        } else {
            Some("dnsseeder.log")
        },
    )?;

    // 显示版本信息
    info!("Version {}", env!("CARGO_PKG_VERSION"));

    // 使用从配置文件加载的配置
    let config = Arc::new(config);

    // 创建按网络类型命名空间的应用目录
    let network_name = config.get_network_name();
    let namespaced_app_dir = std::path::Path::new(&config.app_dir).join(network_name);
    let app_dir_str = namespaced_app_dir.to_string_lossy().to_string();

    // 确保应用目录存在
    std::fs::create_dir_all(&namespaced_app_dir)?;
    info!("Created application directory: {}", app_dir_str);

    // 创建地址管理器
    let address_manager = Arc::new(AddressManager::new(&app_dir_str)?);

    // 创建共识配置
    let consensus_config = create_consensus_config(config.testnet, config.net_suffix);

    // 创建爬虫
    let mut crawler = Crawler::new(
        address_manager.clone(),
        consensus_config.clone(),
        config.clone(),
    )?;

    // 启动性能分析服务器（如果启用）
    let profiling_server = if let Some(profile_port) = &config.profile {
        let port = profile_port.parse::<u16>().unwrap_or(8080);
        let server = ProfilingServer::new(port);

        // 启动性能分析服务器
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

    // 启动 DNS 服务器
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

    // 启动 gRPC 服务器
    let grpc_server = GrpcServer::new(address_manager.clone());
    let grpc_handle = tokio::spawn(async move {
        if let Err(e) = grpc_server.start(&config.grpc_listen).await {
            error!("gRPC server error: {}", e);
        }
    });

    // 启动爬虫
    let crawler_clone = crawler.clone();
    let crawler_handle = tokio::spawn(async move {
        if let Err(e) = crawler.start().await {
            error!("Crawler error: {}", e);
        }
    });

    // 等待中断信号
    info!("Waiting for interrupt signal...");
    signal::ctrl_c().await?;
    info!("Received interrupt signal, shutting down...");

    // 优雅关闭
    let shutdown_signal = Arc::new(AtomicBool::new(true));
    shutdown_signal.store(false, Ordering::SeqCst);

    // 关闭爬虫
    crawler_clone.shutdown().await;

    // 关闭性能分析服务器
    if let Some(server) = profiling_server {
        if let Err(e) = server.stop().await {
            error!("Failed to stop profiling server: {}", e);
        }
    }

    // 关闭地址管理器
    address_manager.shutdown().await;

    // 等待所有服务完成
    tokio::select! {
        _ = dns_handle => info!("DNS server shutdown complete"),
        _ = grpc_handle => info!("gRPC server shutdown complete"),
        _ = crawler_handle => info!("Crawler shutdown complete"),
    }

    info!("All services shutdown complete");
    Ok(())
}
