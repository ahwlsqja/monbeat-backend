# S04: Engine Bridge + Vibe-Score — Research

**Date:** 2026-03-22
**Depth:** Deep — novel integration pipeline (NestJS → Rust CLI subprocess), no CLI binary exists yet, core project differentiator

## Summary

S04 must deliver two interconnected capabilities: (1) a Rust CLI binary that wraps the monad-core parallel executor and accepts JSON stdin/stdout, and (2) NestJS services (EngineModule, VibeScoreModule) that compile Solidity, construct transaction blocks, invoke the CLI, and compute a vibe-score from execution results.

**No CLI binary exists.** The monad-core workspace is a pure library (`[workspace]` with 7 crates, zero `[[bin]]` targets). A new crate (e.g., `crates/cli/`) must be created with a `main.rs` that deserializes JSON input into `Vec<Transaction>` + `BlockEnv`, runs `execute_block_parallel()` from `monad-scheduler`, and serializes the results (including per-tx incarnation counts for conflict detection) back to JSON stdout.

The critical design question is **how to construct a meaningful transaction block from compiled bytecode**. A single deploy tx (to=None, data=bytecode) doesn't create storage conflicts. To produce a differentiating vibe-score, we need to:
1. Deploy the contract (tx0: CREATE with init code)
2. Generate multiple function call txs that interact with the deployed contract's storage (tx1..txN: CALL with encoded function selectors)

The NestJS `VibeScoreService` must use ethers.js `Interface.encodeFunctionData()` to encode function calls based on the ABI, then construct a block of N parallel call transactions. ParallelConflict's `increment()` calls will conflict on the global `counter` slot, causing re-executions (incarnation > 0), while FixedContract's `store(value)` calls from different senders will only conflict if they write the same slot.

## Recommendation

**Two-phase approach: Rust CLI first, then NestJS bridge.**

1. **Create `crates/cli/` Rust binary** — reads JSON `{ transactions: [...], block_env: {...} }` from stdin, calls `execute_block_parallel()`, outputs JSON `{ results: [...], stats: { conflicts, reExecutions, totalGas } }` to stdout. The CLI must also expose per-tx `incarnation` from TxState (incarnation > 0 = re-execution after conflict). This requires modifying `execute_block_parallel()` to return incarnation data, OR reading it from the Scheduler before `collect_results()` consumes the state.

2. **NestJS EngineService** — `spawnSync` the CLI binary, pipe JSON, parse output. Graceful degradation if binary missing.

3. **NestJS VibeScoreService** — orchestrates: compile → construct block (deploy tx + N call txs) → engine execute → score calculation. Score formula: `100 - (conflictPenalty + reExecutionPenalty + gasInefficiencyPenalty)`.

4. **Transaction Block Construction Strategy**:
   - Deploy tx: `{ sender, to: null, data: bytecode, gas_limit: 2_000_000, nonce: 0 }`
   - Call txs: For each callable function in ABI (non-view, non-pure), encode N calls from N different senders. ParallelConflict `increment()` from 8 senders = 8 conflicting txs on slot 0. FixedContract `store(i)` from 8 senders = 8 txs writing different values to the same slot (still conflicts, but shows the pattern).
   - Better differentiation: FixedContract calls from different senders with `store(senderIndex)` all write to the SAME `storedValue` slot — this actually will also conflict. The real differentiation should be: a mapping-based contract (each sender writes to `mapping[sender]` = different slots = no conflicts) vs ParallelConflict (all write to the same `counter` slot).

   **However**, for S04 scope, we can use a simpler approach: compare incarnation counts between ParallelConflict.increment() and FixedContract.store(). Even though both access a single slot, ParallelConflict reads-then-writes (`counter = counter + 1`) creating a read-write dependency, while FixedContract does a blind write (`storedValue = _value`), which may differ in conflict patterns. If they produce similar scores, that's still valid — the engine IS doing real parallel execution. The scoring can be refined later.

## Implementation Landscape

### Key Files

**Rust CLI (new):**
- `crates/cli/Cargo.toml` — new binary crate depending on `monad-types`, `monad-state`, `monad-scheduler`, `monad-evm`, `serde_json`
- `crates/cli/src/main.rs` — JSON stdin → `execute_block_parallel()` → JSON stdout
- `Cargo.toml` (workspace) — add `crates/cli` to `members`

