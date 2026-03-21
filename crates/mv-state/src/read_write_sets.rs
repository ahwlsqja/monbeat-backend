//! ReadSet and WriteSet for Block-STM conflict detection.
//!
//! A `ReadSet` records every state location a transaction read during execution,
//! along with the origin of that read (MVHashMap version or base storage). After
//! execution, the scheduler re-validates the ReadSet by re-reading each location
//! from the MVHashMap — if any read now returns a different version or an ESTIMATE
//! marker, validation fails and the transaction must be re-executed.
//!
//! A `WriteSet` records every state location a transaction wrote. After execution,
//! `apply_to()` publishes all writes to the MVHashMap so subsequent transactions
//! can see them.

use std::collections::BTreeMap;

use crate::types::{Incarnation, LocationKey, MvReadResult, ReadOrigin, TxIndex, WriteValue};
use crate::versioned_state::MVHashMap;

/// Records all state reads performed by a transaction during execution.
///
/// Used for OCC (optimistic concurrency control) validation: after execution,
/// each recorded read is re-checked against the MVHashMap. If any value has
/// changed (different version or ESTIMATE), the transaction has a conflict
/// and must be re-executed.
pub struct ReadSet {
    reads: BTreeMap<LocationKey, ReadOrigin>,
}

impl ReadSet {
    /// Create a new, empty ReadSet.
    pub fn new() -> Self {
        Self {
            reads: BTreeMap::new(),
        }
    }

    /// Record a read at the given location with its origin.
    pub fn record(&mut self, location: LocationKey, origin: ReadOrigin) {
        self.reads.insert(location, origin);
    }

    /// Validate all recorded reads against the current MVHashMap state.
    ///
    /// For each recorded read, re-reads the location from `mv_state` at `tx_index`.
    /// Validation passes (returns `true`) only if every re-read matches the
    /// originally recorded origin. Fails if:
    /// - A read that was from base storage now has a value in MVHashMap
    /// - A read that was from MVHashMap now has a different (tx_index, incarnation)
    /// - Any location now returns an ESTIMATE marker
    /// - A read that was NotFound now has a value
    pub fn validate(&self, mv_state: &MVHashMap, tx_index: TxIndex) -> bool {
        for (location, recorded_origin) in &self.reads {
            let current = mv_state.read(location, tx_index);
            let still_valid = match (&current, recorded_origin) {
                // Both from storage (MVHashMap had nothing) — still valid.
                (MvReadResult::NotFound, ReadOrigin::Storage) => true,
                // Both from storage, neither existed — still valid.
                (MvReadResult::NotFound, ReadOrigin::NotFound) => true,
                // Both from MVHashMap — valid only if same version.
                (
                    MvReadResult::Value(_, current_tx, current_inc),
                    ReadOrigin::MvHashMap {
                        tx_index: orig_tx,
                        incarnation: orig_inc,
                    },
                ) => current_tx == orig_tx && current_inc == orig_inc,
                // ESTIMATE marker — always invalid (dependency is being re-executed).
                (MvReadResult::Estimate(_), _) => false,
                // Mismatch: was from storage, now from MVHashMap (new write appeared).
                (MvReadResult::Value(_, _, _), ReadOrigin::Storage) => false,
                (MvReadResult::Value(_, _, _), ReadOrigin::NotFound) => false,
                // Was from MVHashMap, now not found (write was cleared).
                (MvReadResult::NotFound, ReadOrigin::MvHashMap { .. }) => false,
            };

            if !still_valid {
                return false;
            }
        }
        true
    }

    /// Number of recorded reads.
    pub fn len(&self) -> usize {
        self.reads.len()
    }

    /// Returns `true` if no reads were recorded.
    pub fn is_empty(&self) -> bool {
        self.reads.is_empty()
    }

    /// Iterate over all recorded reads (for diagnostics).
    pub fn iter(&self) -> impl Iterator<Item = (&LocationKey, &ReadOrigin)> {
        self.reads.iter()
    }
}

impl Default for ReadSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Records all state writes performed by a transaction during execution.
///
/// After execution completes, `apply_to()` publishes all writes to the MVHashMap
/// so that subsequent transactions can read the updated values.
pub struct WriteSet {
    writes: BTreeMap<LocationKey, WriteValue>,
}

impl WriteSet {
    /// Create a new, empty WriteSet.
    pub fn new() -> Self {
        Self {
            writes: BTreeMap::new(),
        }
    }

