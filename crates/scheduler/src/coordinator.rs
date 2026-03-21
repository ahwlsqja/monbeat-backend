//! Block-STM coordinator — manages task dispatch, per-transaction status,
//! incarnation tracking, and completion detection.
//!
//! The coordinator is the algorithmic core of Block-STM parallel execution.
//! It maintains two atomic counters (`execution_idx` and `validation_idx`)
//! that workers compete to claim tasks from. Validation always takes priority
//! over execution to maximize the chance of detecting conflicts early.
//!
//! # Completion Detection
//!
//! The `done()` method uses a double-collect algorithm: it reads `decrease_cnt`,
//! checks that both counters are past `block_size` and no tasks are active, then
//! reads `decrease_cnt` again. If both reads match, no in-flight finish operations
//! could have changed the state between the checks, so completion is confirmed.

use std::sync::atomic::{AtomicU32, Ordering};

use parking_lot::{Mutex, MutexGuard};

use monad_mv_state::read_write_sets::{ReadSet, WriteSet};

use crate::types::{Incarnation, SchedulerTask, TxIndex, TxState, TxStatus};

/// Block-STM scheduler coordinator.
///
/// Manages task dispatch for a block of `block_size` transactions. Workers
/// call `next_task()` to claim work, `finish_execution()` after executing,
/// and `finish_validation()` after validating. The scheduler handles conflict
/// resolution by re-queuing transactions for re-execution with incremented
/// incarnation counters.
///
/// All fields use atomic operations (`SeqCst` ordering) for correctness-first
/// design — ordering can be relaxed in future optimization passes.
pub struct Scheduler {
    /// Number of transactions in the block.
    block_size: u32,

    /// Next transaction index to execute. Workers fetch_add to claim.
    /// Once past `block_size`, no more execution tasks are available.
    execution_idx: AtomicU32,

    /// Next transaction index to validate. Workers fetch_add to claim.
    /// Validation takes priority over execution in `next_task()`.
    /// Can be lowered via CAS loop when a validation failure triggers
    /// cascade re-validation of subsequent transactions.
    validation_idx: AtomicU32,

    /// Number of currently in-flight tasks (incremented when claimed,
    /// decremented when finished). Used in completion detection.
    num_active_tasks: AtomicU32,

    /// Monotonically increasing counter of completed task finish calls.
    /// Used by double-collect in `done()` to ensure no finish operation
    /// was in-flight between the two stability checks.
    decrease_cnt: AtomicU32,

    /// Per-transaction state: status, incarnation, and cached read/write sets.
    /// Protected by `Mutex` for safe concurrent access.
    tx_states: Vec<Mutex<TxState>>,
}

impl Scheduler {
    /// Create a new scheduler for a block of `block_size` transactions.
    ///
    /// All transactions start in `ReadyToExecute` state with incarnation 0.
    pub fn new(block_size: usize) -> Self {
        let tx_states = (0..block_size)
            .map(|_| Mutex::new(TxState::new()))
            .collect();

        Self {
            block_size: block_size as u32,
            execution_idx: AtomicU32::new(0),
            validation_idx: AtomicU32::new(0),
            num_active_tasks: AtomicU32::new(0),
            decrease_cnt: AtomicU32::new(0),
            tx_states,
        }
    }

    /// Claim the next task for a worker thread.
    ///
    /// Validation takes priority over execution. If no tasks are available
    /// and the block is complete, returns `SchedulerTask::Done`.
    ///
    /// The method loops internally to handle race conditions where a claimed
    /// index turns out to be out of range or the transaction is not in the
    /// expected state.
    pub fn next_task(&self) -> SchedulerTask {
        loop {
            // Priority 1: Try to claim a validation task.
            let val_idx = self.validation_idx.load(Ordering::SeqCst);
            if val_idx < self.block_size {
                let claimed = self.validation_idx.fetch_add(1, Ordering::SeqCst);
                if claimed < self.block_size {
                    // Check if this tx is ready for validation.
                    let mut state = self.tx_states[claimed as usize].lock();
                    if state.status == TxStatus::Executed {
                        state.status = TxStatus::Validating;
                        self.num_active_tasks.fetch_add(1, Ordering::SeqCst);
                        return SchedulerTask::Validate(claimed);
                    }
                    // Not ready for validation — fall through to try execution.
                }
            }

            // Priority 2: Try to claim an execution task.
            let exec_idx = self.execution_idx.load(Ordering::SeqCst);
            if exec_idx < self.block_size {
                let claimed = self.execution_idx.fetch_add(1, Ordering::SeqCst);
                if claimed < self.block_size {
                    let mut state = self.tx_states[claimed as usize].lock();
                    if state.status == TxStatus::ReadyToExecute {
                        state.status = TxStatus::Executing;
                        let incarnation = state.incarnation;
                        self.num_active_tasks.fetch_add(1, Ordering::SeqCst);
                        return SchedulerTask::Execute(claimed, incarnation);
                    }
                    // Not ready for execution (might have already been claimed).
                }
            }

            // Both counters past block_size — check if done.
            if self.done() {
                return SchedulerTask::Done;
            }

            // Not done but no tasks claimable right now — hint to the CPU
            // and retry. This happens when tasks are in-flight and new
            // validation/execution slots haven't opened yet.
            std::hint::spin_loop();
        }
    }

