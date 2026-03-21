//! # NINE FORK Precompile Provider
//!
//! Wraps revm's `EthPrecompiles` and extends it with Monad-specific precompiles
//! (MIP-4 reserve balance at address 0x20).
//!
//! ## Architecture
//!
//! Rather than mutating the static `Precompiles` registry (which is `&'static`),
//! this module implements `PrecompileProvider` by delegating to `EthPrecompiles`
//! for standard precompiles and handling MIP-4 directly. The `with_precompiles()`
//! method on revm's `Evm` struct allows swapping in our custom provider.
//!
//! ## Usage
//!
//! ```ignore
//! use monad_nine_fork::nine_fork_precompiles::NineForkPrecompiles;
//! use revm::context::Context;
//! use revm::MainBuilder;
//!
//! let ctx = Context::mainnet().with_db(db);
//! let evm = ctx.build_mainnet().with_precompiles(NineForkPrecompiles::new());
//! ```

use alloy_primitives::Address;
use revm::context::Cfg;
use revm::context_interface::ContextTr;
use revm::context_interface::local::LocalContextTr;
use revm::handler::{EthPrecompiles, PrecompileProvider};
use revm::interpreter::{CallInput, Gas, InstructionResult, InterpreterResult};
use revm::precompile::PrecompileError;
use revm::primitives::hardfork::SpecId;
use revm::primitives::Bytes;

use crate::mip4_reserve::{create_mip4_precompile, MIP4_RESERVE_ADDRESS};

/// Custom precompile provider for NINE FORK that extends standard Ethereum
/// precompiles with Monad-specific MIP-4 reserve balance precompile.
///
/// Delegates all standard precompile addresses (0x01-0x13, KZG, BLS12-381,
/// P256VERIFY) to `EthPrecompiles`, and handles address 0x20 (MIP-4) directly.
#[derive(Debug, Clone)]
pub struct NineForkPrecompiles {
    /// Inner Ethereum precompiles (standard OSAKA set).
    eth: EthPrecompiles,
    /// MIP-4 precompile instance (cached, not recreated per call).
    mip4: revm::precompile::Precompile,
}

impl NineForkPrecompiles {
    /// Creates a new NINE FORK precompile provider with OSAKA spec defaults.
    pub fn new() -> Self {
        Self {
            eth: EthPrecompiles::new(SpecId::OSAKA),
            mip4: create_mip4_precompile(),
        }
    }

    /// Creates a new NINE FORK precompile provider with a specific spec ID.
    pub fn with_spec(spec: SpecId) -> Self {
        Self {
            eth: EthPrecompiles::new(spec),
            mip4: create_mip4_precompile(),
        }
    }
}

impl Default for NineForkPrecompiles {
    fn default() -> Self {
        Self::new()
    }
}

impl<CTX: ContextTr> PrecompileProvider<CTX> for NineForkPrecompiles {
    type Output = InterpreterResult;

    fn set_spec(&mut self, spec: <CTX::Cfg as Cfg>::Spec) -> bool {
        // Delegate spec changes to the inner EthPrecompiles
        <EthPrecompiles as PrecompileProvider<CTX>>::set_spec(&mut self.eth, spec)
    }

    fn run(
        &mut self,
        context: &mut CTX,
        inputs: &revm::interpreter::CallInputs,
    ) -> Result<Option<Self::Output>, String> {
        // Check if this is our MIP-4 precompile address
        if inputs.bytecode_address == MIP4_RESERVE_ADDRESS {
            let mut result = InterpreterResult {
                result: InstructionResult::Return,
                gas: Gas::new(inputs.gas_limit),
                output: Bytes::new(),
            };

            // Extract input bytes from CallInput (mirrors EthPrecompiles::run pattern)
            let exec_result = {
                let r: std::cell::Ref<'_, [u8]>;
                let input_bytes: &[u8] = match &inputs.input {
                    CallInput::SharedBuffer(range) => {
                        if let Some(slice) =
                            context.local().shared_memory_buffer_slice(range.clone())
                        {
                            r = slice;
                            r.as_ref()
                        } else {
                            &[]
                        }
                    }
                    CallInput::Bytes(bytes) => bytes.0.iter().as_slice(),
                };
                self.mip4.execute(input_bytes, inputs.gas_limit)
            };

            match exec_result {
                Ok(output) => {
                    result.gas.record_refund(output.gas_refunded);
                    let underflow = result.gas.record_cost(output.gas_used);
                    assert!(underflow, "Gas underflow is not possible");
                    result.result = if output.reverted {
                        InstructionResult::Revert
                    } else {
                        InstructionResult::Return
                    };
                    result.output = output.bytes;
                }
                Err(PrecompileError::Fatal(e)) => return Err(e),
                Err(e) => {
                    result.result = if e.is_oog() {
                        InstructionResult::PrecompileOOG
                    } else {
                        InstructionResult::PrecompileError
                    };
                }
            }
            return Ok(Some(result));
        }

        // Delegate to standard Ethereum precompiles
        self.eth.run(context, inputs)
    }

