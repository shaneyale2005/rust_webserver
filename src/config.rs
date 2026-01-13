// Copyright (c) 2026 shaneyale (shaneyale86@gmail.com)
// All rights reserved.

//! # 配置管理模块 (Config Management)
//!
//! 该模块负责 Web 服务器运行时参数的加载与解析。
//! 支持通过 TOML 文件进行持久化配置，并集成了 `serde` 框架实现自动化的序列化与反序列化。
//! 
//! ## 核心逻辑
//! - 提供硬编码的默认值作为保底逻辑。
//! - 支持根据系统硬件自动调整并发线程数（使用 `num_cpus`）。
//! - 包含针对流式传输（Streaming）和范围请求（Range Requests）的调优参数。

use num_cpus;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use core::str;
use log::{error, warn};
use std::fs::File;
use std::io::prelude::*;

/// 服务器运行时的全局配置对象。
///
/// 包含网络设置、资源路径、线程模型以及缓存策略等核心参数。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// 静态资源文件的根目录路径。
    www_root: String,
    /// 服务器监听的 TCP 端口号。
    port: u16,
    /// 工作线程池的数量。若设置为 0，系统将尝试匹配 CPU 物理核心数。
    worker_threads: usize,
    /// 文件缓存条目的最大容量。
    cache_size: usize,
    /// 运行环境标识。通常用于区分本地开发环境与线上环境。
    local: bool,
    /// 启用流式传输的文件大小阈值（字节）。超过此大小的文件将采用分块传输。
    #[serde(default = "default_streaming_threshold")]
    streaming_threshold: u64,
    /// 每次 I/O 读取及分块发送时的缓冲区大小（字节）。
    #[serde(default = "default_chunk_size")]
    chunk_size: usize,
    /// 是否支持 HTTP Range 请求（用于断点续传或视频拖拽）。
    #[serde(default = "default_enable_range_requests")]
    enable_range_requests: bool,
}

/// 默认流式传输阈值：10MB
fn default_streaming_threshold() -> u64 {
    10485760 // 10MB
}

/// 默认分块大小：256KB
fn default_chunk_size() -> usize {
    262144 // 256KB
}

/// 默认开启范围请求支持
fn default_enable_range_requests() -> bool {
    true
}

impl Config {
    /// 构造一个具有初始默认值的配置实例。
    ///
    /// 该方法通常作为系统启动时的硬编码保底方案。
    pub fn new() -> Self {
        Self {
            www_root: ".".to_string(),
            port: 7878,
            worker_threads: 0,
            cache_size: 5,
            local: true,
            streaming_threshold: default_streaming_threshold(),
            chunk_size: default_chunk_size(),
            enable_range_requests: default_enable_range_requests(),
        }
    }

    /// 从指定的 TOML 配置文件中解析并构建配置对象。
    ///
    /// # 参数
    ///
    /// * `filename` - 配置文件的相对或绝对路径。
    ///
    /// # 异常处理 (Panics)
    ///
    /// - 如果指定的文件不存在或无法打开，程序将触发 `panic`。
    /// - 如果文件读取过程中发生 I/O 错误，程序将触发 `panic`。
    ///
    /// # 鲁棒性逻辑
    ///
    /// 1. **格式降级**：如果 TOML 解析失败，将打印 `error!` 日志并回退至 `Config::new()` 默认配置。
    /// 2. **自动线程扩展**：若配置中的 `worker_threads` 为 0，会自动调用 `num_cpus::get()` 获取当前机器的核心数。
    /// 3. **缓存保护**：强制修正 `cache_size` 至少为 5，以防止缓存逻辑失效。
    pub fn from_toml(filename: &str) -> Self {
        let mut file = match File::open(filename) {
            Ok(f) => f,
            Err(e) => panic!("no such file {} exception:{}", filename, e),
        };
        let mut str_val = String::new();
        match file.read_to_string(&mut str_val) {
            Ok(s) => s,
            Err(e) => panic!("Error Reading file: {}", e),
        };

        let mut raw_config = match toml::from_str(&str_val) {
            Ok(t) => t,
            Err(_) => {
                error!("无法成功从配置文件构建配置对象，使用默认配置");
                Config::new()
            }
        };
        if raw_config.worker_threads == 0 {
            raw_config.worker_threads = num_cpus::get();
        }
        if raw_config.cache_size == 0 {
            warn!("cache_size被设置为0，但目前尚不支持禁用缓存，因此该值将被改为5。");
            raw_config.cache_size = 5;
        }
        raw_config
    }
}

/// 配置项的只读访问接口（Getters）。
impl Config {
    /// 获取静态资源根目录。
    pub fn www_root(&self) -> &str {
        &self.www_root
    }

    /// 获取服务器端口号。
    pub fn port(&self) -> u16 {
        self.port
    }

    /// 获取工作线程数。
    pub fn worker_threads(&self) -> usize {
        self.worker_threads
    }

    /// 获取缓存容量上限。
    pub fn cache_size(&self) -> usize {
        self.cache_size
    }

    /// 获取运行环境标识。
    pub fn local(&self) -> bool {
        self.local
    }

    /// 获取流式传输的字节阈值。
    pub fn streaming_threshold(&self) -> u64 {
        self.streaming_threshold
    }

    /// 获取 I/O 分块大小。
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    /// 获取是否支持范围请求。
    pub fn enable_range_requests(&self) -> bool {
        self.enable_range_requests
    }
}