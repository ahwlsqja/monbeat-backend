//! Differential test harness: parallel (Block-STM) vs. sequential execution.
//!
//! Runs the same block of transactions both ways and asserts identical final
//! state. This is the primary verification for PARA-08.
//!
//! Test addresses use 0xE0+ range to avoid precompile collisions (0x01-0x13, 0x100).
//! Gas limits ≤ 16,777,216 (EIP-7825 OSAKA cap).

use std::collections::BTreeSet;
use std::sync::Arc;

use alloy_primitives::{address, Address, Bytes, U256};

use monad_evm::EvmExecutor;
use monad_mv_state::types::{LocationKey, WriteValue};
use monad_scheduler::execute_block_parallel;
use monad_state::InMemoryState;
use monad_types::{AccountInfo, BlockEnv, Transaction};

// ── Address constants (all ≥ 0xE0) ─────────────────────────────────────

fn sender_a() -> Address {
    address!("0x00000000000000000000000000000000000000E1")
}
fn sender_b() -> Address {
    address!("0x00000000000000000000000000000000000000E2")
}
fn sender_c() -> Address {
    address!("0x00000000000000000000000000000000000000E3")
}
fn sender_d() -> Address {
    address!("0x00000000000000000000000000000000000000E4")
}

fn receiver_a() -> Address {
    address!("0x00000000000000000000000000000000000000F1")
}
fn receiver_b() -> Address {
    address!("0x00000000000000000000000000000000000000F2")
}
fn receiver_c() -> Address {
    address!("0x00000000000000000000000000000000000000F3")
}
fn receiver_d() -> Address {
    address!("0x00000000000000000000000000000000000000F4")
}

fn coinbase_addr() -> Address {
    address!("0x00000000000000000000000000000000000000C0")
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn make_block_env() -> BlockEnv {
    BlockEnv {
        number: 1,
        coinbase: coinbase_addr(),
        timestamp: 1_700_000_000,
        gas_limit: 16_777_216, // EIP-7825 OSAKA cap
        base_fee: U256::ZERO,
        difficulty: U256::ZERO,
    }
}

fn make_transfer(from: Address, to: Address, value: U256, nonce: u64) -> Transaction {
    Transaction {
        sender: from,
        to: Some(to),
        value,
        data: Bytes::new(),
        gas_limit: 100_000,
        nonce,
        gas_price: U256::from(1_000_000_000u64), // 1 gwei
    }
}

/// Execute transactions sequentially via EvmExecutor, applying state diffs
/// after each tx so that subsequent txs see updated state.
///
/// Returns the final InMemoryState after all transactions have executed.
/// Gas fees (gas_used * gas_price) are accumulated and credited to coinbase
/// at the end, matching the parallel path's LazyBeneficiaryTracker behavior.
fn run_sequential(
    transactions: &[Transaction],
    base_state: &InMemoryState,
    block_env: &BlockEnv,
) -> InMemoryState {
    let mut state = base_state.clone();
    let mut total_gas_fee = U256::ZERO;

    for tx in transactions {
        let (result, state_changes) =
            EvmExecutor::execute_tx_with_state_changes(tx, &state, block_env)
                .expect("sequential execution should succeed");

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

        // Accumulate gas fee for coinbase (disable_fee_charge=true means
        // revm doesn't credit coinbase; we do it manually, matching the
        // parallel path's LazyBeneficiaryTracker).
        let gas_fee = U256::from(result.gas_used()) * tx.gas_price;
        total_gas_fee += gas_fee;
    }

    // Apply total gas fees to coinbase.
    let coinbase_acct = state
        .get_account(&block_env.coinbase)
        .cloned()
        .unwrap_or_else(|| AccountInfo::new(U256::ZERO, 0));
    state.insert_account(
        block_env.coinbase,
        AccountInfo::new(coinbase_acct.balance + total_gas_fee, coinbase_acct.nonce),
    );

    state
}

/// Execute transactions in parallel via Block-STM, then apply WriteSets
/// in order to build final state.
///
/// Returns the final InMemoryState after merging all write-sets plus
/// coinbase gas fees from LazyBeneficiaryTracker.
fn run_parallel(
    transactions: &[Transaction],
    base_state: &InMemoryState,
    block_env: &BlockEnv,
) -> InMemoryState {
    let state_provider: Arc<dyn monad_state::StateProvider> = Arc::new(base_state.clone());
    let result = execute_block_parallel(transactions, state_provider, block_env, 4);

    // Start from a clone of base state and apply write-sets in block order.
    let mut final_state = base_state.clone();

    for (_exec_result, write_set) in &result.tx_results {
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
                    // Mismatched location/value types — shouldn't happen.
                    panic!("mismatched LocationKey/WriteValue: {:?} / {:?}", location, value);
                }
            }
        }
    }

    // Apply coinbase gas fees from LazyBeneficiaryTracker.
    let total_fees = result.beneficiary_tracker.total_fees();
    let coinbase_acct = final_state
        .get_account(&block_env.coinbase)
        .cloned()
        .unwrap_or_else(|| AccountInfo::new(U256::ZERO, 0));
    final_state.insert_account(
        block_env.coinbase,
        AccountInfo::new(coinbase_acct.balance + total_fees, coinbase_acct.nonce),
    );

    final_state
}

