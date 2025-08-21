use crate::types::NetAddress;
use anyhow::Result;
use kaspa_consensus_core::config::Config as ConsensusConfig;
use kaspa_core::time::unix_now;
use kaspa_p2p_lib::{
    common::ProtocolError,
    make_message,
    pb::{kaspad_message::Payload, RequestAddressesMessage, VersionMessage},
    Adaptor, ConnectionInitializer, Hub, IncomingRoute, KaspadHandshake, KaspadMessagePayloadType,
    PeerKey, Router,
};
use kaspa_utils_tower::counters::TowerConnectionCounters;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tonic::async_trait;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// DNS种子连接初始化器，专门用于地址收集
pub struct DnsSeederConnectionInitializer {
    version_message: VersionMessage,
    addresses_tx: mpsc::Sender<Vec<NetAddress>>,
}

impl DnsSeederConnectionInitializer {
    pub fn new(
        consensus_config: &ConsensusConfig,
        addresses_tx: mpsc::Sender<Vec<NetAddress>>,
    ) -> Self {
        let version_message = VersionMessage {
            protocol_version: 5, // Kaspa 协议版本
            services: 0,
            timestamp: unix_now() as i64,
            address: None,
            id: Vec::from(Uuid::new_v4().as_bytes()),
            user_agent: "/kaspa-dnsseeder:0.1.0/".to_string(),
            disable_relay_tx: true,
            subnetwork_id: None,
            network: consensus_config.params.network_name().to_string(),
        };

        Self {
            version_message,
            addresses_tx,
        }
    }
}

#[async_trait]
impl ConnectionInitializer for DnsSeederConnectionInitializer {
    async fn initialize_connection(&self, router: Arc<Router>) -> Result<(), ProtocolError> {
        // 1. 订阅握手消息并启动路由器
        let mut handshake = KaspadHandshake::new(&router);
        router.start();

        // 2. 执行握手
        debug!("Starting handshake with peer");
        let peer_version = handshake.handshake(self.version_message.clone()).await?;
        info!(
            "Handshake completed with peer. User agent: {}",
            peer_version.user_agent
        );

        // 3. 订阅地址相关消息
        let addresses_receiver = router.subscribe(vec![KaspadMessagePayloadType::Addresses]);

        // 4. 发送 Ready 消息完成握手
        handshake.exchange_ready_messages().await?;

        // 5. 请求地址
        debug!("Requesting addresses from peer");
        let request_addresses = make_message!(
            Payload::RequestAddresses,
            RequestAddressesMessage {
                include_all_subnetworks: true,
                subnetwork_id: None,
            }
        );
        router.enqueue(request_addresses).await?;

        // 6. 启动ping-pong处理协程（保持连接活跃）
        let router_clone = router.clone();
        tokio::spawn(async move {
            if let Err(e) = DnsseedNetAdapter::handle_ping_pong(router_clone).await {
                debug!("Ping-pong handler error: {}", e);
            }
        });

        // 7. 等待地址响应
        // 启动地址响应处理协程
        let addresses_tx = self.addresses_tx.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::handle_addresses_response(addresses_receiver, addresses_tx).await
            {
                error!("Failed to handle addresses response: {}", e);
            }
        });

        Ok(())
    }
}

