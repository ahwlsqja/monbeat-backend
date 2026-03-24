//! Top-level parallel block execution using Block-STM.
//!
//! The [`execute_block_parallel()`] function composes the coordinator (Scheduler)
//! and worker functions (execute_transaction, validate_transaction) into a
//! multi-threaded execution pipeline. It spawns worker threads via
//! `crossbeam::thread::scope`, runs the Block-STM main loop where workers
//! compete for tasks, and returns results in block order after all transactions
//! are validated.
//!
//! # Observability
//!
//! - `ParallelExecutionResult::tx_results` — per-tx (ExecutionResult, WriteSet, ReadSet) in block order
//! - `ParallelExecutionResult::beneficiary_tracker` — accumulated gas fees via LazyBeneficiaryTracker
//! - Worker count is capped at `min(num_workers, block_size, MAX_WORKERS)` — observable via function args
//! - On completion, all TxState entries have status `Validated` and `incarnation >= 0`
//!   (incarnation > 0 indicates re-execution after conflict)

use std::sync::{Arc, Mutex};

use monad_mv_state::{
    lazy_updates::LazyBeneficiaryTracker,
    read_write_sets::{ReadSet, WriteSet},
    versioned_state::MVHashMap,
};
use monad_state::StateProvider;
use monad_types::{BlockEnv, ExecutionResult, Transaction};

use crate::{
    coordinator::Scheduler,
    types::SchedulerTask,
    worker::{execute_transaction, validate_transaction, ExecutionOutcome},
};

/// Maximum number of worker threads, capped for CI test stability.
const MAX_WORKERS: usize = 4;

/// Result of parallel block execution.
///
/// Contains per-transaction execution results, write-sets, and read-sets in block order,
/// plus the accumulated gas fees for the block beneficiary.
pub struct ParallelExecutionResult {
    /// One `(ExecutionResult, WriteSet, ReadSet)` per transaction, in block order (tx_index 0, 1, 2, ...).
    /// ReadSets are preserved after successful validation for downstream conflict analysis.
    pub tx_results: Vec<(ExecutionResult, WriteSet, ReadSet)>,
    /// Accumulated gas fees for the block beneficiary, tracked via side-channel
    /// to avoid false conflicts on the coinbase address.
    pub beneficiary_tracker: LazyBeneficiaryTracker,
    /// Final incarnation number per transaction. Incarnation > 0 means re-execution
    /// after a conflict was detected. Useful for CLI output and scoring.
    pub incarnations: Vec<u32>,
}

