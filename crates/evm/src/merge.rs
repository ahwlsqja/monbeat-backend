//! Sequential merge of parallel execution results and deterministic state root.
//!
//! After Block-STM parallel execution completes, the merge phase applies all
//! validated WriteSets in block order to produce the final state, generates
//! per-transaction receipts, applies beneficiary fees, and computes a
//! deterministic state root via keccak256 over BTreeMap-sorted entries.
//!
//! # Architecture Note
//!
//! This module accepts decomposed inputs (tx_results + beneficiary fees) rather
//! than `ParallelExecutionResult` directly. This avoids a cyclic dependency:
//! `monad-scheduler` depends on `monad-evm` (for `EvmExecutor`), so `monad-evm`
//! cannot depend on `monad-scheduler`. The `execute_block()` API (T02) will
//! bridge between `ParallelExecutionResult` and this merge interface.
//!
//! # Correctness
//!
//! The merge logic mirrors the pattern proven correct in the differential test
//! harness (`crates/evm/tests/parallel_execution.rs::run_parallel()`). The
//! critical invariant is: WriteSets are applied in strict block order (tx 0, 1, 2, …)
//! and the state root uses BTreeMap-sorted traversal (never HashMap iteration).

use alloy_primitives::{keccak256, B256};

use monad_mv_state::read_write_sets::WriteSet;
use monad_mv_state::types::{LocationKey, WriteValue};
use monad_state::InMemoryState;
use monad_types::{AccountInfo, BlockEnv, ExecutionResult, Log, Receipt, U256};

/// Intermediate result of merging parallel execution outputs.
///
/// Exposes the merged state (pre-state-root) for debugging and inspection.
/// The `BlockResult` (in monad-types) is constructed from this by computing
/// the state root and combining fields.
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// The final post-execution state after applying all WriteSets in block order
    /// and crediting beneficiary fees.
    pub state: InMemoryState,
    /// Per-transaction receipts in block order.
    pub receipts: Vec<Receipt>,
    /// Total gas consumed by all transactions.
    pub gas_used: u64,
    /// All logs emitted by all transactions, in block order.
    pub logs: Vec<Log>,
}

/// Merge parallel execution results into final state with receipts.
///
/// Iterates `tx_results` in block order. For each `(ExecutionResult, WriteSet)`:
/// - Applies WriteSet entries to a mutable `InMemoryState` clone
/// - Generates a `Receipt` with `success`, `cumulative_gas_used`, `logs`, `contract_address`
///
/// After all transactions, applies `beneficiary_total_fees` to the coinbase balance.
///
/// # Arguments
///
/// - `base_state` — pre-block state
/// - `tx_results` — per-tx `(ExecutionResult, WriteSet)` in block order (from `ParallelExecutionResult`)
/// - `beneficiary_total_fees` — total accumulated gas fees from `LazyBeneficiaryTracker::total_fees()`
/// - `block_env` — block environment (needed for coinbase address)
pub fn merge_parallel_results(
    base_state: &InMemoryState,
    tx_results: &[(ExecutionResult, WriteSet)],
    beneficiary_total_fees: U256,
    block_env: &BlockEnv,
) -> MergeResult {
    let mut final_state = base_state.clone();
    let mut receipts = Vec::with_capacity(tx_results.len());
    let mut cumulative_gas: u64 = 0;
    let mut all_logs: Vec<Log> = Vec::new();

    for (exec_result, write_set) in tx_results {
        // Apply WriteSet entries to evolving state (same pattern as run_parallel).
        for (location, value) in write_set.iter() {
            match (location, value) {
                (LocationKey::Balance(addr), WriteValue::Balance(bal)) => {
                    let mut acct = final_state
                        .get_account(addr)
                        .cloned()
                        .unwrap_or_else(|| AccountInfo::new(U256::ZERO, 0));
                    acct.balance = *bal;
                    final_state.insert_account(*addr, acct);
                }
                (LocationKey::Nonce(addr), WriteValue::Nonce(n)) => {
                    let mut acct = final_state
                        .get_account(addr)
                        .cloned()
                        .unwrap_or_else(|| AccountInfo::new(U256::ZERO, 0));
                    acct.nonce = *n;
                    final_state.insert_account(*addr, acct);
                }
                (LocationKey::CodeHash(addr), WriteValue::CodeHash(hash)) => {
                    let mut acct = final_state
                        .get_account(addr)
                        .cloned()
                        .unwrap_or_else(|| AccountInfo::new(U256::ZERO, 0));
                    acct.code_hash = *hash;
                    final_state.insert_account(*addr, acct);
                }
                (LocationKey::Storage(addr, slot), WriteValue::Storage(val)) => {
                    final_state.insert_storage(*addr, *slot, *val);
                }
                _ => {
                    panic!(
                        "mismatched LocationKey/WriteValue: {:?} / {:?}",
                        location, value
                    );
                }
            }
        }

        // Accumulate gas.
        let tx_gas = exec_result.gas_used();
        cumulative_gas += tx_gas;

        // Extract logs from execution result (only Success variant has logs).
        let tx_logs = match exec_result {
            ExecutionResult::Success { logs, .. } => logs.clone(),
            _ => vec![],
        };

        // Build receipt.
        let receipt = Receipt {
            success: exec_result.is_success(),
            cumulative_gas_used: cumulative_gas,
            logs: tx_logs.clone(),
            contract_address: None,
        };
        receipts.push(receipt);

        // Collect all logs for the block.
        all_logs.extend(tx_logs);
    }

    // Apply beneficiary fees to coinbase.
    if beneficiary_total_fees > U256::ZERO {
        let coinbase_acct = final_state
            .get_account(&block_env.coinbase)
            .cloned()
            .unwrap_or_else(|| AccountInfo::new(U256::ZERO, 0));
        final_state.insert_account(
            block_env.coinbase,
            AccountInfo::new(coinbase_acct.balance + beneficiary_total_fees, coinbase_acct.nonce),
        );
    }

    MergeResult {
        state: final_state,
        receipts,
        gas_used: cumulative_gas,
        logs: all_logs,
    }
}

