# Kaseeder

这是一个用Rust语言重写的DNS种子节点服务，用于Kaspa网络。该项目是原始Go语言版本的完整Rust实现。

## 特性

- 🚀 **高性能**: 使用Rust的零成本抽象和内存安全特性
- 🔄 **异步处理**: 基于Tokio异步运行时，支持高并发
- 🗄️ **持久化存储**: 使用Sled数据库存储节点地址信息
- 🌐 **DNS服务**: 完整的DNS服务器实现，支持A、AAAA、TXT记录
- 📊 **监控界面**: 内置HTTP性能分析服务器
- 🔌 **gRPC支持**: 提供gRPC API接口（通过HTTP实现）
- 🧵 **多线程爬取**: 支持可配置的并发网络爬取
- 📝 **结构化日志**: 使用tracing框架的现代化日志系统

## 系统要求

- Rust 1.70+ 
- Linux/macOS/Windows
- 网络连接（用于发现Kaspa节点）

## 快速开始

### 1. 安装Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. 克隆项目

```bash
git clone <repository-url>
cd dnsseeder
```

### 3. 构建项目

```bash
cargo build --release
```

### 4. 运行DNS种子节点

```bash
# 基本用法
./target/release/dnsseeder \
    -H seed.example.com \
    -n ns.example.com \
    -s 127.0.0.1:16111

# 测试网模式
./target/release/dnsseeder \
    -H seed-testnet.example.com \
    -n ns-testnet.example.com \
    -s 127.0.0.1:16110 \
    --testnet

# 自定义配置
./target/release/dnsseeder \
    -H seed.example.com \
    -n ns.example.com \
    -s 127.0.0.1:16111 \
    --listen 0.0.0.0:5354 \
    --grpclisten 0.0.0.0:3737 \
    --threads 16 \
    --loglevel debug
```

## 命令行参数

| 参数 | 短参数 | 描述 | 默认值 |
|------|--------|------|--------|
| `--appdir` | `-b` | 数据存储目录 | `~/.dnsseeder` |
| `--peers` | `-p` | 已知节点地址列表 | 无 |
| `--host` | `-H` | 种子DNS地址 | 必需 |
| `--listen` | `-l` | 监听地址:端口 | `127.0.0.1:5354` |
| `--nameserver` | `-n` | 域名服务器主机名 | 必需 |
| `--seeder` | `-s` | 工作节点的IP地址 | 无 |
| `--profile` |  | 启用HTTP分析服务器端口 | 无 |
| `--grpclisten` |  | gRPC监听地址:端口 | `127.0.0.1:3737` |
| `--minprotocolversion` | `-v` | 最小协议版本 | `0` |
| `--minuseragentversion` |  | 最小用户代理版本 | 无 |
| `--netsuffix` |  | 测试网网络后缀号 | `0` |
| `--nologfiles` |  | 禁用文件日志 | false |
| `--loglevel` |  | 日志级别 | `info` |
| `--threads` |  | 爬取线程数 | `8` |
| `--testnet` |  | 测试网模式 | false |

## 配置示例

### 主网配置

```bash
./target/release/dnsseeder \
    -H seed.kaspa.org \
    -n ns.kaspa.org \
    -s 127.0.0.1:16111 \
    --listen 0.0.0.0:53 \
    --threads 16 \
    --loglevel info
```

### 测试网配置

```bash
./target/release/dnsseeder \
    -H seed-testnet.kaspa.org \
    -n ns-testnet.kaspa.org \
    -s 127.0.0.1:16110 \
    --testnet \
    --netsuffix 1 \
    --listen 0.0.0.0:5354 \
    --threads 8
```

## 项目结构

```
src/
├── main.rs          # 主程序入口
├── lib.rs           # 库模块定义
├── config.rs        # 配置管理
├── types.rs         # 核心类型定义
├── manager.rs       # 地址管理器
├── netadapter.rs    # 网络适配器
├── dns.rs           # DNS服务器
├── crawler.rs       # 网络爬取器
├── grpc.rs          # gRPC服务
├── logging.rs       # 日志系统
├── profiling.rs     # 性能分析
└── version.rs       # 版本信息
```

