//! Worker functions for Block-STM parallel execution.
//!
//! These functions bridge the scheduler coordinator with actual EVM execution.
//! Each worker task (Execute or Validate) calls the corresponding function here.
//!
//! - [`execute_transaction()`] creates an MvDatabase, runs revm, extracts state diffs,
//!   converts them to a WriteSet, and returns the execution outcome.
//! - [`validate_transaction()`] re-checks a transaction's read-set against the
//!   current MVHashMap state for OCC conflict detection.
//! - [`convert_state_diffs()`] maps revm's `HashMap<Address, Account>` into a
//!   deterministic WriteSet (BTreeMap), skipping the coinbase address.
//!
//! # Observability
//!
//! - `ExecutionOutcome::EstimateHit { blocking_tx }` — identifies which prior tx
//!   caused an ESTIMATE suspension. The scheduler re-queues the blocked tx.
//! - `ExecutionOutcome::Success { gas_fee, .. }` — gas fee is `gas_used * gas_price`,
//!   ready for LazyBeneficiaryTracker recording.
//! - `validate_transaction()` returns `bool` — `false` signals a conflict; the caller
//!   inspects the MVHashMap to determine which tx caused the version change.

use std::sync::Arc;

use alloy_primitives::{Address, U256};

use monad_mv_state::{
    mv_database::MvDatabase,
    read_write_sets::{ReadSet, WriteSet},
    types::{LocationKey, TxIndex, WriteValue},
    versioned_state::MVHashMap,
};
use monad_state::StateProvider;
use monad_types::{BlockEnv, EvmError, ExecutionResult, Transaction};

use revm::{
    context::{Context, TxEnv},
    context_interface::result::{ExecutionResult as RevmExecutionResult, Output},
    primitives::TxKind,
    state::Account,
    ExecuteEvm, MainBuilder, MainContext,
};

use crate::types::Incarnation;

// ── ExecutionOutcome ────────────────────────────────────────────────────────

/// The result of executing a single transaction through the Block-STM worker.
///
/// Callers match on variants to decide next steps:
/// - `Success` → publish WriteSet to MVHashMap, record gas fee, proceed to validation
/// - `EstimateHit` → don't publish anything, re-queue after the blocking tx completes
/// - `ExecutionError` → handle the error (e.g., the tx reverts are still `Success` in revm,
///   so this variant is for infrastructure errors like DB failures)
pub enum ExecutionOutcome {
    /// Transaction executed successfully. Contains the read/write sets,
    /// execution result, and calculated gas fee.
    Success {
        /// All state locations read during execution (for OCC validation).
        read_set: ReadSet,
        /// All state locations written (to publish to MVHashMap).
        write_set: WriteSet,
        /// The EVM execution result (Success/Revert/Halt).
        result: ExecutionResult,
        /// Gas fee: `gas_used * gas_price` for LazyBeneficiaryTracker.
        gas_fee: U256,
    },
    /// A read hit an ESTIMATE marker — the blocking transaction is being
    /// re-executed. This transaction should be re-queued.
    EstimateHit {
        /// The tx_index of the transaction whose write caused the ESTIMATE.
        blocking_tx: TxIndex,
    },
    /// An infrastructure error occurred (not a revert — reverts are `Success`).
    ExecutionError(EvmError),
}

impl std::fmt::Debug for ExecutionOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionOutcome::Success {
                result, gas_fee, read_set, write_set,
            } => f
                .debug_struct("Success")
                .field("result", result)
                .field("gas_fee", gas_fee)
                .field("read_set_len", &read_set.len())
                .field("write_set_len", &write_set.len())
                .finish(),
            ExecutionOutcome::EstimateHit { blocking_tx } => f
                .debug_struct("EstimateHit")
                .field("blocking_tx", blocking_tx)
                .finish(),
            ExecutionOutcome::ExecutionError(e) => f
                .debug_tuple("ExecutionError")
                .field(e)
                .finish(),
        }
    }
}

// ── execute_transaction ─────────────────────────────────────────────────────

