//! Block-level execution API composing parallel results and sequential execution.
//!
//! Provides two public functions:
//!
//! - [`execute_block()`] — takes pre-computed parallel execution results (per-tx
//!   `(ExecutionResult, WriteSet)` pairs + beneficiary fees) and produces a
//!   `BlockResult` via sequential merge + deterministic state root computation.
//!
//! - [`execute_block_sequential()`] — runs transactions sequentially through
//!   `EvmExecutor`, applying state changes after each tx. Produces the same
//!   `BlockResult` structure, enabling differential comparison with the parallel
//!   path (proving PARA-08: parallel == sequential state root).
//!
//! # Architecture
//!
//! `execute_block()` accepts decomposed inputs rather than calling
//! `execute_block_parallel()` directly. This follows decision D015 — the cyclic
//! dependency `monad-scheduler → monad-evm → monad-scheduler` would otherwise
//! prevent compilation. The integration tests (T03: `block_execution.rs`) compose
//! `execute_block_parallel()` → `execute_block()` in the test scope where
//! `monad-scheduler` is available as a dev-dependency.
//!
//! # Observability
//!
//! - `BlockResult.state_root` — deterministic keccak256 hash; compare parallel
//!   vs sequential to verify PARA-08
//! - `BlockResult.receipts` — per-tx success/failure, cumulative gas, logs
//! - `BlockResult.gas_used` — total block gas consumption
//! - Both functions produce `Debug`-printable output; `cargo test -- --nocapture`
//!   surfaces detailed mismatch info on assertion failures

use alloy_primitives::B256;

use monad_mv_state::read_write_sets::WriteSet;
use monad_state::InMemoryState;
use monad_types::{
    AccountInfo, BlockEnv, BlockResult, EvmError, ExecutionResult, Log, Receipt, Transaction, U256,
};

use crate::executor::EvmExecutor;
use crate::merge::{compute_state_root, merge_parallel_results};

/// Execute a block by merging pre-computed parallel execution results.
///
/// This is the top-level block execution API promised by S05. It composes:
/// 1. `merge_parallel_results()` — applies WriteSets in block order, generates receipts
/// 2. `compute_state_root()` — deterministic keccak256 over BTreeMap-sorted state
///
/// Returns a `BlockResult` with state root, receipts, total gas, and all logs.
///
/// # Arguments
///
/// - `base_state` — pre-block state
/// - `tx_results` — per-tx `(ExecutionResult, WriteSet)` in block order
///   (from `ParallelExecutionResult::tx_results`)
/// - `beneficiary_total_fees` — total gas fees from `LazyBeneficiaryTracker::total_fees()`
/// - `block_env` — block environment (coinbase, number, etc.)
///
/// # Empty Block Fast Path
///
/// If `tx_results` is empty, returns immediately with `B256::ZERO` state root
/// and empty receipts/logs.
pub fn execute_block(
    base_state: &InMemoryState,
    tx_results: &[(ExecutionResult, WriteSet)],
    beneficiary_total_fees: U256,
    block_env: &BlockEnv,
) -> Result<BlockResult, EvmError> {
    // Empty block fast path.
    if tx_results.is_empty() {
        return Ok(BlockResult {
            state_root: B256::ZERO,
            receipts: vec![],
            gas_used: 0,
            logs: vec![],
        });
    }

    // Merge parallel results into final state with receipts.
    let merge_result =
        merge_parallel_results(base_state, tx_results, beneficiary_total_fees, block_env);

    // Compute deterministic state root.
    let state_root = compute_state_root(&merge_result.state);

    Ok(BlockResult {
        state_root,
        receipts: merge_result.receipts,
        gas_used: merge_result.gas_used,
        logs: merge_result.logs,
    })
}

