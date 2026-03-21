//! # monad-types
//!
//! Foundational Ethereum/Monad types shared across all monad-core crates.
//!
//! This crate provides the canonical type definitions for transactions, blocks,
//! execution results, receipts, logs, and error types. It uses `alloy-primitives`
//! for Address, U256, B256, and Bytes to avoid type conversion overhead at
//! boundaries with revm.
//!
//! ## Global Allocator
//!
//! This crate sets `mimalloc` as the global allocator for the entire workspace.
//! Any binary or test that links this crate will use mimalloc for all heap
//! allocations, providing ~5x better multi-threaded allocation performance
//! compared to the default system allocator.

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub mod account;
pub mod block;
pub mod error;
pub mod result;
pub mod transaction;

// Re-export core types at crate root for ergonomic access.
pub use account::{AccountInfo, KECCAK_EMPTY};
pub use block::BlockEnv;
pub use error::EvmError;
pub use result::{ExecutionResult, Log, Receipt, BlockResult};
pub use transaction::Transaction;

// Re-export alloy-primitives types used throughout the workspace.
pub use alloy_primitives::{Address, Bytes, B256, U256};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reexports_accessible() {
        // Verify all re-exported types are accessible from crate root.
        let _tx = Transaction {
            sender: Address::ZERO,
            to: None,
            value: U256::ZERO,
            data: Bytes::new(),
            gas_limit: 21_000,
            nonce: 0,
            gas_price: U256::ZERO,
        };
        let _block = BlockEnv::default();
        let _result = ExecutionResult::Success {
            gas_used: 0,
            output: Bytes::new(),
            logs: vec![],
        };
        let _receipt = Receipt {
            success: true,
            cumulative_gas_used: 0,
            logs: vec![],
            contract_address: None,
        };
        let _log = Log {
            address: Address::ZERO,
            topics: vec![],
            data: Bytes::new(),
        };
        let _err = EvmError::Internal("test".to_string());
    }

    #[test]
    fn test_alloy_primitives_reexported() {
        // Ensure alloy-primitives types are available via monad-types.
        let addr = Address::ZERO;
        let val = U256::from(42u64);
        let hash = B256::ZERO;
        let data = Bytes::new();

        assert_eq!(addr, Address::ZERO);
        assert_eq!(val, U256::from(42u64));
        assert_eq!(hash, B256::ZERO);
        assert!(data.is_empty());
    }

    #[test]
    fn test_transaction_create_detection() {
        let create_tx = Transaction {
            sender: Address::ZERO,
            to: None,
            value: U256::ZERO,
            data: Bytes::from(vec![0x60, 0x00]),
            gas_limit: 100_000,
            nonce: 0,
            gas_price: U256::from(1u64),
        };
        assert!(create_tx.is_create());

        let call_tx = Transaction {
            sender: Address::ZERO,
            to: Some(Address::with_last_byte(1)),
            value: U256::from(100u64),
            data: Bytes::new(),
            gas_limit: 21_000,
            nonce: 1,
            gas_price: U256::from(1u64),
        };
        assert!(!call_tx.is_create());
    }

    #[test]
    fn test_execution_result_variants() {
        let success = ExecutionResult::Success {
            gas_used: 21_000,
            output: Bytes::new(),
            logs: vec![],
        };
        assert!(success.is_success());
        assert_eq!(success.gas_used(), 21_000);

        let revert = ExecutionResult::Revert {
            gas_used: 50_000,
            output: Bytes::from(vec![0x08]),
        };
        assert!(!revert.is_success());

        let halt = ExecutionResult::Halt {
            gas_used: 30_000_000,
            reason: "OutOfGas".to_string(),
        };
        assert!(!halt.is_success());
    }

    #[test]
    fn test_block_env_has_sane_defaults() {
        let env = BlockEnv::default();
        assert!(env.gas_limit > 0, "default gas limit should be non-zero");
        assert_eq!(env.number, 0);
    }

    #[test]
    fn test_error_display_messages() {
        let errors: Vec<EvmError> = vec![
            EvmError::TransactionValidation("nonce mismatch".into()),
            EvmError::StateAccess("db error".into()),
            EvmError::Internal("panic".into()),
            EvmError::Precompile {
                address: "0x01".into(),
                reason: "bad input".into(),
            },
            EvmError::InvalidBlockEnv("negative timestamp".into()),
        ];
        // All errors should produce non-empty display strings.
        for err in &errors {
            let msg = err.to_string();
            assert!(!msg.is_empty(), "error display should not be empty");
        }
    }
}