## 核心组件

### 地址管理器 (AddressManager)

负责管理网络节点地址的存储、检索和状态跟踪。使用Sled数据库提供持久化存储。

### 网络适配器 (NetworkAdapter)

处理与Kaspa节点的网络连接和通信，实现Kaspa协议的消息交换。

### DNS服务器 (DnsServer)

响应DNS查询请求，为新节点提供可用的网络节点信息。

### 网络爬取器 (Crawler)

主动发现和验证网络节点，维护活跃节点列表。

## 监控和调试

### HTTP分析服务器

启用`--profile`参数后，可以通过HTTP访问性能监控界面：

```bash
# 启动时启用
./target/release/dnsseeder --profile 8080

# 访问监控界面
curl http://localhost:8080/
```

### 日志系统

支持多种日志级别和输出格式：

```bash
# 设置日志级别
export RUST_LOG=dnsseeder=debug

# 或通过命令行参数
./target/release/dnsseeder --loglevel debug
```

### 健康检查

```bash
# 检查服务状态
curl http://localhost:3737/health

# 获取统计信息
curl http://localhost:3737/stats

# 获取节点地址
curl "http://localhost:3737/addresses?limit=10"
```

## 部署建议

### 系统服务

创建systemd服务文件：

```ini
[Unit]
Description=DNSSeeder for Kaspa Network
After=network.target

[Service]
Type=simple
User=dnsseeder
ExecStart=/usr/local/bin/dnsseeder -H seed.example.com -n ns.example.com -s 127.0.0.1:16111
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### Docker部署

```dockerfile
FROM rust:1.70 as builder
WORKDIR /usr/src/dnsseeder
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/dnsseeder/target/release/dnsseeder /usr/local/bin/
EXPOSE 53 3737
CMD ["dnsseeder", "-H", "seed.example.com", "-n", "ns.example.com"]
```

## 性能调优

### 线程配置

- **小规模部署**: 4-8个线程
- **中等规模**: 8-16个线程  
- **大规模部署**: 16-32个线程

### 内存优化

- 使用`--release`模式编译
- 定期清理过期地址
- 监控内存使用情况

### 网络优化

- 配置合适的连接超时
- 使用连接池
- 启用TCP_NODELAY

## 故障排除

### 常见问题

1. **端口被占用**
   ```bash
   # 检查端口使用情况
   netstat -tulpn | grep :53
   
   # 使用不同端口
   ./target/release/dnsseeder --listen 127.0.0.1:5354
   ```

2. **权限不足**
   ```bash
   # 绑定特权端口需要root权限
   sudo ./target/release/dnsseeder --listen 0.0.0.0:53
   ```

3. **数据库错误**
   ```bash
   # 清理损坏的数据库
   rm -rf ~/.dnsseeder/addresses.db
   ```

### 调试模式

```bash
# 启用详细日志
RUST_LOG=dnsseeder=trace ./target/release/dnsseeder

# 启用backtrace
RUST_BACKTRACE=1 ./target/release/dnsseeder
```

## 开发

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_crawler_creation

# 运行集成测试
cargo test --test integration_tests
```

### 代码格式化

```bash
# 格式化代码
cargo fmt

# 检查代码风格
cargo clippy
```

### 构建文档

```bash
# 生成API文档
cargo doc --open

# 检查文档完整性
cargo doc --document-private-items
```

## 贡献

欢迎贡献代码！请遵循以下步骤：

1. Fork项目
2. 创建功能分支
3. 提交更改
4. 推送到分支
5. 创建Pull Request

## 许可证

本项目采用ISC许可证。详见[LICENSE](LICENSE)文件。

## 支持

如有问题或建议，请：

- 提交Issue
- 参与讨论
- 贡献代码

---

**注意**: 这是Rust版本的实现，与原始Go版本功能相同但性能可能有所提升。建议在生产环境中充分测试后再部署。