/// Execute a block of transactions in parallel using the Block-STM algorithm.
///
/// Creates shared state (MVHashMap, LazyBeneficiaryTracker, Scheduler), spawns
/// `num_workers` threads via `crossbeam::thread::scope`, and runs the Block-STM
/// main loop. Each worker competes for tasks from the scheduler, executing or
/// validating transactions until all are validated.
///
/// # Arguments
///
/// - `transactions` — the ordered list of transactions in the block
/// - `base_state` — the pre-block state provider (read-only baseline)
/// - `block_env` — block-level environment (number, coinbase, timestamp, etc.)
/// - `num_workers` — requested number of worker threads (capped at block_size and MAX_WORKERS)
///
/// # Returns
///
/// A `ParallelExecutionResult` with per-transaction results in block order and
/// accumulated beneficiary fees.
///
/// # Panics
///
/// Panics if any worker thread panics (crossbeam propagates panics on scope exit).
pub fn execute_block_parallel(
    transactions: &[Transaction],
    base_state: Arc<dyn StateProvider>,
    block_env: &BlockEnv,
    num_workers: usize,
) -> ParallelExecutionResult {
    let block_size = transactions.len();

    // Empty block: return immediately.
    if block_size == 0 {
        return ParallelExecutionResult {
            tx_results: Vec::new(),
            beneficiary_tracker: LazyBeneficiaryTracker::new(),
            incarnations: Vec::new(),
        };
    }

    // Cap workers at min(requested, block_size, MAX_WORKERS).
    let actual_workers = num_workers.min(block_size).min(MAX_WORKERS).max(1);

    // Shared state for all workers.
    let mv_state = Arc::new(MVHashMap::new());
    let beneficiary_tracker = Arc::new(Mutex::new(LazyBeneficiaryTracker::new()));
    let scheduler = Arc::new(Scheduler::new(block_size));

    // Spawn scoped worker threads. crossbeam::scope allows borrowing stack data
    // (transactions, block_env) without requiring 'static bounds.
    crossbeam::thread::scope(|s| {
        for _worker_id in 0..actual_workers {
            let scheduler = Arc::clone(&scheduler);
            let mv_state = Arc::clone(&mv_state);
            let base_state = Arc::clone(&base_state);
            let beneficiary_tracker = Arc::clone(&beneficiary_tracker);

            s.spawn(move |_| {
                worker_loop(
                    &scheduler,
                    transactions,
                    &mv_state,
                    &base_state,
                    block_env,
                    &beneficiary_tracker,
                );
            });
        }
    })
    .expect("worker thread panicked");

    // All workers have joined — collect results in block order.
    let tx_results = scheduler.collect_results();
    let incarnations: Vec<u32> = (0..block_size)
        .map(|i| scheduler.get_tx_state(i as u32).incarnation)
        .collect();
    let beneficiary_tracker = Arc::try_unwrap(beneficiary_tracker)
        .expect("all worker references dropped after scope exit")
        .into_inner()
        .expect("mutex not poisoned");

    ParallelExecutionResult {
        tx_results,
        beneficiary_tracker,
        incarnations,
    }
}

/// The main loop executed by each worker thread.
///
/// Continuously claims tasks from the scheduler and dispatches to the
/// appropriate handler (execute or validate) until `SchedulerTask::Done`
/// is received.
fn worker_loop(
    scheduler: &Scheduler,
    transactions: &[Transaction],
    mv_state: &Arc<MVHashMap>,
    base_state: &Arc<dyn StateProvider>,
    block_env: &BlockEnv,
    beneficiary_tracker: &Arc<Mutex<LazyBeneficiaryTracker>>,
) {
    loop {
        match scheduler.next_task() {
            SchedulerTask::Execute(tx_idx, incarnation) => {
                handle_execute(
                    scheduler,
                    transactions,
                    tx_idx,
                    incarnation,
                    mv_state,
                    base_state,
                    block_env,
                    beneficiary_tracker,
                );
            }
            SchedulerTask::Validate(tx_idx) => {
                handle_validate(scheduler, tx_idx, mv_state, beneficiary_tracker);
            }
            SchedulerTask::Done => break,
        }
    }
}

/// Handle an Execute task: run the transaction through revm, then publish
/// results or re-queue depending on the outcome.
fn handle_execute(
    scheduler: &Scheduler,
    transactions: &[Transaction],
    tx_idx: u32,
    incarnation: u32,
    mv_state: &Arc<MVHashMap>,
    base_state: &Arc<dyn StateProvider>,
    block_env: &BlockEnv,
    beneficiary_tracker: &Arc<Mutex<LazyBeneficiaryTracker>>,
) {
    let outcome = execute_transaction(
        &transactions[tx_idx as usize],
        tx_idx,
        incarnation,
        mv_state,
        base_state,
        block_env,
    );

    match outcome {
        ExecutionOutcome::Success {
            read_set,
            write_set,
            result,
            gas_fee,
        } => {
            // Stale incarnation guard: check that the incarnation hasn't changed
            // while we were executing. Another thread may have aborted this tx.
            {
                let state = scheduler.get_tx_state(tx_idx);
                if state.incarnation != incarnation {
                    // Incarnation changed — discard results silently.
                    // The scheduler will re-dispatch this tx with the new incarnation.
                    return;
                }
            }

            // Publish the write-set to MVHashMap so subsequent txs can read our writes.
            write_set.apply_to(mv_state, tx_idx, incarnation);

            // Record gas fee in the side-channel beneficiary tracker.
            beneficiary_tracker
                .lock()
                .expect("beneficiary tracker mutex not poisoned")
                .record_gas_fee(tx_idx, gas_fee);

            // Signal the scheduler that execution completed with results.
            scheduler.finish_execution(tx_idx, incarnation, read_set, write_set, result);
        }
        ExecutionOutcome::EstimateHit { .. } => {
            // Don't publish any partial results. Re-queue this tx for execution
            // after the blocking tx completes.
            scheduler.finish_execution_estimate_hit(tx_idx);
        }
        ExecutionOutcome::ExecutionError(err) => {
            // Store error result so collect_results can retrieve something.
            scheduler.finish_execution_with_error(tx_idx, incarnation, err);
        }
    }
}

