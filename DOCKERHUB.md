# FluxDNS

一个功能完整的 DNS 代理服务，支持多种协议和 Web 管理界面。

## 支持的架构

- `linux/amd64`
- `linux/arm64` (ARMv8)

## 支持的标签

- `latest` - 最新稳定版本
- `x.x.x` - 特定版本号
- `x.x` - 主次版本号
- `x` - 主版本号

## 快速开始

### Docker Run

```bash
docker run -d \
  --name fluxdns \
  --restart unless-stopped \
  --user 1000:1000 \
  -e TZ=Asia/Shanghai \
  -e ADMIN_USERNAME=admin \
  -e ADMIN_PASSWORD=admin \
  -p 8080:8080 \
  -p 53:53/udp \
  -p 53:53/tcp \
  -p 853:853 \
  -p 443:443 \
  -v ./data:/app/data \
  -v ./logs:/app/logs \
  --cap-add NET_BIND_SERVICE \
  lhstack/fluxdns:latest
```

### Docker Compose

```yaml
version: '3.8'

services:
  fluxdns:
    image: lhstack/fluxdns:latest
    container_name: fluxdns
    restart: unless-stopped
    user: "1000:1000"
    environment:
      - TZ=Asia/Shanghai
      - DATABASE_URL=sqlite:/app/data/fluxdns.db?mode=rwc
      - WEB_PORT=8080
      - ADMIN_USERNAME=admin
      - ADMIN_PASSWORD=admin
      - LOG_PATH=/app/logs
      - LOG_LEVEL=info
    ports:
      - "8080:8080"      # Web 管理界面
      - "53:53/udp"      # DNS UDP
      - "53:53/tcp"      # DNS TCP
      - "853:853"        # DoT/DoQ
      - "443:443"        # DoH/DoH3
    volumes:
      - ./data:/app/data
      - ./logs:/app/logs
    cap_add:
      - NET_BIND_SERVICE
```

## 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `TZ` | `Asia/Shanghai` | 时区 |
| `DATABASE_URL` | `sqlite:/app/data/fluxdns.db?mode=rwc` | 数据库路径 |
| `WEB_PORT` | `8080` | Web 管理端口 |
| `ADMIN_USERNAME` | `admin` | 管理员用户名 |
| `ADMIN_PASSWORD` | `admin` | 管理员密码 |
| `LOG_PATH` | `/app/logs` | 日志目录 |
| `LOG_LEVEL` | `info` | 日志级别 |

## 端口说明

| 端口 | 协议 | 说明 |
|------|------|------|
| 8080 | TCP | Web 管理界面 |
| 53 | UDP/TCP | 标准 DNS |
| 853 | TCP | DoT (DNS over TLS) / DoQ (DNS over QUIC) |
| 443 | TCP | DoH (DNS over HTTPS) / DoH3 (DNS over HTTP/3) |

## 数据持久化

建议挂载以下目录：

- `/app/data` - 数据库文件
- `/app/logs` - 日志文件
- `/app/certs` - TLS 证书 (可选)

## TLS 配置

如需启用 DoT/DoH/DoQ/DoH3，需要挂载 TLS 证书：

```yaml
volumes:
  - ./certs:/app/certs:ro
```

证书文件：
- `cert.pem` - 证书文件
- `key.pem` - 私钥文件

## 功能特性

### DNS 协议支持
- UDP DNS (端口 53)
- DoH - DNS over HTTPS (端口 443)
- DoT - DNS over TLS (端口 853)
- DoQ - DNS over QUIC (端口 853)
- DoH3 - DNS over HTTP/3 (端口 443)

### 核心功能
- 多上游 DNS 服务器支持
- 查询策略：并发、轮询、随机、最快响应
- DNS 缓存管理
- 域名重写规则
- 本地 DNS 记录
- 查询日志记录

## 源码

GitHub: [https://github.com/lhstack/fluxdns](https://github.com/lhstack/fluxdns)

## 许可证

Apache License 2.0