/// Compare two InMemoryState instances field-by-field on a given set of addresses.
///
/// Checks balance and nonce for every address. Panics with detailed mismatch info.
fn assert_states_equal(
    label: &str,
    sequential: &InMemoryState,
    parallel: &InMemoryState,
    addresses: &[Address],
) {
    // Sort addresses for deterministic comparison order.
    let sorted: BTreeSet<Address> = addresses.iter().copied().collect();

    for addr in &sorted {
        let seq_acct = sequential.get_account(addr);
        let par_acct = parallel.get_account(addr);

        match (seq_acct, par_acct) {
            (Some(s), Some(p)) => {
                assert_eq!(
                    s.balance, p.balance,
                    "[{}] Balance mismatch for {}: sequential={}, parallel={}",
                    label, addr, s.balance, p.balance
                );
                assert_eq!(
                    s.nonce, p.nonce,
                    "[{}] Nonce mismatch for {}: sequential={}, parallel={}",
                    label, addr, s.nonce, p.nonce
                );
            }
            (None, None) => {
                // Both absent — consistent.
            }
            (Some(s), None) => {
                panic!(
                    "[{}] Address {} exists in sequential (bal={}, nonce={}) but not in parallel",
                    label, addr, s.balance, s.nonce
                );
            }
            (None, Some(p)) => {
                panic!(
                    "[{}] Address {} exists in parallel (bal={}, nonce={}) but not in sequential",
                    label, addr, p.balance, p.nonce
                );
            }
        }
    }
}

// ── Test Cases ──────────────────────────────────────────────────────────

/// Test 1: Independent transfers (zero conflicts).
///
/// 4 transactions from 4 different senders to 4 different receivers.
/// Each sender transfers 1 ETH. No shared state between transactions,
/// so the parallel executor should produce identical results without
/// any re-executions.
#[test]
fn test_differential_independent_transfers() {
    let one_eth = U256::from(1_000_000_000_000_000_000u128);
    let hundred_eth = U256::from(100) * one_eth;

    let base_state = InMemoryState::new()
        .with_account(sender_a(), AccountInfo::new(hundred_eth, 0))
        .with_account(sender_b(), AccountInfo::new(hundred_eth, 0))
        .with_account(sender_c(), AccountInfo::new(hundred_eth, 0))
        .with_account(sender_d(), AccountInfo::new(hundred_eth, 0))
        .with_account(coinbase_addr(), AccountInfo::new(U256::ZERO, 0));

    let block_env = make_block_env();

    let transactions = vec![
        make_transfer(sender_a(), receiver_a(), one_eth, 0),
        make_transfer(sender_b(), receiver_b(), one_eth, 0),
        make_transfer(sender_c(), receiver_c(), one_eth, 0),
        make_transfer(sender_d(), receiver_d(), one_eth, 0),
    ];

    let seq_state = run_sequential(&transactions, &base_state, &block_env);
    let par_state = run_parallel(&transactions, &base_state, &block_env);

    let all_addrs = vec![
        sender_a(), sender_b(), sender_c(), sender_d(),
        receiver_a(), receiver_b(), receiver_c(), receiver_d(),
        coinbase_addr(),
    ];

    assert_states_equal("independent_transfers", &seq_state, &par_state, &all_addrs);

    // Additional sanity checks on the sequential result.
    for recv in [receiver_a(), receiver_b(), receiver_c(), receiver_d()] {
        let acct = seq_state.get_account(&recv).expect("receiver should exist");
        assert_eq!(acct.balance, one_eth, "receiver should have 1 ETH");
    }

    for sender in [sender_a(), sender_b(), sender_c(), sender_d()] {
        let acct = seq_state.get_account(&sender).expect("sender should exist");
        assert_eq!(acct.nonce, 1, "sender nonce should be 1 after one tx");
        assert!(acct.balance < hundred_eth, "sender balance should decrease");
    }
}