/// Execute a single transaction through revm, backed by MvDatabase.
///
/// Creates a fresh MvDatabase and revm EVM instance for this execution.
/// On success, extracts state diffs from `evm.finalize()`, converts them
/// to a WriteSet (skipping the coinbase address), and calculates the gas fee.
///
/// If the execution reads an ESTIMATE marker (a prior transaction is being
/// re-executed), returns `EstimateHit` without publishing any state.
pub fn execute_transaction(
    tx: &Transaction,
    tx_index: TxIndex,
    _incarnation: Incarnation,
    mv_state: &Arc<MVHashMap>,
    base_state: &Arc<dyn StateProvider>,
    block_env: &BlockEnv,
) -> ExecutionOutcome {
    // Each worker creates its own MvDatabase (revm EVM is not Send).
    let mut db = MvDatabase::new(Arc::clone(mv_state), Arc::clone(base_state), tx_index);

    // Build the revm context with our block environment.
    let basefee: u64 = block_env.base_fee.try_into().unwrap_or(u64::MAX);

    let ctx = Context::mainnet()
        .with_db(&mut db)
        .modify_block_chained(|block| {
            block.number = U256::from(block_env.number);
            block.timestamp = U256::from(block_env.timestamp);
            block.gas_limit = block_env.gas_limit;
            block.basefee = basefee;
            block.difficulty = block_env.difficulty;
            block.beneficiary = block_env.coinbase;
        })
        .modify_cfg_chained(|cfg| {
            cfg.chain_id = 1;
            cfg.disable_eip3607 = true;
            cfg.disable_base_fee = true;
            cfg.disable_fee_charge = true;
        });

    let mut evm = ctx.build_mainnet();

    // Build the transaction environment.
    let tx_kind = match tx.to {
        Some(addr) => TxKind::Call(addr),
        None => TxKind::Create,
    };

    let gas_price: u128 = tx.gas_price.try_into().unwrap_or(u128::MAX);

    let tx_env = match TxEnv::builder()
        .caller(tx.sender)
        .kind(tx_kind)
        .value(tx.value)
        .data(tx.data.clone())
        .gas_limit(tx.gas_limit)
        .gas_price(gas_price)
        .nonce(tx.nonce)
        .build()
    {
        Ok(env) => env,
        Err(e) => {
            return ExecutionOutcome::ExecutionError(EvmError::TransactionValidation(format!(
                "{:?}",
                e
            )));
        }
    };

    // Execute the transaction.
    let result = match evm.transact_one(tx_env) {
        Ok(result) => result,
        Err(e) => {
            // Check if the error contains an ESTIMATE marker read.
            let err_debug = format!("{:?}", e);
            if err_debug.contains("ReadEstimate") {
                // Extract the blocking tx_index from the error.
                // The error chain: EVMError::Database(MvDatabaseError(EvmError::ReadEstimate { tx_index, .. }))
                // We parse the tx_index from the Debug representation.
                let blocking_tx = extract_estimate_tx_index(&err_debug).unwrap_or(0);
                return ExecutionOutcome::EstimateHit { blocking_tx };
            }
            // State-dependent execution errors (nonce mismatch, insufficient
            // balance, etc.) mean this transaction read stale state from a
            // prior tx that hasn't completed yet. Treat as an ESTIMATE hit —
            // re-queue this tx so it retries after the dependency resolves.
            //
            // Without this, the read_set captures only what was actually read
            // (e.g., tx0's writes), but not the missing dependency (tx1's writes).
            // Validation could pass because the read_set matches, leaving the
            // error result as final even after the correct state becomes available.
            if tx_index > 0 {
                return ExecutionOutcome::EstimateHit {
                    blocking_tx: tx_index - 1,
                };
            }
            // tx_index == 0 can't depend on prior txs; genuine error.
            return ExecutionOutcome::ExecutionError(EvmError::Internal(format!(
                "revm execution error: {:?}",
                e
            )));
        }
    };

    // Map the revm result to our type system.
    let monad_result = map_execution_result(&result);

    // Calculate gas fee: gas_used * gas_price (fee charge is disabled in revm).
    let gas_used = monad_result.gas_used();
    let gas_fee = U256::from(gas_used) * tx.gas_price;

    // Finalize to get state diffs — HashMap<Address, Account>.
    let state_diffs = evm.finalize();

    // Convert state diffs to WriteSet, skipping the coinbase address.
    let write_set = convert_state_diffs(state_diffs, block_env.coinbase);

    // Extract the ReadSet from MvDatabase.
    let read_set = db.take_read_set();

    ExecutionOutcome::Success {
        read_set,
        write_set,
        result: monad_result,
        gas_fee,
    }
}

