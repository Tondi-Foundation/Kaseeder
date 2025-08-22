# Rust DNS Seeder 使用指南

## 概述

本指南介绍如何使用已经完全对齐Go版本功能的Rust DNS Seeder。所有功能包括DNS种子发现、爬虫流程、节点状态管理等都已与Go版本保持一致。

## 编译和安装

### 1. 构建项目

```bash
# 开发版本构建
cargo build

# 生产版本构建（推荐）
cargo build --release
```

### 2. 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_config_creation
cargo test test_dns_record_creation
cargo test test_address_manager_creates_directory
```

## 配置

### 1. 配置文件

复制示例配置文件：

```bash
cp kaseeder.conf.example kaseeder.conf
```

### 2. 主网配置示例

```toml
# kaseeder.conf - 主网配置
host = "seed.kaspa.org"
nameserver = "ns1.kaspa.org"
listen = "0.0.0.0:5354"
grpc_listen = "0.0.0.0:3737"
app_dir = "./data"
seeder = "seeder1.kaspad.net:16111"
known_peers = "seeder1.kaspad.net:16111,seeder2.kaspad.net:16111,seeder3.kaspad.net:16111"
threads = 8
min_proto_ver = 0
testnet = false
net_suffix = 0
log_level = "info"
nologfiles = false
error_log_file = "logs/kaseeder_error.log"
```

### 3. 测试网配置示例

```toml
# kaseeder.conf - testnet-11配置
host = "seed-testnet-11.kaspa.org"
nameserver = "ns1.kaspa.org"
listen = "0.0.0.0:5354"
grpc_listen = "0.0.0.0:3737"
app_dir = "./data/testnet-11"
seeder = "seed10.testnet.kaspa.org:16311"
known_peers = "seed10.testnet.kaspa.org:16311"
threads = 8
min_proto_ver = 0
testnet = true
net_suffix = 11
log_level = "debug"
nologfiles = false
error_log_file = "logs/kaseeder_error.log"
```

## 运行

### 1. 使用配置文件运行

```bash
# 使用默认配置文件
./target/release/kaseeder

# 使用指定配置文件
./target/release/kaseeder --config kaseeder.dev.conf
```

### 2. 使用命令行参数运行

```bash
# 主网模式
./target/release/kaseeder \
  --host seed.kaspa.org \
  --nameserver ns1.kaspa.org \
  --listen 0.0.0.0:5354 \
  --grpc-listen 0.0.0.0:3737 \
  --known-peers seeder1.kaspad.net:16111,seeder2.kaspad.net:16111 \
  --threads 8 \
  --log-level info

# 测试网模式
./target/release/kaseeder \
  --host seed-testnet-11.kaspa.org \
  --nameserver ns1.kaspa.org \
  --listen 0.0.0.0:5354 \
  --grpc-listen 0.0.0.0:3737 \
  --known-peers seed10.testnet.kaspa.org:16311 \
  --testnet \
  --net-suffix 11 \
  --threads 8 \
  --log-level debug
```

## 功能验证

### 1. DNS查询测试

测试DNS种子服务是否正常工作：

```bash
# 查询IPv4地址
dig @127.0.0.1 -p 5354 seed.kaspa.org A

# 查询IPv6地址
dig @127.0.0.1 -p 5354 seed.kaspa.org AAAA

# 查询NS记录
dig @127.0.0.1 -p 5354 seed.kaspa.org NS
```

期望输出：
```
;; ANSWER SECTION:
seed.kaspa.org.    30    IN    A    192.168.1.100
seed.kaspa.org.    30    IN    A    10.0.0.50

;; AUTHORITY SECTION:
seed.kaspa.org.    86400    IN    NS    ns1.kaspa.org.
```

### 2. gRPC API测试

```bash
# 使用grpcurl测试（需要安装grpcurl）
grpcurl -plaintext 127.0.0.1:3737 list

# 获取地址列表
grpcurl -plaintext 127.0.0.1:3737 kaseeder.DnsSeeder/GetAddresses

