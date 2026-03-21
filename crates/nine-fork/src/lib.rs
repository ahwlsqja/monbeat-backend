//! # monad-nine-fork
//!
//! NINE FORK compliance verification for the Monad EVM.
//!
//! This crate implements and verifies compliance with the NINE FORK specification
//! (MIP-3, MIP-4, MIP-5) through the monad-core EVM execution pipeline:
//!
//! - **MIP-3 (Linear Memory Pool):** Verifies that revm's `SharedMemory` with
//!   checkpoint-based allocation satisfies the linear memory pool requirement.
//!   Nested CALL/REVERT sequences must correctly restore memory state.
//!
//! - **MIP-4 (Reserve Balance Precompile):** Custom precompile at address `0x20`
//!   that checks if an account has dipped into its reserve balance. Includes
//!   `dippedIntoReserve` per-transaction flag tracking and init-selfdestruct bypass.
//!
//! - **MIP-5 (CLZ Opcode):** Verifies that revm's OSAKA spec provides the CLZ
//!   (Count Leading Zeros) opcode at `0x1E`. Uses `U256::leading_zeros()` which
//!   maps to hardware `lzcnt` on supported architectures.
//!
//! Additionally verifies EIP-7823 modexp input bounds (1024-byte limit) which is
//! enforced in revm's OSAKA modexp variant.
//!
//! ## Architecture
//!
//! All verification flows through `EvmExecutor::execute_tx_with_state_changes()`,
//! proving that the features work through the complete EVM pipeline (bytecode →
//! interpreter → state changes), not just at the instruction/precompile level.

pub mod mip3_memory;
pub mod mip4_reserve;
pub mod mip5_clz;
pub mod modexp_validate;
pub mod nine_fork_precompiles;

// Placeholder module for subsequent tasks in S02
// pub mod safety;
