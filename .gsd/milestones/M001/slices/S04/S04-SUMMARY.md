---
id: S04
parent: M001
milestone: M001
provides:
  - monad-cli Rust binary with JSON stdin/stdout pipeline and incarnation tracking
  - Serialize/Deserialize derives on ExecutionResult and BlockResult for JSON serialization
  - NestJS EngineService subprocess bridge with timeout and graceful degradation
  - NestJS VibeScoreService orchestrator (compile → ABI-based block construction → engine → scoring)
  - POST /api/vibe-score endpoint returning real EVM parallel execution-based scores
  - Heuristic fallback via OptimizerService when engine binary unavailable
  - 28 new unit tests (engine + vibe-score) plus 8 Rust CLI integration tests
requires:
  - slice: S01
    provides: PrismaService, ConfigModule, AppModule structure
  - slice: S02
    provides: CompileService.compile() → { contractName, abi, bytecode }
affects:
  - S05 (frontend integration consumes POST /api/vibe-score endpoint)
  - S06 (Railway deployment must include Rust CLI binary in Docker)
key_files:
  - crates/cli/src/main.rs
  - crates/cli/Cargo.toml
  - crates/types/src/result.rs
  - crates/scheduler/src/parallel_executor.rs
  - backend/src/engine/engine.service.ts
  - backend/src/engine/engine.module.ts
  - backend/src/vibe-score/vibe-score.service.ts
  - backend/src/vibe-score/vibe-score.controller.ts
  - backend/src/vibe-score/vibe-score.module.ts
  - backend/src/vibe-score/dto/vibe-score-request.dto.ts
  - backend/src/vibe-score/dto/vibe-score-result.dto.ts
  - backend/src/config/configuration.ts
  - backend/src/app.module.ts
  - backend/test/engine.service.spec.ts
  - backend/test/vibe-score.service.spec.ts
  - backend/test/vibe-score.controller.spec.ts
key_decisions:
  - Incarnation data stored in ParallelExecutionResult (read before collect_results() takes ownership) — no other way to surface it through subprocess boundary
  - EngineService returns null on any failure (never throws) — callers choose fallback behavior
  - Transaction block uses 8 rotating sender addresses (0xE1..0xE8) matching CLI pre-funded accounts
  - Vibe-Score formula with capped penalties (conflicts max 40, re-execution max 30, failure max 20) — minimum score 10
  - DB write failures in VibeScoreService are logged but don't crash the scoring pipeline
  - run_pipeline() extracted as public function for direct Rust test invocation without subprocess overhead
patterns_established:
  - Rust CLI I/O types (CliInput/CliOutput) use serde JSON serialization; NestJS TypeScript interfaces mirror exact same shapes
  - ABI filtering pattern excludes view/pure/constructor functions for state-changing tx block construction
  - Default argument generation for ABI parameter types (uint→0, address→sender, bool→false, string→"", bytes→"0x")
  - Graceful degradation pattern — EngineService null → VibeScoreService heuristic fallback with engineBased=false flag
  - Pre-funded test state factory (16 accounts 0xE1..0xF0 + coinbase 0xC0, 100 ETH each) shared between CLI and test code
observability_surfaces:
  - EngineService logs CLI spawn/completion/timeout/error with duration in ms
  - VibeScoreService logs each pipeline phase (compile, block construction, engine, scoring) with timing
  - POST /api/vibe-score response includes engineBased boolean distinguishing real vs heuristic scores
  - CLI stdout JSON includes stats.num_conflicts and stats.num_re_executions
  - CLI stderr JSON {"error":"..."} on parse/execution failure with exit code 1
  - Per-transaction incarnations array exposes re-execution counts
drill_down_paths:
  - .gsd/milestones/M001/slices/S04/tasks/T01-SUMMARY.md
  - .gsd/milestones/M001/slices/S04/tasks/T02-SUMMARY.md
  - .gsd/milestones/M001/slices/S04/tasks/T03-SUMMARY.md
duration: 25 minutes
verification_result: passed
completed_at: 2026-03-22
---

# S04: Engine Bridge + Vibe-Score

**Bridged monad-core Rust parallel EVM engine to NestJS via CLI subprocess with JSON I/O, delivering real incarnation-based vibe-scores through POST /api/vibe-score with heuristic fallback**

## What Happened

Three tasks built the full pipeline from Solidity source to EVM-execution-based vibe-score:

**T01 (Rust CLI binary)** added serde derives to `ExecutionResult`/`BlockResult` in monad-types, added `incarnations: Vec<u32>` to `ParallelExecutionResult` in monad-scheduler (read from scheduler state before `collect_results()` takes ownership), and created the `monad-cli` binary crate. The CLI reads a JSON `{ transactions, block_env }` from stdin, creates an `InMemoryState` with 16 pre-funded test accounts, calls `execute_block_parallel()`, and outputs `{ results, incarnations, stats }` as JSON to stdout. Errors go to stderr as JSON with exit code 1. 8 integration tests cover independent transfers, conflicting same-sender transactions, empty blocks, and JSON roundtrip.

**T02 (NestJS services + controller)** created `EngineService` (spawns CLI via `spawnSync` with JSON piping, 30s timeout, returns null on any error), `VibeScoreService` (compile → ABI filter → tx block construction → engine execution → score calculation with capped penalties → DB persist → heuristic fallback), `VibeScoreController` (`POST /api/vibe-score`), DTOs, modules, and wired everything into AppModule. The tx block construction computes a deploy address via `ethers.getCreateAddress()`, then generates call txs from 8 rotating senders with ABI-encoded function data for each state-changing function.

