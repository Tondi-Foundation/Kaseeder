#!/bin/bash

# Kaseeder Performance Test Script
# 测试优化后的节点发现性能

echo "🚀 Kaseeder 性能测试开始..."
echo "=================================="

# 检查kaseeder是否正在运行
if ! pgrep -f "kaseeder" > /dev/null; then
    echo "❌ Kaseeder 未运行，请先启动服务"
    exit 1
fi

echo "✅ Kaseeder 服务正在运行"

# 测试DNS查询性能
echo ""
echo "📊 测试DNS查询性能..."
echo "----------------------------------"

# 测试A记录查询
echo "🔍 测试A记录查询..."
time dig @127.0.0.1 -p 5354 seed.kaspa.org A

# 测试AAAA记录查询
echo ""
echo "🔍 测试AAAA记录查询..."
time dig @127.0.0.1 -p 5354 seed.kaspa.org AAAA

# 测试NS记录查询
echo ""
echo "🔍 测试NS记录查询..."
time dig @127.0.0.1 -p 5354 seed.kaspa.org NS

# 测试gRPC API性能
echo ""
echo "📊 测试gRPC API性能..."
echo "----------------------------------"

# 测试健康检查
echo "🏥 测试健康检查..."
time curl -s http://localhost:3737/health

# 测试统计信息获取
echo ""
echo "📈 测试统计信息获取..."
time curl -s http://localhost:3737/stats

# 测试地址列表获取
echo ""
echo "🌐 测试地址列表获取..."
time curl -s "http://localhost:3737/addresses?limit=100"

# 性能监控
echo ""
echo "📊 性能监控..."
echo "----------------------------------"

# 获取当前节点数量
echo "当前发现的节点数量:"
curl -s http://localhost:3737/stats | jq -r '.total_nodes // "N/A"'

# 获取活跃节点数量
echo "当前活跃节点数量:"
curl -s http://localhost:3737/stats | jq -r '.active_nodes // "N/A"'

# 获取连接统计
echo "成功连接数:"
curl -s http://localhost:3737/stats | jq -r '.successful_connections // "N/A"'

echo "失败连接数:"
curl -s http://localhost:3737/stats | jq -r '.failed_connections // "N/A"'

# 系统资源使用情况
echo ""
echo "💻 系统资源使用情况..."
echo "----------------------------------"

# CPU使用率
echo "CPU使用率:"
top -bn1 | grep "Cpu(s)" | awk '{print $2}' | cut -d'%' -f1

# 内存使用情况
echo "内存使用情况:"
free -h | grep "Mem:"

# 网络连接数
echo "网络连接数:"
netstat -an | wc -l

echo ""
echo "✅ 性能测试完成！"
echo "=================================="
echo ""
echo "💡 优化建议:"
echo "1. 如果节点发现速度仍然较慢，可以检查网络连接"
echo "2. 如果CPU使用率过高，可以减少线程数"
echo "3. 如果内存使用过多，可以减少最大地址数量"
echo "4. 监控日志文件查看详细的性能指标"
echo ""
echo "📝 查看详细日志:"
echo "tail -f logs/kaseeder.log"
echo ""
echo "🔧 性能监控:"
echo "http://localhost:8080 (如果启用了profile)"
echo "http://localhost:3737/stats (gRPC统计)"
