# Kaspa DNS Seeder 配置指南

## 📋 目录

1. [快速开始](#快速开始)
2. [基础配置](#基础配置)
3. [高级配置](#高级配置)
4. [环境配置示例](#环境配置示例)
5. [性能调优](#性能调优)
6. [监控和日志](#监控和日志)
7. [故障排除](#故障排除)
8. [最佳实践](#最佳实践)

## 🚀 快速开始

### 1. 复制配置文件
```bash
cp kaseeder.conf.example kaseeder.conf
```

### 2. 编辑基本设置
编辑 `kaseeder.conf` 文件，至少需要修改以下设置：

```toml
# 您的DNS主机名（必须修改）
host = "seed.example.com"

# 您的权威域名服务器（必须修改）
nameserver = "ns1.example.com"

# 数据目录（建议修改）
app_dir = "/var/lib/kaseeder"
```

### 3. 运行服务
```bash
./target/release/kaseeder --config kaseeder.conf
```

## ⚙️ 基础配置

### DNS服务设置

| 参数 | 说明 | 示例 | 必须 |
|------|------|------|------|
| `host` | DNS主机名，客户端查询的域名 | `"seed.kaspa.org"` | ✅ |
| `nameserver` | 权威域名服务器 | `"ns1.kaspa.org"` | ✅ |
| `listen` | DNS服务监听地址 | `"0.0.0.0:8354"` | ✅ |
| `grpc_listen` | gRPC API监听地址 | `"127.0.0.1:6737"` | ✅ |

### 存储设置

| 参数 | 说明 | 示例 | 默认值 |
|------|------|------|--------|
| `app_dir` | 数据存储目录 | `"./data/async_test"` | `"./data"` |

### 网络设置

| 参数 | 说明 | 示例 | 默认值 |
|------|------|------|--------|
| `testnet` | 是否为测试网 | `false` | `false` |
| `net_suffix` | 测试网后缀 | `11` | `0` |
| `threads` | 爬虫线程数 | `8` | `8` |

### 协议设置

| 参数 | 说明 | 示例 | 默认值 |
|------|------|------|--------|
| `min_proto_ver` | 最低协议版本 | `1` | `0` |
| `min_ua_ver` | 最低用户代理版本 | `"0.12.0"` | `None` |

### 种子节点设置

| 参数 | 说明 | 示例 | 默认值 |
|------|------|------|--------|
| `known_peers` | 已知节点列表（逗号分隔） | `"192.168.1.100:16111,node.example.com:16111"` | `""` |
| `seeder` | 默认种子节点 | `"bootstrap.kaspa.org:16111"` | `""` |

## 🔧 高级配置

### 日志轮换配置 (`[advanced_logging]`)

```toml
[advanced_logging]
# 轮换策略
rotation_strategy = "daily"        # daily, hourly, size, hybrid
rotation_interval_hours = 24       # 仅用于 hourly 策略
max_file_size_mb = 100            # 文件大小限制（MB）
max_rotated_files = 10            # 保留的旧文件数量

# 压缩设置  
compress_rotated_logs = true      # 是否压缩旧日志
compression_level = 6             # 压缩级别 1-9

# 文件名选项
include_hostname = true           # 在文件名中包含主机名
include_pid = true               # 在文件名中包含进程ID

# 性能选项
enable_buffering = true          # 启用缓冲
buffer_size_bytes = 65536        # 缓冲区大小（字节）

# 监控选项
enable_file_monitoring = true    # 启用文件监控
file_monitoring_interval = 300   # 监控间隔（秒）
```

### 性能监控配置 (`[monitoring]`)

```toml
[monitoring]
enabled = true                   # 启用性能监控
interval_seconds = 60           # 监控间隔（秒）

# 收集的指标类型
collect_memory_stats = true     # 内存统计
collect_cpu_stats = true        # CPU统计
collect_network_stats = true    # 网络统计
collect_disk_stats = true       # 磁盘统计

# 历史数据
max_history_points = 1000       # 保留的历史数据点数量

# HTTP指标端点（用于Prometheus集成）
http_metrics = false            # 启用HTTP指标端点
http_metrics_port = 9090        # HTTP指标端口
```

## 🏗️ 环境配置示例

### 生产环境配置

```toml
# 基础设置
host = "seed.mainnet.example.com"
nameserver = "ns1.example.com"
listen = "0.0.0.0:53"            # 标准DNS端口
grpc_listen = "127.0.0.1:6737"   # 仅本地访问
app_dir = "/var/lib/kaseeder"
threads = 16                      # 高性能服务器

# 日志配置
log_level = "info"
no_log_files = false
error_log_file = "/var/log/kaseeder/error.log"

[advanced_logging]
rotation_strategy = "hybrid"      # 日常+大小限制
max_file_size_mb = 200           # 更大的文件
max_rotated_files = 30           # 更多历史文件
compression_level = 9            # 最大压缩
compress_rotated_logs = true

[monitoring]
enabled = true
http_metrics = true              # 启用Prometheus集成
http_metrics_port = 9090
collect_memory_stats = true
collect_cpu_stats = true
collect_network_stats = true
collect_disk_stats = true
```

### 开发环境配置

```toml
# 基础设置
host = "dev-seed.example.com"
nameserver = "ns1.example.com"
listen = "127.0.0.1:8354"        # 仅本地访问
grpc_listen = "127.0.0.1:6737"
app_dir = "./data/dev"
threads = 4                       # 较少的线程

# 日志配置
log_level = "debug"              # 详细日志
no_log_files = false
profile = "6060"                 # 启用性能分析

[advanced_logging]
rotation_strategy = "size"       # 基于大小轮换
max_file_size_mb = 10           # 较小的文件
max_rotated_files = 5           # 较少的历史文件
compress_rotated_logs = false   # 不压缩（便于调试）

[monitoring]
enabled = true
interval_seconds = 30           # 更频繁的监控
http_metrics = false
```

### 测试网配置

```toml
# 基础设置
host = "testnet-seed.example.com"
nameserver = "ns1.example.com"
testnet = true                   # 启用测试网
net_suffix = 11                  # 测试网-11
app_dir = "./data/testnet"
threads = 8

# 协议设置
min_proto_ver = 1
min_ua_ver = "0.12.0"

# 日志配置
log_level = "debug"              # 测试网使用详细日志

[advanced_logging]
rotation_strategy = "daily"
max_file_size_mb = 50
max_rotated_files = 7
compress_rotated_logs = true

[monitoring]
enabled = true
http_metrics = false
```

### 资源受限环境配置

```toml
# 基础设置
host = "seed.low-resource.example.com"
nameserver = "ns1.example.com"
app_dir = "./data/minimal"
threads = 2                      # 最少线程

# 最小化日志
log_level = "warn"               # 仅警告和错误
no_log_files = true              # 仅控制台输出

[advanced_logging]
enable_buffering = false         # 禁用缓冲以节省内存

[monitoring]
enabled = false                  # 禁用监控以节省资源
```

## 🚀 性能调优

### 线程配置

```bash
# 查看CPU核心数
nproc

# 推荐线程配置：
# 4核CPU: threads = 4-8
# 8核CPU: threads = 8-16
# 16核CPU: threads = 16-32
```

### 内存优化

```toml
[advanced_logging]
buffer_size_bytes = 32768        # 减少缓冲区（低内存）
# 或
buffer_size_bytes = 131072       # 增加缓冲区（高性能）

[monitoring]
max_history_points = 500         # 减少历史数据（低内存）
# 或  
max_history_points = 2000        # 增加历史数据（详细监控）
```

### 磁盘I/O优化

```toml
[advanced_logging]
rotation_strategy = "size"       # 基于大小轮换（减少I/O）
max_file_size_mb = 200          # 更大的文件（减少轮换频率）
compress_rotated_logs = true    # 启用压缩（节省空间）
compression_level = 6           # 平衡压缩速度和比率
```

## 📊 监控和日志

### 启用Prometheus监控

```toml
[monitoring]
enabled = true
http_metrics = true
http_metrics_port = 9090
```

访问指标端点：
```bash
curl http://localhost:9090/metrics
```

### 日志文件位置

```bash
# 主日志文件
ls -la logs/

# 错误日志
cat logs/kaseeder_error.log

# 压缩的历史日志
ls -la logs/*.gz
```

### gRPC健康检查

```bash
# 使用grpcurl检查健康状态
grpcurl -plaintext localhost:6737 kaseeder.DnsSeeder/HealthCheck

# 获取地址统计
grpcurl -plaintext localhost:6737 kaseeder.DnsSeeder/GetAddressStats
```

## 🔍 故障排除

### 常见问题及解决方法

#### 1. DNS查询失败

**问题**: 客户端无法解析DNS查询

**解决方法**:
```bash
# 检查DNS服务是否运行
sudo netstat -ulnp | grep :8354

# 测试DNS查询
dig @localhost -p 8354 seed.kaspa.org

# 检查防火墙设置
sudo ufw status
```

#### 2. 日志轮换失败

**问题**: 日志文件未正确轮换

**解决方法**:
```toml
# 检查配置
[advanced_logging]
rotation_strategy = "daily"
max_file_size_mb = 100          # 确保值大于0
max_rotated_files = 10          # 确保值大于0

# 检查目录权限
chmod 755 logs/
```

#### 3. 性能问题

**问题**: 系统响应慢或CPU使用率高

**解决方法**:
```toml
# 调整线程数
threads = 4                     # 减少线程数

# 启用性能分析
profile = "6060"

# 减少监控频率
[monitoring]
interval_seconds = 120          # 增加监控间隔
```

#### 4. 内存使用过高

**问题**: 内存使用持续增长

**解决方法**:
```toml
# 减少缓冲区大小
[advanced_logging]
buffer_size_bytes = 32768

# 减少历史数据点
[monitoring]
max_history_points = 500

# 更频繁的日志轮换
max_file_size_mb = 50
```

### 调试模式

启用详细调试：
```toml
log_level = "debug"
profile = "6060"
```

### 日志分析

```bash
# 查看错误日志
grep ERROR logs/kaseeder.log

# 查看连接统计
grep "connection" logs/kaseeder.log

# 查看DNS查询
grep "DNS query" logs/kaseeder.log

# 实时监控
tail -f logs/kaseeder.log
```

## 💡 最佳实践

### 安全性

1. **网络访问控制**
   ```toml
   # 仅本地访问gRPC
   grpc_listen = "127.0.0.1:6737"
   
   # 生产环境禁用性能分析
   profile = "0"
   ```

2. **防火墙配置**
   ```bash
   # 仅允许DNS端口
   sudo ufw allow 8354/udp
   
   # 限制gRPC访问（如果需要远程访问）
   sudo ufw allow from 192.168.1.0/24 to any port 6737
   ```

### 可靠性

1. **日志轮换**
   ```toml
   [advanced_logging]
   rotation_strategy = "hybrid"    # 结合时间和大小
   compress_rotated_logs = true   # 节省磁盘空间
   max_rotated_files = 30         # 保留足够历史
   ```

2. **监控设置**
   ```toml
   [monitoring]
   enabled = true
   http_metrics = true            # Prometheus集成
   collect_memory_stats = true    # 监控内存使用
   ```

### 性能

1. **线程配置**
   - 使用 `threads = CPU核心数` 到 `CPU核心数 × 2`
   - 监控CPU使用率，调整线程数

2. **存储优化**
   - 使用SSD存储 `app_dir`
   - 定期清理旧日志文件
   - 启用日志压缩

3. **网络优化**
   - 使用多个网络接口时，配置适当的监听地址
   - 监控网络I/O统计

### 运维

1. **自动化部署**
   ```bash
   # 创建systemd服务
   sudo cp kaseeder.service /etc/systemd/system/
   sudo systemctl enable kaseeder
   sudo systemctl start kaseeder
   ```

2. **备份策略**
   ```bash
   # 定期备份配置和数据
   cp kaseeder.conf /backup/
   cp -r data/ /backup/
   ```

3. **监控告警**
   - 设置内存使用率告警
   - 监控DNS查询响应时间
   - 设置日志错误告警

---

## 📞 支持

如果遇到问题，请检查：

1. **日志文件**: `logs/kaseeder.log` 和 `logs/kaseeder_error.log`
2. **配置验证**: 运行 `./kaseeder --config kaseeder.conf --validate`
3. **系统资源**: 检查CPU、内存、磁盘空间
4. **网络连接**: 确保DNS和gRPC端口可访问

更多信息请参考项目文档和源代码注释。
