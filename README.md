# Kaseeder

A high-performance DNS seeder for the Kaspa network, written in Rust. Kaseeder crawls the Kaspa P2P network to discover and maintain a list of active nodes, providing reliable DNS resolution for Kaspa clients.

## 🚀 Features

- **High Performance**: Built with Rust and Tokio for maximum efficiency
- **DNS Seeding**: Provides DNS resolution for Kaspa network nodes
- **P2P Crawling**: Actively crawls the Kaspa network to discover peers
- **gRPC API**: RESTful API for monitoring and management
- **Configurable**: Flexible configuration with command-line overrides
- **Production Ready**: Comprehensive error handling and logging
- **Docker Support**: Containerized deployment with Docker and Docker Compose

## 📋 System Requirements

- **Rust**: 1.89+ (latest stable recommended)
- **OS**: Linux, macOS, Windows
- **Memory**: 512MB RAM minimum, 1GB+ recommended
- **Network**: Internet connection for P2P communication
- **Ports**: 5354 (DNS), 3737 (gRPC), 8080 (Profiling)

## 🛠️ Installation

### From Source

1. **Clone the repository**:
   ```bash
   git clone https://github.com/your-username/kaseeder.git
   cd kaseeder
   ```

2. **Build the project**:
   ```bash
   # Debug build
   cargo build
   
   # Release build (recommended for production)
   cargo build --release
   ```

3. **Install system-wide** (optional):
   ```bash
   cargo install --path .
   ```

### Using Docker

```bash
# Pull the image
docker pull kaseeder/kaseeder:latest

# Or build locally
docker build -t kaseeder .
```

## 🚀 Quick Start

### Basic Usage

1. **Start with default configuration**:
   ```bash
   ./target/release/kaseeder
   ```

2. **Start with custom configuration**:
   ```bash
   ./target/release/kaseeder \
     --host 0.0.0.0 \
     --nameserver ns1.example.com \
     --seeder seeder.example.com \
     --grpc-listen 0.0.0.0:3737 \
     --log-level info
   ```

3. **Start with configuration file**:
   ```bash
   ./target/release/kaseeder --config kaseeder.conf
   ```

### Docker Quick Start

```bash
# Run with Docker
docker run -d \
  --name kaseeder \
  -p 5354:5354 \
  -p 3737:3737 \
  -p 8080:8080 \
  kaseeder/kaseeder:latest

# Run with Docker Compose
docker-compose up -d
```

## ⚙️ Configuration

### Configuration File

Create `kaseeder.conf` in your working directory:

```toml
# Basic Configuration
listen = "0.0.0.0:5354"
grpc_listen = "0.0.0.0:3737"
log_level = "info"
nologfiles = false
error_log_file = "logs/error.log"

# Network Configuration
network = "kaspa-mainnet"
min_ua_ver = "1.0.0"
seeder = "seeder.example.com"
known_peers = ["peer1.example.com:16111", "peer2.example.com:16111"]

# Performance Configuration
profile = true
max_addresses = 10000
threads = 4

# Application Configuration
app_dir = "data"
```

### Command-Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--host` | DNS server host | `0.0.0.0` |
| `--nameserver` | Nameserver domain | `ns1.example.com` |
| `--seeder` | Seeder domain | `seeder.example.com` |
| `--grpc-listen` | gRPC server address | `0.0.0.0:3737` |
| `--log-level` | Logging level | `info` |
| `--app-dir` | Application data directory | `data` |
| `--config` | Configuration file path | `kaseeder.conf` |
| `--help` | Show help message | - |

### Environment Variables

```bash
# Set configuration via environment
export KASEEDER_HOST=0.0.0.0
export KASEEDER_GRPC_LISTEN=0.0.0.0:3737
export KASEEDER_LOG_LEVEL=debug
export KASEEDER_APP_DIR=/var/lib/kaseeder

# Run the application
./target/release/kaseeder
```

## 🐳 Docker Deployment

### Dockerfile

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/kaseeder /usr/local/bin/
EXPOSE 5354 3737 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3737/health || exit 1
CMD ["kaseeder"]
```

### Docker Compose

```yaml
version: '3.8'
services:
  kaseeder:
    build: .
    container_name: kaseeder
    ports:
      - "5354:5354"   # DNS
      - "3737:3737"   # gRPC
      - "8080:8080"   # Profiling
    volumes:
      - ./data:/app/data
      - ./logs:/app/logs
    environment:
      - KASEEDER_LOG_LEVEL=info
      - KASEEDER_NETWORK=kaspa-mainnet
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3737/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

