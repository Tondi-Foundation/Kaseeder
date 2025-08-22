# Kaspa DNS Seeder (Rust Version)

[![ISC License](http://img.shields.io/badge/license-ISC-blue.svg)](https://choosealicense.com/licenses/isc/)

Kaspa DNS Seeder exposes a list of known peers to any new peer joining the Kaspa network via the DNS protocol. This is the Rust implementation, fully aligned with the Go version.

When DNSSeeder is started for the first time, it will connect to the kaspad node specified with the `--seeder` flag and listen for `addr` messages. These messages contain the IPs of all peers known by the node. DNSSeeder will then connect to each of these peers, listen for their `addr` messages, and continue to traverse the network in this fashion. DNSSeeder maintains a list of all known peers and periodically checks that they are online and available. The list is stored on disk in a json file, so on subsequent start ups the kaspad node specified with `--seeder` does not need to be online.

When DNSSeeder is queried for node information, it responds with details of a random selection of the reliable nodes it knows about.

This project is currently under active development and is in Beta state.

## Features

- **Fully aligned with Go version**: All functionality, configuration options, and behavior match the Go implementation
- **DNS server**: Responds to A, AAAA, and NS queries
- **Peer discovery**: Automatically discovers and validates network peers
- **Network support**: Mainnet and testnet-11 support
- **gRPC API**: Provides programmatic access to peer information
- **Performance profiling**: Built-in HTTP profiling server
- **Persistent storage**: Saves peer information to disk for fast startup

## Requirements

- Rust 1.70 or later
- Network access to Kaspa nodes

## Installation

### From Source

```bash
git clone https://github.com/your-repo/kaseeder
cd kaseeder
cargo build --release
```

### From Binary

Download the latest release binary for your platform from the releases page.

## Configuration

### Configuration File

Create a configuration file `kaseeder.conf` in your working directory or use the example:

```bash
cp kaseeder.conf.example kaseeder.conf
```

Edit the configuration file with your settings:

```toml
# DNS server configuration
host = "seed.kaspa.org"
nameserver = "ns1.kaspa.org"
listen = "127.0.0.1:5354"

# gRPC server configuration
grpc_listen = "127.0.0.1:3737"

# Application directory for data storage
app_dir = "./data"

# Seed node address (IP:port or just IP)
seeder = "192.168.1.100:16111"

# Known peer addresses (comma-separated list)
known_peers = "192.168.1.100:16111,192.168.1.101:16111"

# Number of crawler threads (1-32)
threads = 8

# Network configuration
testnet = false
net_suffix = 0  # Only testnet-11 (suffix 11) is supported

# Logging configuration
log_level = "info"
nologfiles = false
error_log_file = "logs/kaseeder_error.log"

# Performance profiling (optional, port 1024-65535)
# profile = "6060"
```

### Testnet Configuration

For testnet-11, use this configuration:

```toml
testnet = true
net_suffix = 11
listen = "127.0.0.1:5354"
grpc_listen = "127.0.0.1:3737"
app_dir = "./data-testnet-11"
seeder = "127.0.0.1:16311"
```

## Usage

### Basic Usage

```bash
# Start with default configuration
./kaseeder

# Start with custom configuration file
./kaseeder --config /path/to/kaseeder.conf

# Start for testnet
./kaseeder --testnet --net-suffix 11 --seeder 127.0.0.1:16311
```

### Command Line Options

```bash
./kaseeder --help
```

Available options:
- `--config`: Configuration file path
- `--host`: DNS server hostname
- `--nameserver`: DNS nameserver
- `--listen`: DNS server listen address
- `--grpc-listen`: gRPC server listen address
- `--app-dir`: Application data directory
- `--seeder`: Seed node address
- `--known-peers`: Known peer addresses (comma-separated)
- `--threads`: Number of crawler threads (1-32)
- `--testnet`: Enable testnet mode
- `--net-suffix`: Testnet network suffix (only 11 supported)
- `--log-level`: Log level (trace, debug, info, warn, error)
- `--profile`: Enable HTTP profiling on specified port

### DNS Configuration

To create a working setup where the DNSSeeder can provide IPs to kaspad instances, set the following DNS records:

```
NAME                        TYPE        VALUE
----                        ----        -----
[your.domain.name]          A           [your ip address]
[ns-your.domain.name]       NS          [your.domain.name]
```

Then redirect DNS traffic on your public IP port 53 to your local DNS seeder port (e.g., 5354).

**Note**: To listen directly on port 53 on most Unix systems, you have to run kaseeder as root, which is discouraged. Instead, use a higher port and redirect traffic.

## Network Ports

- **Mainnet**: 16111
- **Testnet-10**: 16211
- **Testnet-11**: 16311 (only supported testnet)

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with specific features
cargo run --features profiling
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_dns_record_creation

# Run with logging
RUST_LOG=debug cargo test
```

## Architecture

The Rust version maintains the same architecture as the Go version:

- **DNS Server**: Handles DNS queries and responses
- **Address Manager**: Manages peer addresses and their states
- **Crawler**: Discovers and validates network peers
- **gRPC Server**: Provides programmatic API access
- **Configuration**: Centralized configuration management

## Differences from Go Version

The Rust version is designed to be functionally identical to the Go version:

- Same configuration options and defaults
- Same DNS response format
- Same peer discovery and validation logic
- Same network parameter handling
- Same file storage format

## Troubleshooting

### Common Issues

1. **Permission denied on port 53**: Use a higher port (e.g., 5354) and redirect traffic
2. **No peers discovered**: Check your seeder configuration and network connectivity
3. **DNS queries not working**: Verify your DNS records and port forwarding

### Logs

Check the logs for detailed information:

```bash
# View error logs
tail -f logs/kaseeder_error.log

# Set log level
RUST_LOG=debug ./kaseeder
```

### Network Connectivity

Test your network connectivity:

```bash
# Test DNS server
dig @127.0.0.1 -p 5354 seed.kaspa.org A

# Test gRPC server
curl http://127.0.0.1:3737/health
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

## License

This project is licensed under the ISC License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Based on the original Go implementation by the Kaspa team
- DNS protocol handling using trust-dns-proto
- Asynchronous runtime using Tokio
