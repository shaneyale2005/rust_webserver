// Copyright (c) 2026 shaneyale (shaneyale86@gmail.com)
// All rights reserved.

//! # FileCache 模块
//!
//! 该模块实现了一个带有时效性验证的高性能文件内容缓存系统。
//! 它结合了 LRU（最近最少使用）淘汰算法与文件修改时间（SystemTime）校验，
//! 确保在高并发场景下既能提升访问速度，又能保证数据的最终一致性。

use std::num::NonZeroUsize;
use std::time::SystemTime;

use bytes::Bytes;
use lru::LruCache;

/// `CacheEntry` 存储缓存的实体数据。
///
/// 包含文件的二进制原始数据以及该数据在读取时的磁盘最后修改时间。
#[derive(Clone)]
struct CacheEntry {
    /// 文件的二进制内容，使用 `Bytes` 以支持跨线程的高效引用计数共享。
    content: Bytes,
    /// 记录文件被缓存时的最后修改时间，用于后续的失效校验。
    modified_time: SystemTime,
}

/// 基于 LRU 策略的文件缓存器。
///
/// 封装了 `lru::LruCache`，通过文件名进行索引。当缓存达到容量上限时，
/// 会自动移除最久未访问的条目。
pub struct FileCache {
    /// 内部维护的 LRU 缓存容器。
    cache: LruCache<String, CacheEntry>,
}

impl FileCache {
    /// 根据指定的容量构造一个新的 `FileCache` 实例。
    ///
    /// # 参数
    ///
    /// * `capacity` - 缓存允许存储的最大条目数量。
    ///
    /// # Panics
    ///
    /// 如果传入的 `capacity` 为 0，该函数会触发 Panic。
    ///
    /// # Examples
    ///
    /// ```
    /// let cache = FileCache::from_capacity(100);
    /// ```
    pub fn from_capacity(capacity: usize) -> Self {
        if capacity == 0 {
            panic!("调用from_capacity时指定的大小是0。如果需要自动设置大小，请在调用处进行处理，而不是传入0");
        }
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
        }
    }

    /// 将文件内容及其元数据放入缓存。
    ///
    /// 如果缓存中已存在同名文件，该操作会覆盖旧条目并将其标记为最近访问。
    ///
    /// # 参数
    ///
    /// * `filename` - 文件的路径或标识符。
    /// * `bytes` - 文件的二进制数据。
    /// * `modified_time` - 文件的最后修改时间。
    pub fn push(&mut self, filename: &str, bytes: Bytes, modified_time: SystemTime) {
        let entry = CacheEntry {
            content: bytes,
            modified_time,
        };
        self.cache.put(filename.to_string(), entry);
    }
    
    /// 静态辅助方法：判断文件大小是否满足进入缓存的阈值要求。
    ///
    /// 通常用于过滤掉超大文件，防止其占用过多的内存空间。
    ///
    /// # 返回值
    ///
    /// 如果 `file_size` 小于等于 `threshold`，返回 `true`。
    pub fn should_cache(file_size: u64, threshold: u64) -> bool {
        file_size <= threshold
    }

    /// 在缓存中查询指定的文件。
    ///
    /// 该函数会通过 `current_modified_time` 校验缓存条目是否依然有效。
    /// 如果磁盘上的文件已被修改，即使缓存存在也会返回 `None`。
    ///
    /// # 注意
    ///
    /// 由于 LRU 算法在查询时会调整内部链表顺序，因此该方法需要 `&mut self`。
    ///
    /// # 返回值
    ///
    /// 返回命中的内容引用 `Option<&Bytes>`。如果未找到或已失效，则返回 `None`。
    pub fn find(&mut self, filename: &str, current_modified_time: SystemTime) -> Option<&Bytes> {
        match self.cache.get(filename) {
            Some(entry) => {
                if entry.modified_time == current_modified_time {
                    Some(&entry.content)
                } else {
                    None
                }
            }
            None => None,
        }
    }
    
    /// 获取当前缓存中已存储的条目数量。
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// 获取缓存的最大容量。
    #[cfg(test)]
    pub fn capacity(&self) -> usize {
        self.cache.cap().get()
    }
}

/// 自动化单元测试模块。
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_cache_creation() {
        let cache = FileCache::from_capacity(10);
        assert_eq!(cache.capacity(), 10);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    #[should_panic(expected = "调用from_capacity时指定的大小是0")]
    fn test_cache_zero_capacity_panics() {
        FileCache::from_capacity(0);
    }

    #[test]
    fn test_cache_push_and_find() {
        let mut cache = FileCache::from_capacity(3);
        let time = SystemTime::now();
        let content = Bytes::from("test content");

        cache.push("file1.txt", content.clone(), time);
        assert_eq!(cache.len(), 1);

        let found = cache.find("file1.txt", time);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), &content);
    }

    #[test]
    fn test_cache_modified_time_invalidation() {
        let mut cache = FileCache::from_capacity(3);
        let time1 = SystemTime::now();
        let time2 = time1 + Duration::from_secs(10);
        let content = Bytes::from("test content");

        cache.push("file1.txt", content.clone(), time1);

        let found = cache.find("file1.txt", time2);
        assert!(found.is_none());

        let found = cache.find("file1.txt", time1);
        assert!(found.is_some());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = FileCache::from_capacity(2);
        let time = SystemTime::now();

        cache.push("file1.txt", Bytes::from("content1"), time);
        cache.push("file2.txt", Bytes::from("content2"), time);
        assert_eq!(cache.len(), 2);

        cache.find("file1.txt", time);

        cache.push("file3.txt", Bytes::from("content3"), time);
        assert_eq!(cache.len(), 2);

        assert!(cache.find("file2.txt", time).is_none());
        assert!(cache.find("file1.txt", time).is_some());
        assert!(cache.find("file3.txt", time).is_some());
    }

    #[test]
    fn test_cache_update_existing() {
        let mut cache = FileCache::from_capacity(3);
        let time1 = SystemTime::now();
        let time2 = time1 + Duration::from_secs(10);

        cache.push("file1.txt", Bytes::from("old content"), time1);
        cache.push("file1.txt", Bytes::from("new content"), time2);

        assert!(cache.find("file1.txt", time1).is_none());

        let found = cache.find("file1.txt", time2);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), &Bytes::from("new content"));
    }

    #[test]
    fn test_cache_not_found() {
        let mut cache = FileCache::from_capacity(3);
        let time = SystemTime::now();

        let found = cache.find("nonexistent.txt", time);
        assert!(found.is_none());
    }

    #[test]
    fn test_cache_multiple_files() {
        let mut cache = FileCache::from_capacity(5);
        let time = SystemTime::now();

        for i in 1..=5 {
            let filename = format!("file{}.txt", i);
            let content = Bytes::from(format!("content{}", i));
            cache.push(&filename, content, time);
        }

        assert_eq!(cache.len(), 5);

        for i in 1..=5 {
            let filename = format!("file{}.txt", i);
            let found = cache.find(&filename, time);
            assert!(found.is_some());
        }
    }
}