//! In-memory cache layer using rkyv for zero-copy serialization.
//!
//! This module provides a thread-safe LRU cache for frequently accessed
//! filesystem metadata, reducing database queries for hot paths.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;
use rkyv::{access, rancor::Error as RkyvError, to_bytes, Archive, Deserialize, Serialize};

/// Maximum number of entries in the cache.
const DEFAULT_MAX_ENTRIES: usize = 10_000;

/// Maximum number of directory listing entries to cache.
const DEFAULT_MAX_DIR_ENTRIES: usize = 1_000;

/// A cached file entry with rkyv serialization support.
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
pub struct CachedFileEntry {
    /// Unique identifier.
    pub id: i64,
    /// Parent directory ID.
    pub parent_id: Option<i64>,
    /// File or directory name.
    pub name: String,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// Size in bytes.
    pub size: u64,
    /// SHA-256 hash as hex string (None for directories).
    pub content_hash: Option<String>,
    /// Creation timestamp (Unix timestamp).
    pub created_at: i64,
    /// Last modification timestamp (Unix timestamp).
    pub modified_at: i64,
}

/// A cached directory listing.
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
pub struct CachedDirListing {
    /// Parent directory path.
    pub path: String,
    /// List of entries in the directory.
    pub entries: Vec<CachedDirEntry>,
    /// When this listing was cached (Unix timestamp).
    pub cached_at: i64,
}

/// A single entry in a cached directory listing.
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
pub struct CachedDirEntry {
    /// File or directory name.
    pub name: String,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// Size in bytes.
    pub size: u64,
    /// Last modification timestamp (Unix timestamp).
    pub modified_at: i64,
}

/// Cache statistics.
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: AtomicU64,
    /// Number of cache misses.
    pub misses: AtomicU64,
    /// Number of entries currently in cache.
    pub entries: AtomicU64,
}

impl CacheStats {
    /// Record a cache hit.
    pub fn hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss.
    pub fn miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the hit rate as a percentage.
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            (hits as f64 / total as f64) * 100.0
        }
    }
}

/// Thread-safe LRU cache for file entries.
pub struct FileEntryCache {
    /// Cached entries keyed by path.
    entries: RwLock<HashMap<String, Vec<u8>>>,
    /// Maximum number of entries.
    max_entries: usize,
    /// Cache statistics.
    pub stats: CacheStats,
}

impl FileEntryCache {
    /// Create a new file entry cache.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_ENTRIES)
    }

    /// Create a new file entry cache with specified capacity.
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::with_capacity(max_entries / 4)),
            max_entries,
            stats: CacheStats::default(),
        }
    }

    /// Get a cached file entry by path.
    pub fn get(&self, path: &str) -> Option<CachedFileEntry> {
        let entries = self.entries.read();
        if let Some(bytes) = entries.get(path) {
            // Zero-copy access using rkyv
            match access::<ArchivedCachedFileEntry, RkyvError>(bytes) {
                Ok(archived) => {
                    self.stats.hit();
                    // Deserialize to owned type
                    rkyv::deserialize::<CachedFileEntry, RkyvError>(archived).ok()
                }
                Err(_) => {
                    self.stats.miss();
                    None
                }
            }
        } else {
            self.stats.miss();
            None
        }
    }

    /// Insert a file entry into the cache.
    pub fn insert(&self, path: String, entry: CachedFileEntry) {
        // Serialize using rkyv
        match to_bytes::<RkyvError>(&entry) {
            Ok(bytes) => {
                let mut entries = self.entries.write();

                // Simple eviction: if at capacity, clear half the cache
                if entries.len() >= self.max_entries {
                    let to_remove: Vec<_> =
                        entries.keys().take(self.max_entries / 2).cloned().collect();
                    for key in to_remove {
                        entries.remove(&key);
                    }
                }

                entries.insert(path, bytes.to_vec());
                self.stats
                    .entries
                    .store(entries.len() as u64, Ordering::Relaxed);
            }
            Err(_) => {
                // Failed to serialize, skip caching
            }
        }
    }

    /// Invalidate a cached entry by path.
    pub fn invalidate(&self, path: &str) {
        let mut entries = self.entries.write();
        entries.remove(path);
        self.stats
            .entries
            .store(entries.len() as u64, Ordering::Relaxed);
    }

    /// Invalidate all entries under a path prefix.
    pub fn invalidate_prefix(&self, prefix: &str) {
        let mut entries = self.entries.write();
        entries.retain(|k, _| !k.starts_with(prefix));
        self.stats
            .entries
            .store(entries.len() as u64, Ordering::Relaxed);
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        let mut entries = self.entries.write();
        entries.clear();
        self.stats.entries.store(0, Ordering::Relaxed);
    }

    /// Get the number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }
}

