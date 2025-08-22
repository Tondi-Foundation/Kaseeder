# Kaspa DNS Seeder 开发指南

## 开发模式 vs 生产模式

为了解决本地测试时频繁遇到stale问题，我们实现了自动的开发模式检测。

### 🔧 **开发模式 (Debug Build)**
当您运行 `cargo build` 或 `cargo run` 时，程序会自动启用开发模式：

```bash
# 开发模式 - 更快的间隔
cargo build
./target/debug/kaseeder --config kaseeder.dev.conf
```

**开发模式设置：**
- 新节点轮询间隔：**30秒** (vs 生产模式5分钟)
- Stale超时：**5分钟** (vs 生产模式24小时)
- 连接重试间隔：
  - 高质量节点：**1分钟** (vs 生产模式5分钟)
  - 中等质量节点：**5分钟** (vs 生产模式30分钟)
  - 低质量节点：**10分钟** (vs 生产模式1小时)

### 🚀 **生产模式 (Release Build)**
当您运行 `cargo build --release` 时，程序会使用生产模式：

```bash
# 生产模式 - 标准间隔
cargo build --release
./target/release/kaseeder --config kaseeder.conf
```

**生产模式设置：**
- 新节点轮询间隔：**5分钟**
- Stale超时：**24小时**
- 连接重试间隔：
  - 高质量节点：**5分钟**
  - 中等质量节点：**30分钟**
  - 低质量节点：**1小时**

## 快速启动脚本

### 开发环境启动
```bash
# 使用开发配置和快速间隔
./scripts/dev_run.sh
```

### 生产环境启动
```bash
# 使用生产配置和标准间隔
./scripts/dev_run.sh --release
```

## 配置文件

### `kaseeder.dev.conf` - 开发配置
- 线程数：4 (适合本地测试)
- 日志级别：debug (详细日志)
- 自动应用开发模式间隔

### `kaseeder.conf` - 生产配置
- 线程数：8 (生产环境)
- 日志级别：info (标准日志)
- 使用生产模式间隔

## Stale问题解决方案

### 问题描述
之前新发现的节点需要等待30分钟才能被重新尝试连接，导致：
- 测试时等待时间过长
- 新节点很快被"冷却"
- 地址选择效率低下

### 解决方案
1. **自动模式检测**：根据编译模式自动选择间隔
2. **开发模式优化**：大幅减少等待时间
3. **智能地址选择**：优先选择新节点和可用节点

## 测试建议

### 1. 开发阶段
```bash
# 使用开发模式快速测试
./scripts/dev_run.sh

# 或手动启动
cargo build
./target/debug/kaseeder --config kaseeder.dev.conf
```

### 2. 生产测试
```bash
# 使用生产模式验证
./scripts/dev_run.sh --release

# 或手动启动
cargo build --release
./target/release/kaseeder --config kaseeder.conf
```

### 3. 诊断工具
```bash
# 诊断特定节点
./target/debug/kaseeder --diagnose "IP:PORT"

# 网络检查
./scripts/network_check.sh
```

## 监控和调试

### 日志级别
- **debug**: 开发模式，显示详细地址选择信息
- **info**: 生产模式，显示标准信息
- **warn**: 警告信息
- **error**: 错误信息

### 关键日志信息
```
Found X candidates: Y new nodes, Z existing nodes
Selected X addresses for crawling (quality-filtered): Y new, Z existing
```

### 性能指标
- 地址发现数量
- 连接成功率
- 地址选择频率
- Stale节点数量

## 故障排除

### 常见问题

1. **仍然没有地址被选择**
   - 检查日志中的候选节点数量
   - 确认DNS种子服务器是否返回地址
   - 验证网络连接

2. **地址选择过于频繁**
   - 检查是否在开发模式
   - 调整配置文件中的线程数
   - 监控系统资源使用

3. **连接失败率高**
   - 使用诊断工具检查特定节点
   - 运行网络检查脚本
   - 检查防火墙和网络配置

### 调试技巧

1. **启用详细日志**
   ```bash
   # 在配置文件中设置
   log_level = "debug"
   ```

2. **监控地址选择过程**
   ```bash
   tail -f logs/kaseeder.log | grep "Selected\|Found"
   ```

3. **检查节点状态**
   ```bash
   # 查看所有节点
   curl http://localhost:3737/v1/peers
   ```

## 总结

通过实现自动的开发模式检测，我们解决了本地测试时的stale问题：

- ✅ **开发模式**：快速间隔，适合本地测试
- ✅ **生产模式**：标准间隔，适合生产环境
- ✅ **自动切换**：无需手动配置
- ✅ **向后兼容**：不影响现有功能

现在您可以享受更流畅的开发体验，而不用担心频繁的stale问题！
