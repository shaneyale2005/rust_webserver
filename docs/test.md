# 测试指南

本文档说明如何运行项目的各种测试。

## 测试结构

项目包含以下测试类型：

- **单元测试** - 位于 `src/` 各模块中，测试独立功能
- **集成测试** - 位于 `tests/integration_test.rs`，测试HTTP请求/响应
- **浏览器测试** - 位于 `tests/browser_test.rs`，测试浏览器API功能
- **安全测试** - 位于 `tests/security_test.rs`，测试安全防护

## 运行测试

### 基本命令

```bash
# 运行所有单元测试
cargo test

# 运行所有测试（包括需要服务器的集成测试）
cargo test -- --include-ignored

# 运行特定测试文件
cargo test --test integration_test
cargo test --test browser_test
cargo test --test security_test

# 运行单个测试
cargo test test_get_request_basic
```

### 单元测试

单元测试不需要服务器运行，可以直接执行：

```bash
cargo test
```

这会运行所有模块中的单元测试，包括：
- 缓存功能测试（创建、推送、查找、时间失效、LRU驱逐、更新）
- 请求解析测试（GET/HEAD/OPTIONS/POST请求，编码头处理，路径解析）
- 响应生成测试（压缩、编码决策、MIME类型获取）
- 工具函数测试（文件大小格式化、HTML构建）

### 集成测试

集成测试需要服务器在端口 **7878** 运行。

#### 步骤：

1. **启动服务器**

```bash
# 终端1
cargo run --release
```

2. **运行集成测试**

```bash
# 终端2
cargo test --test integration_test -- --include-ignored
```

#### 集成测试包括：

- 基本GET请求
- HEAD请求（无响应体）
- OPTIONS请求
- 404错误处理
- 压缩支持
- Server响应头
- 并发请求

### 浏览器API测试

浏览器测试验证文件浏览器功能。

```bash
# 启动服务器后运行
cargo test --test browser_test -- --include-ignored
```

测试内容：
- 首次访问根目录返回JSON
- 不带Accept头返回HTML
- 子目录访问返回JSON
- 多次子目录访问

### 安全测试

安全测试验证服务器对各种攻击的防护能力。

```bash
# 启动服务器后运行
cargo test --test security_test -- --include-ignored
```

测试内容：
- 路径遍历攻击
- URL编码攻击
- CRLF注入
- SQL注入尝试
- XSS尝试
- 空字节注入
- 超大请求
- 慢速攻击
- 命令注入

## 性能基准测试

运行性能基准测试：

```bash
# 缓存性能测试
cargo bench --bench cache_benchmark

# 请求解析性能测试
cargo bench --bench request_benchmark
```

## 压力测试

使用提供的脚本进行压力测试：

```bash
# 启动服务器后运行
./scripts/stress_test.sh
```

## 测试技巧

### 查看详细输出

```bash
# 显示println!输出
cargo test -- --nocapture

# 显示测试名称
cargo test -- --show-output
```

### 并行控制

```bash
# 单线程运行
cargo test -- --test-threads=1

# 指定线程数
cargo test -- --test-threads=4
```

### 调试失败的测试

```bash
# 设置日志级别
RUST_LOG=debug cargo test

# 显示回溯信息
RUST_BACKTRACE=1 cargo test

# 完整回溯
RUST_BACKTRACE=full cargo test
```

## 相关文档

- [使用文档](usage.md) - 服务器使用说明
- [README.md](../README.md) - 项目概述