# 获取统计信息
grpcurl -plaintext 127.0.0.1:3737 kaseeder.DnsSeeder/GetStats
```

### 3. 日志监控

观察运行日志以确认DNS种子发现和爬虫功能正常：

```bash
# 实时查看日志
tail -f logs/kaseeder.log

# 查看错误日志
tail -f logs/kaseeder_error.log
```

关键日志信息：
- `Processing X known peers` - 已知节点处理
- `DNS seeding found X addresses` - DNS种子发现
- `Main loop: Addresses() returned X peers` - 主循环地址选择
- `Node status: Good:X Stale:X Bad:X New:X` - 节点状态统计
- `Processing X peers for polling` - 节点轮询

## 故障排除

### 1. 端口被占用

如果遇到"Address already in use"错误：

```bash
# 检查端口占用
sudo netstat -tulpn | grep :5354
sudo netstat -tulpn | grep :3737

# 杀死占用进程
sudo kill -9 <PID>

# 或者使用不同端口
./target/release/kaseeder --listen 0.0.0.0:5355 --grpc-listen 0.0.0.0:3738
```

### 2. DNS解析问题

如果DNS种子发现失败：

```bash
# 测试DNS解析
nslookup seeder1.kaspad.net
nslookup seed10.testnet.kaspa.org

# 检查网络连接
ping seeder1.kaspad.net
ping seed10.testnet.kaspa.org
```

### 3. 节点连接问题

如果没有发现节点：

```bash
# 测试节点连接
telnet seeder1.kaspad.net 16111
telnet seed10.testnet.kaspa.org 16311

# 检查防火墙设置
sudo ufw status
```

### 4. 权限问题

如果遇到文件权限错误：

```bash
# 创建必要的目录
mkdir -p data logs

# 设置正确的权限
chmod 755 data logs
chmod 644 kaseeder.conf
```

## 性能调优

### 1. 线程数配置

根据服务器性能调整线程数：

```toml
# 单核心或低性能服务器
threads = 2

# 多核心服务器
threads = 8

# 高性能服务器
threads = 16
```

### 2. 日志级别

生产环境建议使用较低的日志级别：

```toml
# 生产环境
log_level = "warn"

# 开发/调试环境
log_level = "debug"
```

### 3. 文件路径优化

使用SSD存储以提高性能：

```toml
app_dir = "/fast-storage/kaseeder-data"
error_log_file = "/fast-storage/kaseeder-logs/error.log"
```

## 网络配置

### 1. 主网默认端口

- DNS: 5354 (UDP)
- gRPC: 3737 (TCP)
- Kaspa节点: 16111 (TCP)

### 2. 测试网默认端口

- DNS: 5354 (UDP)
- gRPC: 3737 (TCP)
- Kaspa testnet-11节点: 16311 (TCP)

### 3. 防火墙配置

```bash
# 开放DNS端口
sudo ufw allow 5354/udp

# 开放gRPC端口
sudo ufw allow 3737/tcp

# 开放节点通信端口
sudo ufw allow 16111/tcp  # 主网
sudo ufw allow 16311/tcp  # testnet-11
```

## 与Go版本的对比

Rust版本现在与Go版本功能完全一致：

| 功能 | Go版本 | Rust版本 | 状态 |
|------|--------|----------|------|
| DNS服务器 | ✅ | ✅ | 完全对齐 |
| gRPC API | ✅ | ✅ | 完全对齐 |
| DNS种子发现 | ✅ | ✅ | 完全对齐 |
| 节点爬虫 | ✅ | ✅ | 完全对齐 |
| 地址管理 | ✅ | ✅ | 完全对齐 |
| 配置管理 | ✅ | ✅ | 完全对齐 |
| 网络参数 | ✅ | ✅ | 完全对齐 |
| 日志系统 | ✅ | ✅ | 完全对齐 |

## 支持和贡献

如果您遇到任何问题或想要贡献代码，请：

1. 查看[FIXES_SUMMARY.md](FIXES_SUMMARY.md)了解详细的修复记录
2. 创建Issue报告问题
3. 提交Pull Request贡献代码

Rust版本现在可以作为Go版本的完全替代品使用！🎉