    /// Record a write at the given location with its value.
    pub fn record(&mut self, location: LocationKey, value: WriteValue) {
        self.writes.insert(location, value);
    }

    /// Publish all recorded writes to the MVHashMap.
    ///
    /// Each write is stored under `(tx_index, incarnation)` so that
    /// subsequent transactions can read the correct version.
    pub fn apply_to(&self, mv_state: &MVHashMap, tx_index: TxIndex, incarnation: Incarnation) {
        for (location, value) in &self.writes {
            mv_state.write(location.clone(), tx_index, incarnation, value.clone());
        }
    }

    /// Number of recorded writes.
    pub fn len(&self) -> usize {
        self.writes.len()
    }

    /// Returns `true` if no writes were recorded.
    pub fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }

    /// Iterate over all recorded writes (for diagnostics).
    pub fn iter(&self) -> impl Iterator<Item = (&LocationKey, &WriteValue)> {
        self.writes.iter()
    }
}

impl Default for WriteSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, U256};

    fn test_addr() -> alloy_primitives::Address {
        address!("0x0000000000000000000000000000000000000001")
    }

    // ---- ReadSet tests ----

    #[test]
    fn read_set_records_and_iterates() {
        let mut rs = ReadSet::new();
        assert!(rs.is_empty());

        rs.record(
            LocationKey::Balance(test_addr()),
            ReadOrigin::Storage,
        );
        rs.record(
            LocationKey::Nonce(test_addr()),
            ReadOrigin::MvHashMap {
                tx_index: 0,
                incarnation: 0,
            },
        );

        assert_eq!(rs.len(), 2);
        assert!(!rs.is_empty());
    }

    #[test]
    fn read_set_validate_passes_when_unchanged() {
        let mv = MVHashMap::new();
        let addr = test_addr();

        // tx=0 wrote balance, tx=1 reads it and records it.
        mv.write(
            LocationKey::Balance(addr),
            0,
            0,
            WriteValue::Balance(U256::from(100)),
        );

        let mut rs = ReadSet::new();
        rs.record(
            LocationKey::Balance(addr),
            ReadOrigin::MvHashMap {
                tx_index: 0,
                incarnation: 0,
            },
        );

        // Validate at tx_index=1 — should still see (0, 0).
        assert!(rs.validate(&mv, 1));
    }

    #[test]
    fn read_set_validate_fails_when_new_write_appears() {
        let mv = MVHashMap::new();
        let addr = test_addr();

        // Initially nothing in MVHashMap for this location.
        let mut rs = ReadSet::new();
        rs.record(LocationKey::Balance(addr), ReadOrigin::Storage);

        // Now another tx writes to the same location.
        mv.write(
            LocationKey::Balance(addr),
            0,
            0,
            WriteValue::Balance(U256::from(999)),
        );

        // Validate at tx_index=1 — should fail (was Storage, now MVHashMap has a value).
        assert!(!rs.validate(&mv, 1));
    }

    #[test]
    fn read_set_validate_fails_on_estimate() {
        let mv = MVHashMap::new();
        let addr = test_addr();

        mv.write(
            LocationKey::Balance(addr),
            0,
            0,
            WriteValue::Balance(U256::from(100)),
        );

        let mut rs = ReadSet::new();
        rs.record(
            LocationKey::Balance(addr),
            ReadOrigin::MvHashMap {
                tx_index: 0,
                incarnation: 0,
            },
        );

        // Mark tx=0 as ESTIMATE.
        mv.mark_estimate(0);

        // Validate at tx_index=1 — should fail (ESTIMATE).
        assert!(!rs.validate(&mv, 1));
    }

    #[test]
    fn read_set_validate_fails_on_version_change() {
        let mv = MVHashMap::new();
        let addr = test_addr();

        mv.write(
            LocationKey::Balance(addr),
            0,
            0,
            WriteValue::Balance(U256::from(100)),
        );

        let mut rs = ReadSet::new();
        rs.record(
            LocationKey::Balance(addr),
            ReadOrigin::MvHashMap {
                tx_index: 0,
                incarnation: 0,
            },
        );

        // tx=0 re-executes with incarnation=1.
        mv.write(
            LocationKey::Balance(addr),
            0,
            1,
            WriteValue::Balance(U256::from(200)),
        );

        // Validate at tx_index=1 — should fail (incarnation changed from 0 to 1).
        assert!(!rs.validate(&mv, 1));
    }

    #[test]
    fn read_set_validate_passes_for_storage_reads() {
        let mv = MVHashMap::new();

        let mut rs = ReadSet::new();
        rs.record(
            LocationKey::Storage(test_addr(), U256::from(5)),
            ReadOrigin::Storage,
        );

        // MVHashMap is empty, so re-reading returns NotFound — matches Storage origin.
        assert!(rs.validate(&mv, 1));
    }

    #[test]
    fn read_set_validate_passes_for_not_found_reads() {
        let mv = MVHashMap::new();

        let mut rs = ReadSet::new();
        rs.record(LocationKey::Balance(test_addr()), ReadOrigin::NotFound);

        // MVHashMap is empty — re-read returns NotFound, matches NotFound origin.
        assert!(rs.validate(&mv, 1));
    }

    #[test]
    fn read_set_validate_fails_when_mvhashmap_read_cleared() {
        let mv = MVHashMap::new();
        let addr = test_addr();

        mv.write(
            LocationKey::Balance(addr),
            0,
            0,
            WriteValue::Balance(U256::from(100)),
        );

        let mut rs = ReadSet::new();
        rs.record(
            LocationKey::Balance(addr),
            ReadOrigin::MvHashMap {
                tx_index: 0,
                incarnation: 0,
            },
        );

        // Clear tx=0's writes.
        mv.clear(0);

        // Validate at tx_index=1 — should fail (was MvHashMap, now NotFound).
        assert!(!rs.validate(&mv, 1));
    }

    // ---- WriteSet tests ----

    #[test]
    fn write_set_records_and_iterates() {
        let mut ws = WriteSet::new();
        assert!(ws.is_empty());

        ws.record(
            LocationKey::Balance(test_addr()),
            WriteValue::Balance(U256::from(500)),
        );
        ws.record(
            LocationKey::Nonce(test_addr()),
            WriteValue::Nonce(3),
        );

        assert_eq!(ws.len(), 2);
        assert!(!ws.is_empty());
    }

    #[test]
    fn write_set_apply_publishes_to_mvhashmap() {
        let mv = MVHashMap::new();
        let addr = test_addr();

        let mut ws = WriteSet::new();
        ws.record(
            LocationKey::Balance(addr),
            WriteValue::Balance(U256::from(1000)),
        );
        ws.record(
            LocationKey::Storage(addr, U256::from(0)),
            WriteValue::Storage(U256::from(42)),
        );

        // Apply as tx=0, incarnation=0.
        ws.apply_to(&mv, 0, 0);

        // Verify in MVHashMap from tx=1's perspective.
        match mv.read(&LocationKey::Balance(addr), 1) {
            MvReadResult::Value(WriteValue::Balance(v), 0, 0) => {
                assert_eq!(v, U256::from(1000));
            }
            other => panic!("expected Balance(1000) from tx=0 inc=0, got {:?}", other),
        }

        match mv.read(&LocationKey::Storage(addr, U256::from(0)), 1) {
            MvReadResult::Value(WriteValue::Storage(v), 0, 0) => {
                assert_eq!(v, U256::from(42));
            }
            other => panic!("expected Storage(42) from tx=0 inc=0, got {:?}", other),
        }
    }

    #[test]
    fn write_set_apply_with_incarnation() {
        let mv = MVHashMap::new();
        let addr = test_addr();

        let mut ws = WriteSet::new();
        ws.record(
            LocationKey::Balance(addr),
            WriteValue::Balance(U256::from(2000)),
        );

        // Apply as tx=0, incarnation=2 (re-execution).
        ws.apply_to(&mv, 0, 2);

        match mv.read(&LocationKey::Balance(addr), 1) {
            MvReadResult::Value(WriteValue::Balance(v), 0, 2) => {
                assert_eq!(v, U256::from(2000));
            }
            other => panic!("expected Balance(2000) at inc=2, got {:?}", other),
        }
    }

    #[test]
    fn write_set_overwrites_same_location() {
        let mut ws = WriteSet::new();
        ws.record(
            LocationKey::Balance(test_addr()),
            WriteValue::Balance(U256::from(100)),
        );
        ws.record(
            LocationKey::Balance(test_addr()),
            WriteValue::Balance(U256::from(200)),
        );

        // BTreeMap keeps last insert.
        assert_eq!(ws.len(), 1);
        let (_, v) = ws.iter().next().unwrap();
        assert_eq!(*v, WriteValue::Balance(U256::from(200)));
    }
}
