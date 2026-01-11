# Web Server 使用文档

## 简介

基于 Rust 的轻量级 Web 服务器，支持 HTTP/1.1、文件缓存、HTTP 压缩和 PHP 页面。

## 安装

```bash
cargo build --release
```

需要安装 Rust 工具链（1.69.0 或更高版本）。

## 运行

```bash
cargo run --release
```

服务器默认在 `127.0.0.1:7878` 监听。

## 配置

配置文件位于 `config/` 目录：

| 文件 | 用途 |
|------|------|
| `development.toml` | 开发环境配置 |
| `production.toml` | 生产环境配置 |
| `log4rs.yaml` | 日志系统配置 |

### 配置项说明

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| `www_root` | Web 根目录 | `./static/` |
| `port` | 监听端口 | 7878 |
| `worker_threads` | 工作线程数 | CPU 核心数 |
| `cache_size` | 缓存大小 | 10 |
| `local` | 是否本地模式 | true |

## 命令

运行时可输入以下命令：

| 命令 | 说明 |
|------|------|
| `stop` | 停止服务器 |
| `help` | 显示帮助 |
| `status` | 查看当前连接数 |

## 功能特性

- **HTTP 方法**：GET、HEAD、OPTIONS
- **压缩编码**：gzip、deflate、brotli
- **文件缓存**：LRU 策略
- **PHP 支持**：自动调用 PHP 解释器
- **目录列表**：自动生成文件列表

## 访问

- 开发环境：http://127.0.0.1:7878
- 生产环境：http://服务器IP

## 注意事项

1. PHP 环境为可选，无 PHP 时无法处理 `.php` 文件
2. 生产环境将 `local` 改为 `false` 可监听所有网卡
3. 服务器安全性较低，不建议在公网长期运行