    /// Record that execution of `tx_index` at `incarnation` completed.
    ///
    /// Stores the read/write sets in the transaction's state for later
    /// validation and publishes the transaction as ready for validation.
    pub fn finish_execution(
        &self,
        tx_index: TxIndex,
        incarnation: Incarnation,
        read_set: ReadSet,
        write_set: WriteSet,
    ) {
        let mut state = self.tx_states[tx_index as usize].lock();

        // Only transition if the incarnation matches (guards against stale finishes).
        if state.incarnation == incarnation {
            state.read_set = Some(read_set);
            state.write_set = Some(write_set);
            state.status = TxStatus::Executed;
        }

        drop(state);

        // Signal that validation should pick up this tx.
        // Lower validation_idx to ensure this tx gets validated.
        self.try_lower_validation_idx(tx_index);

        self.num_active_tasks.fetch_sub(1, Ordering::SeqCst);
        self.decrease_cnt.fetch_add(1, Ordering::SeqCst);
    }

    /// Record that validation of `tx_index` completed.
    ///
    /// If `valid` is true, the transaction is marked as `Validated`.
    /// If `valid` is false, the transaction is aborted: incarnation is
    /// incremented, status reset to `ReadyToExecute`, and `validation_idx`
    /// is lowered via CAS loop to trigger cascade re-validation of
    /// subsequent transactions.
    pub fn finish_validation(&self, tx_index: TxIndex, valid: bool) {
        if valid {
            let mut state = self.tx_states[tx_index as usize].lock();
            state.status = TxStatus::Validated;
            drop(state);
        } else {
            // Abort: increment incarnation, reset for re-execution.
            let mut state = self.tx_states[tx_index as usize].lock();
            state.status = TxStatus::Aborting;
            state.incarnation += 1;
            state.read_set = None;
            state.write_set = None;
            state.status = TxStatus::ReadyToExecute;
            drop(state);

            // Lower execution_idx to re-execute this tx.
            self.try_lower_execution_idx(tx_index);

            // Lower validation_idx so that tx_index+1 and beyond get re-validated.
            // This is the cascade: if tx2 fails, tx3, tx4, ... need re-validation.
            self.try_lower_validation_idx(tx_index + 1);
        }

        self.num_active_tasks.fetch_sub(1, Ordering::SeqCst);
        self.decrease_cnt.fetch_add(1, Ordering::SeqCst);
    }

    /// Atomically lower `validation_idx` to `new_val` if it's currently higher.
    ///
    /// Uses a CAS loop to handle concurrent updates. This is called when a
    /// validation failure requires cascade re-validation starting from a
    /// lower index.
    fn try_lower_validation_idx(&self, new_val: TxIndex) {
        loop {
            let current = self.validation_idx.load(Ordering::SeqCst);
            if current <= new_val {
                return; // Already low enough.
            }
            match self.validation_idx.compare_exchange(
                current,
                new_val,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return,
                Err(_) => continue, // Retry — someone else changed it.
            }
        }
    }

    /// Atomically lower `execution_idx` to `new_val` if it's currently higher.
    ///
    /// Used when a validation failure requires re-execution of a transaction.
    fn try_lower_execution_idx(&self, new_val: TxIndex) {
        loop {
            let current = self.execution_idx.load(Ordering::SeqCst);
            if current <= new_val {
                return;
            }
            match self.execution_idx.compare_exchange(
                current,
                new_val,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return,
                Err(_) => continue,
            }
        }
    }

    /// Check whether all transactions have been validated and no tasks are in-flight.
    ///
    /// Uses the double-collect algorithm: reads `decrease_cnt`, checks the
    /// completion conditions, then reads `decrease_cnt` again. If both reads
    /// match, no `finish_*` call was in-flight between the two checks, so the
    /// completion state is stable.
    pub fn done(&self) -> bool {
        let cnt_before = self.decrease_cnt.load(Ordering::SeqCst);

        let exec_done = self.execution_idx.load(Ordering::SeqCst) >= self.block_size;
        let val_done = self.validation_idx.load(Ordering::SeqCst) >= self.block_size;
        let no_active = self.num_active_tasks.load(Ordering::SeqCst) == 0;

        if !(exec_done && val_done && no_active) {
            return false;
        }

        let cnt_after = self.decrease_cnt.load(Ordering::SeqCst);

        // If decrease_cnt changed between the two reads, a finish operation
        // was in-flight and may have lowered a counter — not safe to conclude done.
        cnt_before == cnt_after
    }

