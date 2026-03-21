//! Read-through caching decorator for `StateProvider`.
//!
//! `CachedStateProvider` wraps any `Arc<dyn StateProvider>` with concurrent
//! [`DashMap`] caches for accounts, storage, code, and block hashes. Each
//! `StateProvider` method checks the cache first; on miss, it delegates to
//! the inner provider, inserts the result into the cache, and returns it.
//!
//! ## Design
//!
//! - **Caches `None` results**: `basic_account()` may return `None` for
//!   non-existent accounts. The cache stores `Option<AccountInfo>` to
//!   avoid repeated misses.
//! - **Thread-safe**: `DashMap` is `Send + Sync` by design, so multiple
//!   EVM workers can safely share a single `CachedStateProvider`.
//! - **Block-scoped**: Callers should create a fresh `CachedStateProvider`
//!   per block execution. The struct does not enforce this — it's the
//!   caller's responsibility.
//!
//! ## Observability
//!
//! Call [`CachedStateProvider::stats()`] to retrieve `(hits, misses)` counts.
//! These are monotonically increasing `AtomicU64` counters — safe to read
//! concurrently while workers are still executing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use monad_types::{AccountInfo, Address, B256, Bytes, EvmError, U256};

use crate::StateProvider;

/// A read-through caching decorator for any [`StateProvider`].
///
/// On each state read, the cache is checked first. On miss, the inner
/// provider is queried, the result is cached, and then returned. This
/// avoids redundant reads when transactions are re-executed after OCC
/// conflicts.
///
/// # Usage
///
/// ```rust
/// use std::sync::Arc;
/// use monad_state::{CachedStateProvider, InMemoryState, StateProvider};
/// use monad_types::{AccountInfo, Address, U256};
///
/// let base = Arc::new(
///     InMemoryState::new()
///         .with_account(Address::with_last_byte(1), AccountInfo::new(U256::from(100u64), 0)),
/// );
/// let cached = CachedStateProvider::new(base);
///
/// // First call: cache miss, delegates to inner
/// let acct = cached.basic_account(Address::with_last_byte(1)).unwrap();
/// assert!(acct.is_some());
///
/// // Second call: cache hit
/// let acct2 = cached.basic_account(Address::with_last_byte(1)).unwrap();
/// assert_eq!(acct, acct2);
///
/// let (hits, misses) = cached.stats();
/// assert_eq!(hits, 1);
/// assert_eq!(misses, 1);
/// ```
pub struct CachedStateProvider {
    /// The underlying state provider to delegate to on cache misses.
    inner: Arc<dyn StateProvider>,
    /// Account cache. `Option<AccountInfo>` allows caching `None` (non-existent
    /// accounts) to avoid repeated misses.
    accounts: DashMap<Address, Option<AccountInfo>>,
    /// Storage cache keyed by `(address, slot)`.
    storage: DashMap<(Address, U256), U256>,
    /// Code cache keyed by code hash. Code is immutable per-block.
    code: DashMap<B256, Bytes>,
    /// Block hash cache keyed by block number.
    block_hashes: DashMap<u64, B256>,
    /// Number of cache hits (monotonically increasing).
    hits: AtomicU64,
    /// Number of cache misses (monotonically increasing).
    misses: AtomicU64,
}

impl CachedStateProvider {
    /// Creates a new `CachedStateProvider` wrapping the given inner provider.
    ///
    /// All caches start empty. Create a fresh instance per block execution.
    pub fn new(inner: Arc<dyn StateProvider>) -> Self {
        Self {
            inner,
            accounts: DashMap::new(),
            storage: DashMap::new(),
            code: DashMap::new(),
            block_hashes: DashMap::new(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Returns `(hits, misses)` cache statistics.
    ///
    /// Safe to call while workers are still executing — reads use `Relaxed`
    /// ordering, which is sufficient for diagnostic counters.
    pub fn stats(&self) -> (u64, u64) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }

    /// Records a cache hit.
    #[inline]
    fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a cache miss.
    #[inline]
    fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }
}

impl StateProvider for CachedStateProvider {
    fn basic_account(&self, address: Address) -> Result<Option<AccountInfo>, EvmError> {
        // Check cache first.
        if let Some(entry) = self.accounts.get(&address) {
            self.record_hit();
            return Ok(entry.value().clone());
        }

        // Cache miss — delegate to inner provider.
        let result = self.inner.basic_account(address)?;
        self.accounts.insert(address, result.clone());
        self.record_miss();
        Ok(result)
    }

