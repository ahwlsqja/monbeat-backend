//! Conflict detection module for CLI JSON output.
//!
//! Analyzes ReadSet/WriteSet data from parallel execution to detect
//! write-write and read-write conflicts between transactions. Produces
//! CLI-specific serializable types without adding serde to mv-state internals.
//!
//! # Observability
//!
//! - `conflict_details.conflicts` length 0 = no conflicts, >0 = conflicts detected
//! - `conflict_details.per_tx[i].reads/writes` length shows state access scope per tx
//! - Empty reads/writes arrays indicate ReadSet/WriteSet was not preserved (data loss visible)
//! - Inspect via: `jq .conflict_details` on CLI stdout

use std::collections::HashSet;

use monad_mv_state::{LocationKey, ReadSet, WriteSet};
use monad_types::ExecutionResult;
use serde::Serialize;

/// Top-level conflict analysis result included in CLI JSON output.
#[derive(Debug, Serialize)]
pub struct ConflictDetails {
    /// Per-transaction access summary (reads and writes as serializable locations).
    pub per_tx: Vec<TxAccessSummary>,
    /// Detected conflicts between transaction pairs.
    pub conflicts: Vec<ConflictPair>,
}

/// Summary of a single transaction's state accesses.
#[derive(Debug, Serialize)]
pub struct TxAccessSummary {
    /// Transaction index within the block (0-based).
    pub tx_index: usize,
    /// Locations read by this transaction.
    pub reads: Vec<LocationInfo>,
    /// Locations written by this transaction.
    pub writes: Vec<LocationInfo>,
}

/// Serializable representation of a state location.
///
/// Converts from `LocationKey` via pattern matching — avoids adding
/// serde derives to the hot-path mv-state crate.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LocationInfo {
    /// Location variant: "Storage", "Balance", "Nonce", or "CodeHash".
    pub location_type: String,
    /// Account address as "0x..." lowercase hex.
    pub address: String,
    /// Storage slot as "0x..." lowercase hex (only for Storage type).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot: Option<String>,
}

/// A detected conflict between two transactions at a specific location.
#[derive(Debug, Serialize)]
pub struct ConflictPair {
    /// The state location where the conflict occurs.
    pub location: LocationInfo,
    /// Index of the first transaction (lower index).
    pub tx_a: usize,
    /// Index of the second transaction (higher index).
    pub tx_b: usize,
    /// Type of conflict: "write-write" or "read-write".
    pub conflict_type: String,
}

/// Convert a `LocationKey` to its serializable `LocationInfo` form.
pub fn location_key_to_info(key: &LocationKey) -> LocationInfo {
    match key {
        LocationKey::Storage(addr, slot) => LocationInfo {
            location_type: "Storage".to_string(),
            address: format!("0x{:x}", addr),
            slot: Some(format!("0x{:x}", slot)),
        },
        LocationKey::Balance(addr) => LocationInfo {
            location_type: "Balance".to_string(),
            address: format!("0x{:x}", addr),
            slot: None,
        },
        LocationKey::Nonce(addr) => LocationInfo {
            location_type: "Nonce".to_string(),
            address: format!("0x{:x}", addr),
            slot: None,
        },
        LocationKey::CodeHash(addr) => LocationInfo {
            location_type: "CodeHash".to_string(),
            address: format!("0x{:x}", addr),
            slot: None,
        },
    }
}