/// Compute a deterministic state root hash from an `InMemoryState`.
///
/// Collects all accounts and storage into BTreeMap-sorted structures, encodes
/// each entry into a canonical byte representation, concatenates all encoded
/// bytes, and applies keccak256.
///
/// # Encoding
///
/// For each account (in address order):
/// - `address` (20 bytes)
/// - `balance` (32 bytes, big-endian via `to_be_bytes::<32>()`)
/// - `nonce` (8 bytes, big-endian)
/// - `code_hash` (32 bytes)
///
/// For each storage entry (in (address, slot) order):
/// - `address` (20 bytes)
/// - `slot` (32 bytes, big-endian)
/// - `value` (32 bytes, big-endian)
///
/// Returns `B256::ZERO` for empty state (no accounts and no storage).
pub fn compute_state_root(state: &InMemoryState) -> B256 {
    let accounts = state.accounts();
    let storage = state.all_storage();

    if accounts.is_empty() && storage.is_empty() {
        return B256::ZERO;
    }

    let mut data = Vec::new();

    // Encode accounts in sorted address order.
    for (address, info) in &accounts {
        data.extend_from_slice(address.as_slice()); // 20 bytes
        data.extend_from_slice(&info.balance.to_be_bytes::<32>()); // 32 bytes
        data.extend_from_slice(&info.nonce.to_be_bytes()); // 8 bytes
        data.extend_from_slice(info.code_hash.as_slice()); // 32 bytes
    }

    // Encode storage entries in sorted (address, slot) order.
    for ((address, slot), value) in &storage {
        data.extend_from_slice(address.as_slice()); // 20 bytes
        data.extend_from_slice(&slot.to_be_bytes::<32>()); // 32 bytes
        data.extend_from_slice(&value.to_be_bytes::<32>()); // 32 bytes
    }

    keccak256(&data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;

    fn test_addr_a() -> alloy_primitives::Address {
        address!("0x00000000000000000000000000000000000000E1")
    }

    fn test_addr_b() -> alloy_primitives::Address {
        address!("0x00000000000000000000000000000000000000E2")
    }

    fn coinbase_addr() -> alloy_primitives::Address {
        address!("0x00000000000000000000000000000000000000C0")
    }

    fn make_block_env() -> BlockEnv {
        BlockEnv {
            number: 1,
            coinbase: coinbase_addr(),
            timestamp: 1_700_000_000,
            gas_limit: 30_000_000,
            base_fee: U256::ZERO,
            difficulty: U256::ZERO,
        }
    }

    // ── compute_state_root tests ────────────────────────────────────

    #[test]
    fn test_state_root_empty_state() {
        let state = InMemoryState::new();
        let root = compute_state_root(&state);
        assert_eq!(root, B256::ZERO, "empty state should produce B256::ZERO");
    }

    #[test]
    fn test_state_root_deterministic() {
        // Two states with the same data inserted in different order should
        // produce the same state root.
        let state_a = InMemoryState::new()
            .with_account(test_addr_a(), AccountInfo::new(U256::from(100u64), 1))
            .with_account(test_addr_b(), AccountInfo::new(U256::from(200u64), 2));

        let state_b = InMemoryState::new()
            .with_account(test_addr_b(), AccountInfo::new(U256::from(200u64), 2))
            .with_account(test_addr_a(), AccountInfo::new(U256::from(100u64), 1));

        let root_a = compute_state_root(&state_a);
        let root_b = compute_state_root(&state_b);

        assert_eq!(
            root_a, root_b,
            "identical states should produce identical roots"
        );
        assert_ne!(root_a, B256::ZERO, "non-empty state root should be non-zero");
    }

    #[test]
    fn test_state_root_different_states() {
        let state_a = InMemoryState::new()
            .with_account(test_addr_a(), AccountInfo::new(U256::from(100u64), 1));

        let state_b = InMemoryState::new()
            .with_account(test_addr_a(), AccountInfo::new(U256::from(200u64), 1));

        let root_a = compute_state_root(&state_a);
        let root_b = compute_state_root(&state_b);

        assert_ne!(
            root_a, root_b,
            "different states should produce different roots"
        );
    }

    #[test]
    fn test_state_root_with_storage() {
        let state_a = InMemoryState::new()
            .with_account(test_addr_a(), AccountInfo::new(U256::from(100u64), 0))
            .with_storage(test_addr_a(), U256::from(0u64), U256::from(42u64));

        let state_b = InMemoryState::new()
            .with_account(test_addr_a(), AccountInfo::new(U256::from(100u64), 0))
            .with_storage(test_addr_a(), U256::from(0u64), U256::from(99u64));

        let root_a = compute_state_root(&state_a);
        let root_b = compute_state_root(&state_b);

        assert_ne!(root_a, root_b, "different storage values → different roots");
    }

    #[test]
    fn test_state_root_stable_across_calls() {
        let state = InMemoryState::new()
            .with_account(test_addr_a(), AccountInfo::new(U256::from(100u64), 1))
            .with_storage(test_addr_a(), U256::from(0u64), U256::from(42u64));

        let root1 = compute_state_root(&state);
        let root2 = compute_state_root(&state);
        let root3 = compute_state_root(&state);

        assert_eq!(root1, root2);
        assert_eq!(root2, root3);
    }

    // ── merge_parallel_results tests ────────────────────────────────

    #[test]
    fn test_merge_empty_block() {
        let base_state = InMemoryState::new()
            .with_account(coinbase_addr(), AccountInfo::new(U256::ZERO, 0));
        let block_env = make_block_env();

        let merge = merge_parallel_results(&base_state, &[], U256::ZERO, &block_env);

        assert!(merge.receipts.is_empty());
        assert_eq!(merge.gas_used, 0);
        assert!(merge.logs.is_empty());
    }

    #[test]
    fn test_merge_single_success_tx() {
        let base_state = InMemoryState::new()
            .with_account(test_addr_a(), AccountInfo::new(U256::from(1000u64), 0))
            .with_account(coinbase_addr(), AccountInfo::new(U256::ZERO, 0));
        let block_env = make_block_env();

        // Simulate a successful tx that writes to test_addr_a balance and nonce.
        let exec_result = ExecutionResult::Success {
            gas_used: 21_000,
            output: alloy_primitives::Bytes::new(),
            logs: vec![],
        };

        let mut write_set = WriteSet::new();
        write_set.record(
            LocationKey::Balance(test_addr_a()),
            WriteValue::Balance(U256::from(500u64)),
        );
        write_set.record(
            LocationKey::Nonce(test_addr_a()),
            WriteValue::Nonce(1),
        );

        let beneficiary_fees = U256::from(21_000_000u64);
        let tx_results = vec![(exec_result, write_set)];

        let merge = merge_parallel_results(&base_state, &tx_results, beneficiary_fees, &block_env);

        assert_eq!(merge.receipts.len(), 1);
        assert!(merge.receipts[0].success);
        assert_eq!(merge.receipts[0].cumulative_gas_used, 21_000);
        assert_eq!(merge.gas_used, 21_000);

        // Check state updates.
        let acct = merge.state.get_account(&test_addr_a()).unwrap();
        assert_eq!(acct.balance, U256::from(500u64));
        assert_eq!(acct.nonce, 1);

        // Coinbase should have received fees.
        let coinbase = merge.state.get_account(&coinbase_addr()).unwrap();
        assert_eq!(coinbase.balance, U256::from(21_000_000u64));
    }

    #[test]
    fn test_merge_cumulative_gas() {
        let base_state = InMemoryState::new()
            .with_account(coinbase_addr(), AccountInfo::new(U256::ZERO, 0));
        let block_env = make_block_env();

        let tx1 = ExecutionResult::Success {
            gas_used: 21_000,
            output: alloy_primitives::Bytes::new(),
            logs: vec![],
        };
        let tx2 = ExecutionResult::Success {
            gas_used: 30_000,
            output: alloy_primitives::Bytes::new(),
            logs: vec![],
        };

        let tx_results = vec![
            (tx1, WriteSet::new()),
            (tx2, WriteSet::new()),
        ];

        let merge = merge_parallel_results(&base_state, &tx_results, U256::ZERO, &block_env);

        assert_eq!(merge.receipts.len(), 2);
        assert_eq!(merge.receipts[0].cumulative_gas_used, 21_000);
        assert_eq!(merge.receipts[1].cumulative_gas_used, 51_000);
        assert_eq!(merge.gas_used, 51_000);
    }

    #[test]
    fn test_merge_reverted_tx() {
        let base_state = InMemoryState::new()
            .with_account(coinbase_addr(), AccountInfo::new(U256::ZERO, 0));
        let block_env = make_block_env();

        let exec_result = ExecutionResult::Revert {
            gas_used: 15_000,
            output: alloy_primitives::Bytes::new(),
        };

        let tx_results = vec![(exec_result, WriteSet::new())];

        let merge = merge_parallel_results(&base_state, &tx_results, U256::ZERO, &block_env);

        assert_eq!(merge.receipts.len(), 1);
        assert!(!merge.receipts[0].success);
        assert_eq!(merge.receipts[0].cumulative_gas_used, 15_000);
        assert!(merge.receipts[0].logs.is_empty());
    }
}
