use clap::Parser;
use dnsseeder::config::Config;
use dnsseeder::dns::DnsServer;
use dnsseeder::grpc::GrpcServer;
use dnsseeder::manager::AddressManager;
use dnsseeder::netadapter::NetworkAdapter;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "dnsseeder")]
#[command(about = "DNS seeder for Kaspa network")]
#[command(version)]
struct Cli {
    /// Directory to store data
    #[arg(short, long, default_value = "~/.dnsseeder")]
    appdir: String,

    /// List of already known peer addresses
    #[arg(short, long)]
    peers: Option<String>,

    /// Seed DNS address
    #[arg(short, long)]
    host: String,

    /// Listen on address:port
    #[arg(short, long, default_value = "127.0.0.1:5354")]
    listen: String,

    /// Hostname of nameserver
    #[arg(short, long)]
    nameserver: String,

    /// IP address of a working node, optionally with a port specifier
    #[arg(short, long)]
    seeder: Option<String>,

    /// Enable HTTP profiling on given port
    #[arg(long)]
    profile: Option<String>,

    /// Listen gRPC requests on address:port
    #[arg(long, default_value = "127.0.0.1:3737")]
    grpclisten: String,

    /// Minimum protocol version for nodes
    #[arg(short, long, default_value = "0")]
    minprotocolversion: u8,

    /// Minimum user agent version for nodes
    #[arg(long)]
    minuseragentversion: Option<String>,

    /// Testnet network suffix number
    #[arg(long, default_value = "0")]
    netsuffix: u16,

    /// Disable logging to file
    #[arg(long)]
    nologfiles: bool,

    /// Log level for stdout
    #[arg(long, default_value = "info")]
    loglevel: String,

    /// Number of threads to use for polling
    #[arg(long, default_value = "8")]
    threads: u8,

    /// Testnet mode
    #[arg(long)]
    testnet: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 解析命令行参数
    let cli = Cli::parse();

    // 初始化日志
    dnsseeder::logging::init_logging(&cli.loglevel, if cli.nologfiles { None } else { Some("dnsseeder.log") })?;

    info!("Starting DNSSeeder version {}", env!("CARGO_PKG_VERSION"));

    // 创建配置
    let config = Config {
        app_dir: cli.appdir,
        known_peers: cli.peers,
        host: cli.host,
        listen: cli.listen,
        nameserver: cli.nameserver,
        seeder: cli.seeder,
        profile: cli.profile,
        grpc_listen: cli.grpclisten,
        min_proto_ver: cli.minprotocolversion,
        min_ua_ver: cli.minuseragentversion,
        net_suffix: cli.netsuffix,
        threads: cli.threads,
        testnet: cli.testnet,
    };

    // 创建按网络类型命名空间的应用目录
    let network_name = config.get_network_name();
    let namespaced_app_dir = std::path::Path::new(&config.app_dir).join(network_name);
    let app_dir_str = namespaced_app_dir.to_string_lossy().to_string();

    // 创建地址管理器
    let address_manager = Arc::new(AddressManager::new(&app_dir_str)?);

    // 创建网络适配器
    let network_adapter = Arc::new(NetworkAdapter::new(&config)?);

    // 启动网络爬取任务
    let address_manager_clone = address_manager.clone();
    let network_adapter_clone = network_adapter.clone();
    let config_clone = config.clone();
    
    tokio::spawn(async move {
        if let Err(e) = dnsseeder::crawler::start_crawler(
            address_manager_clone,
            network_adapter_clone,
            config_clone,
        ).await {
            error!("Crawler error: {}", e);
        }
    });

    // 启动DNS服务器
    let dns_server = DnsServer::new(
        address_manager.clone(),
        config.host.clone(),
        config.nameserver.clone(),
        config.listen.clone(),
    );

    let dns_server_handle = tokio::spawn(async move {
        if let Err(e) = dns_server.start().await {
            error!("DNS server error: {}", e);
        }
    });

    // 启动gRPC服务器
    let grpc_server = GrpcServer::new(address_manager.clone());
    let grpc_server_handle = tokio::spawn(async move {
        if let Err(e) = grpc_server.start(&config.grpc_listen).await {
            error!("gRPC server error: {}", e);
        }
    });

    // 启动HTTP分析服务器（如果启用）
    if let Some(profile_port) = &config.profile {
        let profile_port = profile_port.clone();
        let _profile_handle = tokio::spawn(async move {
            // 暂时注释掉profiling功能
            // if let Err(e) = dnsseeder::profiling::start_profiling_server(profile_port).await {
            //     error!("Profiling server error: {}", e);
            // }
            info!("Profiling server would start on port {}", profile_port);
        });
    }

    // 等待中断信号
    info!("Waiting for interrupt signal...");
    signal::ctrl_c().await?;
    info!("Received interrupt signal, shutting down...");

    info!("Gracefully shutting down the seeder...");
    
    // 等待所有任务完成
    let _ = tokio::try_join!(dns_server_handle, grpc_server_handle);
    
    info!("Seeder shutdown complete");
    Ok(())
}
