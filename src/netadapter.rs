use crate::config::Config;
use crate::types::{NetAddress, NetworkMessage, VersionMessage, AddressesMessage, RequestAddressesMessage};
use anyhow::Result;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, info};

// 使用rusty-kaspa中的P2P协议
// 注意：core模块是私有的，我们只能使用公共API
use kaspa_p2p_lib::common::ProtocolError;

pub struct NetworkAdapter {
    config: Config,
}

impl NetworkAdapter {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }

    pub async fn connect_to_peer(&self, address: &NetAddress) -> Result<PeerConnection> {
        let socket_addr = SocketAddr::new(address.ip.0, address.port);
        
        debug!("Connecting to peer: {}", address.to_string());
        
        let stream = timeout(Duration::from_secs(10), TcpStream::connect(socket_addr)).await??;
        stream.set_nodelay(true)?;
        
        let connection = PeerConnection::new(stream, address.clone());
        
        Ok(connection)
    }

    pub async fn poll_peer(&self, address: &NetAddress) -> Result<Vec<NetAddress>> {
        let mut connection = self.connect_to_peer(address).await?;
        
        // 发送版本消息
        let version_msg = VersionMessage {
            protocol_version: 1,
            user_agent: "DNSSeeder/0.1.0".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            nonce: rand::random(),
        };
        
        let network_msg = NetworkMessage::version(&version_msg);
        connection.send_message(&network_msg).await?;
        
        // 等待版本响应
        let response = connection.receive_message().await?;
        if response.command != "version" {
            return Err(anyhow::anyhow!("Expected version message, got: {}", response.command));
        }
        
        // 检查协议版本
        if let Ok(peer_version) = bincode::deserialize::<VersionMessage>(&response.payload) {
            if self.config.min_proto_ver > 0 && peer_version.protocol_version < self.config.min_proto_ver as u32 {
                return Err(anyhow::anyhow!(
                    "Peer protocol version {} is below minimum: {}",
                    peer_version.protocol_version,
                    self.config.min_proto_ver
                ));
            }
            
            // 检查用户代理版本
            if let Some(ref min_ua_ver) = self.config.min_ua_ver {
                if !self.check_user_agent_version(min_ua_ver, &peer_version.user_agent) {
                    return Err(anyhow::anyhow!(
                        "Peer user agent {} doesn't satisfy minimum: {}",
                        peer_version.user_agent,
                        min_ua_ver
                    ));
                }
            }
        }
        
        // 请求地址列表
        let request = RequestAddressesMessage {
            include_all_subnetworks: true,
            subnetwork_id: None,
        };
        
        let addr_request = NetworkMessage::request_addresses(&request);
        connection.send_message(&addr_request).await?;
        
        // 等待地址响应
        let addr_response = connection.receive_message().await?;
        if addr_response.command != "addr" {
            return Err(anyhow::anyhow!("Expected addr message, got: {}", addr_response.command));
        }
        
        let addresses = bincode::deserialize::<AddressesMessage>(&addr_response.payload)?;
        
        info!(
            "Peer {} sent {} addresses",
            address.to_string(),
            addresses.addresses.len()
        );
        
        Ok(addresses.addresses)
    }

    fn check_user_agent_version(&self, min_version: &str, peer_version: &str) -> bool {
        // 简单的版本比较逻辑，可以根据需要扩展
        min_version <= peer_version
    }

    // 使用rusty-kaspa的P2P协议进行连接
    pub async fn connect_with_kaspa_protocol(&self, address: &NetAddress) -> Result<()> {
        let socket_addr = SocketAddr::new(address.ip.0, address.port);
        
        debug!("Connecting with Kaspa P2P protocol to: {}", address.to_string());
        
        // 这里应该实现与rusty-kaspa P2P协议的集成
        // 由于协议复杂性，这里提供一个基础框架
        
        // 1. 建立TCP连接
        let stream = timeout(Duration::from_secs(10), TcpStream::connect(socket_addr)).await??;
        stream.set_nodelay(true)?;
        
        // 2. 执行握手
        // TODO: 实现Kaspa P2P握手协议
        
        // 3. 交换版本信息
        // TODO: 实现版本交换
        
        // 4. 请求地址
        // TODO: 实现地址请求
        
        Ok(())
    }
}

pub struct PeerConnection {
    stream: TcpStream,
    address: NetAddress,
}

impl PeerConnection {
    fn new(stream: TcpStream, address: NetAddress) -> Self {
        Self { stream, address }
    }

    async fn send_message(&mut self, message: &NetworkMessage) -> Result<()> {
        // 这里应该实现Kaspa协议的消息发送
        // 简化实现，实际需要按照Kaspa协议规范
        debug!("Sending message to {}: {}", self.address.to_string(), message.command);
        Ok(())
    }

    async fn receive_message(&mut self) -> Result<NetworkMessage> {
        // 这里应该实现Kaspa协议的消息接收
        // 简化实现，实际需要按照Kaspa协议规范
        let message = NetworkMessage::new("version", vec![]);
        debug!("Received message from {}: {}", self.address.to_string(), message.command);
        Ok(message)
    }
}

impl Drop for PeerConnection {
    fn drop(&mut self) {
        debug!("Closing connection to {}", self.address.to_string());
    }
}

// 错误类型转换 - 使用newtype模式
pub struct KaspaProtocolError(ProtocolError);

impl From<ProtocolError> for anyhow::Error {
    fn from(err: ProtocolError) -> Self {
        anyhow::anyhow!("Protocol error: {:?}", err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NetAddress;
    use std::net::IpAddr;

    #[tokio::test]
    async fn test_network_adapter_creation() {
        let config = Config::new();
        let adapter = NetworkAdapter::new(&config);
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_user_agent_version_check() {
        let config = Config::new();
        let adapter = NetworkAdapter::new(&config).unwrap();
        
        assert!(adapter.check_user_agent_version("1.0.0", "1.0.0"));
        assert!(adapter.check_user_agent_version("1.0.0", "1.1.0"));
        assert!(!adapter.check_user_agent_version("1.1.0", "1.0.0"));
    }
}