**Rust Engine (existing, may need small changes):**
- `crates/scheduler/src/parallel_executor.rs` — `execute_block_parallel()` returns `ParallelExecutionResult` with `tx_results` and `beneficiary_tracker`. **Does NOT return per-tx incarnation.** Need to add incarnation data to the output, OR create a wrapper in the CLI that reads TxState incarnations before `collect_results()` consumes them.
- `crates/scheduler/src/coordinator.rs` — `Scheduler.get_tx_state(tx_idx)` returns `MutexGuard<TxState>` with `incarnation` field. The CLI can read these before calling `collect_results()`.
- `crates/evm/src/tracer.rs` — `FailureTracer` produces `TraceResult` with JSON serialization. EngineService should include trace data for failed txs.
- `crates/types/src/transaction.rs` — `Transaction` already derives `Serialize, Deserialize`
- `crates/types/src/block.rs` — `BlockEnv` already derives `Serialize, Deserialize`
- `crates/types/src/result.rs` — `ExecutionResult` does NOT derive `Serialize, Deserialize` (only `Debug, Clone, PartialEq, Eq`). Must add `Serialize, Deserialize` to `ExecutionResult`, `BlockResult`, and their fields.

**NestJS (new modules):**
- `backend/src/engine/engine.service.ts` — subprocess management, JSON piping, timeout handling
- `backend/src/engine/engine.module.ts` — EngineModule with EngineService
- `backend/src/vibe-score/vibe-score.service.ts` — compile → block construction → engine → scoring
- `backend/src/vibe-score/vibe-score.controller.ts` — POST /api/vibe-score
- `backend/src/vibe-score/vibe-score.module.ts` — VibeScoreModule importing ContractsModule, EngineModule
- `backend/src/vibe-score/dto/vibe-score-request.dto.ts` — { source: string, functionNames?: string[] }
- `backend/src/vibe-score/dto/vibe-score-result.dto.ts` — { vibeScore, conflicts, reExecutions, gasEfficiency, traceResults, suggestions }

**Existing NestJS (to be consumed):**
- `backend/src/contracts/compile.service.ts` — `CompileService.compile(source)` → `{ contractName, abi, bytecode }`
- `backend/src/contracts/contracts.module.ts` — exports `CompileService`
- `backend/src/config/configuration.ts` — needs `engine.binaryPath` key (ENGINE_BINARY_PATH already in .env.example)
- `backend/src/app.module.ts` — needs EngineModule + VibeScoreModule imports
- `backend/prisma/schema.prisma` — `VibeScore` model already exists with `engineBased`, `conflicts`, `reExecutions`, `gasEfficiency` fields

### Build Order

**Phase 1: Rust CLI binary (riskiest — proves engine works end-to-end)**
1. Add `Serialize, Deserialize` derives to `ExecutionResult` in `crates/types/src/result.rs`
2. Create `crates/cli/` with `Cargo.toml` and `main.rs`
3. `main.rs` flow: read JSON stdin → deserialize → create `InMemoryState` with funded accounts → `execute_block_parallel()` → read incarnations from `Scheduler` → serialize results → stdout
4. Verify: `echo '{"transactions":[...], "block_env":{...}}' | cargo run -p monad-cli` produces valid JSON output
5. **Key design for incarnation tracking:** The `Scheduler` is consumed inside `execute_block_parallel()`. To get incarnation data, either: (a) modify `ParallelExecutionResult` to include `Vec<u32>` incarnations, or (b) create a wrapper function in the CLI that directly uses `Scheduler` and `crossbeam::scope`. Option (a) is cleaner — add an `incarnations: Vec<u32>` field to `ParallelExecutionResult` and populate it in `collect_results()`.

**Phase 2: NestJS EngineService**
1. Add `engine.binaryPath` to `configuration.ts`
2. Create `EngineService` with `executeBlock(transactions, blockEnv)` method using `child_process.spawnSync`
3. Test with mocked subprocess

**Phase 3: NestJS VibeScoreService**
1. Create `VibeScoreService` that uses `CompileService` + `EngineService`
2. Transaction block construction logic (deploy + call txs)
3. Score calculation from engine results
4. Heuristic fallback when engine is unavailable (port existing `calculateMonadParallelismScore` from Vibe-Loom)

**Phase 4: Controller + Module wiring + Tests**
1. `VibeScoreController` with `POST /api/vibe-score`
2. Wire into AppModule
3. Unit tests for all services

### Verification Approach

1. **Rust CLI integration test:** `cd crates/cli && cargo test` — create a test that constructs 4 conflicting txs (same sender, sequential nonces), runs the CLI, asserts incarnation > 0 for at least one tx
2. **Rust CLI manual smoke test:** pipe JSON with 2 independent transfers → verify both succeed with incarnation 0
3. **NestJS unit tests:** EngineService (mocked subprocess), VibeScoreService (mocked EngineService + CompileService), VibeScoreController
4. **NestJS build:** `cd backend && npm run build` — zero errors
5. **NestJS test suite:** `cd backend && npm test` — all tests pass (existing 34 + new ~15-20)
6. **Differentiation proof:** ParallelConflict.sol → engine → `conflicts > 0, vibeScore < 50` vs FixedContract.sol → engine → `vibeScore > 50` (exact thresholds TBD based on actual engine behavior)

