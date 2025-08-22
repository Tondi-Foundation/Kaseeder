# Kaspa DNS Seeder (Rust Version)

[![ISC License](http://img.shields.io/badge/license-ISC-blue.svg)](https://choosealicense.com/licenses/isc/)

Kaspa DNS Seeder exposes a list of known peers to any new peer joining the Kaspa network via the DNS protocol. This is the Rust implementation, fully aligned with the Go version with enhanced performance optimizations.

When DNSSeeder is started for the first time, it will connect to the kaspad node specified with the `--seeder` flag and listen for `addr` messages. These messages contain the IPs of all peers known by the node. DNSSeeder will then connect to each of these peers, listen for their `addr` messages, and continue to traverse the network in this fashion. DNSSeeder maintains a list of all known peers and periodically checks that they are online and available. The list is stored on disk in a json file, so on subsequent start ups the kaspad node specified with `--seeder` does not need to be online.

When DNSSeeder is queried for node information, it responds with details of a random selection of the reliable nodes it knows about.

This project is currently under active development and is in Beta state with production-ready performance optimizations.

## Features

- **Fully aligned with Go version**: All functionality, configuration options, and behavior match the Go implementation
- **Enhanced performance**: Optimized for high-throughput peer discovery with configurable thread pools
- **DNS server**: Responds to A, AAAA, and NS queries
- **Peer discovery**: Automatically discovers and validates network peers
- **Network support**: Mainnet and testnet-11 support
- **gRPC API**: Provides programmatic access to peer information
- **Performance profiling**: Built-in HTTP profiling server
- **Persistent storage**: Saves peer information to disk for fast startup
- **Fast failure handling**: Optimized connection timeouts and retry strategies

## Performance Features

- **Multi-threaded crawling**: Default 8 threads for concurrent peer processing
- **Optimized timeouts**: Fast failure detection with 5-second connection timeout
- **Efficient retry logic**: Single retry attempt for faster node cycling
- **Concurrent processing**: Process up to 24 nodes per round (4x improvement over 2 threads)
- **Network optimization**: Reduced address response timeout to 8 seconds

## Requirements

- Rust 1.70 or later
- Network access to Kaspa nodes
- Recommended: 4+ CPU cores for optimal performance

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

Create a configuration file `kaseeder.conf` in your working directory:

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

# Number of crawler threads (1-32, default: 8 for optimal performance)
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

## Performance Optimization Tutorial

### Understanding Thread Configuration

The `threads` parameter significantly impacts performance:

```toml
# High performance (recommended for production)
threads = 8

# Balanced performance
threads = 4

# Conservative (for low-resource environments)
threads = 2
```

**Performance Impact:**
- **2 threads**: 6 nodes per round, ~1 node/second
- **4 threads**: 12 nodes per round, ~2 nodes/second  
- **8 threads**: 24 nodes per round, ~4 nodes/second

### Optimizing Connection Timeouts

The system uses optimized timeouts for fast failure detection:

```toml
# Connection timeout: 5 seconds (optimized)
# Address response timeout: 8 seconds (optimized)
# Retry attempts: 1 (fast failure)
```

**Benefits:**
- Quick detection of unreachable nodes
- Faster cycling through node lists
- Reduced resource waste on dead connections

### Monitoring Performance

Use the built-in profiling to monitor performance:

```bash
# Enable profiling on port 6060
./kaseeder --profile 6060

# Access profiling data
curl http://localhost:6060/debug/pprof/
```

**Key Metrics to Monitor:**
- Nodes processed per second
- Connection success rate
- Average response time
- Thread utilization

### Performance Tuning Guidelines

1. **Thread Count**: Start with 8 threads, adjust based on CPU cores
2. **Network Capacity**: Ensure sufficient bandwidth for concurrent connections
3. **Memory Usage**: Monitor memory consumption with high thread counts
4. **Disk I/O**: Use SSD storage for better peer data persistence

## Usage

### Basic Usage

```bash
# Start with default configuration (8 threads)
./kaseeder

# Start with custom configuration file
./kaseeder --config /path/to/kaseeder.conf

# Start for testnet
./kaseeder --testnet --net-suffix 11 --seeder 127.0.0.1:16311

# Start with custom thread count
./kaseeder --threads 16
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
- `--threads`: Number of crawler threads (1-32, default: 8)
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

# Release build (recommended for performance testing)
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

# Performance testing
cargo run --release -- --threads 8 --profile 6060
```

## Architecture

The Rust version maintains the same architecture as the Go version with performance enhancements:

- **DNS Server**: Handles DNS queries and responses
- **Address Manager**: Manages peer addresses and their states
- **Crawler**: Multi-threaded peer discovery and validation
- **gRPC Server**: Provides programmatic API access
- **Configuration**: Centralized configuration management
- **Performance Engine**: Optimized connection handling and timeout management

## Performance Comparison

### Rust vs Go Version

| Metric | Go Version | Rust (2 threads) | Rust (8 threads) |
|--------|------------|------------------|------------------|
| Nodes per round | 6 | 6 | **24** |
| Processing speed | 1 node/sec | 1 node/sec | **4 nodes/sec** |
| Connection timeout | Fast skip | 1s+2s retry | **5s fast fail** |
| Retry mechanism | No retry | 3 attempts | **1 attempt** |
| Thread utilization | Single | Dual | **Octa-core** |

### Optimization Results

- **4x improvement** in node processing capacity
- **Faster failure detection** with optimized timeouts
- **Better resource utilization** with configurable thread pools
- **Reduced latency** through efficient connection management

## Troubleshooting

### Common Issues

1. **Permission denied on port 53**: Use a higher port (e.g., 5354) and redirect traffic
2. **No peers discovered**: Check your seeder configuration and network connectivity
3. **DNS queries not working**: Verify your DNS records and port forwarding
4. **High CPU usage**: Reduce thread count if CPU is overloaded
5. **Memory issues**: Monitor memory consumption with high thread counts

### Performance Issues

1. **Slow peer discovery**: Increase thread count (up to 8)
2. **High connection failures**: Check network stability and firewall settings
3. **Resource exhaustion**: Reduce thread count for low-resource environments

### Logs

Check the logs for detailed information:

```bash
# View error logs
tail -f logs/kaseeder_error.log

# Set log level
RUST_LOG=debug ./kaseeder

# Monitor performance
RUST_LOG=info ./kaseeder --threads 8
```

### Network Connectivity

Test your network connectivity:

```bash
# Test DNS server
dig @127.0.0.1 -p 5354 seed.kaspa.org A

# Test gRPC server
curl http://127.0.0.1:3737/health

# Performance testing
./kaseeder --profile 6060 --threads 8
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

### Performance Contributions

When contributing performance improvements:

1. **Benchmark your changes**: Use `cargo bench` and profiling tools
2. **Test with different thread counts**: Ensure improvements scale
3. **Document performance impact**: Include metrics in your PR
4. **Consider resource usage**: Balance performance with resource consumption

## License

This project is licensed under the ISC License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Based on the original Go implementation by the Kaspa team
- DNS protocol handling using trust-dns-proto
- Asynchronous runtime using Tokio
- Performance optimizations inspired by production DNS seeder requirements
