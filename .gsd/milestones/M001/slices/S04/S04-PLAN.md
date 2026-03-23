# S04: Engine Bridge + Vibe-Score

**Goal:** NestJS VibeScoreModule calls monad-core Rust CLI binary to produce real EVM parallel execution-based vibe-scores, with per-transaction incarnation tracking for conflict detection.
**Demo:** `POST /api/vibe-score` with ParallelConflict.sol source → engine returns conflicts > 0, vibe-score < 80. Same endpoint with FixedContract.sol → different incarnation pattern. When engine binary is unavailable, falls back to heuristic scoring (OptimizerService).

## Must-Haves

- Rust CLI binary (`monad-cli`) reads JSON stdin `{ transactions, block_env }`, runs `execute_block_parallel()`, outputs JSON `{ results, incarnations, stats }` to stdout
- `ExecutionResult` and `BlockResult` derive `Serialize, Deserialize` (currently missing)
- `ParallelExecutionResult` includes per-tx incarnation counts (currently not tracked)
- NestJS `EngineService` spawns CLI subprocess with JSON piping, timeout handling, and graceful degradation
- NestJS `VibeScoreService` orchestrates: CompileService.compile() → ABI-based tx block construction → EngineService.executeBlock() → score calculation from incarnation data
- `POST /api/vibe-score` endpoint accepting Solidity source, returning `{ vibeScore, conflicts, reExecutions, gasEfficiency, engineBased, suggestions }`
- Heuristic fallback via existing `OptimizerService` when engine binary is unavailable
- ≥15 new unit tests across engine and vibe-score modules

## Proof Level

- This slice proves: integration (Rust engine ↔ NestJS subprocess ↔ Solidity compilation pipeline)
- Real runtime required: yes (Rust binary must compile and execute EVM transactions)
- Human/UAT required: no

## Verification

- `cd /home/ahwlsqja/monad-core/.gsd/worktrees/M001 && cargo build -p monad-cli` — Rust CLI binary compiles
- `cd /home/ahwlsqja/monad-core/.gsd/worktrees/M001 && cargo test -p monad-cli` — CLI integration tests pass
- `cd /home/ahwlsqja/monad-core/.gsd/worktrees/M001/backend && npm run build` — NestJS compiles with zero errors
- `cd /home/ahwlsqja/monad-core/.gsd/worktrees/M001/backend && npm test` — all tests pass (existing + ≥15 new)
- `cd /home/ahwlsqja/monad-core/.gsd/worktrees/M001/backend && npm test -- --testPathPattern=engine.service` — EngineService tests pass
- `cd /home/ahwlsqja/monad-core/.gsd/worktrees/M001/backend && npm test -- --testPathPattern=vibe-score` — VibeScore tests pass
- EngineService handles missing binary gracefully (returns null, no crash)
- VibeScoreService falls back to heuristic when engine returns null

## Observability / Diagnostics

- Runtime signals: EngineService logs CLI spawn/completion/timeout/error with duration; VibeScoreService logs compile→engine→score pipeline phases with timing
- Inspection surfaces: `POST /api/vibe-score` response includes `engineBased: boolean` flag distinguishing real vs heuristic scores; VibeScore DB records persist `engineBased`, `conflicts`, `reExecutions`, `gasEfficiency` fields
- Failure visibility: EngineService logs CLI stderr on failure, timeout errors include duration; VibeScoreService logs fallback reason when engine unavailable
- Redaction constraints: none (no secrets in engine pipeline)

## Integration Closure

- Upstream surfaces consumed: `backend/src/contracts/compile.service.ts` (CompileService.compile() → { contractName, abi, bytecode }), `backend/src/analysis/optimizer.service.ts` (OptimizerService.calculateScore() for heuristic fallback), `backend/src/config/configuration.ts` (ENGINE_BINARY_PATH), `crates/scheduler/src/parallel_executor.rs` (execute_block_parallel API)
- New wiring introduced in this slice: EngineModule + VibeScoreModule registered in AppModule; Rust CLI binary added to workspace; `POST /api/vibe-score` HTTP endpoint
- What remains before the milestone is truly usable end-to-end: S05 (frontend integration to call the API), S06 (Railway deployment with Rust binary in Docker)

## Tasks

- [x] **T01: Create Rust CLI binary with incarnation tracking and JSON I/O** `est:1h`
  - Why: No binary target exists in the monad-core workspace. The CLI is the bridge between NestJS and the parallel execution engine. Without it, vibe-score is heuristic-only.
  - Files: `crates/types/src/result.rs`, `crates/scheduler/src/parallel_executor.rs`, `Cargo.toml`, `crates/cli/Cargo.toml`, `crates/cli/src/main.rs`
  - Do: (1) Add `Serialize, Deserialize` derives to `ExecutionResult` and `BlockResult` in result.rs — requires adding `serde` import and derive macros to the enum and its variants (Halt.reason is String, already serializable; Success/Revert output is Bytes which has serde support via alloy-primitives). (2) Add `incarnations: Vec<u32>` field to `ParallelExecutionResult` in parallel_executor.rs. Modify `execute_block_parallel()` to read incarnation from each `TxState` via `scheduler.get_tx_state(i).incarnation` BEFORE calling `collect_results()` (which consumes the state). (3) Create `crates/cli/Cargo.toml` depending on monad-types, monad-state, monad-scheduler, serde_json, serde. (4) Create `crates/cli/src/main.rs` — reads JSON from stdin, deserializes into `CliInput { transactions, block_env }`, creates `InMemoryState` with pre-funded test accounts, calls `execute_block_parallel()`, serializes `CliOutput { results, incarnations, stats }` to JSON stdout. (5) Add `"crates/cli"` to workspace members in root Cargo.toml. Include Rust integration test that constructs conflicting transactions and asserts incarnation > 0.
  - Verify: `cargo build -p monad-cli && cargo test -p monad-cli`
  - Done when: `monad-cli` binary compiles, reads JSON stdin, outputs JSON with incarnation data, and integration test passes