/// Detect conflicts between transactions based on their ReadSet/WriteSet data.
///
/// Produces:
/// - `per_tx`: access summary per transaction (reads and writes as `LocationInfo`)
/// - `conflicts`: all write-write and read-write conflicts between tx pairs
///
/// Both write-write and read-write conflicts are included even for the same
/// location+pair — more information is better for downstream analysis (S02).
pub fn detect_conflicts(
    tx_results: &[(ExecutionResult, WriteSet, ReadSet)],
) -> ConflictDetails {
    // Build per-tx summaries
    let per_tx: Vec<TxAccessSummary> = tx_results
        .iter()
        .enumerate()
        .map(|(i, (_exec, write_set, read_set))| {
            let reads: Vec<LocationInfo> = read_set
                .iter()
                .map(|(key, _origin)| location_key_to_info(key))
                .collect();
            let writes: Vec<LocationInfo> = write_set
                .iter()
                .map(|(key, _value)| location_key_to_info(key))
                .collect();
            TxAccessSummary {
                tx_index: i,
                reads,
                writes,
            }
        })
        .collect();

    // Detect conflicts between all tx pairs (tx_a < tx_b)
    let mut conflicts = Vec::new();

    for tx_a in 0..tx_results.len() {
        for tx_b in (tx_a + 1)..tx_results.len() {
            let (_, write_set_a, read_set_a) = &tx_results[tx_a];
            let (_, write_set_b, read_set_b) = &tx_results[tx_b];

            // Collect keys into HashSets for intersection
            let write_keys_a: HashSet<&LocationKey> =
                write_set_a.iter().map(|(k, _)| k).collect();
            let write_keys_b: HashSet<&LocationKey> =
                write_set_b.iter().map(|(k, _)| k).collect();
            let read_keys_a: HashSet<&LocationKey> =
                read_set_a.iter().map(|(k, _)| k).collect();
            let read_keys_b: HashSet<&LocationKey> =
                read_set_b.iter().map(|(k, _)| k).collect();

            // Write-write conflicts
            for key in write_keys_a.intersection(&write_keys_b) {
                conflicts.push(ConflictPair {
                    location: location_key_to_info(key),
                    tx_a,
                    tx_b,
                    conflict_type: "write-write".to_string(),
                });
            }

            // Read-write conflicts: a reads, b writes
            for key in read_keys_a.intersection(&write_keys_b) {
                conflicts.push(ConflictPair {
                    location: location_key_to_info(key),
                    tx_a,
                    tx_b,
                    conflict_type: "read-write".to_string(),
                });
            }

            // Read-write conflicts: a writes, b reads
            for key in write_keys_a.intersection(&read_keys_b) {
                conflicts.push(ConflictPair {
                    location: location_key_to_info(key),
                    tx_a,
                    tx_b,
                    conflict_type: "read-write".to_string(),
                });
            }
        }
    }

    ConflictDetails { per_tx, conflicts }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, Address, Bytes, U256};
    use monad_mv_state::{LocationKey, ReadOrigin, ReadSet, WriteSet, WriteValue};
    use monad_types::ExecutionResult;

    fn test_addr_1() -> Address {
        address!("0x0000000000000000000000000000000000000001")
    }

    fn test_addr_2() -> Address {
        address!("0x0000000000000000000000000000000000000002")
    }

    fn success_result() -> ExecutionResult {
        ExecutionResult::Success {
            gas_used: 21000,
            output: Bytes::new(),
            logs: vec![],
        }
    }

    #[test]
    fn test_location_key_to_info() {
        // Storage
        let info = location_key_to_info(&LocationKey::Storage(test_addr_1(), U256::from(42)));
        assert_eq!(info.location_type, "Storage");
        assert!(info.address.starts_with("0x"));
        assert!(info.slot.is_some());
        assert!(info.slot.as_ref().unwrap().starts_with("0x"));

        // Balance
        let info = location_key_to_info(&LocationKey::Balance(test_addr_1()));
        assert_eq!(info.location_type, "Balance");
        assert!(info.address.starts_with("0x"));
        assert!(info.slot.is_none());

        // Nonce
        let info = location_key_to_info(&LocationKey::Nonce(test_addr_1()));
        assert_eq!(info.location_type, "Nonce");
        assert!(info.slot.is_none());

        // CodeHash
        let info = location_key_to_info(&LocationKey::CodeHash(test_addr_1()));
        assert_eq!(info.location_type, "CodeHash");
        assert!(info.slot.is_none());
    }

    #[test]
    fn test_detect_write_write_conflict() {
        let addr = test_addr_1();

        // tx0: writes Balance(addr)
        let mut ws0 = WriteSet::new();
        ws0.record(LocationKey::Balance(addr), WriteValue::Balance(U256::from(100)));
        let rs0 = ReadSet::new();

        // tx1: writes Balance(addr) — same location
        let mut ws1 = WriteSet::new();
        ws1.record(LocationKey::Balance(addr), WriteValue::Balance(U256::from(200)));
        let rs1 = ReadSet::new();

        let tx_results = vec![
            (success_result(), ws0, rs0),
            (success_result(), ws1, rs1),
        ];

        let details = detect_conflicts(&tx_results);
        assert_eq!(details.per_tx.len(), 2);

        let ww_conflicts: Vec<_> = details
            .conflicts
            .iter()
            .filter(|c| c.conflict_type == "write-write")
            .collect();
        assert_eq!(ww_conflicts.len(), 1);
        assert_eq!(ww_conflicts[0].tx_a, 0);
        assert_eq!(ww_conflicts[0].tx_b, 1);
        assert_eq!(ww_conflicts[0].location.location_type, "Balance");
    }

    #[test]
    fn test_detect_read_write_conflict() {
        let addr = test_addr_1();

        // tx0: reads Balance(addr)
        let ws0 = WriteSet::new();
        let mut rs0 = ReadSet::new();
        rs0.record(LocationKey::Balance(addr), ReadOrigin::Storage);

        // tx1: writes Balance(addr)
        let mut ws1 = WriteSet::new();
        ws1.record(LocationKey::Balance(addr), WriteValue::Balance(U256::from(200)));
        let rs1 = ReadSet::new();

        let tx_results = vec![
            (success_result(), ws0, rs0),
            (success_result(), ws1, rs1),
        ];

        let details = detect_conflicts(&tx_results);

        let rw_conflicts: Vec<_> = details
            .conflicts
            .iter()
            .filter(|c| c.conflict_type == "read-write")
            .collect();
        assert!(
            !rw_conflicts.is_empty(),
            "expected at least one read-write conflict"
        );
        assert_eq!(rw_conflicts[0].tx_a, 0);
        assert_eq!(rw_conflicts[0].tx_b, 1);
        assert_eq!(rw_conflicts[0].location.location_type, "Balance");
    }

    #[test]
    fn test_no_conflict_independent_txs() {
        let addr1 = test_addr_1();
        let addr2 = test_addr_2();

        // tx0: reads/writes addr1 only
        let mut ws0 = WriteSet::new();
        ws0.record(LocationKey::Balance(addr1), WriteValue::Balance(U256::from(100)));
        let mut rs0 = ReadSet::new();
        rs0.record(LocationKey::Nonce(addr1), ReadOrigin::Storage);

        // tx1: reads/writes addr2 only — completely independent
        let mut ws1 = WriteSet::new();
        ws1.record(LocationKey::Balance(addr2), WriteValue::Balance(U256::from(200)));
        let mut rs1 = ReadSet::new();
        rs1.record(LocationKey::Nonce(addr2), ReadOrigin::Storage);

        let tx_results = vec![
            (success_result(), ws0, rs0),
            (success_result(), ws1, rs1),
        ];

        let details = detect_conflicts(&tx_results);
        assert!(
            details.conflicts.is_empty(),
            "independent txs should produce no conflicts, got: {:?}",
            details.conflicts
        );
    }

    #[test]
    fn test_per_tx_summary() {
        let addr = test_addr_1();

        // tx0: reads Balance, writes Nonce
        let mut ws0 = WriteSet::new();
        ws0.record(LocationKey::Nonce(addr), WriteValue::Nonce(5));
        let mut rs0 = ReadSet::new();
        rs0.record(LocationKey::Balance(addr), ReadOrigin::Storage);

        let tx_results = vec![(success_result(), ws0, rs0)];

        let details = detect_conflicts(&tx_results);
        assert_eq!(details.per_tx.len(), 1);

        let summary = &details.per_tx[0];
        assert_eq!(summary.tx_index, 0);
        assert_eq!(summary.reads.len(), 1);
        assert_eq!(summary.reads[0].location_type, "Balance");
        assert_eq!(summary.writes.len(), 1);
        assert_eq!(summary.writes[0].location_type, "Nonce");
    }

    #[test]
    fn test_empty_tx_results() {
        let details = detect_conflicts(&[]);
        assert!(details.per_tx.is_empty());
        assert!(details.conflicts.is_empty());
    }

    #[test]
    fn test_storage_location_includes_slot() {
        let addr = test_addr_1();
        let slot = U256::from(7);

        let mut ws0 = WriteSet::new();
        ws0.record(
            LocationKey::Storage(addr, slot),
            WriteValue::Storage(U256::from(42)),
        );

        let tx_results = vec![(success_result(), ws0, ReadSet::new())];

        let details = detect_conflicts(&tx_results);
        let write = &details.per_tx[0].writes[0];
        assert_eq!(write.location_type, "Storage");
        assert!(write.slot.is_some());
    }
}
