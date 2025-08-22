#!/bin/bash

# Check Kaseeder DNS Seeder Status Script
# This script checks the status of the running DNS seeder

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔍 Kaspa DNS Seeder Status Check${NC}"
echo "====================================="

# Check if seeder is running
echo -e "\n${BLUE}📊 Process Status:${NC}"
if pgrep -f "kaseeder" > /dev/null; then
    echo -e "${GREEN}✅ DNS Seeder is running${NC}"
    
    # Get process info
    PID=$(pgrep -f "kaseeder")
    echo "Process ID: $PID"
    
    # Get memory usage
    if command -v ps > /dev/null; then
        MEMORY=$(ps -o rss= -p $PID 2>/dev/null | awk '{print $1/1024 " MB"}' || echo "Unknown")
        echo "Memory Usage: $MEMORY"
    fi
    
    # Get uptime
    if [ -d "/proc/$PID" ]; then
        START_TIME=$(stat -c %Y /proc/$PID)
        CURRENT_TIME=$(date +%s)
        UPTIME=$((CURRENT_TIME - START_TIME))
        UPTIME_DAYS=$((UPTIME / 86400))
        UPTIME_HOURS=$(((UPTIME % 86400) / 3600))
        UPTIME_MINUTES=$(((UPTIME % 3600) / 60))
        echo "Uptime: ${UPTIME_DAYS}d ${UPTIME_HOURs}h ${UPTIME_MINUTES}m"
    fi
else
    echo -e "${RED}❌ DNS Seeder is not running${NC}"
fi

# Check port status
echo -e "\n${BLUE}🌐 Port Status:${NC}"

# Check DNS port (5354)
if netstat -tuln 2>/dev/null | grep -q ":5354"; then
    echo -e "${GREEN}✅ Port 5354 (DNS) is listening${NC}"
    netstat -tuln 2>/dev/null | grep ":5354"
else
    echo -e "${RED}❌ Port 5354 (DNS) is not listening${NC}"
fi

# Check gRPC port (3737)
if netstat -tuln 2>/dev/null | grep -q ":3737"; then
    echo -e "${GREEN}✅ Port 3737 (gRPC) is listening${NC}"
    netstat -tuln 2>/dev/null | grep ":3737"
else
    echo -e "${RED}❌ Port 3737 (gRPC) is not listening${NC}"
fi

# Check data directory
echo -e "\n${BLUE}📁 Data Directory Status:${NC}"
if [ -d "data" ]; then
    echo -e "${GREEN}✅ Data directory exists${NC}"
    
    # Check peers.json
    if [ -f "data/peers.json" ]; then
        PEER_COUNT=$(jq 'length' data/peers.json 2>/dev/null || echo "0")
        echo "Peers file: ${PEER_COUNT} nodes"
        
        # Check file size and modification time
        FILE_SIZE=$(ls -lh data/peers.json | awk '{print $5}')
        MOD_TIME=$(ls -l data/peers.json | awk '{print $6, $7, $8}')
        echo "File size: $FILE_SIZE, Modified: $MOD_TIME"
    else
        echo -e "${YELLOW}⚠️  Peers file not found${NC}"
    fi
else
    echo -e "${RED}❌ Data directory not found${NC}"
fi

# Check logs
echo -e "\n${BLUE}📝 Log Status:${NC}"
if [ -d "logs" ]; then
    echo -e "${GREEN}✅ Logs directory exists${NC}"
    
    # List log files
    LOG_FILES=$(ls -la logs/ 2>/dev/null | grep -E "\.(log|txt)$" || echo "No log files found")
    if [ "$LOG_FILES" != "No log files found" ]; then
        echo "Log files:"
        echo "$LOG_FILES"
        
        # Check latest log entries
        LATEST_LOG=$(ls -t logs/*.log 2>/dev/null | head -1)
        if [ -n "$LATEST_LOG" ]; then
            echo -e "\n${BLUE}📋 Latest log entries from $LATEST_LOG:${NC}"
            tail -5 "$LATEST_LOG" 2>/dev/null || echo "Unable to read log file"
        fi
    else
        echo "No log files found"
    fi
else
    echo -e "${RED}❌ Logs directory not found${NC}"
fi

# Test DNS functionality
echo -e "\n${BLUE}🔍 DNS Functionality Test:${NC}"
if command -v dig > /dev/null; then
    echo "Testing DNS resolution..."
    if dig @127.0.0.1 -p 5354 seed.kaspa.org A +short > /dev/null 2>&1; then
        echo -e "${GREEN}✅ DNS server is responding${NC}"
    else
        echo -e "${RED}❌ DNS server is not responding${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  dig command not available, skipping DNS test${NC}"
fi

# Test gRPC functionality
echo -e "\n${BLUE}🔌 gRPC Functionality Test:${NC}"
if command -v curl > /dev/null; then
    echo "Testing gRPC server..."
    if curl -s http://127.0.0.1:3737 > /dev/null 2>&1; then
        echo -e "${GREEN}✅ gRPC server is responding${NC}"
    else
        echo -e "${RED}❌ gRPC server is not responding${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  curl command not available, skipping gRPC test${NC}"
fi

# Summary
echo -e "\n${BLUE}📊 Summary:${NC}"
if pgrep -f "kaseeder" > /dev/null; then
    echo -e "${GREEN}✅ DNS Seeder is operational${NC}"
    echo "To stop: pkill -f kaseeder"
    echo "To restart: ./scripts/start_seeder.sh"
else
    echo -e "${RED}❌ DNS Seeder is not operational${NC}"
    echo "To start: ./scripts/start_seeder.sh"
fi

echo -e "\n${BLUE}📚 For more information, check:${NC}"
echo "- Logs: logs/ directory"
echo "- Configuration: kaseeder.conf"
echo "- Data: data/ directory"
