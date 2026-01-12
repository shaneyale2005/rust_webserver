use num_cpus;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use core::str;
use log::{error, warn};
use std::fs::File;
use std::io::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    www_root: String,
    port: u16,
    worker_threads: usize,
    cache_size: usize,
    local: bool,
    #[serde(default = "default_streaming_threshold")]
    streaming_threshold: u64,
    #[serde(default = "default_chunk_size")]
    chunk_size: usize,
    #[serde(default = "default_enable_range_requests")]
    enable_range_requests: bool,
}

fn default_streaming_threshold() -> u64 {
    10485760 // 10MB
}

fn default_chunk_size() -> usize {
    262144 // 256KB
}

fn default_enable_range_requests() -> bool {
    true
}

impl Config {
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

impl Config {
    pub fn www_root(&self) -> &str {
        &self.www_root
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn worker_threads(&self) -> usize {
        self.worker_threads
    }

    pub fn cache_size(&self) -> usize {
        self.cache_size
    }

    pub fn local(&self) -> bool {
        self.local
    }

    pub fn streaming_threshold(&self) -> u64 {
        self.streaming_threshold
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    pub fn enable_range_requests(&self) -> bool {
        self.enable_range_requests
    }
}
