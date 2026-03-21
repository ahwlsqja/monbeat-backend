//! MvDatabase — revm `Database` implementation backed by MVHashMap.
//!
//! Routes every state read through the MVHashMap first (to see writes from prior
//! transactions in the block), then falls back to the base `StateProvider` for
//! cache misses. Records all reads in a `ReadSet` for later OCC validation, and
//! exposes a `WriteSet` for the execution harness to record writes into.
//!
//! # Field-Level Tracking
//!
//! revm's `Database::basic()` returns the entire `AccountInfo` (balance, nonce,
//! code_hash) as one struct, but Block-STM needs per-field conflict detection.
//! MvDatabase splits `basic()` into three separate MVHashMap lookups (Balance,
//! Nonce, CodeHash), records each as an independent read, and reassembles the
//! result. If any field has an ESTIMATE marker, the entire call returns an error.

use std::sync::Arc;

use monad_state::StateProvider;
use monad_types::{EvmError, B256, U256};

use revm::{
    database_interface::{DBErrorMarker, Database},
    state::{AccountInfo as RevmAccountInfo, Bytecode},
};

use crate::read_write_sets::{ReadSet, WriteSet};
use crate::types::{LocationKey, MvReadResult, ReadOrigin, TxIndex, WriteValue};
use crate::versioned_state::MVHashMap;

// ── Error type ──────────────────────────────────────────────────────────────

/// Error type for MvDatabase, wrapping `EvmError`.
///
/// Implements `DBErrorMarker` so revm accepts it as a valid database error.
/// The critical case is `EvmError::ReadEstimate` — when a read hits an ESTIMATE
/// marker, this error propagates up through the EVM execution, signaling the
/// scheduler to suspend the transaction.
#[derive(Debug)]
pub struct MvDatabaseError(pub EvmError);

impl std::fmt::Display for MvDatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MvDatabase error: {}", self.0)
    }
}

impl std::error::Error for MvDatabaseError {}
impl DBErrorMarker for MvDatabaseError {}

impl From<EvmError> for MvDatabaseError {
    fn from(e: EvmError) -> Self {
        MvDatabaseError(e)
    }
}

// ── MvDatabase ──────────────────────────────────────────────────────────────

/// Database implementation for Block-STM parallel execution.
///
/// Each worker thread gets its own `MvDatabase` instance that shares the
/// `MVHashMap` (via `Arc`) but owns its own `ReadSet` and `WriteSet`.
///
/// The `WriteSet` is NOT populated during `Database` reads — it is filled
/// by the caller after EVM execution extracts state changes from revm's
/// journal. This separation is critical: only actual writes (SSTORE, balance
/// changes, nonce increments) go into the WriteSet, not reads.
pub struct MvDatabase {
    /// Shared multi-version state — all workers read/write to this.
    mv_state: Arc<MVHashMap>,
    /// Base state fallback — used when MVHashMap has no entry for a location.
    base_state: Arc<dyn StateProvider>,
    /// This transaction's index in the block.
    tx_index: TxIndex,
    /// Records all state reads during execution (for OCC validation).
    read_set: ReadSet,
    /// Records all state writes (populated by caller after execution).
    write_set: WriteSet,
}

impl MvDatabase {
    /// Create a new MvDatabase for the given transaction.
    pub fn new(
        mv_state: Arc<MVHashMap>,
        base_state: Arc<dyn StateProvider>,
        tx_index: TxIndex,
    ) -> Self {
        Self {
            mv_state,
            base_state,
            tx_index,
            read_set: ReadSet::new(),
            write_set: WriteSet::new(),
        }
    }

    /// Move the ReadSet out for scheduler consumption after execution.
    pub fn take_read_set(&mut self) -> ReadSet {
        std::mem::take(&mut self.read_set)
    }

    /// Move the WriteSet out for scheduler consumption after execution.
    pub fn take_write_set(&mut self) -> WriteSet {
        std::mem::take(&mut self.write_set)
    }

    /// Record a write (called by the execution harness after extracting
    /// state changes from revm's journal, NOT during Database reads).
    pub fn record_write(&mut self, location: LocationKey, value: WriteValue) {
        self.write_set.record(location, value);
    }

    /// Get this transaction's index.
    pub fn tx_index(&self) -> TxIndex {
        self.tx_index
    }