**T03 (unit tests)** added 28 tests: 10 for EngineService (mocked spawnSync covering success, timeout, missing binary, bad JSON, non-zero exit), 13 for VibeScoreService (engine path, heuristic fallback, DB persistence, error handling, score formula boundaries), and 5 for VibeScoreController (endpoint behavior, error propagation).

## Verification

All slice-level verification checks passed:

| # | Check | Result |
|---|-------|--------|
| 1 | `cargo build -p monad-cli` | ✅ compiles |
| 2 | `cargo test -p monad-cli` | ✅ 8 passed |
| 3 | `cd backend && npm run build` | ✅ zero TS errors |
| 4 | `cd backend && npm test` | ✅ 96 passed (14 suites) |
| 5 | `npm test -- --testPathPattern=engine.service` | ✅ 10 passed |
| 6 | `npm test -- --testPathPattern=vibe-score` | ✅ 18 passed |

EngineService handles missing binary gracefully (returns null, no crash) — verified by 4 separate test cases. VibeScoreService falls back to heuristic when engine returns null — verified by 2 test cases with engineBased=false in response.

## New Requirements Surfaced

- none

## Deviations

None — all tasks executed as planned. T03 exceeded the ≥15 test target by delivering 28 tests.

## Known Limitations

- **No real cross-contract conflict detection yet** — the tx block construction generates independent function calls from different senders, which may not produce meaningful conflicts for all contract patterns. True conflict detection requires contracts that share global state slots.
- **ENGINE_BINARY_PATH must be configured** — the engine won't activate without the env var pointing to the compiled `monad-cli` binary. In dev, must run `cargo build -p monad-cli` first and set the path.
- **Score formula weights are initial estimates** — conflict penalty (max 40), re-execution penalty (max 30), failure penalty (max 20) are reasonable starting points but not calibrated against real-world contract diversity.

## Follow-ups

- S05 must call `POST /api/vibe-score` from the Next.js frontend and display the score with the `engineBased` flag
- S06 Dockerfile must compile `monad-cli` in a Rust build stage and set `ENGINE_BINARY_PATH` env var pointing to the binary in the final container
- Future iteration: construct multi-contract tx blocks with shared storage slots for more meaningful conflict patterns

## Files Created/Modified

- `crates/types/src/result.rs` — Added `Serialize, Deserialize` derives to `ExecutionResult` and `BlockResult`
- `crates/scheduler/src/parallel_executor.rs` — Added `incarnations: Vec<u32>` to `ParallelExecutionResult`, populated before result collection
- `Cargo.toml` — Added `"crates/cli"` to workspace members
- `crates/cli/Cargo.toml` — New binary crate manifest for monad-cli
- `crates/cli/src/main.rs` — CLI binary with JSON I/O pipeline, pre-funded state factory, 8 integration tests
- `backend/src/config/configuration.ts` — Added `engine.binaryPath` config from `ENGINE_BINARY_PATH` env var
- `backend/src/engine/engine.service.ts` — Subprocess bridge with spawnSync, timeout, error handling
- `backend/src/engine/engine.module.ts` — NestJS module providing/exporting EngineService
- `backend/src/vibe-score/dto/vibe-score-request.dto.ts` — Request DTO with class-validator
- `backend/src/vibe-score/dto/vibe-score-result.dto.ts` — Result interface with scoring fields
- `backend/src/vibe-score/vibe-score.service.ts` — Orchestrator: compile → block → engine → score → DB
- `backend/src/vibe-score/vibe-score.controller.ts` — POST /api/vibe-score controller
- `backend/src/vibe-score/vibe-score.module.ts` — Module importing ContractsModule, EngineModule, AnalysisModule
- `backend/src/app.module.ts` — Added EngineModule and VibeScoreModule imports
- `backend/test/engine.service.spec.ts` — 10 unit tests for EngineService
- `backend/test/vibe-score.service.spec.ts` — 13 unit tests for VibeScoreService
- `backend/test/vibe-score.controller.spec.ts` — 5 unit tests for VibeScoreController

## Forward Intelligence

### What the next slice should know
- `POST /api/vibe-score` accepts `{ source: string }` body and returns `{ vibeScore, conflicts, reExecutions, gasEfficiency, engineBased, suggestions, traceResults }`. The `engineBased` boolean tells the frontend whether the score is real (engine) or heuristic.
- The endpoint works without ENGINE_BINARY_PATH configured — it falls back to OptimizerService heuristic scoring with `engineBased: false`. This means S05 frontend integration can work immediately without the Rust binary.
- For S06 Docker: the binary is at `target/release/monad-cli` after `cargo build --release -p monad-cli`. Set `ENGINE_BINARY_PATH=/app/monad-cli` or wherever it's placed in the container.

### What's fragile
- `ethers.getCreateAddress()` in VibeScoreService computes the deploy address deterministically from sender nonce=0. If the test account addresses or nonce assumptions change, tx block construction will target the wrong contract address.
- The TypeScript interfaces in engine.service.ts (`CliOutput`, `TxResult`, `CliStats`) must exactly match the Rust serde types in `crates/cli/src/main.rs`. Any Rust struct field rename/addition without a matching TS update will cause silent parse failures (EngineService returns null).

### Authoritative diagnostics
- `cargo test -p monad-cli` — if this fails, the Rust engine bridge is broken. 8 tests cover the pipeline end-to-end.
- `npm test -- --testPathPattern=engine.service` — 10 tests verify all EngineService failure modes return null (not throw).
- `npm test -- --testPathPattern=vibe-score` — 18 tests verify scoring pipeline, heuristic fallback, DB persistence.

### What assumptions changed
- No assumptions changed — the plan was accurate. Incarnation tracking worked as expected (read before collect_results), serde derives were straightforward, and the subprocess bridge pattern is reliable.