/// Handle a Validate task: check the transaction's read-set against the
/// current MVHashMap state. On failure, trigger mark_estimate + clear +
/// cascade lowering.
fn handle_validate(
    scheduler: &Scheduler,
    tx_idx: u32,
    mv_state: &Arc<MVHashMap>,
    beneficiary_tracker: &Arc<Mutex<LazyBeneficiaryTracker>>,
) {
    let read_set = scheduler.take_read_set(tx_idx);
    let valid = validate_transaction(tx_idx, &read_set, mv_state);

    if !valid {
        // Mark all of this tx's MVHashMap entries as ESTIMATE so concurrent
        // readers know this tx is being re-executed.
        mv_state.mark_estimate(tx_idx);
        // Clear the stale entries (they'll be rewritten on re-execution).
        mv_state.clear(tx_idx);
        // Clear the stale gas fee (will be re-recorded on re-execution).
        beneficiary_tracker
            .lock()
            .expect("beneficiary tracker mutex not poisoned")
            .clear_tx(tx_idx);
    } else {
        // Validation succeeded — return the ReadSet to TxState so
        // collect_results() can include it for conflict analysis.
        scheduler.return_read_set(tx_idx, read_set);
    }

    scheduler.finish_validation(tx_idx, valid);
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, Bytes, U256};
    use monad_state::InMemoryState;
    use monad_types::AccountInfo;

    // ── Test helpers ────────────────────────────────────────────────────

    fn sender_a() -> alloy_primitives::Address {
        address!("0x00000000000000000000000000000000000000E1")
    }

    fn sender_b() -> alloy_primitives::Address {
        address!("0x00000000000000000000000000000000000000E2")
    }

    fn receiver_a() -> alloy_primitives::Address {
        address!("0x00000000000000000000000000000000000000F1")
    }

    fn receiver_b() -> alloy_primitives::Address {
        address!("0x00000000000000000000000000000000000000F2")
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

    fn make_transfer(
        from: alloy_primitives::Address,
        to: alloy_primitives::Address,
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

    // ── Integration tests ───────────────────────────────────────────────

    /// Two independent value transfers from different senders to different receivers.
    /// No conflicts expected — both should complete without re-execution.
    #[test]
    fn test_parallel_independent_transfers() {
        let base_state: Arc<dyn StateProvider> = Arc::new(
            InMemoryState::new()
                .with_account(sender_a(), AccountInfo::new(U256::from(1_000_000_000_000u64), 0))
                .with_account(sender_b(), AccountInfo::new(U256::from(1_000_000_000_000u64), 0))
                .with_account(coinbase_addr(), AccountInfo::new(U256::ZERO, 0)),
        );

        let block_env = make_block_env();
        let transactions = vec![
            make_transfer(sender_a(), receiver_a(), 1000, 0),
            make_transfer(sender_b(), receiver_b(), 2000, 0),
        ];

        let result = execute_block_parallel(&transactions, base_state, &block_env, 2);

        // Should have exactly 2 results.
        assert_eq!(result.tx_results.len(), 2, "should have 2 tx results");

        // Both should succeed.
        for (i, (exec_result, write_set, _read_set)) in result.tx_results.iter().enumerate() {
            assert!(
                exec_result.is_success(),
                "tx {} should succeed, got: {:?}",
                i,
                exec_result
            );
            assert!(
                !write_set.is_empty(),
                "tx {} should have state changes",
                i
            );
        }

        // Beneficiary tracker should have fees for both transactions.
        assert_eq!(
            result.beneficiary_tracker.len(),
            2,
            "should have fees for 2 txs"
        );
        assert!(
            result.beneficiary_tracker.total_fees() > U256::ZERO,
            "total fees should be positive"
        );
    }

    /// Empty transaction list returns empty results immediately.
    #[test]
    fn test_parallel_empty_block() {
        let base_state: Arc<dyn StateProvider> = Arc::new(InMemoryState::new());
        let block_env = make_block_env();
        let transactions: Vec<Transaction> = vec![];

        let result = execute_block_parallel(&transactions, base_state, &block_env, 4);

        assert!(result.tx_results.is_empty(), "empty block = empty results");
        assert!(
            result.beneficiary_tracker.is_empty(),
            "no fees for empty block"
        );
    }

    /// Single transaction executes correctly through the parallel path.
    #[test]
    fn test_parallel_single_transaction() {
        let base_state: Arc<dyn StateProvider> = Arc::new(
            InMemoryState::new()
                .with_account(sender_a(), AccountInfo::new(U256::from(1_000_000_000_000u64), 0))
                .with_account(coinbase_addr(), AccountInfo::new(U256::ZERO, 0)),
        );

        let block_env = make_block_env();
        let transactions = vec![make_transfer(sender_a(), receiver_a(), 5000, 0)];

        let result = execute_block_parallel(&transactions, base_state, &block_env, 4);

        assert_eq!(result.tx_results.len(), 1);
        assert!(result.tx_results[0].0.is_success());
        assert!(!result.tx_results[0].1.is_empty());
        assert_eq!(result.beneficiary_tracker.len(), 1);
    }

    /// Verify that ReadSets are preserved after successful validation and
    /// included in collect_results() output. This is the prerequisite for
    /// downstream conflict analysis.
    #[test]
    fn test_read_set_preserved_after_validation() {
        use monad_mv_state::types::LocationKey;

        let base_state: Arc<dyn StateProvider> = Arc::new(
            InMemoryState::new()
                .with_account(sender_a(), AccountInfo::new(U256::from(1_000_000_000_000u64), 0))
                .with_account(sender_b(), AccountInfo::new(U256::from(1_000_000_000_000u64), 0))
                .with_account(coinbase_addr(), AccountInfo::new(U256::ZERO, 0)),
        );

        let block_env = make_block_env();
        let transactions = vec![
            make_transfer(sender_a(), receiver_a(), 1000, 0),
            make_transfer(sender_b(), receiver_b(), 2000, 0),
        ];

        let result = execute_block_parallel(&transactions, base_state, &block_env, 2);

        assert_eq!(result.tx_results.len(), 2, "should have 2 tx results");

        // Both transactions should succeed.
        for (i, (exec_result, _, _)) in result.tx_results.iter().enumerate() {
            assert!(
                exec_result.is_success(),
                "tx {} should succeed, got: {:?}",
                i,
                exec_result
            );
        }

        // Verify ReadSets are non-empty (each value transfer reads sender balance + nonce).
        for (i, (_, _, read_set)) in result.tx_results.iter().enumerate() {
            assert!(
                !read_set.is_empty(),
                "tx {} ReadSet should be non-empty after validation, got empty (data loss)",
                i,
            );

            // Collect location keys from the ReadSet for inspection.
            let location_keys: Vec<&LocationKey> = read_set.iter().map(|(k, _)| k).collect();

            // Value transfers must read the sender's Balance and Nonce at minimum.
            let has_balance = location_keys
                .iter()
                .any(|k| matches!(k, LocationKey::Balance(_)));
            let has_nonce = location_keys
                .iter()
                .any(|k| matches!(k, LocationKey::Nonce(_)));

            assert!(
                has_balance,
                "tx {} ReadSet should contain a Balance read, got: {:?}",
                i, location_keys
            );
            assert!(
                has_nonce,
                "tx {} ReadSet should contain a Nonce read, got: {:?}",
                i, location_keys
            );
        }
    }
}
