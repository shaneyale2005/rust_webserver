use std::num::NonZeroUsize;
use std::time::SystemTime;

use bytes::Bytes;
use lru::LruCache;

#[derive(Clone)]
struct CacheEntry {
    content: Bytes,
    modified_time: SystemTime,
}

pub struct FileCache {
    cache: LruCache<String, CacheEntry>,
}

impl FileCache {
    // 根据容量构造
    pub fn from_capacity(capacity: usize) -> Self {
        if capacity == 0 {
            panic!("调用from_capacity时指定的大小是0。如果需要自动设置大小，请在调用处进行处理，而不是传入0");
        }
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
        }
    }
    // 放入
    pub fn push(&mut self, filename: &str, bytes: Bytes, modified_time: SystemTime) {
        let entry = CacheEntry {
            content: bytes,
            modified_time,
        };
        self.cache.put(filename.to_string(), entry);
    }
    
    // 检查文件大小是否适合缓存
    pub fn should_cache(file_size: u64, threshold: u64) -> bool {
        file_size <= threshold
    }
    // 查询有效缓存
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
    
    // 测试
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    #[cfg(test)]
    pub fn capacity(&self) -> usize {
        self.cache.cap().get()
    }
}

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
