//! EVM transaction executor.
//!
//! Provides `EvmExecutor` which bridges our types with revm to execute
//! single transactions and return results in our type system.

use std::collections::HashMap;

use monad_state::StateProvider;
use monad_types::{
    AccountInfo, Address, BlockEnv, EvmError, Transaction, U256,
    ExecutionResult as MonadExecutionResult,
    Log as MonadLog,
};

use revm::{
    context::{Context, TxEnv},
    context_interface::result::{ExecutionResult as RevmExecutionResult, Output},
    primitives::TxKind,
    state::Account,
    ExecuteEvm, MainBuilder, MainContext,
};

use crate::db_bridge::DbBridge;

/// State changes resulting from transaction execution.
///
/// Maps address → (updated AccountInfo, storage changes).
pub type StateChanges = HashMap<Address, (AccountInfo, HashMap<U256, U256>)>;

/// EVM transaction executor.
///
/// Creates revm instances, configures them from our types, executes
/// transactions, and returns results in our type system.
pub struct EvmExecutor;

impl EvmExecutor {
    /// Executes a single transaction against the given state.
    ///
    /// Returns our `ExecutionResult` with gas_used, output, and logs.
    pub fn execute_tx(
        tx: &Transaction,
        state: &dyn StateProvider,
        block_env: &BlockEnv,
    ) -> Result<MonadExecutionResult, EvmError> {
        let (result, _) = Self::execute_tx_with_state_changes(tx, state, block_env)?;
        Ok(result)
    }

    /// Executes a single transaction and returns both the result and state diffs.
    ///
    /// The state changes map contains the updated account info and storage
    /// for every touched account. Used in tests to verify post-execution state.
    pub fn execute_tx_with_state_changes(
        tx: &Transaction,
        state: &dyn StateProvider,
        block_env: &BlockEnv,
    ) -> Result<(MonadExecutionResult, StateChanges), EvmError> {
        let db = DbBridge::new(state);

        // Build the EVM context with our block environment
        // base_fee is U256 in our types but u64 in revm v36
        let basefee: u64 = block_env
            .base_fee
            .try_into()
            .unwrap_or(u64::MAX);

        let ctx = Context::mainnet()
            .with_db(db)
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
                // Disable EIP-3607 (rejects tx from senders with code)
                cfg.disable_eip3607 = true;
                // Disable base fee to simplify gas calculations in tests
                cfg.disable_base_fee = true;
                // Disable fee charge (no gas deducted via coinbase)
                cfg.disable_fee_charge = true;
            });

        let mut evm = ctx.build_mainnet();

        // Build the transaction environment
        let tx_kind = match tx.to {
            Some(addr) => TxKind::Call(addr),
            None => TxKind::Create,
        };

        // gas_price is U256 in our types but u128 in revm v36 TxEnv
        let gas_price: u128 = tx
            .gas_price
            .try_into()
            .unwrap_or(u128::MAX);

        let tx_env = TxEnv::builder()
            .caller(tx.sender)
            .kind(tx_kind)
            .value(tx.value)
            .data(tx.data.clone())
            .gas_limit(tx.gas_limit)
            .gas_price(gas_price)
            .nonce(tx.nonce)
            .build()
            .map_err(|e| EvmError::TransactionValidation(format!("{:?}", e)))?;

        // Execute the transaction
        let result = evm
            .transact_one(tx_env)
            .map_err(|e| EvmError::Internal(format!("revm execution error: {:?}", e)))?;

        // Convert revm result to our types
        let monad_result = map_execution_result(&result);

        // Finalize to get state changes
        let state_diffs = evm.finalize();
        let mapped_changes = map_state_changes(state_diffs);

        Ok((monad_result, mapped_changes))
    }
}

/// Maps a revm `ExecutionResult` to our `ExecutionResult`.
fn map_execution_result(result: &RevmExecutionResult) -> MonadExecutionResult {
    match result {
        RevmExecutionResult::Success {
            gas,
            output,
            logs,
            ..
        } => {
            let output_bytes = match output {
                Output::Call(data) => data.clone(),
                Output::Create(data, _addr) => data.clone(),
            };

            let mapped_logs: Vec<MonadLog> = logs
                .iter()
                .map(|log| MonadLog {
                    address: log.address,
                    topics: log.topics().to_vec(),
                    data: log.data.data.clone(),
                })
                .collect();

            MonadExecutionResult::Success {
                gas_used: gas.used(),
                output: output_bytes,
                logs: mapped_logs,
            }
        }
        RevmExecutionResult::Revert { gas, output, .. } => MonadExecutionResult::Revert {
            gas_used: gas.used(),
            output: output.clone(),
        },
        RevmExecutionResult::Halt { gas, reason, .. } => MonadExecutionResult::Halt {
            gas_used: gas.used(),
            reason: format!("{:?}", reason),
        },
    }
}

/// Maps revm state changes to our `StateChanges` type.
fn map_state_changes(changes: impl IntoIterator<Item = (Address, Account)>) -> StateChanges {
    let mut result = HashMap::new();

    for (address, account) in changes {
        let acct_info = AccountInfo {
            balance: account.info.balance,
            nonce: account.info.nonce,
            code_hash: account.info.code_hash,
            code: account.info.code.map(|c| c.original_bytes()),
        };

        let mut storage_changes = HashMap::new();
        for (slot, value) in account.storage {
            storage_changes.insert(slot, value.present_value());
        }

        result.insert(address, (acct_info, storage_changes));
    }

    result
}
