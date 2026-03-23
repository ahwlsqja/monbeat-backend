---
id: T01
parent: S04
milestone: M001
provides:
  - monad-cli binary with JSON stdin/stdout pipeline
  - Serialize/Deserialize derives on ExecutionResult and BlockResult
  - Incarnation tracking in ParallelExecutionResult
  - Pre-funded test state factory (16 accounts + coinbase)
key_files:
  - crates/cli/src/main.rs
  - crates/cli/Cargo.toml
  - crates/types/src/result.rs
  - crates/scheduler/src/parallel_executor.rs
  - Cargo.toml
key_decisions:
  - Extracted run_pipeline() as a public function for direct test invocation instead of subprocess-based CLI testing
  - Used 16 pre-funded accounts (0xE1..0xF0) with 100 ETH each, matching the test helpers already in parallel_executor.rs
patterns_established:
  - CLI I/O types (CliInput/CliOutput) use serde for JSON serialization; TxResult is a simplified view of ExecutionResult
  - Incarnation counts are read from scheduler state before collect_results() takes ownership
  - Error output goes to stderr as JSON with non-zero exit code
observability_surfaces:
  - CLI stdout JSON includes stats.num_conflicts and stats.num_re_executions for conflict visibility
  - CLI stderr JSON {"error":"..."} on parse/execution failure with exit code 1
  - Per-transaction incarnations array exposes re-execution counts
duration: 12 minutes
verification_result: passed
completed_at: 2026-03-22
blocker_discovered: false
---

# T01: Create Rust CLI binary with incarnation tracking and JSON I/O

**Added monad-cli binary crate with JSON stdin→engine→stdout pipeline, serde derives on result types, and incarnation tracking in ParallelExecutionResult**

## What Happened

Executed all 5 steps from the task plan:

1. Added `Serialize, Deserialize` derives to `ExecutionResult` enum and `BlockResult` struct in `crates/types/src/result.rs`. All inner types already support serde via alloy-primitives features, so no additional work was needed. All 31 existing type tests pass.

2. Added `pub incarnations: Vec<u32>` field to `ParallelExecutionResult` in `crates/scheduler/src/parallel_executor.rs`. The incarnation values are read from `scheduler.get_tx_state(i).incarnation` in a loop BEFORE `collect_results()` is called (since `collect_results()` takes ownership of stored results). Updated the empty block early return to include `incarnations: Vec::new()`. All 24 scheduler tests pass unchanged.

3. Created `crates/cli/Cargo.toml` with binary target `monad-cli` and dependencies on monad-types, monad-state, monad-scheduler, serde, serde_json, and alloy-primitives.

4. Created `crates/cli/src/main.rs` with the full CLI pipeline:
   - `CliInput` (Deserialize): transactions + block_env
   - `CliOutput` (Serialize/Deserialize): results + incarnations + stats
   - `TxResult`: simplified view (success, gas_used, hex output, error, logs_count)
   - `Stats`: total_gas, num_transactions, num_conflicts, num_re_executions
   - `create_prefunded_state()`: 16 accounts (0xE1..0xF0) with 100 ETH + coinbase (0xC0)
   - `run_pipeline()`: public function for direct testing without subprocess
   - `main()`: stdin → parse → pipeline → stdout, with error JSON to stderr on failure
   - 8 integration tests covering independent transfers, empty block, single transfer, JSON roundtrip, prefunded accounts, conflicting same-sender transactions, multiple independent transfers, and JSON parsing

5. Added `"crates/cli"` to workspace `Cargo.toml` members list.

## Verification

All task-level verification checks pass:
- `cargo test -p monad-types`: 31 passed, 0 failed
- `cargo test -p monad-scheduler`: 24 passed, 0 failed
- `cargo build -p monad-cli`: compiles successfully
- `cargo test -p monad-cli`: 8 passed, 0 failed
- End-to-end CLI test via `echo JSON | cargo run -p monad-cli`: produces correct JSON with success=true, gas_used=21000, incarnations=[0]
- Error handling test with invalid JSON input: produces `{"error":"..."}` on stderr, exit code 1

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p monad-types` | 0 | ✅ pass | 4.8s |
| 2 | `cargo test -p monad-scheduler` | 0 | ✅ pass | 4.8s |
| 3 | `cargo build -p monad-cli` | 0 | ✅ pass | 2.7s |
| 4 | `cargo test -p monad-cli` | 0 | ✅ pass | 2.7s |

## Diagnostics

- **Inspect CLI output:** `echo '{"transactions":[],"block_env":{"number":1,"coinbase":"0x00000000000000000000000000000000000000C0","timestamp":1700000000,"gas_limit":30000000,"base_fee":"0x0","difficulty":"0x0"}}' | cargo run --quiet -p monad-cli`
- **Error path:** Send malformed JSON → stderr gets `{"error":"..."}`, exit code 1
- **Conflict detection:** Send two transactions from the same sender → `stats.num_conflicts` may be > 0 and `incarnations` array shows re-execution counts

## Deviations

None — all steps executed as planned.

## Known Issues

None discovered.

## Files Created/Modified

- `crates/types/src/result.rs` — Added `Serialize, Deserialize` derives to `ExecutionResult` and `BlockResult`
- `crates/scheduler/src/parallel_executor.rs` — Added `incarnations: Vec<u32>` field to `ParallelExecutionResult`, populated from scheduler state before result collection
- `crates/cli/Cargo.toml` — New binary crate manifest for monad-cli
- `crates/cli/src/main.rs` — CLI binary with JSON I/O pipeline, pre-funded state factory, and 8 integration tests
- `Cargo.toml` — Added `"crates/cli"` to workspace members