    /// Get mutable access to a transaction's state.
    ///
    /// Returns a `MutexGuard` so workers can read/write the cached read/write
    /// sets and check status.
    pub fn get_tx_state(&self, tx_index: TxIndex) -> MutexGuard<'_, TxState> {
        self.tx_states[tx_index as usize].lock()
    }

    /// Return the number of transactions in this block.
    pub fn block_size(&self) -> usize {
        self.block_size as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monad_mv_state::read_write_sets::{ReadSet, WriteSet};

    /// Helper: create empty read/write sets for finish_execution calls.
    fn empty_sets() -> (ReadSet, WriteSet) {
        (ReadSet::new(), WriteSet::new())
    }

    #[test]
    fn test_initial_state() {
        let scheduler = Scheduler::new(4);
        for i in 0..4u32 {
            let state = scheduler.get_tx_state(i);
            assert_eq!(state.status, TxStatus::ReadyToExecute);
            assert_eq!(state.incarnation, 0);
            assert!(state.read_set.is_none());
            assert!(state.write_set.is_none());
        }
    }

    #[test]
    fn test_execution_dispatch_order() {
        let scheduler = Scheduler::new(3);

        // Single-threaded: next_task should return Execute in order 0, 1, 2.
        assert_eq!(scheduler.next_task(), SchedulerTask::Execute(0, 0));
        assert_eq!(scheduler.next_task(), SchedulerTask::Execute(1, 0));
        assert_eq!(scheduler.next_task(), SchedulerTask::Execute(2, 0));
    }

    #[test]
    fn test_validation_priority() {
        let scheduler = Scheduler::new(3);

        // Execute tx0.
        let task = scheduler.next_task();
        assert_eq!(task, SchedulerTask::Execute(0, 0));

        // Finish execution of tx0 — this should make it available for validation.
        let (rs, ws) = empty_sets();
        scheduler.finish_execution(0, 0, rs, ws);

        // Now next_task should prefer Validate(0) over Execute(1) because
        // validation_idx was lowered to 0 by finish_execution.
        let task = scheduler.next_task();
        assert_eq!(task, SchedulerTask::Validate(0));

        // Now should get Execute(1).
        let task = scheduler.next_task();
        assert_eq!(task, SchedulerTask::Execute(1, 0));
    }

    #[test]
    fn test_completion_detection() {
        let scheduler = Scheduler::new(2);

        // Execute both transactions.
        assert_eq!(scheduler.next_task(), SchedulerTask::Execute(0, 0));
        let (rs, ws) = empty_sets();
        scheduler.finish_execution(0, 0, rs, ws);

        assert_eq!(scheduler.next_task(), SchedulerTask::Validate(0));
        scheduler.finish_validation(0, true);

        assert_eq!(scheduler.next_task(), SchedulerTask::Execute(1, 0));
        let (rs, ws) = empty_sets();
        scheduler.finish_execution(1, 0, rs, ws);

        assert_eq!(scheduler.next_task(), SchedulerTask::Validate(1));
        scheduler.finish_validation(1, true);

        // Both validated — done should be true.
        assert!(scheduler.done());
    }

    #[test]
    fn test_cascade_validation_lowering() {
        let scheduler = Scheduler::new(4);

        // Execute tx0 and immediately validate it (validation priority).
        assert_eq!(scheduler.next_task(), SchedulerTask::Execute(0, 0));
        let (rs, ws) = empty_sets();
        scheduler.finish_execution(0, 0, rs, ws);

        // Validation priority: Validate(0) before Execute(1).
        assert_eq!(scheduler.next_task(), SchedulerTask::Validate(0));
        scheduler.finish_validation(0, true);

        // Execute tx1.
        assert_eq!(scheduler.next_task(), SchedulerTask::Execute(1, 0));
        let (rs, ws) = empty_sets();
        scheduler.finish_execution(1, 0, rs, ws);
        assert_eq!(scheduler.next_task(), SchedulerTask::Validate(1));
        scheduler.finish_validation(1, true);

        // Execute tx2.
        assert_eq!(scheduler.next_task(), SchedulerTask::Execute(2, 0));
        let (rs, ws) = empty_sets();
        scheduler.finish_execution(2, 0, rs, ws);

        // Execute tx3 before validating tx2 (simulate worker contention).
        // To do this, we manually execute tx3 first, then validate.
        assert_eq!(scheduler.next_task(), SchedulerTask::Validate(2));

        // Validate tx2 — FAILS. This should:
        // 1. Increment tx2's incarnation to 1
        // 2. Set tx2 to ReadyToExecute
        // 3. Lower validation_idx to 3 (tx2+1), so tx3 gets re-validated
        // 4. Lower execution_idx to 2, so tx2 gets re-executed
        scheduler.finish_validation(2, false);

        // Verify tx2 was reset.
        {
            let state = scheduler.get_tx_state(2);
            assert_eq!(state.status, TxStatus::ReadyToExecute);
            assert_eq!(state.incarnation, 1);
        }

        // Next task should be re-execution of tx2 at incarnation 1
        // (execution_idx was lowered to 2).
        let task = scheduler.next_task();
        assert_eq!(task, SchedulerTask::Execute(2, 1));

        // Execute tx3 next.
        let task = scheduler.next_task();
        assert_eq!(task, SchedulerTask::Execute(3, 0));
        let (rs, ws) = empty_sets();
        scheduler.finish_execution(3, 0, rs, ws);

        // Re-execute tx2.
        let (rs, ws) = empty_sets();
        scheduler.finish_execution(2, 1, rs, ws);

        // Should validate tx2 (now incarnation 1) — validation priority.
        let task = scheduler.next_task();
        assert_eq!(task, SchedulerTask::Validate(2));
        scheduler.finish_validation(2, true);

        // Should re-validate tx3 due to cascade (validation_idx was lowered to 3).
        let task = scheduler.next_task();
        assert_eq!(task, SchedulerTask::Validate(3));
        scheduler.finish_validation(3, true);

        // All done.
        assert!(scheduler.done());
    }

    #[test]
    fn test_incarnation_increment_on_abort() {
        let scheduler = Scheduler::new(2);

        // Execute tx0.
        assert_eq!(scheduler.next_task(), SchedulerTask::Execute(0, 0));
        let (rs, ws) = empty_sets();
        scheduler.finish_execution(0, 0, rs, ws);

        // Validate tx0 — fails.
        assert_eq!(scheduler.next_task(), SchedulerTask::Validate(0));
        scheduler.finish_validation(0, false);

        // Verify incarnation incremented.
        {
            let state = scheduler.get_tx_state(0);
            assert_eq!(state.incarnation, 1);
            assert_eq!(state.status, TxStatus::ReadyToExecute);
            assert!(state.read_set.is_none(), "read_set should be cleared");
            assert!(state.write_set.is_none(), "write_set should be cleared");
        }

        // Next task should re-execute tx0 at incarnation 1.
        let task = scheduler.next_task();
        assert_eq!(task, SchedulerTask::Execute(0, 1));
    }

    #[test]
    fn test_done_false_while_tasks_active() {
        let scheduler = Scheduler::new(1);

        // Claim the only execution task — num_active_tasks is now 1.
        assert_eq!(scheduler.next_task(), SchedulerTask::Execute(0, 0));

        // Even though execution_idx is past block_size, done() should be
        // false because there's an active task.
        assert!(!scheduler.done());

        // Finish execution.
        let (rs, ws) = empty_sets();
        scheduler.finish_execution(0, 0, rs, ws);

        // Still not done — needs validation.
        assert!(!scheduler.done());

        // Claim and finish validation.
        assert_eq!(scheduler.next_task(), SchedulerTask::Validate(0));
        scheduler.finish_validation(0, true);

        // Now done.
        assert!(scheduler.done());
    }

    #[test]
    fn test_finish_execution_stores_sets() {
        let scheduler = Scheduler::new(1);

        assert_eq!(scheduler.next_task(), SchedulerTask::Execute(0, 0));

        let rs = ReadSet::new();
        let mut ws = WriteSet::new();
        ws.record(
            monad_mv_state::types::LocationKey::Balance(
                alloy_primitives::address!("0x0000000000000000000000000000000000000001"),
            ),
            monad_mv_state::types::WriteValue::Balance(alloy_primitives::U256::from(100)),
        );
        scheduler.finish_execution(0, 0, rs, ws);

        let state = scheduler.get_tx_state(0);
        assert_eq!(state.status, TxStatus::Executed);
        assert!(state.read_set.is_some());
        assert!(state.write_set.is_some());
        assert_eq!(state.write_set.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_block_size_accessor() {
        let scheduler = Scheduler::new(5);
        assert_eq!(scheduler.block_size(), 5);
    }

    #[test]
    fn test_empty_block() {
        let scheduler = Scheduler::new(0);
        // No transactions — should be immediately done.
        assert!(scheduler.done());
        assert_eq!(scheduler.next_task(), SchedulerTask::Done);
    }
}