impl DnsSeederConnectionInitializer {
    async fn handle_addresses_response(
        mut addresses_receiver: IncomingRoute,
        addresses_tx: mpsc::Sender<Vec<NetAddress>>,
    ) -> Result<(), ProtocolError> {
        // 等待地址消息，带超时
        tokio::select! {
            msg_opt = addresses_receiver.recv() => {
                if let Some(msg) = msg_opt {
                    if let Some(Payload::Addresses(addresses_msg)) = msg.payload {
                        debug!("Received {} addresses from peer", addresses_msg.address_list.len());

                        // 转换地址格式
                        let addresses: Vec<NetAddress> = addresses_msg.address_list
                            .into_iter()
                            .filter_map(|addr| {
                                // 解析 IP 地址字节
                                if addr.ip.len() == 4 {
                                    // IPv4
                                    let ip_bytes: [u8; 4] = [addr.ip[0], addr.ip[1], addr.ip[2], addr.ip[3]];
                                    let ipv4 = std::net::Ipv4Addr::from(ip_bytes);
                                    Some(NetAddress::new(std::net::IpAddr::V4(ipv4), addr.port as u16))
                                } else if addr.ip.len() == 16 {
                                    // IPv6
                                    let mut ip_bytes = [0u8; 16];
                                    ip_bytes.copy_from_slice(&addr.ip);
                                    let ipv6 = std::net::Ipv6Addr::from(ip_bytes);
                                    Some(NetAddress::new(std::net::IpAddr::V6(ipv6), addr.port as u16))
                                } else {
                                    warn!("Invalid IP address length: {}", addr.ip.len());
                                    None
                                }
                            })
                            .collect();

                        // 发送地址到主线程
                        if let Err(e) = addresses_tx.send(addresses).await {
                            warn!("Failed to send addresses to main thread: {}", e);
                        }
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                warn!("Timeout waiting for addresses from peer");
            }
        }

        Ok(())
    }
}

/// DNS种子网络适配器，使用真正的kaspa-p2p-lib
pub struct DnsseedNetAdapter {
    adaptor: Arc<Adaptor>,
    addresses_rx: Arc<Mutex<mpsc::Receiver<Vec<NetAddress>>>>,
}

impl DnsseedNetAdapter {
    /// 创建新的网络适配器实例
    pub fn new(consensus_config: Arc<ConsensusConfig>) -> Result<Self> {
        let (addresses_tx, addresses_rx) = mpsc::channel(100);

        let initializer = Arc::new(DnsSeederConnectionInitializer::new(
            &consensus_config,
            addresses_tx,
        ));

        let hub = Hub::new();
        let counters = Arc::new(TowerConnectionCounters::default());

        let adaptor = Adaptor::client_only(hub, initializer, counters);

        Ok(Self {
            adaptor,
            addresses_rx: Arc::new(Mutex::new(addresses_rx)),
        })
    }

    /// 连接到指定地址并获取地址列表
    pub async fn connect_and_get_addresses(
        &self,
        address: &str,
    ) -> Result<(VersionMessage, Vec<NetAddress>)> {
        info!("Connecting to peer: {}", address);

        // 实现指数退避重连策略
        let mut retry_count = 0;
        let max_retries = 3;
        let base_delay = Duration::from_secs(1);

        loop {
            match self.try_connect_peer(address).await {
                Ok((peer_key, version_message, addresses)) => {
                    info!(
                        "Successfully connected to peer: {} (key: {})",
                        address, peer_key
                    );
                    return Ok((version_message, addresses));
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= max_retries {
                        return Err(anyhow::anyhow!(
                            "Failed to connect to peer {} after {} retries: {}",
                            address,
                            max_retries,
                            e
                        ));
                    }

                    let delay = base_delay * 2_u32.pow(retry_count as u32 - 1);
                    warn!(
                        "Connection attempt {} failed for {}: {}. Retrying in {:?}...",
                        retry_count, address, e, delay
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    /// 尝试连接单个节点
    async fn try_connect_peer(
        &self,
        address: &str,
    ) -> Result<(PeerKey, VersionMessage, Vec<NetAddress>)> {
        // 连接到对等节点
        let peer_key = self
            .adaptor
            .connect_peer_with_retries(
                address.to_string(),
                1,                      // 单次连接尝试
                Duration::from_secs(5), // 连接超时5秒
            )
            .await
            .map_err(|e| {
                // 分类错误类型
                match e {
                    kaspa_p2p_lib::ConnectionError::ProtocolError(_) => {
                        anyhow::anyhow!("Protocol error connecting to {}: {}", address, e)
                    }
                    kaspa_p2p_lib::ConnectionError::NoAddress => {
                        anyhow::anyhow!("Invalid address format for {}: {}", address, e)
                    }
                    kaspa_p2p_lib::ConnectionError::IoError(_) => {
                        anyhow::anyhow!("I/O error connecting to {}: {}", address, e)
                    }
                    _ => {
                        anyhow::anyhow!("Connection failed to {}: {}", address, e)
                    }
                }
            })?;

        // 等待地址响应，带超时
        let addresses = self.wait_for_addresses_with_timeout(peer_key).await?;

        // 获取对等节点信息（包括版本信息）
        let version_message = self.get_peer_version_info(peer_key).await?;

        // 断开连接
        self.adaptor.terminate(peer_key).await;

        Ok((peer_key, version_message, addresses))
    }

    /// 等待地址响应，带超时
    async fn wait_for_addresses_with_timeout(&self, peer_key: PeerKey) -> Result<Vec<NetAddress>> {
        let mut addresses_rx = self.addresses_rx.lock().await;

        tokio::select! {
            result = addresses_rx.recv() => {
                match result {
                    Some(addresses) => {
                        info!("Received {} addresses from peer {}", addresses.len(), peer_key);
                        Ok(addresses)
                    }
                    None => {
                        warn!("Address channel closed for peer {}", peer_key);
                        Ok(Vec::new())
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                warn!("Timeout waiting for addresses from peer {}", peer_key);
                Ok(Vec::new())
            }
        }
    }

    /// 获取对等节点版本信息
    async fn get_peer_version_info(&self, peer_key: PeerKey) -> Result<VersionMessage> {
        let peers = self.adaptor.active_peers();
        let version_message = peers
            .iter()
            .find(|peer| peer.key() == peer_key)
            .map(|peer| {
                let props = peer.properties();
                VersionMessage {
                    protocol_version: props.protocol_version,
                    services: 0,
                    timestamp: unix_now() as i64,
                    address: None,
                    id: Vec::new(),
                    user_agent: props.user_agent.clone(),
                    disable_relay_tx: props.disable_relay_tx,
                    subnetwork_id: props.subnetwork_id.clone().map(|id| {
                        kaspa_p2p_lib::pb::SubnetworkId {
                            bytes: <[u8]>::to_vec(id.as_ref()),
                        }
                    }),
                    network: "".to_string(), // 网络名称不在 properties 中
                }
            })
            .unwrap_or_else(|| {
                warn!("Could not find peer properties for {}", peer_key);
                VersionMessage {
                    protocol_version: 0,
                    services: 0,
                    timestamp: unix_now() as i64,
                    address: None,
                    id: Vec::new(),
                    user_agent: "unknown".to_string(),
                    disable_relay_tx: false,
                    subnetwork_id: None,
                    network: "".to_string(),
                }
            });

        Ok(version_message)
    }

    /// 关闭适配器
    pub async fn close(&self) {
        self.adaptor.close().await;
    }

    /// 处理ping-pong消息，保持连接活跃
    async fn handle_ping_pong(router: Arc<Router>) -> Result<(), ProtocolError> {
        // 订阅ping消息
        let mut ping_receiver = router.subscribe(vec![KaspadMessagePayloadType::Ping]);

        loop {
            tokio::select! {
                msg_opt = ping_receiver.recv() => {
                    if let Some(msg) = msg_opt {
                        if let Some(Payload::Ping(ping_msg)) = msg.payload {
                            debug!("Received ping message with nonce: {}", ping_msg.nonce);

                            // 发送pong响应
                            let pong_message = make_message!(
                                Payload::Pong,
                                kaspa_p2p_lib::pb::PongMessage { nonce: ping_msg.nonce }
                            );

                            if let Err(e) = router.enqueue(pong_message).await {
                                warn!("Failed to send pong response: {}", e);
                                break;
                            }

                            debug!("Sent pong response with nonce: {}", ping_msg.nonce);
                        }
                    } else {
                        // 连接已关闭
                        debug!("Ping receiver closed, stopping ping-pong handler");
                        break;
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    // 定期发送ping消息保持连接活跃
                    let ping_message = make_message!(
                        Payload::Ping,
                        kaspa_p2p_lib::pb::PingMessage { nonce: rand::random::<u64>() }
                    );

                    if let Err(e) = router.enqueue(ping_message).await {
                        debug!("Failed to send ping message: {}", e);
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}

impl Clone for DnsseedNetAdapter {
    fn clone(&self) -> Self {
        Self {
            adaptor: Arc::clone(&self.adaptor),
            addresses_rx: Arc::clone(&self.addresses_rx),
        }
    }
}
