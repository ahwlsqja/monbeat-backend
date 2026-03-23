---
estimated_steps: 5
estimated_files: 5
---

# T01: Create Rust CLI binary with incarnation tracking and JSON I/O

**Slice:** S04 — Engine Bridge + Vibe-Score
**Milestone:** M001

## Description

Create the `monad-cli` Rust binary that bridges the parallel execution engine to NestJS. No binary target exists in the monad-core workspace — only library crates. This task adds serialization support to existing types, adds incarnation tracking to `ParallelExecutionResult`, creates the CLI crate, and writes an integration test proving conflict detection works end-to-end. This is the highest-risk task in S04 because it crosses the Rust engine boundary.

**Skills:** None required (pure Rust, no frontend/NestJS skills needed).

## Steps

1. **Add `Serialize, Deserialize` to `ExecutionResult` and `BlockResult`** in `crates/types/src/result.rs`.
   - `ExecutionResult` enum: add `#[derive(Serialize, Deserialize)]` (currently only has `Debug, Clone, PartialEq, Eq`). Add `use serde::{Serialize, Deserialize};` — wait, `serde` is already imported for `Log` and `Receipt`. Just add the derives to `ExecutionResult` and `BlockResult`.
   - `BlockResult` struct: same treatment — add `Serialize, Deserialize` derives.
   - All inner types (`Bytes`, `Address`, `B256`, `Vec<Log>`, `String`, `u64`) already implement serde via alloy-primitives features. `Log` already derives `Serialize, Deserialize`.
   - Verify: `cargo test -p monad-types` passes.

2. **Add incarnation tracking to `ParallelExecutionResult`** in `crates/scheduler/src/parallel_executor.rs`.
   - Add field `pub incarnations: Vec<u32>` to `ParallelExecutionResult`.
   - In `execute_block_parallel()`, BEFORE calling `scheduler.collect_results()`, iterate over `0..block_size` and read each `scheduler.get_tx_state(i as u32).incarnation` into a `Vec<u32>`. This must happen before `collect_results()` because `collect_results()` takes ownership of `result` and `write_set` from each TxState.
   - Populate the `incarnations` field in the returned `ParallelExecutionResult`.
   - Update the empty block early return to include `incarnations: Vec::new()`.
   - Verify: `cargo test -p monad-scheduler` passes (existing tests should still work — they don't check the incarnations field).

3. **Create `crates/cli/Cargo.toml`** with binary target:
   ```toml
   [package]
   name = "monad-cli"
   version = "0.1.0"
   edition = "2021"
   
   [[bin]]
   name = "monad-cli"
   path = "src/main.rs"
   
   [dependencies]
   monad-types = { path = "../types" }
   monad-state = { path = "../state" }
   monad-scheduler = { path = "../scheduler" }
   serde = { workspace = true }
   serde_json = "1.0"
   alloy-primitives = { workspace = true }
   ```

4. **Create `crates/cli/src/main.rs`** implementing the JSON stdin → engine → JSON stdout pipeline:
   - Define `CliInput { transactions: Vec<Transaction>, block_env: BlockEnv }` with `Deserialize`.
   - Define `CliOutput { results: Vec<TxResult>, incarnations: Vec<u32>, stats: Stats }` with `Serialize`.
   - `TxResult` — simplified serializable view of `ExecutionResult`: `{ success: bool, gas_used: u64, output: String (hex), error: Option<String>, logs_count: usize }`.
   - `Stats` — `{ total_gas: u64, num_transactions: usize, num_conflicts: usize, num_re_executions: usize }`.
   - `main()`: read stdin → `serde_json::from_reader::<CliInput>(stdin)` → create `InMemoryState` with 16 pre-funded test accounts (addresses 0xE1..0xF0, each with 100 ETH, nonce 0) plus a coinbase (0xC0) → `execute_block_parallel(transactions, state, block_env, 4)` → map results to `CliOutput` → `serde_json::to_writer(stdout, &output)`.
   - Compute `num_conflicts` = count of incarnations > 0. `num_re_executions` = sum of all incarnations.
   - Handle errors gracefully — if stdin parse fails, output JSON `{ "error": "..." }` to stderr and exit 1.

5. **Add `"crates/cli"` to workspace `Cargo.toml` members** list and write a CLI integration test:
   - In `crates/cli/src/main.rs` (or `crates/cli/tests/cli_test.rs`), write a `#[test]` that:
     - Constructs 2 simple value transfer transactions (independent senders/receivers) as `CliInput`.
     - Serializes to JSON, runs the CLI logic directly (call a helper function, not subprocess), deserializes output.
     - Asserts: 2 results, both success, incarnations are [0, 0] (no conflicts for independent transfers).
   - Optionally: a second test with conflicting transactions (same sender, different nonces) that may produce incarnation > 0.

## Must-Haves

- [ ] `ExecutionResult` and `BlockResult` derive `Serialize, Deserialize`
- [ ] `ParallelExecutionResult` has `incarnations: Vec<u32>` field populated from scheduler state
- [ ] `monad-cli` binary compiles (`cargo build -p monad-cli`)
- [ ] CLI reads JSON from stdin and writes JSON to stdout
- [ ] Pre-funded test accounts (≥8) in InMemoryState so call transactions have gas
- [ ] At least one integration test passes

## Verification

- `cargo test -p monad-types` — existing type tests + serialization still works
- `cargo test -p monad-scheduler` — existing scheduler/parallel tests pass with new incarnations field
- `cargo build -p monad-cli` — binary compiles
- `cargo test -p monad-cli` — CLI integration test passes

## Observability Impact

- Signals added/changed: CLI writes JSON to stdout with `stats.num_conflicts` and `stats.num_re_executions` fields; errors go to stderr as JSON
- How a future agent inspects this: `echo '{"transactions":[],"block_env":{...}}' | cargo run -p monad-cli` and inspect stdout JSON
- Failure state exposed: Non-zero exit code + stderr JSON `{ "error": "..." }` on parse/execution failure

## Inputs

- `crates/types/src/result.rs` — ExecutionResult and BlockResult structs to add serde derives to
- `crates/scheduler/src/parallel_executor.rs` — ParallelExecutionResult to add incarnations field, execute_block_parallel() to modify
- `crates/scheduler/src/coordinator.rs` — Scheduler.get_tx_state() used to read incarnation values
- `crates/types/src/transaction.rs` — Transaction struct (already has Serialize/Deserialize)
- `crates/types/src/block.rs` — BlockEnv struct (already has Serialize/Deserialize)
- `crates/state/src/in_memory.rs` — InMemoryState for creating pre-funded test state
- `Cargo.toml` — workspace members list to add crates/cli

## Expected Output

- `crates/types/src/result.rs` — ExecutionResult and BlockResult now derive Serialize, Deserialize
- `crates/scheduler/src/parallel_executor.rs` — ParallelExecutionResult has incarnations field, populated in execute_block_parallel()
- `crates/cli/Cargo.toml` — new binary crate manifest
- `crates/cli/src/main.rs` — CLI binary with JSON I/O, pre-funded accounts, integration test
- `Cargo.toml` — workspace members includes "crates/cli"
