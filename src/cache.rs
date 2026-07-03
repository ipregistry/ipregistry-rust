//! Response caching.

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use lru::LruCache;

use crate::model::IpInfo;

/// Abstracts the storage used by a [`Client`](crate::Client) to memoize IP
/// lookups. Implementations must be safe for concurrent use from multiple
/// threads and should never block for long, as they are called from async
/// contexts.
///
/// Only successful single and batch IP lookups are cached. Origin lookups are
/// never cached, because the requester IP is only known from the response.
///
/// Keys are opaque strings derived from the looked-up IP address and the
/// request parameters. `Cache` is implemented for `Arc<C>`, so you can keep a
/// handle to a cache after handing it to a client:
///
/// ```
/// use std::sync::Arc;
/// use ipregistry::{Cache, Client, InMemoryCache};
///
/// let cache = Arc::new(InMemoryCache::new());
/// let client = Client::builder("YOUR_API_KEY")
///     .cache(Arc::clone(&cache))
///     .build()
///     .unwrap();
/// // ... later:
/// cache.invalidate_all();
/// ```
pub trait Cache: Send + Sync {
    /// Returns the cached value for `key`, or `None` when absent or expired.
    fn get(&self, key: &str) -> Option<IpInfo>;
    /// Stores `value` under `key`.
    fn set(&self, key: &str, value: IpInfo);
    /// Removes the entry for `key`, if present.
    fn invalidate(&self, key: &str);
    /// Removes every entry.
    fn invalidate_all(&self);
}

impl<C: Cache + ?Sized> Cache for Arc<C> {
    fn get(&self, key: &str) -> Option<IpInfo> {
        (**self).get(key)
    }

    fn set(&self, key: &str, value: IpInfo) {
        (**self).set(key, value)
    }

    fn invalidate(&self, key: &str) {
        (**self).invalidate(key)
    }

    fn invalidate_all(&self) {
        (**self).invalidate_all()
    }
}

/// Default maximum number of entries held by an [`InMemoryCache`].
pub const DEFAULT_CACHE_MAX_SIZE: usize = 4096;

/// Default time-to-live of [`InMemoryCache`] entries.
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(10 * 60);

/// A thread-safe, in-process [`Cache`] with time-based expiration and a
/// bounded size using least-recently-used eviction.
///
/// By default it holds up to [`DEFAULT_CACHE_MAX_SIZE`] entries for
/// [`DEFAULT_CACHE_TTL`] each; use [`InMemoryCache::builder`] to change that.
pub struct InMemoryCache {
    ttl: Duration,
    entries: Mutex<LruCache<String, CacheEntry>>,
}

struct CacheEntry {
    value: IpInfo,
    expires_at: Instant,
}

impl InMemoryCache {
    /// Creates a cache with the default maximum size and TTL.
    pub fn new() -> Self {
        Self::builder().build()
    }

    /// Returns a builder to customize the maximum size and TTL.
    pub fn builder() -> InMemoryCacheBuilder {
        InMemoryCacheBuilder::default()
    }

    /// Returns the current number of entries, including expired entries not
    /// yet evicted. It is primarily useful in tests.
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// Returns whether the cache holds no entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for InMemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for InMemoryCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryCache")
            .field("ttl", &self.ttl)
            .field("len", &self.len())
            .finish()
    }
}

impl Cache for InMemoryCache {
    fn get(&self, key: &str) -> Option<IpInfo> {
        let mut entries = self.entries.lock().unwrap();
        match entries.get(key) {
            Some(entry) if entry.expires_at > Instant::now() => Some(entry.value.clone()),
            Some(_) => {
                entries.pop(key);
                None
            }
            None => None,
        }
    }

    fn set(&self, key: &str, value: IpInfo) {
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + self.ttl,
        };
        self.entries.lock().unwrap().put(key.to_owned(), entry);
    }

    fn invalidate(&self, key: &str) {
        self.entries.lock().unwrap().pop(key);
    }

    fn invalidate_all(&self) {
        self.entries.lock().unwrap().clear();
    }
}

