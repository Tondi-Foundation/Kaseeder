# Kaspa DNS Seeder

A high-performance, production-ready DNS seeder for the Kaspa network, written in Rust. This DNS seeder provides reliable peer discovery services for Kaspa nodes by maintaining a database of active network peers and responding to DNS queries.

## Features

- **High Performance**: Built with Rust for maximum performance and memory safety
- **Multi-Network Support**: Supports both mainnet and testnet configurations
- **Real-time Peer Discovery**: Continuously crawls the network to discover new peers
- **DNS Protocol Support**: Responds to A and AAAA record queries
- **gRPC API**: Provides programmatic access to peer information
- **Health Monitoring**: Built-in system monitoring and health checks
- **Configurable**: Extensive configuration options for different deployment scenarios
- **Production Ready**: Includes logging, error handling, and graceful shutdown

## Architecture

The DNS seeder consists of several key components:

- **DNS Server**: Handles DNS queries and responds with peer addresses
- **Crawler**: Discovers and validates network peers
- **Address Manager**: Maintains a database of known peers
- **gRPC Server**: Provides API access to peer information
- **Monitor**: System health and performance monitoring
- **Network Adapter**: Handles Kaspa protocol communication

## Quick Start

### Prerequisites

- Rust 1.70+ (2021 edition)
- Access to Kaspa network nodes
- Network connectivity for peer discovery

### Installation

1. **Clone the repository:**
   ```bash
   git clone https://github.com/your-org/dnsseeder.git
   cd dnsseeder
   ```

2. **Build the project:**
   ```bash
   cargo build --release
   ```

3. **Create configuration file:**
   ```bash
   cp dnsseeder.conf.example dnsseeder.conf
   # Edit dnsseeder.conf with your settings
   ```

4. **Run the DNS seeder:**
   ```bash
   ./target/release/dnsseeder -c dnsseeder.conf
   ```

## Configuration

### Basic Configuration

Create a `dnsseeder.conf` file with the following settings:

```toml
# DNS Server Configuration
host = "seed.kaspa.org"
nameserver = "ns1.kaspa.org"
listen = "0.0.0.0:53"

# gRPC Server Configuration
grpc_listen = "0.0.0.0:50051"

# Application Configuration
app_dir = "./data"
threads = 8

# Network Configuration
testnet = false
net_suffix = 0

# Seed Node Configuration
seeder = "127.0.0.1:16111"
known_peers = "peer1.example.com:16111,peer2.example.com:16111"

# Version Requirements
min_proto_ver = 0

# Logging Configuration
log_level = "info"
nologfiles = false

# Performance Analysis
profile = "8080"
```

### Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `host` | DNS server hostname | `seed.kaspa.org` |
| `nameserver` | Nameserver hostname | `ns1.kaspa.org` |
| `listen` | DNS server bind address | `0.0.0.0:53` |
| `grpc_listen` | gRPC server bind address | `0.0.0.0:50051` |
| `app_dir` | Application data directory | `./data` |
| `threads` | Number of crawler threads | `8` |
| `testnet` | Enable testnet mode | `false` |
| `net_suffix` | Testnet suffix number | `0` |
| `seeder` | Initial seed node address | `None` |
| `known_peers` | Comma-separated known peer list | `None` |
| `min_proto_ver` | Minimum protocol version | `0` |
| `log_level` | Logging level | `info` |
| `nologfiles` | Disable log files | `false` |
| `profile` | Performance profiling port | `None` |

## Usage

### Command Line Options

```bash
./target/release/dnsseeder [OPTIONS]

Options:
  -c, --config <FILE>          Configuration file path
  --host <HOST>                DNS server hostname
  --nameserver <NAMESERVER>     Nameserver hostname
  --listen <ADDRESS>            DNS server bind address
  --grpc-listen <ADDRESS>       gRPC server bind address
  --app-dir <DIR>               Application data directory
  --seeder <ADDRESS>            Seed node address
  --known-peers <PEERS>         Known peer addresses
  --threads <NUM>               Number of crawler threads
  --min-proto-ver <VERSION>     Minimum protocol version
  --testnet                     Enable testnet mode
  --net-suffix <SUFFIX>         Testnet suffix number
  --log-level <LEVEL>           Logging level
  --nologfiles                  Disable log files
  --profile <PORT>              Performance profiling port
  -h, --help                    Print help information
  -V, --version                 Print version information
```

### DNS Queries

The DNS seeder responds to standard DNS queries:

```bash
# Query for IPv4 addresses
dig @seed.kaspa.org seed.kaspa.org A

# Query for IPv6 addresses
dig @seed.kaspa.org seed.kaspa.org AAAA
```

### gRPC API

The gRPC server provides programmatic access to peer information:

```bash
# Get peer addresses
grpcurl -plaintext localhost:50051 dnsseeder.DnsSeederService/GetAddresses

# Get statistics
grpcurl -plaintext localhost:50051 dnsseeder.DnsSeederService/GetStats

# Health check
grpcurl -plaintext localhost:50051 dnsseeder.DnsSeederService/HealthCheck
```

## Deployment

### Production Deployment

