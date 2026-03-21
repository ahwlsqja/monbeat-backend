//! Block-STM scheduler types for task dispatch and per-transaction state tracking.
//!
//! These types define the scheduler's view of each transaction: what task to
//! dispatch next (`SchedulerTask`), the current status of each transaction
//! (`TxStatus`), and the per-transaction state including incarnation and
//! cached read/write sets (`TxState`).

pub use monad_mv_state::types::{Incarnation, TxIndex};
use monad_mv_state::read_write_sets::{ReadSet, WriteSet};

/// A task dispatched by the scheduler to a worker thread.
///
/// Workers call `Scheduler::next_task()` to claim work. The scheduler
/// enforces validation priority over execution: if any transaction needs
/// validation, that is dispatched before new executions.
#[derive(Debug, PartialEq, Eq)]
pub enum SchedulerTask {
    /// Execute transaction `tx_index` at the given incarnation.
    /// Workers build an MvDatabase, run revm, and report results.
    Execute(TxIndex, Incarnation),
    /// Validate the read-set of transaction `tx_index` against the current
    /// MVHashMap state. If validation fails, the scheduler will abort and
    /// re-execute the transaction.
    Validate(TxIndex),
    /// No more tasks available — the block is fully processed.
    /// Workers should exit their loop upon receiving this.
    Done,
}

/// The execution lifecycle status of a single transaction.
///
/// Transitions follow the Block-STM state machine:
/// ```text
/// ReadyToExecute → Executing → Executed → Validating → Validated
///                                                    ↘ Aborting → ReadyToExecute (re-execute)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxStatus {
    /// Transaction is queued for execution (initial state, or after abort).
    ReadyToExecute,
    /// Transaction is currently being executed by a worker.
    Executing,
    /// Execution completed; read/write sets are stored, awaiting validation.
    Executed,
    /// Transaction's read-set is currently being validated.
    Validating,
    /// Validation passed — this transaction's results are final.
    Validated,
    /// Validation failed — transaction will be re-executed with incremented incarnation.
    Aborting,
}

/// Per-transaction state tracked by the scheduler.
///
/// Each transaction in the block has one `TxState`. The scheduler uses it to
/// track lifecycle status, incarnation count, and cached read/write sets
/// between execution and validation phases.
pub struct TxState {
    /// Current lifecycle status of this transaction.
    pub status: TxStatus,
    /// Incarnation counter — incremented on each re-execution after validation failure.
    /// Starts at 0 for the first execution.
    pub incarnation: Incarnation,
    /// Read-set recorded during the most recent execution, used for validation.
    pub read_set: Option<ReadSet>,
    /// Write-set recorded during the most recent execution, applied to MVHashMap.
    pub write_set: Option<WriteSet>,
}

impl TxState {
    /// Create a new TxState in the initial `ReadyToExecute` state.
    pub fn new() -> Self {
        Self {
            status: TxStatus::ReadyToExecute,
            incarnation: 0,
            read_set: None,
            write_set: None,
        }
    }
}

impl Default for TxState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tx_state_default_is_ready_to_execute() {
        let state = TxState::new();
        assert_eq!(state.status, TxStatus::ReadyToExecute);
        assert_eq!(state.incarnation, 0);
        assert!(state.read_set.is_none());
        assert!(state.write_set.is_none());
    }

    #[test]
    fn scheduler_task_equality() {
        assert_eq!(SchedulerTask::Execute(0, 0), SchedulerTask::Execute(0, 0));
        assert_ne!(SchedulerTask::Execute(0, 0), SchedulerTask::Execute(1, 0));
        assert_ne!(SchedulerTask::Execute(0, 0), SchedulerTask::Validate(0));
        assert_eq!(SchedulerTask::Done, SchedulerTask::Done);
    }

    #[test]
    fn tx_status_copy_semantics() {
        let s = TxStatus::Executing;
        let s2 = s; // Copy
        assert_eq!(s, s2);
    }
}