/// Builds an [`InMemoryCache`]. Create one with [`InMemoryCache::builder`].
#[derive(Debug, Clone)]
pub struct InMemoryCacheBuilder {
    max_size: usize,
    ttl: Duration,
}

impl Default for InMemoryCacheBuilder {
    fn default() -> Self {
        Self {
            max_size: DEFAULT_CACHE_MAX_SIZE,
            ttl: DEFAULT_CACHE_TTL,
        }
    }
}

impl InMemoryCacheBuilder {
    /// Sets the maximum number of entries the cache holds before it starts
    /// evicting the least recently used entry. A value of `0` leaves the
    /// default ([`DEFAULT_CACHE_MAX_SIZE`]).
    pub fn max_size(mut self, n: usize) -> Self {
        if n > 0 {
            self.max_size = n;
        }
        self
    }

    /// Sets how long an entry stays valid after being written. A zero
    /// duration leaves the default ([`DEFAULT_CACHE_TTL`]).
    pub fn ttl(mut self, ttl: Duration) -> Self {
        if !ttl.is_zero() {
            self.ttl = ttl;
        }
        self
    }

    /// Builds the cache.
    pub fn build(self) -> InMemoryCache {
        let capacity = NonZeroUsize::new(self.max_size)
            .unwrap_or_else(|| NonZeroUsize::new(DEFAULT_CACHE_MAX_SIZE).unwrap());
        InMemoryCache {
            ttl: self.ttl,
            entries: Mutex::new(LruCache::new(capacity)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(ip: &str) -> IpInfo {
        IpInfo {
            ip: Some(ip.parse().unwrap()),
            ..Default::default()
        }
    }

    #[test]
    fn get_returns_stored_value() {
        let cache = InMemoryCache::new();
        assert_eq!(cache.get("k"), None);
        cache.set("k", info("8.8.8.8"));
        assert_eq!(cache.get("k"), Some(info("8.8.8.8")));
    }

    #[test]
    fn set_overwrites_existing_entry() {
        let cache = InMemoryCache::new();
        cache.set("k", info("8.8.8.8"));
        cache.set("k", info("1.1.1.1"));
        assert_eq!(cache.get("k"), Some(info("1.1.1.1")));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn evicts_least_recently_used_entry() {
        let cache = InMemoryCache::builder().max_size(2).build();
        cache.set("a", info("1.1.1.1"));
        cache.set("b", info("2.2.2.2"));
        // Touch "a" so that "b" becomes the least recently used entry.
        assert!(cache.get("a").is_some());
        cache.set("c", info("3.3.3.3"));
        assert!(cache.get("a").is_some());
        assert!(cache.get("b").is_none());
        assert!(cache.get("c").is_some());
    }

    #[test]
    fn expires_entries_after_ttl() {
        let cache = InMemoryCache::builder()
            .ttl(Duration::from_millis(10))
            .build();
        cache.set("k", info("8.8.8.8"));
        assert!(cache.get("k").is_some());
        std::thread::sleep(Duration::from_millis(20));
        assert!(cache.get("k").is_none());
        assert!(cache.is_empty(), "expired entry should be removed on read");
    }

    #[test]
    fn invalidate_removes_single_entry() {
        let cache = InMemoryCache::new();
        cache.set("a", info("1.1.1.1"));
        cache.set("b", info("2.2.2.2"));
        cache.invalidate("a");
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
    }

    #[test]
    fn invalidate_all_removes_every_entry() {
        let cache = InMemoryCache::new();
        cache.set("a", info("1.1.1.1"));
        cache.set("b", info("2.2.2.2"));
        cache.invalidate_all();
        assert!(cache.is_empty());
    }

    #[test]
    fn zero_builder_values_keep_defaults() {
        let cache = InMemoryCache::builder()
            .max_size(0)
            .ttl(Duration::ZERO)
            .build();
        assert_eq!(cache.ttl, DEFAULT_CACHE_TTL);
        cache.set("k", info("8.8.8.8"));
        assert!(cache.get("k").is_some());
    }
}