    fn storage(&self, address: Address, slot: U256) -> Result<U256, EvmError> {
        let key = (address, slot);
        if let Some(entry) = self.storage.get(&key) {
            self.record_hit();
            return Ok(*entry.value());
        }

        let result = self.inner.storage(address, slot)?;
        self.storage.insert(key, result);
        self.record_miss();
        Ok(result)
    }

    fn code_by_hash(&self, code_hash: B256) -> Result<Bytes, EvmError> {
        if let Some(entry) = self.code.get(&code_hash) {
            self.record_hit();
            return Ok(entry.value().clone());
        }

        let result = self.inner.code_by_hash(code_hash)?;
        self.code.insert(code_hash, result.clone());
        self.record_miss();
        Ok(result)
    }

    fn block_hash(&self, number: u64) -> Result<B256, EvmError> {
        if let Some(entry) = self.block_hashes.get(&number) {
            self.record_hit();
            return Ok(*entry.value());
        }

        let result = self.inner.block_hash(number)?;
        self.block_hashes.insert(number, result);
        self.record_miss();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InMemoryState;

    /// Helper: create a CachedStateProvider wrapping an InMemoryState with
    /// one account, one storage slot, one code entry, and one block hash.
    fn make_populated_cached() -> CachedStateProvider {
        let addr = Address::with_last_byte(0x01);
        let inner = Arc::new(
            InMemoryState::new()
                .with_account(addr, AccountInfo::new(U256::from(1000u64), 1))
                .with_storage(addr, U256::from(0u64), U256::from(42u64))
                .with_code(B256::with_last_byte(0xAA), Bytes::from(vec![0x60, 0x00]))
                .with_block_hash(100, B256::with_last_byte(0xBB)),
        );
        CachedStateProvider::new(inner)
    }

    // ── 1. Basic hit/miss for accounts ──

    #[test]
    fn test_account_miss_then_hit() {
        let cached = make_populated_cached();
        let addr = Address::with_last_byte(0x01);

        // First call: miss
        let result = cached.basic_account(addr).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().balance, U256::from(1000u64));
        assert_eq!(cached.stats(), (0, 1));

        // Second call: hit
        let result2 = cached.basic_account(addr).unwrap();
        assert!(result2.is_some());
        assert_eq!(cached.stats(), (1, 1));
    }

    // ── 2. None account caching ──

    #[test]
    fn test_none_account_is_cached() {
        let cached = make_populated_cached();
        let missing_addr = Address::with_last_byte(0xFF);

        // First call: miss, returns None
        let result = cached.basic_account(missing_addr).unwrap();
        assert!(result.is_none());
        assert_eq!(cached.stats(), (0, 1));

        // Second call: hit, still returns None (cached)
        let result2 = cached.basic_account(missing_addr).unwrap();
        assert!(result2.is_none());
        assert_eq!(cached.stats(), (1, 1));
    }

    // ── 3. Storage caching ──

    #[test]
    fn test_storage_miss_then_hit() {
        let cached = make_populated_cached();
        let addr = Address::with_last_byte(0x01);
        let slot = U256::from(0u64);

        // Miss
        let val = cached.storage(addr, slot).unwrap();
        assert_eq!(val, U256::from(42u64));
        assert_eq!(cached.stats(), (0, 1));

        // Hit
        let val2 = cached.storage(addr, slot).unwrap();
        assert_eq!(val2, U256::from(42u64));
        assert_eq!(cached.stats(), (1, 1));
    }

    #[test]
    fn test_storage_zero_is_cached() {
        let cached = make_populated_cached();
        let addr = Address::with_last_byte(0x01);
        let missing_slot = U256::from(999u64);

        // Miss — returns U256::ZERO for non-existent slot
        let val = cached.storage(addr, missing_slot).unwrap();
        assert_eq!(val, U256::ZERO);
        assert_eq!(cached.stats(), (0, 1));

        // Hit — cached zero
        let val2 = cached.storage(addr, missing_slot).unwrap();
        assert_eq!(val2, U256::ZERO);
        assert_eq!(cached.stats(), (1, 1));
    }

