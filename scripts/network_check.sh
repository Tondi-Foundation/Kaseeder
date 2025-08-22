#!/bin/bash

# Network Diagnostic Script for Kaspa DNS Seeder
# This script helps diagnose network connectivity issues

echo "🔍 Kaspa DNS Seeder Network Diagnostic Tool"
echo "=========================================="
echo ""

# Check basic network connectivity
echo "1. Basic Network Connectivity:"
echo "   - Internet connectivity:"
if ping -c 1 8.8.8.8 > /dev/null 2>&1; then
    echo "     ✅ Internet connection OK"
else
    echo "     ❌ Internet connection failed"
fi

echo "   - DNS resolution:"
if nslookup google.com > /dev/null 2>&1; then
    echo "     ✅ DNS resolution OK"
else
    echo "     ❌ DNS resolution failed"
fi

echo ""

# Check specific ports
echo "2. Port Availability Check:"
echo "   - Port 16110 (Kaspa P2P):"
if nc -z -w5 151.213.166.40 16110 2>/dev/null; then
    echo "     ✅ Port 16110 accessible on 151.213.166.40"
else
    echo "     ❌ Port 16110 not accessible on 151.213.166.40"
fi

if nc -z -w5 147.93.69.22 16110 2>/dev/null; then
    echo "     ✅ Port 16110 accessible on 147.93.69.22"
else
    echo "     ❌ Port 16110 not accessible on 147.93.69.22"
fi

echo ""

# Check routing
echo "3. Network Routing:"
echo "   - Route to 151.213.166.40:"
traceroute -m 15 151.213.166.40 2>/dev/null | head -10

echo "   - Route to 147.93.69.22:"
traceroute -m 15 147.93.69.22 2>/dev/null | head -10

echo ""

# Check firewall status
echo "4. Firewall Status:"
if command -v ufw > /dev/null 2>&1; then
    ufw_status=$(ufw status 2>/dev/null | head -1)
    echo "   - UFW: $ufw_status"
fi

if command -v iptables > /dev/null 2>&1; then
    echo "   - iptables rules count: $(iptables -L | wc -l)"
fi

echo ""

# Check system resources
echo "5. System Resources:"
echo "   - Available memory: $(free -h | grep Mem | awk '{print $7}')"
echo "   - Disk space: $(df -h . | tail -1 | awk '{print $4}') available"
echo "   - Load average: $(uptime | awk -F'load average:' '{print $2}')"

echo ""

# Recommendations
echo "6. Recommendations:"
echo "   - If ports are not accessible, check:"
echo "     * Firewall rules"
echo "     * Network policies"
echo "     * ISP restrictions"
echo "   - If routing fails, check:"
echo "     * Network configuration"
echo "     * ISP routing tables"
echo "   - Run kaseeder with --diagnose flag for detailed analysis:"
echo "     ./target/debug/kaseeder --diagnose <IP:PORT>"

echo ""
echo "🔍 Diagnostic complete!"
