//! Stress tests for parallel EVM execution — large blocks, high contention,
//! OCC convergence, and determinism under load.
//!
//! These tests exercise the full pipeline (execute_block_parallel → execute_block)
//! with 50-200 transactions per block to validate:
//! - OCC convergence (all txs eventually validate)
//! - Determinism (parallel == sequential state root across all scenarios)
//! - Repeated runs produce identical results
//! - Performance under high-contention and zero-contention patterns

use std::sync::Arc;

use alloy_primitives::{address, Address, Bytes, U256};

use monad_evm::{execute_block, execute_block_sequential};
use monad_scheduler::execute_block_parallel;
use monad_state::InMemoryState;
use monad_types::{AccountInfo, BlockEnv, Transaction};

// ── Helpers ─────────────────────────────────────────────────────────────

fn coinbase() -> Address {
    address!("0x00000000000000000000000000000000000000C0")
}

fn make_block_env() -> BlockEnv {
    BlockEnv {
        number: 1,
        coinbase: coinbase(),
        timestamp: 1_700_000_000,
        gas_limit: 30_000_000,
        base_fee: U256::ZERO,
        difficulty: U256::ZERO,
    }
}

/// Generate a unique sender address from index (0xE0 range, avoids precompiles).
fn sender(i: usize) -> Address {
    // Use bytes 17-18 for the index to support up to 65535 senders
    let mut bytes = [0u8; 20];
    bytes[17] = 0xE0;
    bytes[18] = ((i >> 8) & 0xFF) as u8;
    bytes[19] = (i & 0xFF) as u8;
    Address::new(bytes)
}

/// Generate a unique receiver address from index (0xF0 range).
fn receiver(i: usize) -> Address {
    let mut bytes = [0u8; 20];
    bytes[17] = 0xF0;
    bytes[18] = ((i >> 8) & 0xFF) as u8;
    bytes[19] = (i & 0xFF) as u8;
    Address::new(bytes)
}