## 🚀 Production Deployment

### Systemd Service

Create `/etc/systemd/system/kaseeder.service`:

```ini
[Unit]
Description=Kaseeder DNS Seeder
After=network.target
Wants=network.target

[Service]
Type=simple
User=kaseeder
Group=kaseeder
WorkingDirectory=/opt/kaseeder
ExecStart=/opt/kaseeder/kaseeder --config /opt/kaseeder/kaseeder.conf
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=kaseeder

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/opt/kaseeder/data /opt/kaseeder/logs

[Install]
WantedBy=multi-user.target
```

### Nginx Reverse Proxy

```nginx
server {
    listen 80;
    server_name seeder.example.com;

    location / {
        proxy_pass http://127.0.0.1:3737;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Firewall Configuration

```bash
# UFW (Ubuntu/Debian)
sudo ufw allow 5354/udp  # DNS
sudo ufw allow 3737/tcp  # gRPC
sudo ufw allow 8080/tcp  # Profiling

# iptables
sudo iptables -A INPUT -p udp --dport 5354 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 3737 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 8080 -j ACCEPT
```

## 📊 Monitoring & Health Checks

### Health Check Endpoints

```bash
# gRPC health check
curl http://localhost:3737/health

# Profiling endpoint
curl http://localhost:8080/metrics

# DNS query test
dig @localhost -p 5354 seeder.example.com
```

### Log Monitoring

```bash
# Follow logs in real-time
tail -f logs/kaseeder.log

# Search for errors
grep ERROR logs/kaseeder.log

# Monitor specific log levels
grep "DEBUG\|INFO\|WARN\|ERROR" logs/kaseeder.log
```

### Performance Metrics

```bash
# Check memory usage
ps aux | grep kaseeder

# Monitor network connections
netstat -tulpn | grep kaseeder

# Check disk usage
du -sh data/
```

## 🔧 Troubleshooting

### Common Issues

1. **Port Already in Use**:
   ```bash
   # Check what's using the port
   sudo netstat -tulpn | grep :5354
   
   # Kill the process
   sudo kill -9 <PID>
   ```

2. **Permission Denied**:
   ```bash
   # Check file permissions
   ls -la kaseeder.conf
   
   # Fix permissions
   chmod 644 kaseeder.conf
   ```

3. **Configuration Errors**:
   ```bash
   # Validate configuration
   ./target/release/kaseeder --config kaseeder.conf --dry-run
   
   # Check configuration syntax
   cat kaseeder.conf | grep -v "^#" | grep -v "^$"
   ```

### Debug Mode

```bash
# Enable debug logging
./target/release/kaseeder --log-level debug

# Check detailed logs
tail -f logs/kaseeder.log | grep DEBUG

# Monitor network activity
sudo tcpdump -i any port 5354 or port 3737
```

### Performance Tuning

```toml
# kaseeder.conf
[performance]
threads = 8                    # Increase for high-traffic scenarios
max_addresses = 50000         # Increase for larger networks
connection_timeout = 10       # Network timeout in seconds
retry_attempts = 5            # Connection retry attempts
```

## 🧪 Testing

### Run Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_config_loading

# Run tests with output
cargo test -- --nocapture

# Run tests in parallel
cargo test -- --test-threads=4
```

### Integration Testing

```bash
# Test DNS resolution
dig @localhost -p 5354 seeder.example.com

# Test gRPC API
curl -X GET http://localhost:3737/v1/status

# Test health check
curl -f http://localhost:3737/health
```

## 📚 API Reference

### gRPC Endpoints

- `GET /v1/status` - Get service status
- `GET /v1/peers` - List discovered peers
- `GET /v1/stats` - Get service statistics
- `GET /health` - Health check endpoint

### DNS Records

- `A` records for IPv4 addresses
- `AAAA` records for IPv6 addresses
- `TXT` records for additional metadata

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Setup

```bash
# Install development dependencies
cargo install cargo-watch
cargo install cargo-audit

# Run with hot reload
cargo watch -x run

# Check for security vulnerabilities
cargo audit

# Format code
cargo fmt

# Lint code
cargo clippy
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Kaspa Network team for the protocol specification
- Rust community for the excellent ecosystem
- Contributors and maintainers

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/your-username/kaseeder/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-username/kaseeder/discussions)
- **Documentation**: [Wiki](https://github.com/your-username/kaseeder/wiki)

---

**Made with ❤️ by the Kaseeder Team**
