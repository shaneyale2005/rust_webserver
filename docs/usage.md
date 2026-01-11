# Web Server 使用文档

## 简介

基于 Rust 的轻量级 Web 服务器，支持 HTTP/1.1、文件缓存、HTTP 压缩和 PHP 页面。

## 安装

```bash
cargo build --release
```

需要安装 Rust 工具链。

## 运行

```bash
cargo run --release
```

服务器默认在 `127.0.0.1:7878` 监听。