/// Execute a block sequentially for differential comparison.
///
/// Iterates transactions in order, executing each through `EvmExecutor` and
/// applying state changes to a mutable clone. Gas fees are accumulated and
/// credited to coinbase at the end, matching the parallel path's
/// `LazyBeneficiaryTracker` behavior.
///
/// The sequential path skips coinbase state changes from revm (since
/// `disable_fee_charge = true`), and instead manually credits the coinbase
/// with accumulated `gas_used * gas_price` fees. This matches the pattern
/// established in `parallel_execution.rs::run_sequential()`.
///
/// # Arguments
///
/// - `transactions` — ordered list of transactions in the block
/// - `base_state` — pre-block state
/// - `block_env` — block environment
///
/// # Returns
///
/// A `BlockResult` with the same structure as `execute_block()`, enabling
/// state root comparison: `parallel_result.state_root == sequential_result.state_root`.
pub fn execute_block_sequential(
    transactions: &[Transaction],
    base_state: &InMemoryState,
    block_env: &BlockEnv,
) -> Result<BlockResult, EvmError> {
    // Empty block fast path.
    if transactions.is_empty() {
        return Ok(BlockResult {
            state_root: B256::ZERO,
            receipts: vec![],
            gas_used: 0,
            logs: vec![],
        });
    }

    let mut state = base_state.clone();
    let mut total_gas_fee = U256::ZERO;
    let mut cumulative_gas: u64 = 0;
    let mut receipts = Vec::with_capacity(transactions.len());
    let mut all_logs: Vec<Log> = Vec::new();

    for tx in transactions {
        let (result, state_changes) =
            EvmExecutor::execute_tx_with_state_changes(tx, &state, block_env)?;

        // Apply state diffs to evolving state.
        // Skip coinbase — gas fees are handled via manual accumulation below,
        // matching the parallel path's LazyBeneficiaryTracker pattern.
        for (addr, (acct_info, storage)) in &state_changes {
            if *addr == block_env.coinbase {
                continue;
            }
            state.insert_account(*addr, acct_info.clone());
            for (slot, value) in storage {
                state.insert_storage(*addr, *slot, *value);
            }
        }

        // Accumulate gas fee for coinbase.
        let gas_fee = U256::from(result.gas_used()) * tx.gas_price;
        total_gas_fee += gas_fee;

        // Build receipt with cumulative gas.
        let tx_gas = result.gas_used();
        cumulative_gas += tx_gas;

        let tx_logs = match &result {
            ExecutionResult::Success { logs, .. } => logs.clone(),
            _ => vec![],
        };

        let receipt = Receipt {
            success: result.is_success(),
            cumulative_gas_used: cumulative_gas,
            logs: tx_logs.clone(),
            contract_address: None,
        };
        receipts.push(receipt);

        all_logs.extend(tx_logs);
    }

    // Apply total gas fees to coinbase.
    let coinbase_acct = state
        .get_account(&block_env.coinbase)
        .cloned()
        .unwrap_or_else(|| AccountInfo::new(U256::ZERO, 0));
    state.insert_account(
        block_env.coinbase,
        AccountInfo::new(
            coinbase_acct.balance + total_gas_fee,
            coinbase_acct.nonce,
        ),
    );

    // Compute deterministic state root.
    let state_root = compute_state_root(&state);

    Ok(BlockResult {
        state_root,
        receipts,
        gas_used: cumulative_gas,
        logs: all_logs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, Address, Bytes};
    use monad_mv_state::read_write_sets::WriteSet;
    use monad_mv_state::types::{LocationKey, WriteValue};

    fn sender_a() -> Address {
        address!("0x00000000000000000000000000000000000000E1")
    }

    fn receiver_a() -> Address {
        address!("0x00000000000000000000000000000000000000F1")
    }

    fn coinbase_addr() -> Address {
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

    fn make_transfer(
        from: Address,
        to: Address,
        value: u64,
        nonce: u64,
    ) -> Transaction {
        Transaction {
            sender: from,
            to: Some(to),
            value: U256::from(value),
            data: Bytes::new(),
            gas_limit: 100_000,
            nonce,
            gas_price: U256::from(1_000_000_000u64), // 1 gwei
        }
    }

    // ── execute_block tests ─────────────────────────────────────────

    #[test]
    fn test_execute_block_empty() {
        let base_state = InMemoryState::new();
        let block_env = make_block_env();

        let result = execute_block(&base_state, &[], U256::ZERO, &block_env)
            .expect("empty block should succeed");

        assert_eq!(result.state_root, B256::ZERO, "empty block → zero state root");
        assert!(result.receipts.is_empty(), "empty block → no receipts");
        assert_eq!(result.gas_used, 0, "empty block → zero gas");
        assert!(result.logs.is_empty(), "empty block → no logs");
    }

    #[test]
    fn test_execute_block_single_tx() {
        let base_state = InMemoryState::new()
            .with_account(sender_a(), AccountInfo::new(U256::from(1_000_000_000_000u64), 0))
            .with_account(coinbase_addr(), AccountInfo::new(U256::ZERO, 0));
        let block_env = make_block_env();

        // Simulate a successful tx that modifies sender balance and nonce.
        let exec_result = ExecutionResult::Success {
            gas_used: 21_000,
            output: Bytes::new(),
            logs: vec![],
        };

        let mut write_set = WriteSet::new();
        write_set.record(
            LocationKey::Balance(sender_a()),
            WriteValue::Balance(U256::from(999_999_979_000u64)),
        );
        write_set.record(
            LocationKey::Nonce(sender_a()),
            WriteValue::Nonce(1),
        );
        write_set.record(
            LocationKey::Balance(receiver_a()),
            WriteValue::Balance(U256::from(1000u64)),
        );

        let beneficiary_fees = U256::from(21_000u64) * U256::from(1_000_000_000u64); // 21000 gas * 1 gwei
        let tx_results = vec![(exec_result, write_set)];

        let result = execute_block(&base_state, &tx_results, beneficiary_fees, &block_env)
            .expect("single tx block should succeed");

        assert_ne!(result.state_root, B256::ZERO, "non-empty block → non-zero state root");
        assert_eq!(result.receipts.len(), 1, "single tx → 1 receipt");
        assert!(result.receipts[0].success, "tx should succeed");
        assert_eq!(result.receipts[0].cumulative_gas_used, 21_000);
        assert_eq!(result.gas_used, 21_000);
    }

    // ── execute_block_sequential tests ──────────────────────────────

    #[test]
    fn test_execute_block_sequential_empty() {
        let base_state = InMemoryState::new();
        let block_env = make_block_env();

        let result = execute_block_sequential(&[], &base_state, &block_env)
            .expect("empty block should succeed");

        assert_eq!(result.state_root, B256::ZERO, "empty block → zero state root");
        assert!(result.receipts.is_empty());
        assert_eq!(result.gas_used, 0);
        assert!(result.logs.is_empty());
    }

    #[test]
    fn test_execute_block_sequential_single_transfer() {
        let base_state = InMemoryState::new()
            .with_account(sender_a(), AccountInfo::new(U256::from(1_000_000_000_000u64), 0))
            .with_account(coinbase_addr(), AccountInfo::new(U256::ZERO, 0));
        let block_env = make_block_env();

        let transactions = vec![make_transfer(sender_a(), receiver_a(), 1000, 0)];

        let result = execute_block_sequential(&transactions, &base_state, &block_env)
            .expect("single transfer should succeed");

        assert_ne!(result.state_root, B256::ZERO, "non-empty block → non-zero state root");
        assert_eq!(result.receipts.len(), 1, "single tx → 1 receipt");
        assert!(result.receipts[0].success, "transfer should succeed");
        assert!(result.receipts[0].cumulative_gas_used > 0, "should consume gas");
        assert_eq!(result.gas_used, result.receipts[0].cumulative_gas_used);
    }

    /// Verify that execute_block() and execute_block_sequential() produce
    /// identical state roots for the same single-tx input, using the real
    /// EVM execution path to generate parallel-equivalent WriteSet data.
    #[test]
    fn test_parallel_sequential_state_root_match() {
        let base_state = InMemoryState::new()
            .with_account(sender_a(), AccountInfo::new(U256::from(1_000_000_000_000u64), 0))
            .with_account(coinbase_addr(), AccountInfo::new(U256::ZERO, 0));
        let block_env = make_block_env();

        let transactions = vec![make_transfer(sender_a(), receiver_a(), 1000, 0)];

        // Sequential path.
        let seq_result = execute_block_sequential(&transactions, &base_state, &block_env)
            .expect("sequential should succeed");

        // Parallel-equivalent path: execute the tx via EvmExecutor to get real
        // state changes, then convert to WriteSet format.
        let tx = &transactions[0];
        let (exec_result, state_changes) =
            EvmExecutor::execute_tx_with_state_changes(tx, &base_state, &block_env)
                .expect("execution should succeed");

        // Convert state_changes to WriteSet, skipping coinbase (matching D015 pattern).
        let mut write_set = WriteSet::new();
        for (addr, (acct_info, storage)) in &state_changes {
            if *addr == block_env.coinbase {
                continue;
            }
            write_set.record(
                LocationKey::Balance(*addr),
                WriteValue::Balance(acct_info.balance),
            );
            write_set.record(
                LocationKey::Nonce(*addr),
                WriteValue::Nonce(acct_info.nonce),
            );
            if acct_info.code_hash != alloy_primitives::B256::ZERO {
                write_set.record(
                    LocationKey::CodeHash(*addr),
                    WriteValue::CodeHash(acct_info.code_hash),
                );
            }
            for (slot, value) in storage {
                write_set.record(
                    LocationKey::Storage(*addr, *slot),
                    WriteValue::Storage(*value),
                );
            }
        }

        let gas_fee = U256::from(exec_result.gas_used()) * tx.gas_price;
        let tx_results = vec![(exec_result, write_set)];

        let par_result = execute_block(&base_state, &tx_results, gas_fee, &block_env)
            .expect("parallel path should succeed");

        assert_eq!(
            seq_result.state_root, par_result.state_root,
            "parallel and sequential state roots must match.\n  sequential: {:?}\n  parallel:   {:?}",
            seq_result.state_root, par_result.state_root
        );
        assert_eq!(seq_result.receipts.len(), par_result.receipts.len());
        assert_eq!(seq_result.gas_used, par_result.gas_used);
    }
}
