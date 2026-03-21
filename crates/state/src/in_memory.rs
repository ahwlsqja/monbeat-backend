use std::collections::{BTreeMap, HashMap};

use monad_types::{AccountInfo, EvmError, Address, Bytes, U256, B256};

use crate::StateProvider;

/// HashMap-backed in-memory state for testing.
///
/// Provides a builder-style API for test ergonomics — tests can chain
/// `.with_account(addr, info)` and `.with_storage(addr, slot, value)` calls
/// to set up state before executing transactions.
///
/// # Example
///
/// ```rust
/// use monad_state::InMemoryState;
/// use monad_types::{AccountInfo, Address, U256};
///
/// let state = InMemoryState::new()
///     .with_account(
///         Address::with_last_byte(1),
///         AccountInfo::new(U256::from(1_000_000u64), 0),
///     )
///     .with_storage(
///         Address::with_last_byte(1),
///         U256::from(0u64),
///         U256::from(42u64),
///     );
/// ```
#[derive(Debug, Clone, Default)]
pub struct InMemoryState {
    /// Account info keyed by address.
    accounts: HashMap<Address, AccountInfo>,
    /// Storage values keyed by (address, slot).
    storage: HashMap<(Address, U256), U256>,
    /// Contract bytecode keyed by code hash.
    code: HashMap<B256, Bytes>,
    /// Block hashes keyed by block number.
    block_hashes: HashMap<u64, B256>,
}

impl InMemoryState {
    /// Creates a new empty in-memory state.
    pub fn new() -> Self {
        Self::default()
    }

    // ── Builder methods (consume and return self for chaining) ──

    /// Adds an account to the state. Returns `self` for chaining.
    pub fn with_account(mut self, address: Address, info: AccountInfo) -> Self {
        self.accounts.insert(address, info);
        self
    }

    /// Adds a storage slot value. Returns `self` for chaining.
    pub fn with_storage(mut self, address: Address, slot: U256, value: U256) -> Self {
        self.storage.insert((address, slot), value);
        self
    }

    /// Adds contract bytecode by code hash. Returns `self` for chaining.
    pub fn with_code(mut self, code_hash: B256, bytecode: Bytes) -> Self {
        self.code.insert(code_hash, bytecode);
        self
    }

    /// Adds a block hash. Returns `self` for chaining.
    pub fn with_block_hash(mut self, number: u64, hash: B256) -> Self {
        self.block_hashes.insert(number, hash);
        self
    }

    // ── Mutation methods (for post-execution state updates) ──

    /// Inserts or updates an account. Used by the executor to commit state changes.
    pub fn insert_account(&mut self, address: Address, info: AccountInfo) {
        self.accounts.insert(address, info);
    }

    /// Returns the account info for an address, if it exists.
    /// Used for post-execution state inspection in tests.
    pub fn get_account(&self, address: &Address) -> Option<&AccountInfo> {
        self.accounts.get(address)
    }

    /// Inserts or updates a storage slot value.
    pub fn insert_storage(&mut self, address: Address, slot: U256, value: U256) {
        self.storage.insert((address, slot), value);
    }

    /// Returns the storage value at (address, slot), or `U256::ZERO` if not set.
    pub fn get_storage(&self, address: &Address, slot: &U256) -> U256 {
        self.storage
            .get(&(*address, *slot))
            .copied()
            .unwrap_or(U256::ZERO)
    }

    /// Inserts contract bytecode by code hash.
    pub fn insert_code(&mut self, code_hash: B256, bytecode: Bytes) {
        self.code.insert(code_hash, bytecode);
    }

    // ── Sorted accessors (for deterministic state root computation) ──

    /// Returns a sorted copy of all accounts as a `BTreeMap`.
    ///
    /// Used for deterministic state root computation — `HashMap` iteration
    /// order is non-deterministic, so we convert to `BTreeMap` (sorted by
    /// `Address`) before hashing.
    pub fn accounts(&self) -> BTreeMap<Address, AccountInfo> {
        self.accounts.iter().map(|(k, v)| (*k, v.clone())).collect()
    }

    /// Returns a sorted copy of all storage entries as a `BTreeMap`.
    ///
    /// Keys are `(Address, U256)` pairs (contract address, storage slot),
    /// sorted lexicographically. Used for deterministic state root computation.
    pub fn all_storage(&self) -> BTreeMap<(Address, U256), U256> {
        self.storage.iter().map(|(k, v)| (*k, *v)).collect()
    }
}

impl StateProvider for InMemoryState {
    fn basic_account(&self, address: Address) -> Result<Option<AccountInfo>, EvmError> {
        Ok(self.accounts.get(&address).cloned())
    }