/// Test 2: Serial dependency chain (all conflicts).
///
/// 3 transactions from the SAME sender to different receivers.
/// tx0 nonce=0, tx1 nonce=1, tx2 nonce=2. Each sends 1 ETH.
/// This forces conflicts on sender balance/nonce — tx1 depends on
/// tx0's output, tx2 depends on tx1's. The parallel executor must
/// re-execute conflicting transactions to produce correct results.
#[test]
fn test_differential_serial_dependency_chain() {
    let one_eth = U256::from(1_000_000_000_000_000_000u128);
    let hundred_eth = U256::from(100) * one_eth;

    let base_state = InMemoryState::new()
        .with_account(sender_a(), AccountInfo::new(hundred_eth, 0))
        .with_account(coinbase_addr(), AccountInfo::new(U256::ZERO, 0));

    let block_env = make_block_env();

    let transactions = vec![
        make_transfer(sender_a(), receiver_a(), one_eth, 0),
        make_transfer(sender_a(), receiver_b(), one_eth, 1),
        make_transfer(sender_a(), receiver_c(), one_eth, 2),
    ];

    let seq_state = run_sequential(&transactions, &base_state, &block_env);
    let par_state = run_parallel(&transactions, &base_state, &block_env);

    let all_addrs = vec![
        sender_a(),
        receiver_a(), receiver_b(), receiver_c(),
        coinbase_addr(),
    ];

    assert_states_equal("serial_dependency", &seq_state, &par_state, &all_addrs);

    // Sanity: sender nonce should be 3, all receivers should have 1 ETH.
    let sender_acct = seq_state.get_account(&sender_a()).expect("sender should exist");
    assert_eq!(sender_acct.nonce, 3, "sender nonce should be 3 after 3 txs");

    for recv in [receiver_a(), receiver_b(), receiver_c()] {
        let acct = seq_state.get_account(&recv).expect("receiver should exist");
        assert_eq!(acct.balance, one_eth, "each receiver should have 1 ETH");
    }
}

/// Test 3: Mixed block with both independent and conflicting transactions.
///
/// tx0: sender_a → receiver_a (nonce 0)  — conflicts with tx1 on sender_a
/// tx1: sender_a → receiver_b (nonce 1)  — conflicts with tx0 on sender_a
/// tx2: sender_b → receiver_c (nonce 0)  — independent
/// tx3: sender_b → receiver_d (nonce 1)  — conflicts with tx2 on sender_b
///
/// This exercises both conflict patterns simultaneously. The parallel
/// executor must handle ESTIMATE markers for tx1 (reading sender_a written
/// by tx0) and tx3 (reading sender_b written by tx2).
#[test]
fn test_differential_mixed_block() {
    let one_eth = U256::from(1_000_000_000_000_000_000u128);
    let hundred_eth = U256::from(100) * one_eth;

    let base_state = InMemoryState::new()
        .with_account(sender_a(), AccountInfo::new(hundred_eth, 0))
        .with_account(sender_b(), AccountInfo::new(hundred_eth, 0))
        .with_account(coinbase_addr(), AccountInfo::new(U256::ZERO, 0));

    let block_env = make_block_env();

    let transactions = vec![
        make_transfer(sender_a(), receiver_a(), one_eth, 0),
        make_transfer(sender_a(), receiver_b(), one_eth, 1),
        make_transfer(sender_b(), receiver_c(), one_eth, 0),
        make_transfer(sender_b(), receiver_d(), one_eth, 1),
    ];

    let seq_state = run_sequential(&transactions, &base_state, &block_env);
    let par_state = run_parallel(&transactions, &base_state, &block_env);

    let all_addrs = vec![
        sender_a(), sender_b(),
        receiver_a(), receiver_b(), receiver_c(), receiver_d(),
        coinbase_addr(),
    ];

    assert_states_equal("mixed_block", &seq_state, &par_state, &all_addrs);

    // Sanity: both senders sent 2 txs each.
    let a_acct = seq_state.get_account(&sender_a()).unwrap();
    assert_eq!(a_acct.nonce, 2, "sender_a nonce should be 2");
    let b_acct = seq_state.get_account(&sender_b()).unwrap();
    assert_eq!(b_acct.nonce, 2, "sender_b nonce should be 2");

    // All receivers got 1 ETH.
    for recv in [receiver_a(), receiver_b(), receiver_c(), receiver_d()] {
        let acct = seq_state.get_account(&recv).expect("receiver should exist");
        assert_eq!(acct.balance, one_eth, "receiver should have 1 ETH");
    }
}