    /// Read a single field from MVHashMap, falling back to a provided default
    /// value from base state. Records the read origin in the ReadSet.
    ///
    /// Returns `Err` if the MVHashMap entry is an ESTIMATE marker.
    fn read_field<T, F>(
        &mut self,
        location: LocationKey,
        extract: F,
        base_value: T,
    ) -> Result<T, MvDatabaseError>
    where
        F: FnOnce(&WriteValue) -> T,
    {
        match self.mv_state.read(&location, self.tx_index) {
            MvReadResult::Value(ref write_value, tx_idx, incarnation) => {
                let value = extract(write_value);
                self.read_set.record(
                    location,
                    ReadOrigin::MvHashMap {
                        tx_index: tx_idx,
                        incarnation,
                    },
                );
                Ok(value)
            }
            MvReadResult::Estimate(est_tx_index) => {
                Err(MvDatabaseError(EvmError::ReadEstimate {
                    tx_index: est_tx_index,
                    location: format!("{:?}", location),
                }))
            }
            MvReadResult::NotFound => {
                self.read_set.record(location, ReadOrigin::Storage);
                Ok(base_value)
            }
        }
    }
}

impl Database for MvDatabase {
    type Error = MvDatabaseError;

    /// Read account info by splitting into per-field MVHashMap lookups.
    ///
    /// Checks Balance, Nonce, and CodeHash independently. Each is recorded
    /// as a separate read in the ReadSet. If any field has an ESTIMATE marker,
    /// the entire call returns an error.
    fn basic(
        &mut self,
        address: alloy_primitives::Address,
    ) -> Result<Option<RevmAccountInfo>, Self::Error> {
        // First, get the base account from storage.
        let base_account = self
            .base_state
            .basic_account(address)
            .map_err(MvDatabaseError::from)?;

        let (base_balance, base_nonce, base_code_hash) = match &base_account {
            Some(acct) => (acct.balance, acct.nonce, acct.code_hash),
            None => {
                // Account doesn't exist in base state. Check if MVHashMap has
                // any fields for it (e.g., a prior tx created the account).
                // Use zero/empty defaults for fields not in MVHashMap.
                (U256::ZERO, 0u64, monad_types::KECCAK_EMPTY)
            }
        };

        // Read each field independently from MVHashMap.
        let balance = self.read_field(
            LocationKey::Balance(address),
            |wv| match wv {
                WriteValue::Balance(b) => *b,
                _ => base_balance,
            },
            base_balance,
        )?;

        let nonce = self.read_field(
            LocationKey::Nonce(address),
            |wv| match wv {
                WriteValue::Nonce(n) => *n,
                _ => base_nonce,
            },
            base_nonce,
        )?;

        let code_hash = self.read_field(
            LocationKey::CodeHash(address),
            |wv| match wv {
                WriteValue::CodeHash(h) => *h,
                _ => base_code_hash,
            },
            base_code_hash,
        )?;

        // If the account doesn't exist in base state AND all three fields
        // came from storage (no MVHashMap writes), the account truly doesn't exist.
        if base_account.is_none() {
            // Check if any field was read from MVHashMap (meaning a prior tx created it).
            let has_mv_writes = balance != U256::ZERO
                || nonce != 0
                || code_hash != monad_types::KECCAK_EMPTY;
            if !has_mv_writes {
                return Ok(None);
            }
        }

        // Get code bytes from base state (code is immutable per-block).
        let code = self
            .base_state
            .code_by_hash(code_hash)
            .map_err(MvDatabaseError::from)?;

        Ok(Some(RevmAccountInfo {
            balance,
            nonce,
            code_hash,
            code: if code.is_empty() {
                None
            } else {
                Some(Bytecode::new_raw(code))
            },
            account_id: None,
        }))
    }

    /// Read a storage slot, checking MVHashMap first then falling back to base state.
    fn storage(
        &mut self,
        address: alloy_primitives::Address,
        index: U256,
    ) -> Result<U256, Self::Error> {
        let location = LocationKey::Storage(address, index);

        match self.mv_state.read(&location, self.tx_index) {
            MvReadResult::Value(ref write_value, tx_idx, incarnation) => {
                let value = match write_value {
                    WriteValue::Storage(v) => *v,
                    _ => {
                        // Unexpected write type at a Storage location — fall back.
                        let base = self
                            .base_state
                            .storage(address, index)
                            .map_err(MvDatabaseError::from)?;
                        base
                    }
                };
                self.read_set.record(
                    location,
                    ReadOrigin::MvHashMap {
                        tx_index: tx_idx,
                        incarnation,
                    },
                );
                Ok(value)
            }
            MvReadResult::Estimate(est_tx_index) => {
                Err(MvDatabaseError(EvmError::ReadEstimate {
                    tx_index: est_tx_index,
                    location: format!("{:?}", location),
                }))
            }
            MvReadResult::NotFound => {
                self.read_set.record(location, ReadOrigin::Storage);
                let value = self
                    .base_state
                    .storage(address, index)
                    .map_err(MvDatabaseError::from)?;
                Ok(value)
            }
        }
    }