    fn warm_addresses(&self) -> Box<impl Iterator<Item = Address>> {
        // Include both standard warm addresses and MIP-4 address
        let eth_addrs: Vec<Address> = self.eth.precompiles.addresses().cloned().collect();
        Box::new(eth_addrs.into_iter().chain(std::iter::once(MIP4_RESERVE_ADDRESS)))
    }

    fn contains(&self, address: &Address) -> bool {
        *address == MIP4_RESERVE_ADDRESS || self.eth.contains(address)
    }
}

/// Executes a transaction using an EVM configured with NINE FORK precompiles
/// (standard OSAKA + MIP-4 at address 0x20).
///
/// This mirrors `EvmExecutor::execute_tx_with_state_changes()` but uses
/// `NineForkPrecompiles` instead of `EthPrecompiles`, enabling tests to
/// call the MIP-4 reserve balance precompile via STATICCALL in EVM bytecode.
///
/// Returns the execution result and state changes, matching the EvmExecutor API.
pub fn execute_with_nine_fork_precompiles(
    tx: &monad_types::Transaction,
    state: &dyn monad_state::StateProvider,
    block_env: &monad_types::BlockEnv,
) -> Result<(monad_types::ExecutionResult, monad_evm::executor::StateChanges), monad_types::EvmError> {
    use monad_evm::db_bridge::DbBridge;
    use monad_types::{
        AccountInfo as MonadAccountInfo, EvmError,
        ExecutionResult as MonadExecutionResult,
        Log as MonadLog, U256,
    };
    use revm::{
        context::{Context, TxEnv},
        context_interface::result::{ExecutionResult as RevmExecutionResult, Output},
        primitives::TxKind,
        ExecuteEvm, MainBuilder, MainContext,
    };
    use std::collections::HashMap;

    let db = DbBridge::new(state);

    let basefee: u64 = block_env.base_fee.try_into().unwrap_or(u64::MAX);

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
            cfg.disable_eip3607 = true;
            cfg.disable_base_fee = true;
            cfg.disable_fee_charge = true;
        });

    // Build mainnet EVM then swap in NINE FORK precompiles
    let mut evm = ctx.build_mainnet().with_precompiles(NineForkPrecompiles::new());

    let tx_kind = match tx.to {
        Some(addr) => TxKind::Call(addr),
        None => TxKind::Create,
    };

    let gas_price: u128 = tx.gas_price.try_into().unwrap_or(u128::MAX);

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

    let result = evm
        .transact_one(tx_env)
        .map_err(|e| EvmError::Internal(format!("revm execution error: {:?}", e)))?;

    // Convert revm result to our types
    let monad_result = match &result {
        RevmExecutionResult::Success { gas, output, logs, .. } => {
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
    };

    // Finalize to get state changes
    let state_diffs = evm.finalize();
    let mut mapped = HashMap::new();
    for (address, account) in state_diffs {
        let acct_info = MonadAccountInfo {
            balance: account.info.balance,
            nonce: account.info.nonce,
            code_hash: account.info.code_hash,
            code: account.info.code.map(|c| c.original_bytes()),
        };
        let mut storage_changes = HashMap::new();
        for (slot, value) in account.storage {
            storage_changes.insert(slot, value.present_value());
        }
        mapped.insert(address, (acct_info, storage_changes));
    }

    Ok((monad_result, mapped))
}