## Constraints

- **No crossbeam in WASM** — the engine must run as a native binary, not WASM. Decision D002 confirms subprocess approach.
- **CLI must be stateless** — each invocation creates a fresh `InMemoryState` with funded test accounts. No persistent state between runs.
- **`ExecutionResult` lacks Serialize/Deserialize** — must be added to `crates/types/src/result.rs`. This is a breaking change for any downstream that pattern-matches on the derive list, but since it's adding derives (not removing), it's backwards-compatible.
- **`Scheduler` consumes TxState on `collect_results()`** — incarnation data must be extracted before or during `collect_results()`. Best approach: modify `collect_results()` to also return incarnation counts.
- **ethers.js v6 for ABI encoding** — `new ethers.Interface(abi).encodeFunctionData('functionName', args)` produces the calldata for call transactions. Already installed in backend.

## Common Pitfalls

- **Deploy tx doesn't create conflicts** — a single CREATE tx can't show parallel execution behavior. Must generate CALL txs post-deploy to demonstrate conflicts. The CLI needs pre-funded sender accounts and the deployed contract address must be computed (CREATE address = keccak256(sender, nonce)).
- **Contract address computation** — after deploy tx, the contract address is `keccak256(rlp([sender, nonce]))[12:]`. The Rust engine handles this internally (revm computes it), but the CLI's output needs to include the deployed address so subsequent call txs know where to send. Alternative: use a two-pass approach — deploy first, extract address from result, then construct call block. Or compute the address client-side in NestJS using `ethers.getCreateAddress({ from: sender, nonce: 0 })`.
- **Sender funding** — all test senders in the CLI's InMemoryState need sufficient balance for gas. Use high balances (100 ETH each) to avoid OutOfGas issues.
- **spawnSync timeout** — the parallel executor on a complex block might take time. Set a reasonable timeout (30s) in EngineService and handle timeout as a service-unavailable error.
- **Empty ABI functions** — if the contract only has view/pure functions, there are no state-changing calls to simulate. VibeScoreService should filter ABI for non-view, non-pure functions.
- **Incarnation interpretation** — incarnation 0 = first execution (no conflict). incarnation 1 = re-executed once after conflict. Higher incarnation = more conflicts. The vibe-score should penalize based on total re-executions across all txs.

## Open Risks

- **Score differentiation may be weak** — both ParallelConflict and FixedContract write to a single storage slot. The difference is read-then-write (counter += 1) vs blind-write (storedValue = _value), but revm's OCC may treat both as conflicts since they touch the same storage location. In that case, the scores would be similar, and the "differentiation proof" would need a mapping-based contract that uses per-sender slots. **Mitigation:** test with the actual engine first, then adjust contract fixtures or scoring formula.
- **Rust compilation time** — adding a new binary crate increases Cargo build time. For S06 Railway Docker, the multi-stage build will need to compile the CLI. This is a known cost.
- **CLI binary path in different environments** — dev: `../../target/debug/monad-cli`, CI: build from source, Railway: multi-stage Docker build copies the binary. The ENGINE_BINARY_PATH env var handles this.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Solidity ABI encoding for call txs | `ethers.Interface.encodeFunctionData()` | Already installed (ethers@^6.16.0), handles all Solidity types |
| CREATE address computation | `ethers.getCreateAddress({ from, nonce })` | Deterministic, matches revm's internal calculation |
| JSON stdin/stdout for CLI | `serde_json` (already in monad-evm deps) | Standard Rust serialization |
| Heuristic fallback scoring | Vibe-Loom `calculateMonadParallelismScore()` | Already proven regex-based analysis, port to NestJS |

## Sources

- monad-core `crates/types/src/transaction.rs` — Transaction struct with `Serialize, Deserialize` already derived
- monad-core `crates/types/src/result.rs` — ExecutionResult **missing** `Serialize, Deserialize` derives (must add)
- monad-core `crates/scheduler/src/parallel_executor.rs` — `execute_block_parallel()` API and `ParallelExecutionResult` struct
- monad-core `crates/scheduler/src/coordinator.rs` — `Scheduler.get_tx_state()` for reading incarnation counts
- monad-core `crates/evm/src/tracer.rs` — `FailureTracer` and `TraceResult` with `to_json()` support
- monad-core `crates/evm/tests/parallel_execution.rs` — differential test harness showing how to set up InMemoryState, run parallel vs sequential, assert state equality
- Vibe-Loom `src/lib/optimizer.ts` — existing regex-based scoring (heuristic fallback source)
- NestJS backend `src/contracts/compile.service.ts` — CompileService.compile() returns { contractName, abi, bytecode }