1. **System Requirements:**
   - Linux/Unix system
   - 2+ CPU cores
   - 4GB+ RAM
   - 100GB+ disk space
   - Stable network connection

2. **Security Considerations:**
   - Run as non-root user
   - Configure firewall rules
   - Use TLS for gRPC (if exposed externally)
   - Regular security updates

3. **Monitoring:**
   - Enable performance profiling
   - Monitor system resources
   - Set up log rotation
   - Configure alerts for critical issues

### Docker Deployment

```bash
# Build Docker image
docker build -t dnsseeder .

# Run container
docker run -d \
  --name dnsseeder \
  -p 53:53/udp \
  -p 50051:50051 \
  -v /path/to/config:/app/config \
  -v /path/to/data:/app/data \
  dnsseeder
```

### Systemd Service

Create `/etc/systemd/system/dnsseeder.service`:

```ini
[Unit]
Description=Kaspa DNS Seeder
After=network.target

[Service]
Type=simple
User=dnsseeder
WorkingDirectory=/opt/dnsseeder
ExecStart=/opt/dnsseeder/dnsseeder -c /opt/dnsseeder/dnsseeder.conf
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

## Development

### Project Structure

```
src/
├── main.rs                 # Application entry point
├── lib.rs                  # Library exports
├── config.rs               # Configuration management
├── constants.rs            # Application constants
├── dns_seed_config.rs      # DNS seed server configuration
├── dns_seed_discovery.rs   # DNS seed discovery logic
├── crawler.rs              # Network crawling logic
├── manager.rs              # Address management
├── dns.rs                  # DNS server implementation
├── grpc.rs                 # gRPC server implementation
├── netadapter.rs           # Network protocol adapter
├── monitor.rs              # System monitoring
├── profiling.rs            # Performance profiling
├── types.rs                # Common data types
├── checkversion.rs         # Version checking
├── kaspa_protocol.rs       # Kaspa protocol configuration
├── logging.rs              # Logging configuration
└── version.rs              # Version information
```

### Building from Source

```bash
# Clone repository
git clone https://github.com/your-org/dnsseeder.git
cd dnsseeder

# Install dependencies
cargo build

# Run tests
cargo test

# Run with specific features
cargo run --release -- -c config/dnsseeder.conf
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test module
cargo test dns_seed_discovery

# Run tests with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration
```

## API Reference

### DNS Server

The DNS server responds to standard DNS queries and provides peer addresses for the Kaspa network.

**Supported Record Types:**
- A (IPv4 addresses)
- AAAA (IPv6 addresses)
- SOA (Start of Authority)

### gRPC Service

**Service:** `dnsseeder.DnsSeederService`

**Methods:**
- `GetAddresses` - Retrieve peer addresses
- `GetStats` - Get system statistics
- `GetAddressStats` - Get address statistics
- `HealthCheck` - Health check endpoint

### Configuration API

The configuration system supports:
- File-based configuration (TOML format)
- Environment variable overrides
- Command-line argument overrides
- Runtime configuration validation

## Monitoring and Maintenance

### Health Checks

The DNS seeder includes built-in health monitoring:

- System resource monitoring (CPU, memory, network)
- Service availability checks
- Performance metrics collection
- Automatic issue detection and reporting

### Logging

Comprehensive logging with configurable levels:

- Application events and errors
- Network activity and peer interactions
- Performance metrics and statistics
- System health and monitoring data

### Performance Profiling

Built-in performance profiling capabilities:

- Request/response timing
- Resource usage monitoring
- Performance bottleneck identification
- Optimization recommendations

## Troubleshooting

### Common Issues

1. **DNS Resolution Failures:**
   - Check network connectivity
   - Verify DNS server configuration
   - Review firewall settings

2. **Peer Discovery Issues:**
   - Verify seed node configuration
   - Check network protocol compatibility
   - Review logging for connection errors

3. **Performance Problems:**
   - Monitor system resources
   - Adjust thread count configuration
   - Review performance profiling data

### Debug Mode

Enable debug logging for troubleshooting:

```bash
./target/release/dnsseeder -c dnsseeder.conf --log-level debug
```

### Performance Analysis

Enable performance profiling:

```bash
./target/release/dnsseeder -c dnsseeder.conf --profile 8080
```

Then access profiling data at `http://localhost:8080`

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

### Code Style

- Follow Rust coding standards
- Use meaningful variable and function names
- Add comprehensive documentation
- Include unit tests for new features

## License

This project is licensed under the ISC License - see the [LICENSE](LICENSE) file for details.

## Support

- **Documentation:** [Wiki](https://github.com/your-org/dnsseeder/wiki)
- **Issues:** [GitHub Issues](https://github.com/your-org/dnsseeder/issues)
- **Discussions:** [GitHub Discussions](https://github.com/your-org/dnsseeder/discussions)
- **Email:** support@your-org.com

## Acknowledgments

- Kaspa development team for protocol specifications
- Rust community for excellent tooling and ecosystem
- Contributors and maintainers of this project

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a detailed history of changes.

---

**Note:** This DNS seeder is designed for production use in the Kaspa network. Please ensure you have proper network access and follow security best practices when deploying.
