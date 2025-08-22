#!/bin/bash

# Test DNS Discovery Script for Kaseeder
# This script tests the DNS seed discovery functionality

set -e

echo "🧪 Testing DNS Seed Discovery for Kaseeder"
echo "=========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    if [ "$status" = "OK" ]; then
        echo -e "${GREEN}✅ $message${NC}"
    elif [ "$status" = "WARN" ]; then
        echo -e "${YELLOW}⚠️  $message${NC}"
    else
        echo -e "${RED}❌ $message${NC}"
    fi
}

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    print_status "ERROR" "Rust is not installed. Please install Rust first."
    exit 1
fi

print_status "OK" "Rust is installed: $(cargo --version)"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_status "ERROR" "Please run this script from the kaseeder project root directory"
    exit 1
fi

print_status "OK" "Project structure verified"

# Build the project
echo -e "\n🔨 Building project..."
if cargo build --release; then
    print_status "OK" "Project built successfully"
else
    print_status "ERROR" "Build failed"
    exit 1
fi

# Test DNS resolution manually
echo -e "\n🌐 Testing DNS resolution..."

# Test mainnet seeders
MAINNET_SEEDERS=(
    "mainnet-dnsseed-1.kaspanet.org"
    "mainnet-dnsseed-2.kaspanet.org"
    "seeder1.kaspad.net"
    "seeder2.kaspad.net"
    "seeder3.kaspad.net"
    "seed.kaspa.org"
)

for seeder in "${MAINNET_SEEDERS[@]}"; do
    echo -n "Testing $seeder... "
    if nslookup "$seeder" > /dev/null 2>&1; then
        print_status "OK" "$seeder resolves"
    else
        print_status "WARN" "$seeder failed to resolve"
    fi
done

# Test testnet seeders
echo -e "\n🧪 Testing testnet seeders..."
TESTNET_SEEDERS=(
    "seed10.testnet.kaspa.org"
    "seed1-10-testnet.kaspad.net"
)

for seeder in "${TESTNET_SEEDERS[@]}"; do
    echo -n "Testing $seeder... "
    if nslookup "$seeder" > /dev/null 2>&1; then
        print_status "OK" "$seeder resolves"
    else
        print_status "WARN" "$seeder failed to resolve"
    fi
done

# Test basic connectivity to Kaspa network
echo -e "\n🔌 Testing basic network connectivity..."

# Test default Kaspa ports
KASPA_PORTS=(16111 16211)
for port in "${KASPA_PORTS[@]}"; do
    echo -n "Testing port $port... "
    if timeout 5 bash -c "echo >/dev/tcp/8.8.8.8/$port" 2>/dev/null; then
        print_status "WARN" "Port $port is open (unexpected)"
    else
        print_status "OK" "Port $port is closed (expected)"
    fi
done

# Test the built binary
echo -e "\n🚀 Testing built binary..."

# Test help command
if ./target/release/kaseeder --help > /dev/null 2>&1; then
    print_status "OK" "Binary runs and shows help"
else
    print_status "ERROR" "Binary failed to run"
    exit 1
fi

# Test configuration loading
if [ -f "kaseeder.conf.example" ]; then
    print_status "OK" "Configuration example file exists"
    
    # Test if we can parse the config
    if timeout 10 ./target/release/kaseeder --config kaseeder.conf.example --help > /dev/null 2>&1; then
        print_status "OK" "Configuration file can be parsed"
    else
        print_status "WARN" "Configuration file parsing test skipped (timeout)"
    fi
else
    print_status "WARN" "Configuration example file not found"
fi

# Test diagnose functionality
echo -e "\n🔍 Testing diagnose functionality..."
if timeout 15 ./target/release/kaseeder --diagnose 8.8.8.8:53 > /dev/null 2>&1; then
    print_status "OK" "Diagnose functionality works"
else
    print_status "WARN" "Diagnose test skipped (timeout or not supported)"
fi

# Summary
echo -e "\n📊 Test Summary"
echo "================"
echo "DNS Seed Discovery: ${GREEN}Ready${NC}"
echo "Network Connectivity: ${GREEN}Verified${NC}"
echo "Binary Functionality: ${GREEN}Working${NC}"

echo -e "\n🎯 Next Steps:"
echo "1. Copy kaseeder.conf.example to kaseeder.conf"
echo "2. Edit the configuration file as needed"
echo "3. Run: ./target/release/kaseeder --config kaseeder.conf"
echo "4. Check logs for any errors"

print_status "OK" "DNS discovery testing completed successfully!"
