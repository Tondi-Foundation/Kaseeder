# Kaseeder

Kaseeder 是一个为 Kaspa 网络设计的 DNS 种子节点服务，用 Rust 编写。它提供高性能的节点发现和 DNS 解析服务。

## 特性

- 🚀 高性能 DNS 种子服务
- 🔄 自动节点发现和爬取
- 🌐 支持主网和测试网
- 📊 内置性能监控和指标收集
- 🔧 灵活的配置管理
- 🐳 Docker 容器化支持
- 📝 结构化日志记录
- 🔒 安全的非特权端口配置

## 系统要求

- Rust 1.75+ 
- Linux/macOS/Windows
- 网络访问权限

## 快速开始

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/your-org/kaseeder.git
cd kaseeder

# 构建项目
cargo build --release

# 运行（使用默认配置）
./target/release/kaseeder
```

### 使用 Docker

```bash
# 构建镜像
docker build -t kaseeder .

# 运行容器
docker run -d \
  --name kaseeder \
  -p 5354:5354 \
  -p 3737:3737 \
  -p 8080:8080 \
  -v kaseeder_data:/app/data \
  kaseeder
```

### 使用 Docker Compose

```bash
# 启动主网服务
docker-compose up -d

# 启动测试网服务
docker-compose --profile testnet up -d
```

## 配置

### 配置文件

创建 `kaseeder.conf` 文件：

```toml
# DNS 服务器配置
host = "seed.kaspa.org"
nameserver = "ns1.kaspa.org"
listen = "0.0.0.0:5354"

# gRPC 服务器配置
grpc_listen = "0.0.0.0:3737"

# 应用配置
app_dir = "./data"
threads = 8
log_level = "info"

# 网络配置
testnet = false
net_suffix = 0

# 种子节点配置
seeder = "127.0.0.1:16111"
known_peers = "192.168.1.100:16111,192.168.1.101:16111"
```

### 命令行参数

```bash
# 基本用法
kaseeder --config kaseeder.conf

# 覆盖配置
kaseeder --host seed.mykaspa.org --threads 16 --testnet

# 查看帮助
kaseeder --help
```

### 环境变量

```bash
export RUST_LOG=kaseeder=info
export KASEEDER_CONFIG=/path/to/config.toml
```

## 部署

### 生产环境部署

1. **系统配置**
   ```bash
   # 创建系统用户
   sudo useradd -r -s /bin/false kaseeder
   
   # 创建目录
   sudo mkdir -p /opt/kaseeder/{bin,config,data,logs}
   sudo chown -R kaseeder:kaseeder /opt/kaseeder
   ```

2. **服务文件**
   ```ini
   # /etc/systemd/system/kaseeder.service
   [Unit]
   Description=Kaseeder DNS Seeder
   After=network.target
   
   [Service]
   Type=simple
   User=kaseeder
   Group=kaseeder
   WorkingDirectory=/opt/kaseeder
   ExecStart=/opt/kaseeder/bin/kaseeder --config /opt/kaseeder/config/kaseeder.conf
   Restart=always
   RestartSec=10
   
   [Install]
   WantedBy=multi-user.target
   ```

3. **启动服务**
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable kaseeder
   sudo systemctl start kaseeder
   ```

### 反向代理配置

#### Nginx 配置

```nginx
# DNS 服务
server {
    listen 53 udp;
    server_name seed.kaspa.org;
    
    location / {
        proxy_pass http://127.0.0.1:5354;
        proxy_protocol off;
    }
}

# gRPC 服务
server {
    listen 443 ssl http2;
    server_name api.kaspa.org;
    
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    location / {
        grpc_pass grpc://127.0.0.1:3737;
    }
}
```

## 监控和日志

### 日志配置

```toml
# 日志级别
log_level = "info"

# 日志文件
nologfiles = false
error_log_file = "logs/kaseeder_error.log"
```

### 性能监控

启用性能分析服务器：

```bash
kaseeder --profile 8080
```

访问 `http://localhost:8080/metrics` 查看性能指标。

### 健康检查

```bash
# HTTP 健康检查
curl http://localhost:3737/health

# gRPC 健康检查
grpcurl -plaintext localhost:3737 grpc.health.v1.Health/Check
```

## 网络配置

### 端口说明

- **5354**: DNS 服务端口（非特权端口）
- **3737**: gRPC/HTTP API 端口
- **8080**: 性能监控端口（可选）

### 防火墙配置

```bash
# UFW
sudo ufw allow 5354/udp
sudo ufw allow 3737/tcp
sudo ufw allow 8080/tcp

# iptables
sudo iptables -A INPUT -p udp --dport 5354 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 3737 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 8080 -j ACCEPT
```

## 故障排除

### 常见问题

1. **端口被占用**
   ```bash
   # 检查端口使用情况
   sudo netstat -tulpn | grep :5354
   
   # 杀死占用进程
   sudo kill -9 <PID>
   ```

2. **权限问题**
   ```bash
   # 确保用户有权限访问数据目录
   sudo chown -R kaseeder:kaseeder /opt/kaseeder/data
   ```

3. **网络连接问题**
   ```bash
   # 测试网络连接
   telnet 127.0.0.1 5354
   curl http://127.0.0.1:3737/health
   ```

### 日志分析

```bash
# 查看实时日志
tail -f logs/kaseeder.log

# 搜索错误日志
grep ERROR logs/kaseeder.log

# 查看性能指标
grep "performance" logs/kaseeder.log
```

## 开发

### 构建开发环境

```bash
# 安装依赖
cargo install cargo-watch

# 开发模式运行
cargo watch -x run

# 运行测试
cargo test

# 代码格式化
cargo fmt

# 代码检查
cargo clippy
```

### 项目结构

```
kaseeder/
├── src/
│   ├── main.rs          # 主程序入口
│   ├── lib.rs           # 库模块导出
│   ├── config.rs        # 配置管理
│   ├── errors.rs        # 错误处理
│   ├── constants.rs     # 常量定义
│   ├── manager.rs       # 地址管理
│   ├── crawler.rs       # 节点爬取
│   ├── dns.rs          # DNS 服务
│   ├── grpc.rs         # gRPC 服务
│   └── ...
├── proto/               # Protocol Buffers 定义
├── docker-compose.yml   # Docker 编排
├── Dockerfile          # Docker 镜像
└── README.md           # 项目文档
```

## 贡献

欢迎贡献代码！请遵循以下步骤：

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 打开 Pull Request

## 许可证

本项目采用 ISC 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 支持

- 📧 邮箱: support@kaseeder.org
- 💬 讨论: [GitHub Discussions](https://github.com/your-org/kaseeder/discussions)
- 🐛 问题报告: [GitHub Issues](https://github.com/your-org/kaseeder/issues)

## 更新日志

### v0.1.0
- 初始版本发布
- 基本的 DNS 种子服务
- 节点发现和爬取功能
- Docker 支持
- 配置管理改进
- 错误处理优化