    /// Read code by hash — goes straight to base state (immutable per-block).
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        let bytes = self
            .base_state
            .code_by_hash(code_hash)
            .map_err(MvDatabaseError::from)?;
        Ok(if bytes.is_empty() {
            Bytecode::default()
        } else {
            Bytecode::new_raw(bytes)
        })
    }

    /// Read block hash — goes straight to base state (immutable per-block).
    fn block_hash(&mut self, number: u64) -> Result<B256, Self::Error> {
        let hash = self
            .base_state
            .block_hash(number)
            .map_err(MvDatabaseError::from)?;
        Ok(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;
    use monad_state::InMemoryState;
    use monad_types::AccountInfo;

    fn test_addr() -> alloy_primitives::Address {
        address!("0x0000000000000000000000000000000000000001")
    }

    fn make_base_state() -> Arc<dyn StateProvider> {
        let state = InMemoryState::new()
            .with_account(
                test_addr(),
                AccountInfo::new(U256::from(1000u64), 5),
            )
            .with_storage(test_addr(), U256::from(0), U256::from(42));
        Arc::new(state)
    }

    // ---- MvDatabase reads from base state when MVHashMap is empty ----

    #[test]
    fn basic_reads_from_base_state() {
        let mv = Arc::new(MVHashMap::new());
        let base = make_base_state();
        let mut db = MvDatabase::new(mv, base, 0);

        let acct = db.basic(test_addr()).unwrap().unwrap();
        assert_eq!(acct.balance, U256::from(1000u64));
        assert_eq!(acct.nonce, 5);

        // Should have recorded 3 reads (Balance, Nonce, CodeHash), all from Storage.
        let rs = db.take_read_set();
        assert_eq!(rs.len(), 3);
        for (_, origin) in rs.iter() {
            assert!(matches!(origin, ReadOrigin::Storage));
        }
    }

    #[test]
    fn storage_reads_from_base_state() {
        let mv = Arc::new(MVHashMap::new());
        let base = make_base_state();
        let mut db = MvDatabase::new(mv, base, 0);

        let val = db.storage(test_addr(), U256::from(0)).unwrap();
        assert_eq!(val, U256::from(42));

        let rs = db.take_read_set();
        assert_eq!(rs.len(), 1);
        let (loc, origin) = rs.iter().next().unwrap();
        assert!(matches!(loc, LocationKey::Storage(_, _)));
        assert!(matches!(origin, ReadOrigin::Storage));
    }

    // ---- MvDatabase reads from MVHashMap when prior tx wrote a value ----

    #[test]
    fn basic_reads_balance_from_mvhashmap() {
        let mv = Arc::new(MVHashMap::new());
        let base = make_base_state();

        // tx=0 wrote a new balance for test_addr.
        mv.write(
            LocationKey::Balance(test_addr()),
            0,
            0,
            WriteValue::Balance(U256::from(9999)),
        );

        // tx=1 reads — should see tx=0's balance, base nonce and code_hash.
        let mut db = MvDatabase::new(mv, base, 1);
        let acct = db.basic(test_addr()).unwrap().unwrap();
        assert_eq!(acct.balance, U256::from(9999));
        assert_eq!(acct.nonce, 5); // from base state

        // ReadSet: Balance from MvHashMap, Nonce and CodeHash from Storage.
        let rs = db.take_read_set();
        assert_eq!(rs.len(), 3);
        let mut mv_count = 0;
        let mut storage_count = 0;
        for (_, origin) in rs.iter() {
            match origin {
                ReadOrigin::MvHashMap { .. } => mv_count += 1,
                ReadOrigin::Storage => storage_count += 1,
                _ => {}
            }
        }
        assert_eq!(mv_count, 1);
        assert_eq!(storage_count, 2);
    }

    #[test]
    fn storage_reads_from_mvhashmap() {
        let mv = Arc::new(MVHashMap::new());
        let base = make_base_state();

        // tx=0 wrote a storage slot.
        mv.write(
            LocationKey::Storage(test_addr(), U256::from(0)),
            0,
            0,
            WriteValue::Storage(U256::from(999)),
        );

        let mut db = MvDatabase::new(mv, base, 1);
        let val = db.storage(test_addr(), U256::from(0)).unwrap();
        assert_eq!(val, U256::from(999));

        let rs = db.take_read_set();
        assert_eq!(rs.len(), 1);
        let (_, origin) = rs.iter().next().unwrap();
        assert!(matches!(
            origin,
            ReadOrigin::MvHashMap {
                tx_index: 0,
                incarnation: 0,
            }
        ));
    }

    // ---- ESTIMATE marker returns error ----

    #[test]
    fn basic_returns_error_on_estimate_balance() {
        let mv = Arc::new(MVHashMap::new());
        let base = make_base_state();

        mv.write(
            LocationKey::Balance(test_addr()),
            0,
            0,
            WriteValue::Balance(U256::from(100)),
        );
        mv.mark_estimate(0);

        let mut db = MvDatabase::new(mv, base, 1);
        let result = db.basic(test_addr());
        assert!(result.is_err());

        let err = result.unwrap_err();
        match &err.0 {
            EvmError::ReadEstimate { tx_index, .. } => assert_eq!(*tx_index, 0),
            other => panic!("expected ReadEstimate, got {:?}", other),
        }
    }

    #[test]
    fn storage_returns_error_on_estimate() {
        let mv = Arc::new(MVHashMap::new());
        let base = make_base_state();

        mv.write(
            LocationKey::Storage(test_addr(), U256::from(0)),
            0,
            0,
            WriteValue::Storage(U256::from(42)),
        );
        mv.mark_estimate(0);

        let mut db = MvDatabase::new(mv, base, 1);
        let result = db.storage(test_addr(), U256::from(0));
        assert!(result.is_err());

        let err = result.unwrap_err();
        match &err.0 {
            EvmError::ReadEstimate { tx_index, .. } => assert_eq!(*tx_index, 0),
            other => panic!("expected ReadEstimate, got {:?}", other),
        }
    }

    // ---- code_by_hash and block_hash bypass MVHashMap ----

    #[test]
    fn code_by_hash_goes_to_base_state() {
        let mv = Arc::new(MVHashMap::new());
        let base = make_base_state();
        let mut db = MvDatabase::new(mv, base, 0);

        // code_by_hash should not check MVHashMap and should not record reads.
        let code = db.code_by_hash(B256::ZERO).unwrap();
        assert!(code.is_empty() || code.bytes().is_empty());

        let rs = db.take_read_set();
        assert!(rs.is_empty(), "code_by_hash should not record reads");
    }

    #[test]
    fn block_hash_goes_to_base_state() {
        let mv = Arc::new(MVHashMap::new());
        let base: Arc<dyn StateProvider> = Arc::new(
            InMemoryState::new().with_block_hash(100, B256::with_last_byte(0xAA)),
        );
        let mut db = MvDatabase::new(mv, base, 0);

        let hash = db.block_hash(100).unwrap();
        assert_eq!(hash, B256::with_last_byte(0xAA));

        let rs = db.take_read_set();
        assert!(rs.is_empty(), "block_hash should not record reads");
    }

    // ---- WriteSet is separate from reads ----

    #[test]
    fn write_set_not_populated_during_reads() {
        let mv = Arc::new(MVHashMap::new());
        let base = make_base_state();
        let mut db = MvDatabase::new(mv, base, 0);

        // Perform reads.
        let _ = db.basic(test_addr()).unwrap();
        let _ = db.storage(test_addr(), U256::from(0)).unwrap();

        // WriteSet should be empty — reads don't populate it.
        let ws = db.take_write_set();
        assert!(ws.is_empty(), "WriteSet must not contain reads");
    }

    #[test]
    fn record_write_populates_write_set() {
        let mv = Arc::new(MVHashMap::new());
        let base = make_base_state();
        let mut db = MvDatabase::new(mv, base, 0);

        db.record_write(
            LocationKey::Balance(test_addr()),
            WriteValue::Balance(U256::from(500)),
        );
        db.record_write(
            LocationKey::Storage(test_addr(), U256::from(1)),
            WriteValue::Storage(U256::from(77)),
        );

        let ws = db.take_write_set();
        assert_eq!(ws.len(), 2);
    }

    // ---- take_read_set and take_write_set move ownership ----

    #[test]
    fn take_sets_resets_to_empty() {
        let mv = Arc::new(MVHashMap::new());
        let base = make_base_state();
        let mut db = MvDatabase::new(mv, base, 0);

        db.record_write(
            LocationKey::Balance(test_addr()),
            WriteValue::Balance(U256::from(500)),
        );
        let _ = db.basic(test_addr()).unwrap();

        let rs = db.take_read_set();
        let ws = db.take_write_set();
        assert!(!rs.is_empty());
        assert!(!ws.is_empty());

        // After take, internal sets are empty.
        let rs2 = db.take_read_set();
        let ws2 = db.take_write_set();
        assert!(rs2.is_empty());
        assert!(ws2.is_empty());
    }

    // ---- Field-level tracking: basic() records per-field reads ----

    #[test]
    fn basic_records_three_separate_field_reads() {
        let mv = Arc::new(MVHashMap::new());
        let base = make_base_state();

        // Write all three fields from different txs.
        mv.write(
            LocationKey::Balance(test_addr()),
            0,
            0,
            WriteValue::Balance(U256::from(2000)),
        );
        mv.write(
            LocationKey::Nonce(test_addr()),
            1,
            0,
            WriteValue::Nonce(10),
        );
        mv.write(
            LocationKey::CodeHash(test_addr()),
            2,
            0,
            WriteValue::CodeHash(B256::with_last_byte(0xFF)),
        );

        let mut db = MvDatabase::new(mv, base, 3);
        let acct = db.basic(test_addr()).unwrap().unwrap();

        assert_eq!(acct.balance, U256::from(2000));
        assert_eq!(acct.nonce, 10);
        assert_eq!(acct.code_hash, B256::with_last_byte(0xFF));

        // All three reads should be from MvHashMap with different tx_indexes.
        let rs = db.take_read_set();
        assert_eq!(rs.len(), 3);

        let reads: std::collections::BTreeMap<_, _> = rs.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        assert_eq!(
            reads[&LocationKey::Balance(test_addr())],
            ReadOrigin::MvHashMap {
                tx_index: 0,
                incarnation: 0,
            }
        );
        assert_eq!(
            reads[&LocationKey::Nonce(test_addr())],
            ReadOrigin::MvHashMap {
                tx_index: 1,
                incarnation: 0,
            }
        );
        assert_eq!(
            reads[&LocationKey::CodeHash(test_addr())],
            ReadOrigin::MvHashMap {
                tx_index: 2,
                incarnation: 0,
            }
        );
    }

    // ---- Missing account returns None ----

    #[test]
    fn basic_returns_none_for_missing_account() {
        let mv = Arc::new(MVHashMap::new());
        let base: Arc<dyn StateProvider> = Arc::new(InMemoryState::new());
        let mut db = MvDatabase::new(mv, base, 0);

        let result = db.basic(test_addr()).unwrap();
        assert!(result.is_none());
    }

    // ---- tx_index getter ----

    #[test]
    fn tx_index_getter() {
        let mv = Arc::new(MVHashMap::new());
        let base: Arc<dyn StateProvider> = Arc::new(InMemoryState::new());
        let db = MvDatabase::new(mv, base, 42);
        assert_eq!(db.tx_index(), 42);
    }

    // ---- ReadSet validation integration ----

    #[test]
    fn read_set_validates_correctly_after_basic() {
        let mv = Arc::new(MVHashMap::new());
        let base = make_base_state();

        // tx=0 writes balance.
        mv.write(
            LocationKey::Balance(test_addr()),
            0,
            0,
            WriteValue::Balance(U256::from(9999)),
        );

        // tx=1 reads via MvDatabase.
        let mut db = MvDatabase::new(Arc::clone(&mv), base, 1);
        let _ = db.basic(test_addr()).unwrap();
        let rs = db.take_read_set();

        // Validate should pass — MVHashMap hasn't changed.
        assert!(rs.validate(&mv, 1));

        // Now tx=0 re-executes with incarnation=1.
        mv.write(
            LocationKey::Balance(test_addr()),
            0,
            1,
            WriteValue::Balance(U256::from(5555)),
        );

        // Validate should fail — balance version changed.
        assert!(!rs.validate(&mv, 1));
    }
}