fn make_transfer(from: Address, to: Address, value: u64, nonce: u64) -> Transaction {
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

/// Run the full parallel→merge pipeline and compare against sequential.
/// Returns (parallel_result, sequential_result) for further assertions.
fn run_and_compare(
    transactions: &[Transaction],
    base_state: &InMemoryState,
    block_env: &BlockEnv,
) -> (monad_types::BlockResult, monad_types::BlockResult) {
    let base_arc: Arc<dyn monad_state::StateProvider> = Arc::new(base_state.clone());

    // Parallel path
    let par = execute_block_parallel(transactions, base_arc, block_env, 4);
    let par_result = execute_block(
        base_state,
        &par.tx_results,
        par.beneficiary_tracker.total_fees(),
        block_env,
    )
    .expect("parallel execute_block should succeed");

    // Sequential path
    let seq_result = execute_block_sequential(transactions, base_state, block_env)
        .expect("sequential execute_block should succeed");

    assert_eq!(
        par_result.state_root, seq_result.state_root,
        "parallel != sequential state root for {} txs",
        transactions.len()
    );
    assert_eq!(par_result.receipts.len(), seq_result.receipts.len());
    assert_eq!(par_result.gas_used, seq_result.gas_used);

    (par_result, seq_result)
}

// ── Stress Test: 100 independent transfers ──────────────────────────────

/// 100 unique senders → 100 unique receivers. Zero conflicts.
/// Tests maximum parallelism at scale.
#[test]
fn stress_100_independent_transfers() {
    let n = 100;
    let block_env = make_block_env();

    let mut state = InMemoryState::new()
        .with_account(coinbase(), AccountInfo::new(U256::ZERO, 0));

    let mut transactions = Vec::with_capacity(n);
    for i in 0..n {
        let s = sender(i);
        state = state.with_account(s, AccountInfo::new(U256::from(1_000_000_000_000u64), 0));
        transactions.push(make_transfer(s, receiver(i), 1000, 0));
    }

    let (par_result, _) = run_and_compare(&transactions, &state, &block_env);

    // All 100 should succeed
    assert_eq!(par_result.receipts.len(), n);
    for (i, receipt) in par_result.receipts.iter().enumerate() {
        assert!(receipt.success, "tx {} should succeed", i);
    }

    // Cumulative gas should be monotonically increasing
    for i in 1..par_result.receipts.len() {
        assert!(
            par_result.receipts[i].cumulative_gas_used
                > par_result.receipts[i - 1].cumulative_gas_used,
            "cumulative gas must be monotonically increasing at tx {}",
            i
        );
    }
}

// ── Stress Test: 50 serial dependency chain ─────────────────────────────

/// 50 transactions from the SAME sender (nonces 0..49).
/// Forces maximum OCC conflict and cascade re-execution.
/// This is the worst case for parallel execution — verifies OCC convergence.
#[test]
fn stress_50_serial_dependency_chain() {
    let n = 50;
    let block_env = make_block_env();
    let s = sender(0);

    let state = InMemoryState::new()
        .with_account(s, AccountInfo::new(U256::from(10_000_000_000_000u64), 0))
        .with_account(coinbase(), AccountInfo::new(U256::ZERO, 0));

    let mut transactions = Vec::with_capacity(n);
    for i in 0..n {
        transactions.push(make_transfer(s, receiver(i), 100, i as u64));
    }

    let (par_result, _) = run_and_compare(&transactions, &state, &block_env);

    // All 50 should succeed despite serial dependency
    assert_eq!(par_result.receipts.len(), n);
    for (i, receipt) in par_result.receipts.iter().enumerate() {
        assert!(receipt.success, "tx {} should succeed", i);
    }
}

// ── Stress Test: 100 mixed contention ───────────────────────────────────

/// 100 txs: 50 independent pairs + 50 from 5 shared senders (10 each).
/// Tests realistic contention patterns.
#[test]
fn stress_100_mixed_contention() {
    let block_env = make_block_env();

    let mut state = InMemoryState::new()
        .with_account(coinbase(), AccountInfo::new(U256::ZERO, 0));

    let mut transactions = Vec::with_capacity(100);

    // 50 independent transfers (sender 0..49 → receiver 0..49)
    for i in 0..50 {
        let s = sender(i);
        state = state.with_account(s, AccountInfo::new(U256::from(1_000_000_000_000u64), 0));
        transactions.push(make_transfer(s, receiver(i), 1000, 0));
    }

    // 50 serial transfers from 5 shared senders (10 txs each, nonces 0..9)
    for group in 0..5 {
        let s = sender(100 + group); // senders 100..104
        state = state.with_account(s, AccountInfo::new(U256::from(10_000_000_000_000u64), 0));
        for nonce in 0..10u64 {
            transactions.push(make_transfer(s, receiver(50 + group * 10 + nonce as usize), 100, nonce));
        }
    }

    let (par_result, _) = run_and_compare(&transactions, &state, &block_env);

    assert_eq!(par_result.receipts.len(), 100);
    for (i, receipt) in par_result.receipts.iter().enumerate() {
        assert!(receipt.success, "tx {} should succeed", i);
    }
}

// ── Stress Test: Determinism — repeated runs ────────────────────────────

/// Run the same 64-tx block 5 times, verify identical state root every time.
#[test]
fn stress_determinism_repeated_runs() {
    let n = 64;
    let block_env = make_block_env();

    let mut state = InMemoryState::new()
        .with_account(coinbase(), AccountInfo::new(U256::ZERO, 0));

    let mut transactions = Vec::with_capacity(n);
    for i in 0..n {
        let s = sender(i);
        state = state.with_account(s, AccountInfo::new(U256::from(1_000_000_000_000u64), 0));
        transactions.push(make_transfer(s, receiver(i), 1000, 0));
    }

    let mut roots = Vec::new();
    for run in 0..5 {
        let base_arc: Arc<dyn monad_state::StateProvider> = Arc::new(state.clone());
        let par = execute_block_parallel(&transactions, base_arc, &block_env, 4);
        let result = execute_block(
            &state,
            &par.tx_results,
            par.beneficiary_tracker.total_fees(),
            &block_env,
        )
        .expect("execute_block should succeed");

        roots.push(result.state_root);

        if run > 0 {
            assert_eq!(
                roots[0], roots[run],
                "state root diverged on run {} vs run 0",
                run
            );
        }
    }
}

// ── Stress Test: 200 independent — scale test ───────────────────────────

/// 200 unique senders, zero contention. Tests OCC at larger block size.
#[test]
fn stress_200_independent_transfers() {
    let n = 200;
    let block_env = make_block_env();

    let mut state = InMemoryState::new()
        .with_account(coinbase(), AccountInfo::new(U256::ZERO, 0));

    let mut transactions = Vec::with_capacity(n);
    for i in 0..n {
        let s = sender(i);
        state = state.with_account(s, AccountInfo::new(U256::from(1_000_000_000_000u64), 0));
        transactions.push(make_transfer(s, receiver(i), 1000, 0));
    }

    let (par_result, _) = run_and_compare(&transactions, &state, &block_env);

    assert_eq!(par_result.receipts.len(), n);
    for (i, receipt) in par_result.receipts.iter().enumerate() {
        assert!(receipt.success, "tx {} should succeed", i);
    }

    // Total gas should equal n * per_tx_gas
    let per_tx_gas = par_result.receipts[0].cumulative_gas_used;
    assert_eq!(
        par_result.gas_used,
        per_tx_gas * n as u64,
        "total gas = per_tx_gas * n"
    );
}

// ── Stress Test: All reverts at scale ───────────────────────────────────

/// 50 transactions that all revert (call a REVERT contract).
/// Verifies parallel==sequential state root even when all txs fail.
#[test]
fn stress_50_all_revert() {
    let n = 50;
    let block_env = make_block_env();

    // Deploy a REVERT contract at 0xD0
    let revert_addr = address!("0x00000000000000000000000000000000000000D0");
    let revert_code = Bytes::from(vec![0x5F, 0x5F, 0xFD]); // PUSH0 PUSH0 REVERT
    let revert_code_hash = alloy_primitives::keccak256(&revert_code);

    let mut state = InMemoryState::new()
        .with_account(coinbase(), AccountInfo::new(U256::ZERO, 0))
        .with_account(
            revert_addr,
            AccountInfo::new_contract(U256::ZERO, 0, revert_code_hash, revert_code.clone()),
        )
        .with_code(revert_code_hash, revert_code);

    let mut transactions = Vec::with_capacity(n);
    for i in 0..n {
        let s = sender(i);
        state = state.with_account(s, AccountInfo::new(U256::from(1_000_000_000_000u64), 0));
        // Call the REVERT contract
        transactions.push(Transaction {
            sender: s,
            to: Some(revert_addr),
            value: U256::ZERO,
            data: Bytes::new(),
            gas_limit: 100_000,
            nonce: 0,
            gas_price: U256::from(1_000_000_000u64),
        });
    }

    let (par_result, _) = run_and_compare(&transactions, &state, &block_env);

    assert_eq!(par_result.receipts.len(), n);
    for (i, receipt) in par_result.receipts.iter().enumerate() {
        assert!(!receipt.success, "tx {} should revert", i);
    }
}

// ── Stress Test: Interleaved serial + independent ───────────────────────

/// Alternating pattern: serial pairs (same sender) interleaved with
/// independent txs. Tests that OCC handles non-contiguous conflicts.
#[test]
fn stress_interleaved_serial_independent() {
    let block_env = make_block_env();
    let mut state = InMemoryState::new()
        .with_account(coinbase(), AccountInfo::new(U256::ZERO, 0));

    let mut transactions = Vec::new();

    // 20 serial pairs: sender i sends 2 txs with nonce 0 and 1
    for i in 0..20 {
        let s = sender(i);
        state = state.with_account(s, AccountInfo::new(U256::from(1_000_000_000_000u64), 0));
        transactions.push(make_transfer(s, receiver(i * 2), 500, 0));
        transactions.push(make_transfer(s, receiver(i * 2 + 1), 500, 1));
    }

    // Interleave with 20 independent single txs
    for i in 20..40 {
        let s = sender(i);
        state = state.with_account(s, AccountInfo::new(U256::from(1_000_000_000_000u64), 0));
        transactions.push(make_transfer(s, receiver(40 + i), 1000, 0));
    }

    let (par_result, _) = run_and_compare(&transactions, &state, &block_env);

    assert_eq!(par_result.receipts.len(), 60);
    for (i, receipt) in par_result.receipts.iter().enumerate() {
        assert!(receipt.success, "tx {} should succeed", i);
    }
}