impl Default for FileEntryCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe cache for directory listings.
pub struct DirListingCache {
    /// Cached listings keyed by directory path.
    listings: RwLock<HashMap<String, Vec<u8>>>,
    /// Maximum number of listings.
    max_entries: usize,
    /// Cache statistics.
    pub stats: CacheStats,
}

impl DirListingCache {
    /// Create a new directory listing cache.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_DIR_ENTRIES)
    }

    /// Create a new directory listing cache with specified capacity.
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            listings: RwLock::new(HashMap::with_capacity(max_entries / 4)),
            max_entries,
            stats: CacheStats::default(),
        }
    }

    /// Get a cached directory listing by path.
    pub fn get(&self, path: &str) -> Option<CachedDirListing> {
        let listings = self.listings.read();
        if let Some(bytes) = listings.get(path) {
            match access::<ArchivedCachedDirListing, RkyvError>(bytes) {
                Ok(archived) => {
                    self.stats.hit();
                    rkyv::deserialize::<CachedDirListing, RkyvError>(archived).ok()
                }
                Err(_) => {
                    self.stats.miss();
                    None
                }
            }
        } else {
            self.stats.miss();
            None
        }
    }

    /// Insert a directory listing into the cache.
    pub fn insert(&self, path: String, listing: CachedDirListing) {
        match to_bytes::<RkyvError>(&listing) {
            Ok(bytes) => {
                let mut listings = self.listings.write();

                if listings.len() >= self.max_entries {
                    let to_remove: Vec<_> = listings
                        .keys()
                        .take(self.max_entries / 2)
                        .cloned()
                        .collect();
                    for key in to_remove {
                        listings.remove(&key);
                    }
                }

                listings.insert(path, bytes.to_vec());
                self.stats
                    .entries
                    .store(listings.len() as u64, Ordering::Relaxed);
            }
            Err(_) => {
                // Failed to serialize, skip caching
            }
        }
    }

    /// Invalidate a cached listing by path.
    pub fn invalidate(&self, path: &str) {
        let mut listings = self.listings.write();
        listings.remove(path);
        // Also invalidate parent directory
        if let Some(parent) = path.rsplit_once('/').map(|(p, _)| p) {
            let parent_path = if parent.is_empty() { "/" } else { parent };
            listings.remove(parent_path);
        }
        self.stats
            .entries
            .store(listings.len() as u64, Ordering::Relaxed);
    }

    /// Clear all cached listings.
    pub fn clear(&self) {
        let mut listings = self.listings.write();
        listings.clear();
        self.stats.entries.store(0, Ordering::Relaxed);
    }
}

impl Default for DirListingCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Global cache instance for the application.
pub struct Cache {
    /// File entry cache.
    pub files: FileEntryCache,
    /// Directory listing cache.
    pub dirs: DirListingCache,
}

impl Cache {
    /// Create a new cache with default settings.
    pub fn new() -> Self {
        Self {
            files: FileEntryCache::new(),
            dirs: DirListingCache::new(),
        }
    }

    /// Invalidate all caches for a given path.
    pub fn invalidate(&self, path: &str) {
        self.files.invalidate(path);
        self.dirs.invalidate(path);
    }

    /// Clear all caches.
    pub fn clear(&self) {
        self.files.clear();
        self.dirs.clear();
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_entry_cache() {
        let cache = FileEntryCache::new();

        let entry = CachedFileEntry {
            id: 1,
            parent_id: Some(0),
            name: "test.txt".to_string(),
            is_dir: false,
            size: 1024,
            content_hash: Some("abc123".to_string()),
            created_at: 1234567890,
            modified_at: 1234567890,
        };

        cache.insert("/test.txt".to_string(), entry.clone());

        let cached = cache.get("/test.txt").unwrap();
        assert_eq!(cached.name, "test.txt");
        assert_eq!(cached.size, 1024);

        cache.invalidate("/test.txt");
        assert!(cache.get("/test.txt").is_none());
    }

    #[test]
    fn test_dir_listing_cache() {
        let cache = DirListingCache::new();

        let listing = CachedDirListing {
            path: "/docs".to_string(),
            entries: vec![CachedDirEntry {
                name: "readme.txt".to_string(),
                is_dir: false,
                size: 512,
                modified_at: 1234567890,
            }],
            cached_at: 1234567890,
        };

        cache.insert("/docs".to_string(), listing);

        let cached = cache.get("/docs").unwrap();
        assert_eq!(cached.entries.len(), 1);
        assert_eq!(cached.entries[0].name, "readme.txt");
    }
}
