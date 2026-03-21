//! Block-STM parallel scheduler for optimistic concurrency control.
//!
//! This crate implements the Block-STM scheduling algorithm that manages
//! parallel transaction execution with OCC (optimistic concurrency control)
//! conflict detection. The coordinator dispatches execution and validation
//! tasks to worker threads, tracks per-transaction status and incarnation
//! counters, and detects completion via a double-collect algorithm.
//!
//! # Components
//!
//! - [`Scheduler`] — the coordinator that dispatches tasks and manages state
//! - [`SchedulerTask`] — the task enum returned by `next_task()`
//! - [`TxStatus`] — per-transaction lifecycle status
//! - [`TxState`] — per-transaction state including read/write sets
//!
//! # Observability
//!
//! - `Scheduler::done()` — check completion status
//! - `Scheduler::get_tx_state()` — inspect per-tx status and incarnation
//! - `Scheduler::block_size()` — number of transactions in the block

pub mod coordinator;
pub mod types;

// Re-exports for ergonomic use.
pub use coordinator::Scheduler;
pub use types::{Incarnation, SchedulerTask, TxIndex, TxState, TxStatus};
