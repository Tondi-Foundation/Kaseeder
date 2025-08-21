#!/bin/bash

echo "测试 kaseeder 文件操作修复..."

# 清理之前的测试数据
rm -rf ./test_data/peers.json*

# 创建测试配置
cat > test_fix.conf << EOF
# kaseeder test configuration file
# For testing file operations

# DNS server configuration
host = "seed.test.local"
nameserver = "ns.test.local"
listen = "127.0.0.1:5354"

# gRPC server configuration
grpc_listen = "127.0.0.1:3737"

# Application configuration
app_dir = "./test_data"
threads = 2

# Network configuration
testnet = false
net_suffix = 0

# Log configuration
log_level = "info"
nologfiles = true

# Performance analysis
profile = "8080"
EOF

echo "配置文件已创建: test_fix.conf"
echo "现在可以运行: cargo run -- --config test_fix.conf"
echo "或者: cargo test"