- [x] **T02: Build NestJS EngineService, VibeScoreService, Controller, and module wiring** `est:1h`
  - Why: Creates the full NestJS pipeline from HTTP request to engine execution to scored response. This is where Solidity source becomes a vibe-score.
  - Files: `backend/src/config/configuration.ts`, `backend/src/engine/engine.service.ts`, `backend/src/engine/engine.module.ts`, `backend/src/vibe-score/vibe-score.service.ts`, `backend/src/vibe-score/vibe-score.controller.ts`, `backend/src/vibe-score/vibe-score.module.ts`, `backend/src/vibe-score/dto/vibe-score-request.dto.ts`, `backend/src/vibe-score/dto/vibe-score-result.dto.ts`, `backend/src/app.module.ts`
  - Do: (1) Add `engine: { binaryPath: process.env.ENGINE_BINARY_PATH }` to configuration.ts. (2) Create EngineService with `executeBlock(transactions, blockEnv)` method using `child_process.spawnSync` — pipe JSON to stdin, parse JSON from stdout, 30s timeout, return null on any error (missing binary, timeout, parse failure). (3) Create EngineModule exporting EngineService. (4) Create VibeScoreService that injects CompileService, EngineService, OptimizerService, PrismaService. Main method: compile source → parse ABI for non-view/non-pure functions → compute deploy address via `ethers.getCreateAddress()` → construct tx block (1 deploy tx + N call txs from different senders with ABI-encoded function data) → call EngineService → calculate score from incarnation data (`100 - conflictPenalty - reExecutionPenalty - gasInefficiencyPenalty`) → if engine unavailable, fall back to OptimizerService.calculateScore(). (5) Create VibeScoreController with `POST /api/vibe-score`. (6) Create VibeScoreModule importing ContractsModule, EngineModule, AnalysisModule. (7) Register EngineModule + VibeScoreModule in AppModule.
  - Verify: `cd backend && npm run build` — zero TypeScript errors
  - Done when: `npm run build` succeeds, all new modules are wired into AppModule, endpoint is registered

- [x] **T03: Write unit tests for EngineService, VibeScoreService, and VibeScoreController** `est:45m`
  - Why: Proves the engine bridge, scoring pipeline, and HTTP endpoint work correctly with mocked dependencies. Validates heuristic fallback, timeout handling, and score differentiation.
  - Files: `backend/test/engine.service.spec.ts`, `backend/test/vibe-score.service.spec.ts`, `backend/test/vibe-score.controller.spec.ts`
  - Do: (1) EngineService tests (~5): mock child_process.spawnSync — test successful execution returns parsed JSON, test binary not found returns null, test timeout returns null, test malformed output returns null, test stderr logging. (2) VibeScoreService tests (~8): mock CompileService + EngineService + OptimizerService + PrismaService — test engine-based scoring with mocked high-incarnation output produces low score, test engine-based scoring with zero-incarnation output produces high score, test heuristic fallback when engine returns null, test score calculation formula, test ABI filtering (view/pure excluded), test empty ABI fallback, test DB persistence of results, test error handling for compilation failure. (3) VibeScoreController tests (~4): test POST /api/vibe-score with valid source, test POST with empty source returns 400, test response shape matches DTO, test controller is defined. Ensure total new tests ≥ 15.
  - Verify: `cd backend && npm test` — all existing + new tests pass
  - Done when: `npm test` shows ≥15 new passing tests in engine/vibe-score suites, total test suite green

## Files Likely Touched

- `crates/types/src/result.rs`
- `crates/scheduler/src/parallel_executor.rs`
- `Cargo.toml`
- `crates/cli/Cargo.toml`
- `crates/cli/src/main.rs`
- `backend/src/config/configuration.ts`
- `backend/src/engine/engine.service.ts`
- `backend/src/engine/engine.module.ts`
- `backend/src/vibe-score/vibe-score.service.ts`
- `backend/src/vibe-score/vibe-score.controller.ts`
- `backend/src/vibe-score/vibe-score.module.ts`
- `backend/src/vibe-score/dto/vibe-score-request.dto.ts`
- `backend/src/vibe-score/dto/vibe-score-result.dto.ts`
- `backend/src/app.module.ts`
- `backend/test/engine.service.spec.ts`
- `backend/test/vibe-score.service.spec.ts`
- `backend/test/vibe-score.controller.spec.ts`