// ── validate_transaction ────────────────────────────────────────────────────

/// Validate a transaction's read-set against the current MVHashMap state.
///
/// Returns `true` if all reads are still valid (same versions), `false` if any
/// read has changed (conflict detected). The caller handles the consequence:
/// on failure, calls `mv_state.mark_estimate(tx_index)` and
/// `mv_state.clear(tx_index)` before re-queuing the transaction.
pub fn validate_transaction(
    tx_index: TxIndex,
    read_set: &ReadSet,
    mv_state: &MVHashMap,
) -> bool {
    read_set.validate(mv_state, tx_index)
}

// ── convert_state_diffs ─────────────────────────────────────────────────────

/// Convert revm state diffs to a deterministic WriteSet.
///
/// Iterates over the (Address, Account) pairs from `evm.finalize()`, records
/// Balance, Nonce, CodeHash, and Storage entries. The coinbase address is
/// **skipped** — gas fees go through LazyBeneficiaryTracker instead.
///
/// Uses BTreeMap internally (via WriteSet) for deterministic ordering,
/// avoiding non-determinism from HashMap iteration.
pub fn convert_state_diffs(
    changes: impl IntoIterator<Item = (Address, Account)>,
    coinbase: Address,
) -> WriteSet {
    let mut write_set = WriteSet::new();

    for (address, account) in changes {
        // Skip coinbase — gas fees are tracked via LazyBeneficiaryTracker.
        if address == coinbase {
            continue;
        }

        // Record balance, nonce, and code_hash for each touched account.
        write_set.record(
            LocationKey::Balance(address),
            WriteValue::Balance(account.info.balance),
        );
        write_set.record(
            LocationKey::Nonce(address),
            WriteValue::Nonce(account.info.nonce),
        );
        write_set.record(
            LocationKey::CodeHash(address),
            WriteValue::CodeHash(account.info.code_hash),
        );

        // Record each storage slot change.
        for (slot, value) in account.storage {
            write_set.record(
                LocationKey::Storage(address, slot),
                WriteValue::Storage(value.present_value()),
            );
        }
    }

    write_set
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Map a revm ExecutionResult to our ExecutionResult type.
fn map_execution_result(result: &RevmExecutionResult) -> ExecutionResult {
    match result {
        RevmExecutionResult::Success {
            gas, output, logs, ..
        } => {
            let output_bytes = match output {
                Output::Call(data) => data.clone(),
                Output::Create(data, _addr) => data.clone(),
            };

            let mapped_logs: Vec<monad_types::Log> = logs
                .iter()
                .map(|log| monad_types::Log {
                    address: log.address,
                    topics: log.topics().to_vec(),
                    data: log.data.data.clone(),
                })
                .collect();

            ExecutionResult::Success {
                gas_used: gas.used(),
                output: output_bytes,
                logs: mapped_logs,
            }
        }
        RevmExecutionResult::Revert { gas, output, .. } => ExecutionResult::Revert {
            gas_used: gas.used(),
            output: output.clone(),
        },
        RevmExecutionResult::Halt { gas, reason, .. } => ExecutionResult::Halt {
            gas_used: gas.used(),
            reason: format!("{:?}", reason),
        },
    }
}

/// Extract the blocking tx_index from a Debug-formatted ESTIMATE error.
///
/// Parses patterns like `ReadEstimate { tx_index: 3, location: "..." }`.
fn extract_estimate_tx_index(err_debug: &str) -> Option<TxIndex> {
    // Look for "tx_index: N" pattern in the debug string.
    let marker = "tx_index: ";
    let start = err_debug.find(marker)?;
    let after = &err_debug[start + marker.len()..];
    let end = after.find(|c: char| !c.is_ascii_digit())?;
    let num_str = &after[..end];
    num_str.parse::<TxIndex>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, Bytes};
    use monad_mv_state::read_write_sets::ReadSet;
    use monad_mv_state::types::ReadOrigin;
    use monad_state::InMemoryState;
    use monad_types::AccountInfo;

    // ── Test helpers ────────────────────────────────────────────────────

    fn sender_addr() -> Address {
        address!("0x00000000000000000000000000000000000000E1")
    }

    fn receiver_addr() -> Address {
        address!("0x00000000000000000000000000000000000000E2")
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

    fn make_transfer_tx(from: Address, to: Address, value: U256, nonce: u64) -> Transaction {
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

    // ── Test: execute a simple value transfer ───────────────────────────

    #[test]
    fn test_execute_simple_transfer() {
        let sender = sender_addr();
        let receiver = receiver_addr();
        let coinbase = coinbase_addr();

        // Set up base state with sender having enough balance.
        let base_state: Arc<dyn StateProvider> = Arc::new(
            InMemoryState::new()
                .with_account(sender, AccountInfo::new(U256::from(1_000_000_000_000u64), 0))
                .with_account(coinbase, AccountInfo::new(U256::ZERO, 0)),
        );

        let mv_state = Arc::new(MVHashMap::new());
        let block_env = make_block_env();
        let tx = make_transfer_tx(sender, receiver, U256::from(1000u64), 0);

        let outcome = execute_transaction(&tx, 0, 0, &mv_state, &base_state, &block_env);

        match outcome {
            ExecutionOutcome::Success {
                read_set,
                write_set,
                result,
                gas_fee,
            } => {
                // Execution should succeed.
                assert!(result.is_success(), "transfer should succeed");

                // Gas fee should be gas_used * gas_price.
                let gas_used = result.gas_used();
                assert!(gas_used > 0, "should use some gas");
                assert_eq!(gas_fee, U256::from(gas_used) * U256::from(1_000_000_000u64));

                // WriteSet should contain entries for sender and receiver (not coinbase).
                assert!(
                    !write_set.is_empty(),
                    "write_set should have balance/nonce changes"
                );

                // Check that sender balance and nonce are in WriteSet.
                let mut has_sender_balance = false;
                let mut has_sender_nonce = false;
                let mut has_receiver_balance = false;
                let mut has_coinbase = false;

                for (loc, _) in write_set.iter() {
                    match loc {
                        LocationKey::Balance(addr) if *addr == sender => {
                            has_sender_balance = true;
                        }
                        LocationKey::Nonce(addr) if *addr == sender => {
                            has_sender_nonce = true;
                        }
                        LocationKey::Balance(addr) if *addr == receiver => {
                            has_receiver_balance = true;
                        }
                        LocationKey::Balance(addr) if *addr == coinbase => {
                            has_coinbase = true;
                        }
                        LocationKey::Nonce(addr) if *addr == coinbase => {
                            has_coinbase = true;
                        }
                        LocationKey::CodeHash(addr) if *addr == coinbase => {
                            has_coinbase = true;
                        }
                        _ => {}
                    }
                }

                assert!(has_sender_balance, "should have sender balance change");
                assert!(has_sender_nonce, "should have sender nonce change");
                assert!(has_receiver_balance, "should have receiver balance change");
                assert!(!has_coinbase, "coinbase must NOT be in WriteSet");

                // ReadSet should be non-empty (read sender's balance/nonce at minimum).
                assert!(!read_set.is_empty(), "read_set should have reads");
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }

    // ── Test: ESTIMATE hit returns EstimateHit ──────────────────────────

    #[test]
    fn test_execute_estimate_hit() {
        let sender = sender_addr();
        let receiver = receiver_addr();
        let coinbase = coinbase_addr();

        // Set up base state.
        let base_state: Arc<dyn StateProvider> = Arc::new(
            InMemoryState::new()
                .with_account(sender, AccountInfo::new(U256::from(1_000_000_000_000u64), 0))
                .with_account(coinbase, AccountInfo::new(U256::ZERO, 0)),
        );

        let mv_state = Arc::new(MVHashMap::new());
        let block_env = make_block_env();

        // Pre-populate MVHashMap: tx=0 wrote sender's balance, then mark as ESTIMATE.
        mv_state.write(
            LocationKey::Balance(sender),
            0,
            0,
            WriteValue::Balance(U256::from(999_999_999_000u64)),
        );
        mv_state.mark_estimate(0);

        // tx=1 tries to execute — reading sender's balance should hit ESTIMATE.
        let tx = make_transfer_tx(sender, receiver, U256::from(1000u64), 0);
        let outcome = execute_transaction(&tx, 1, 0, &mv_state, &base_state, &block_env);

        match outcome {
            ExecutionOutcome::EstimateHit { blocking_tx } => {
                assert_eq!(blocking_tx, 0, "should be blocked by tx 0");
            }
            other => panic!("expected EstimateHit, got {:?}", other),
        }
    }

    // ── Test: convert_state_diffs skips coinbase ────────────────────────

    #[test]
    fn test_convert_state_diffs_skips_coinbase() {
        use revm::state::AccountInfo as RevmAccountInfo;

        let coinbase = coinbase_addr();
        let user = sender_addr();

        // Create mock state diffs.
        let mut changes: Vec<(Address, Account)> = Vec::new();

        // User account change.
        let mut user_account = Account::new_not_existing(0);
        user_account.info = RevmAccountInfo {
            balance: U256::from(900u64),
            nonce: 1,
            code_hash: monad_types::KECCAK_EMPTY,
            code: None,
            account_id: None,
        };
        changes.push((user, user_account));

        // Coinbase account change (should be skipped).
        let mut coinbase_account = Account::new_not_existing(0);
        coinbase_account.info = RevmAccountInfo {
            balance: U256::from(21000u64),
            nonce: 0,
            code_hash: monad_types::KECCAK_EMPTY,
            code: None,
            account_id: None,
        };
        changes.push((coinbase, coinbase_account));

        let write_set = convert_state_diffs(changes, coinbase);

        // Should have user entries but NOT coinbase entries.
        let mut has_user = false;
        let mut has_coinbase = false;
        for (loc, _) in write_set.iter() {
            match loc {
                LocationKey::Balance(addr) | LocationKey::Nonce(addr) | LocationKey::CodeHash(addr)
                    if *addr == user =>
                {
                    has_user = true;
                }
                LocationKey::Balance(addr) | LocationKey::Nonce(addr) | LocationKey::CodeHash(addr)
                    if *addr == coinbase =>
                {
                    has_coinbase = true;
                }
                _ => {}
            }
        }

        assert!(has_user, "user changes should be in WriteSet");
        assert!(!has_coinbase, "coinbase must be excluded from WriteSet");
    }

    // ── Test: convert_state_diffs includes storage slots ────────────────

    #[test]
    fn test_convert_state_diffs_includes_storage() {
        use revm::state::{AccountInfo as RevmAccountInfo, EvmStorageSlot};

        let user = sender_addr();
        let coinbase = coinbase_addr();

        let mut account = Account::new_not_existing(0);
        account.info = RevmAccountInfo {
            balance: U256::from(500u64),
            nonce: 2,
            code_hash: monad_types::KECCAK_EMPTY,
            code: None,
            account_id: None,
        };
        // Add a storage slot change.
        account
            .storage
            .insert(U256::from(7u64), EvmStorageSlot::new(U256::from(42u64), 0));

        let changes = vec![(user, account)];
        let write_set = convert_state_diffs(changes, coinbase);

        // Should have Balance, Nonce, CodeHash, and Storage(7).
        let mut has_storage = false;
        for (loc, val) in write_set.iter() {
            if let LocationKey::Storage(addr, slot) = loc {
                if *addr == user && *slot == U256::from(7u64) {
                    assert_eq!(*val, WriteValue::Storage(U256::from(42u64)));
                    has_storage = true;
                }
            }
        }
        assert!(has_storage, "storage slot should be in WriteSet");
    }

    // ── Test: validate_transaction passes when unchanged ────────────────

    #[test]
    fn test_validate_passes_unchanged() {
        let mv = MVHashMap::new();
        let addr = sender_addr();

        // tx=0 wrote balance.
        mv.write(
            LocationKey::Balance(addr),
            0,
            0,
            WriteValue::Balance(U256::from(100)),
        );

        // Build a ReadSet that recorded reading tx=0's value.
        let mut rs = ReadSet::new();
        rs.record(
            LocationKey::Balance(addr),
            ReadOrigin::MvHashMap {
                tx_index: 0,
                incarnation: 0,
            },
        );

        // Validate at tx_index=1 — should pass (no changes).
        assert!(validate_transaction(1, &rs, &mv));
    }

    // ── Test: validate_transaction fails on version change ──────────────

    #[test]
    fn test_validate_fails_on_version_change() {
        let mv = MVHashMap::new();
        let addr = sender_addr();

        // tx=0, incarnation=0 wrote balance.
        mv.write(
            LocationKey::Balance(addr),
            0,
            0,
            WriteValue::Balance(U256::from(100)),
        );

        // Build a ReadSet referencing (tx=0, incarnation=0).
        let mut rs = ReadSet::new();
        rs.record(
            LocationKey::Balance(addr),
            ReadOrigin::MvHashMap {
                tx_index: 0,
                incarnation: 0,
            },
        );

        // tx=0 re-executes with incarnation=1 — version changes.
        mv.write(
            LocationKey::Balance(addr),
            0,
            1,
            WriteValue::Balance(U256::from(200)),
        );

        // Validate at tx_index=1 — should fail.
        assert!(!validate_transaction(1, &rs, &mv));
    }

    // ── Test: extract_estimate_tx_index ─────────────────────────────────

    #[test]
    fn test_extract_estimate_tx_index() {
        let s = r#"Database(MvDatabaseError(ReadEstimate { tx_index: 3, location: "Balance(0x...)" }))"#;
        assert_eq!(extract_estimate_tx_index(s), Some(3));

        let s2 = r#"ReadEstimate { tx_index: 42, location: "Storage" }"#;
        assert_eq!(extract_estimate_tx_index(s2), Some(42));

        let s3 = "some other error";
        assert_eq!(extract_estimate_tx_index(s3), None);
    }

    // ── Test: gas_fee calculation ───────────────────────────────────────

    #[test]
    fn test_gas_fee_calculation() {
        let sender = sender_addr();
        let receiver = receiver_addr();
        let coinbase = coinbase_addr();

        let base_state: Arc<dyn StateProvider> = Arc::new(
            InMemoryState::new()
                .with_account(sender, AccountInfo::new(U256::from(1_000_000_000_000u64), 0))
                .with_account(coinbase, AccountInfo::new(U256::ZERO, 0)),
        );

        let mv_state = Arc::new(MVHashMap::new());
        let block_env = make_block_env();

        // Use a specific gas_price to verify the calculation.
        let gas_price = U256::from(2_000_000_000u64); // 2 gwei
        let tx = Transaction {
            sender,
            to: Some(receiver),
            value: U256::from(100u64),
            data: Bytes::new(),
            gas_limit: 100_000,
            nonce: 0,
            gas_price,
        };

        let outcome = execute_transaction(&tx, 0, 0, &mv_state, &base_state, &block_env);

        match outcome {
            ExecutionOutcome::Success {
                result, gas_fee, ..
            } => {
                let expected_fee = U256::from(result.gas_used()) * gas_price;
                assert_eq!(gas_fee, expected_fee, "gas_fee = gas_used * gas_price");
            }
            other => panic!("expected Success, got {:?}", other),
        }
    }
}