    // ── 4. Code caching ──

    #[test]
    fn test_code_miss_then_hit() {
        let cached = make_populated_cached();
        let code_hash = B256::with_last_byte(0xAA);

        // Miss
        let code = cached.code_by_hash(code_hash).unwrap();
        assert_eq!(code, Bytes::from(vec![0x60, 0x00]));
        assert_eq!(cached.stats(), (0, 1));

        // Hit
        let code2 = cached.code_by_hash(code_hash).unwrap();
        assert_eq!(code2, Bytes::from(vec![0x60, 0x00]));
        assert_eq!(cached.stats(), (1, 1));
    }

    // ── 5. Block hash caching ──

    #[test]
    fn test_block_hash_miss_then_hit() {
        let cached = make_populated_cached();

        // Miss
        let hash = cached.block_hash(100).unwrap();
        assert_eq!(hash, B256::with_last_byte(0xBB));
        assert_eq!(cached.stats(), (0, 1));

        // Hit
        let hash2 = cached.block_hash(100).unwrap();
        assert_eq!(hash2, B256::with_last_byte(0xBB));
        assert_eq!(cached.stats(), (1, 1));
    }

    // ── 6. Stats counting across methods ──

    #[test]
    fn test_stats_counts_across_all_methods() {
        let cached = make_populated_cached();
        let addr = Address::with_last_byte(0x01);

        // 4 misses (one per method)
        cached.basic_account(addr).unwrap();
        cached.storage(addr, U256::from(0u64)).unwrap();
        cached.code_by_hash(B256::with_last_byte(0xAA)).unwrap();
        cached.block_hash(100).unwrap();
        assert_eq!(cached.stats(), (0, 4));

        // 4 hits (same calls again)
        cached.basic_account(addr).unwrap();
        cached.storage(addr, U256::from(0u64)).unwrap();
        cached.code_by_hash(B256::with_last_byte(0xAA)).unwrap();
        cached.block_hash(100).unwrap();
        assert_eq!(cached.stats(), (4, 4));
    }

    // ── 7. Send + Sync assertion ──

    #[test]
    fn test_cached_state_provider_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CachedStateProvider>();
    }

    // ── 8. Concurrent access from multiple threads ──

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let addr = Address::with_last_byte(0x01);
        let inner = Arc::new(
            InMemoryState::new()
                .with_account(addr, AccountInfo::new(U256::from(500u64), 2))
                .with_storage(addr, U256::from(0u64), U256::from(77u64)),
        );
        let cached = Arc::new(CachedStateProvider::new(inner));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let c = Arc::clone(&cached);
                thread::spawn(move || {
                    // Each thread reads account and storage 10 times
                    for _ in 0..10 {
                        let acct = c.basic_account(addr).unwrap();
                        assert_eq!(acct.unwrap().balance, U256::from(500u64));

                        let val = c.storage(addr, U256::from(0u64)).unwrap();
                        assert_eq!(val, U256::from(77u64));
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread panicked");
        }

        // All threads got correct values. Stats: at most 2 misses
        // (one account, one storage — the first thread to access each),
        // the rest are hits.
        let (hits, misses) = cached.stats();
        assert!(misses >= 1, "should have at least 1 miss per method");
        assert!(hits + misses == 160, "total accesses should be 8 threads * 10 iters * 2 methods = 160");
    }

    // ── 9. CachedStateProvider implements StateProvider trait ──

    #[test]
    fn test_cached_provider_as_trait_object() {
        let inner = Arc::new(InMemoryState::new().with_account(
            Address::with_last_byte(0x01),
            AccountInfo::new(U256::from(42u64), 0),
        ));
        let cached = CachedStateProvider::new(inner);

        // Verify it can be used as a trait object
        let provider: &dyn StateProvider = &cached;
        let result = provider.basic_account(Address::with_last_byte(0x01)).unwrap();
        assert_eq!(result.unwrap().balance, U256::from(42u64));
    }

    // ── 10. Fresh cache starts empty ──

    #[test]
    fn test_new_cache_starts_empty() {
        let inner = Arc::new(InMemoryState::new());
        let cached = CachedStateProvider::new(inner);
        assert_eq!(cached.stats(), (0, 0));
    }
}
