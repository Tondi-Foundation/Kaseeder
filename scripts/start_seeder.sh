#!/bin/bash

# Start Kaseeder DNS Seeder Script
# This script starts the Kaspa DNS seeder with proper configuration

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 Starting Kaspa DNS Seeder${NC}"
echo "================================"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ Please run this script from the kaseeder project root directory${NC}"
    exit 1
fi

# Check if binary exists
if [ ! -f "target/release/kaseeder" ]; then
    echo -e "${YELLOW}⚠️  Binary not found. Building project...${NC}"
    if ! cargo build --release; then
        echo -e "${RED}❌ Build failed${NC}"
        exit 1
    fi
fi

# Check if configuration file exists
if [ ! -f "kaseeder.conf" ]; then
    echo -e "${YELLOW}⚠️  Configuration file not found. Creating from example...${NC}"
    if [ -f "kaseeder.conf.example" ]; then
        cp kaseeder.conf.example kaseeder.conf
        echo -e "${GREEN}✅ Configuration file created from example${NC}"
    else
        echo -e "${RED}❌ Configuration example file not found${NC}"
        exit 1
    fi
fi

# Create necessary directories
echo -e "${GREEN}📁 Creating necessary directories...${NC}"
mkdir -p data logs

# Check if ports are available
echo -e "${GREEN}🔍 Checking port availability...${NC}"

# Check DNS port (5354)
if netstat -tuln 2>/dev/null | grep -q ":5354"; then
    echo -e "${YELLOW}⚠️  Port 5354 is already in use${NC}"
    echo "You may need to stop other services using this port"
else
    echo -e "${GREEN}✅ Port 5354 is available${NC}"
fi

# Check gRPC port (3737)
if netstat -tuln 2>/dev/null | grep -q ":3737"; then
    echo -e "${YELLOW}⚠️  Port 3737 is already in use${NC}"
    echo "You may need to stop other services using this port"
else
    echo -e "${GREEN}✅ Port 3737 is available${NC}"
fi

# Display configuration
echo -e "\n${GREEN}📋 Configuration:${NC}"
echo "Host: $(grep '^host =' kaseeder.conf | cut -d'"' -f2)"
echo "Listen: $(grep '^listen =' kaseeder.conf | cut -d'"' -f2)"
echo "gRPC Listen: $(grep '^grpc_listen =' kaseeder.conf | cut -d'"' -f2)"
echo "Threads: $(grep '^threads =' kaseeder.conf | cut -d'=' -f2 | tr -d ' ')"
echo "App Directory: $(grep '^app_dir =' kaseeder.conf | cut -d'"' -f2)"

# Check for known peers
if grep -q '^known_peers =' kaseeder.conf; then
    echo "Known Peers: $(grep '^known_peers =' kaseeder.conf | cut -d'"' -f2)"
fi

# Start the seeder
echo -e "\n${GREEN}🚀 Starting DNS Seeder...${NC}"
echo "Press Ctrl+C to stop"
echo "Logs will be written to logs/ directory"
echo ""

# Run the seeder
exec ./target/release/kaseeder --config kaseeder.conf
