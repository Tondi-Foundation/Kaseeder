# 多阶段构建Dockerfile
FROM rust:1.70-slim as builder

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /usr/src/dnsseeder

# 复制Cargo文件
COPY Cargo.toml Cargo.lock ./

# 创建虚拟目标以缓存依赖
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# 复制源代码
COPY src ./src

# 构建应用
RUN cargo build --release

# 运行时镜像
FROM debian:bullseye-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# 创建非root用户
RUN groupadd -r dnsseeder && useradd -r -g dnsseeder dnsseeder

# 创建必要的目录
RUN mkdir -p /app/data /app/logs && \
    chown -R dnsseeder:dnsseeder /app

# 复制二进制文件
COPY --from=builder /usr/src/dnsseeder/target/release/dnsseeder /usr/local/bin/

# 设置权限
RUN chmod +x /usr/local/bin/dnsseeder

# 切换到非root用户
USER dnsseeder

# 设置工作目录
WORKDIR /app

# 暴露端口
EXPOSE 5354 3737

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3737/health || exit 1

# 默认命令
ENTRYPOINT ["dnsseeder"]

# 默认参数
CMD ["--help"]