    fn storage(&self, address: Address, slot: U256) -> Result<U256, EvmError> {
        Ok(self
            .storage
            .get(&(address, slot))
            .copied()
            .unwrap_or(U256::ZERO))
    }

    fn code_by_hash(&self, code_hash: B256) -> Result<Bytes, EvmError> {
        Ok(self
            .code
            .get(&code_hash)
            .cloned()
            .unwrap_or_default())
    }

    fn block_hash(&self, number: u64) -> Result<B256, EvmError> {
        Ok(self
            .block_hashes
            .get(&number)
            .copied()
            .unwrap_or(B256::ZERO))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monad_types::KECCAK_EMPTY;

    #[test]
    fn test_insert_and_read_account() {
        let addr = Address::with_last_byte(0x01);
        let info = AccountInfo::new(U256::from(1_000_000u64), 5);
        let state = InMemoryState::new().with_account(addr, info.clone());

        let result = state.basic_account(addr).unwrap();
        assert_eq!(result, Some(info));
    }

    #[test]
    fn test_missing_account_returns_none() {
        let state = InMemoryState::new();
        let result = state.basic_account(Address::with_last_byte(0xFF)).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_insert_and_read_storage() {
        let addr = Address::with_last_byte(0x01);
        let slot = U256::from(0u64);
        let value = U256::from(42u64);
        let state = InMemoryState::new().with_storage(addr, slot, value);

        let result = state.storage(addr, slot).unwrap();
        assert_eq!(result, value);
    }

    #[test]
    fn test_missing_storage_returns_zero() {
        let state = InMemoryState::new();
        let result = state.storage(Address::with_last_byte(0x01), U256::from(99u64)).unwrap();
        assert_eq!(result, U256::ZERO);
    }

    #[test]
    fn test_code_round_trip() {
        let code_hash = B256::with_last_byte(0xAB);
        let bytecode = Bytes::from(vec![0x60, 0x00, 0x60, 0x00, 0xf3]);
        let state = InMemoryState::new().with_code(code_hash, bytecode.clone());

        let result = state.code_by_hash(code_hash).unwrap();
        assert_eq!(result, bytecode);
    }

    #[test]
    fn test_missing_code_returns_empty_bytes() {
        let state = InMemoryState::new();
        let result = state.code_by_hash(B256::with_last_byte(0xFF)).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_block_hash_round_trip() {
        let hash = B256::with_last_byte(0xCC);
        let state = InMemoryState::new().with_block_hash(100, hash);

        let result = state.block_hash(100).unwrap();
        assert_eq!(result, hash);
    }

    #[test]
    fn test_missing_block_hash_returns_zero() {
        let state = InMemoryState::new();
        let result = state.block_hash(999).unwrap();
        assert_eq!(result, B256::ZERO);
    }

    #[test]
    fn test_builder_chaining() {
        let addr = Address::with_last_byte(0x01);
        let state = InMemoryState::new()
            .with_account(addr, AccountInfo::new(U256::from(100u64), 0))
            .with_storage(addr, U256::from(0u64), U256::from(42u64))
            .with_code(B256::with_last_byte(0xAA), Bytes::from(vec![0xFE]))
            .with_block_hash(1, B256::with_last_byte(0xBB));

        assert!(state.basic_account(addr).unwrap().is_some());
        assert_eq!(state.storage(addr, U256::from(0u64)).unwrap(), U256::from(42u64));
        assert_eq!(state.code_by_hash(B256::with_last_byte(0xAA)).unwrap().len(), 1);
        assert_eq!(state.block_hash(1).unwrap(), B256::with_last_byte(0xBB));
    }

    #[test]
    fn test_mutation_methods() {
        let addr = Address::with_last_byte(0x02);
        let mut state = InMemoryState::new();

        // insert_account / get_account
        state.insert_account(addr, AccountInfo::new(U256::from(500u64), 1));
        let acct = state.get_account(&addr).unwrap();
        assert_eq!(acct.balance, U256::from(500u64));
        assert_eq!(acct.nonce, 1);

        // insert_storage / get_storage
        state.insert_storage(addr, U256::from(7u64), U256::from(99u64));
        assert_eq!(state.get_storage(&addr, &U256::from(7u64)), U256::from(99u64));
        assert_eq!(state.get_storage(&addr, &U256::from(8u64)), U256::ZERO);

        // insert_code
        let code_hash = B256::with_last_byte(0xDD);
        state.insert_code(code_hash, Bytes::from(vec![0x00]));
        assert_eq!(state.code_by_hash(code_hash).unwrap(), Bytes::from(vec![0x00]));
    }

    #[test]
    fn test_overwrite_account() {
        let addr = Address::with_last_byte(0x03);
        let mut state = InMemoryState::new()
            .with_account(addr, AccountInfo::new(U256::from(100u64), 0));

        // Overwrite with updated balance
        state.insert_account(addr, AccountInfo::new(U256::from(200u64), 1));
        let acct = state.get_account(&addr).unwrap();
        assert_eq!(acct.balance, U256::from(200u64));
        assert_eq!(acct.nonce, 1);
    }

    #[test]
    fn test_state_provider_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InMemoryState>();
    }

    #[test]
    fn test_eoa_account_has_keccak_empty_code_hash() {
        let addr = Address::with_last_byte(0x04);
        let state = InMemoryState::new()
            .with_account(addr, AccountInfo::new(U256::from(1u64), 0));

        let acct = state.basic_account(addr).unwrap().unwrap();
        assert_eq!(acct.code_hash, KECCAK_EMPTY);
        assert!(acct.is_empty_code());
    }

    #[test]
    fn test_multiple_storage_slots_same_address() {
        let addr = Address::with_last_byte(0x05);
        let state = InMemoryState::new()
            .with_storage(addr, U256::from(0u64), U256::from(10u64))
            .with_storage(addr, U256::from(1u64), U256::from(20u64))
            .with_storage(addr, U256::from(2u64), U256::from(30u64));

        assert_eq!(state.storage(addr, U256::from(0u64)).unwrap(), U256::from(10u64));
        assert_eq!(state.storage(addr, U256::from(1u64)).unwrap(), U256::from(20u64));
        assert_eq!(state.storage(addr, U256::from(2u64)).unwrap(), U256::from(30u64));
        assert_eq!(state.storage(addr, U256::from(3u64)).unwrap(), U256::ZERO);
    }

    #[test]
    fn test_accounts_returns_sorted_btree_map() {
        // Insert accounts in reverse address order.
        let addr3 = Address::with_last_byte(0x03);
        let addr1 = Address::with_last_byte(0x01);
        let addr2 = Address::with_last_byte(0x02);
        let state = InMemoryState::new()
            .with_account(addr3, AccountInfo::new(U256::from(300u64), 3))
            .with_account(addr1, AccountInfo::new(U256::from(100u64), 1))
            .with_account(addr2, AccountInfo::new(U256::from(200u64), 2));

        let sorted = state.accounts();
        assert_eq!(sorted.len(), 3);

        // BTreeMap keys should be in ascending address order.
        let keys: Vec<Address> = sorted.keys().copied().collect();
        assert_eq!(keys, vec![addr1, addr2, addr3]);
        assert_eq!(sorted[&addr1].balance, U256::from(100u64));
        assert_eq!(sorted[&addr2].balance, U256::from(200u64));
        assert_eq!(sorted[&addr3].balance, U256::from(300u64));
    }

    #[test]
    fn test_accounts_empty_state() {
        let state = InMemoryState::new();
        let sorted = state.accounts();
        assert!(sorted.is_empty());
    }

    #[test]
    fn test_all_storage_returns_sorted_btree_map() {
        let addr2 = Address::with_last_byte(0x02);
        let addr1 = Address::with_last_byte(0x01);
        let state = InMemoryState::new()
            .with_storage(addr2, U256::from(1u64), U256::from(20u64))
            .with_storage(addr1, U256::from(0u64), U256::from(10u64))
            .with_storage(addr1, U256::from(1u64), U256::from(15u64));

        let sorted = state.all_storage();
        assert_eq!(sorted.len(), 3);

        // Keys should be in ascending (address, slot) order.
        let keys: Vec<(Address, U256)> = sorted.keys().copied().collect();
        assert_eq!(keys[0], (addr1, U256::from(0u64)));
        assert_eq!(keys[1], (addr1, U256::from(1u64)));
        assert_eq!(keys[2], (addr2, U256::from(1u64)));
    }

    #[test]
    fn test_all_storage_empty_state() {
        let state = InMemoryState::new();
        let sorted = state.all_storage();
        assert!(sorted.is_empty());
    }

    #[test]
    fn test_accounts_is_deterministic() {
        // Two states built with the same data should produce identical BTreeMaps.
        let addr1 = Address::with_last_byte(0x01);
        let addr2 = Address::with_last_byte(0x02);

        let state_a = InMemoryState::new()
            .with_account(addr1, AccountInfo::new(U256::from(100u64), 0))
            .with_account(addr2, AccountInfo::new(U256::from(200u64), 1));

        let state_b = InMemoryState::new()
            .with_account(addr2, AccountInfo::new(U256::from(200u64), 1))
            .with_account(addr1, AccountInfo::new(U256::from(100u64), 0));

        assert_eq!(state_a.accounts(), state_b.accounts());
    }
}
