---
id: T03
parent: S04
milestone: M001
provides:
  - 10 EngineService unit tests covering success, null returns, timeout, bad JSON, missing binary
  - 13 VibeScoreService unit tests covering engine path, heuristic fallback, scoring, DB persistence, error handling
  - 5 VibeScoreController unit tests covering endpoint behavior and error propagation
key_files:
  - backend/test/engine.service.spec.ts
  - backend/test/vibe-score.service.spec.ts
  - backend/test/vibe-score.controller.spec.ts
key_decisions:
  - Mocked ethers module at module level for VibeScoreService tests to avoid real ABI encoding in unit tests
  - Used jest.mock('child_process') + jest.mock('fs') for EngineService to test all spawnSync failure paths without real binaries
patterns_established:
  - EngineService test pattern uses mockedSpawnSync to control return values/throws for each failure mode (timeout, non-zero exit, bad JSON, missing binary)
  - VibeScoreService test pattern provides a makeEngineResult() factory with override support for varying conflict/incarnation scenarios
observability_surfaces:
  - Tests verify EngineService returns null (not throws) on all failure modes, confirming graceful degradation
  - Tests verify heuristic fallback activates when engine returns null, with engineBased=false in response
  - Tests verify DB persistence is called with correct field types (String for conflicts/reExecutions/gasEfficiency)
duration: 5 minutes
verification_result: passed
completed_at: 2026-03-22
blocker_discovered: false
---

# T03: Write unit tests for EngineService, VibeScoreService, and VibeScoreController

**Added 28 unit tests across 3 test suites covering engine subprocess bridge, vibe-score scoring pipeline, heuristic fallback, DB persistence, and controller endpoint behavior**

## What Happened

Created three test files with comprehensive mocked unit tests for all new S04 NestJS services and controller:

1. **`engine.service.spec.ts`** (10 tests): Mocks `child_process.spawnSync` and `fs.existsSync`. Tests cover successful JSON parse, empty binary path, undefined binary path, binary file not found on disk, spawnSync ETIMEDOUT, spawnSync throwing exceptions, non-zero exit status, invalid JSON stdout, and correct JSON input piping. All test the contract that EngineService returns null on any failure mode.

2. **`vibe-score.service.spec.ts`** (13 tests): Mocks CompileService, EngineService, OptimizerService, and PrismaService. Tests cover engine-based scoring with engineBased=true, high score (≥80) with no conflicts, lower score when conflicts detected, gasEfficiency calculation with failed txs, heuristic fallback when engine returns null, heuristic fallback for view-only ABI, DB persistence with correct fields, DB failure resilience, compilation error propagation, ABI state-changing function filtering, and score formula edge cases (perfect score, capped penalties).

3. **`vibe-score.controller.spec.ts`** (5 tests): Mocks VibeScoreService. Tests cover controller definition, service call with correct source argument, response shape matching VibeScoreResultDto, error propagation from service, and argument passing behavior.

## Verification

- `npm test -- --testPathPattern=engine.service` — 10 tests pass
- `npm test -- --testPathPattern=vibe-score` — 18 tests pass (13 service + 5 controller)
- `npm test` — all 96 tests pass (68 existing + 28 new), zero regressions
- `cargo build -p monad-cli` — Rust CLI binary compiles
- `cargo test -p monad-cli` — 8 CLI tests pass
- `npm run build` — NestJS compiles with zero errors

All slice-level verification checks pass. This is the final task of S04.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd backend && npm test -- --testPathPattern=engine.service --verbose` | 0 | ✅ pass (10 tests) | 3.5s |
| 2 | `cd backend && npm test -- --testPathPattern=vibe-score --verbose` | 0 | ✅ pass (18 tests) | 7.8s |
| 3 | `cd backend && npm test --verbose` | 0 | ✅ pass (96 tests, 14 suites) | 8.4s |
| 4 | `cd backend && npm run build` | 0 | ✅ pass | 4.2s |
| 5 | `cargo build -p monad-cli` | 0 | ✅ pass | 0.3s |
| 6 | `cargo test -p monad-cli` | 0 | ✅ pass (8 tests) | 0.05s |

## Diagnostics

- **Run engine tests only:** `cd backend && npm test -- --testPathPattern=engine.service --verbose`
- **Run vibe-score tests only:** `cd backend && npm test -- --testPathPattern=vibe-score --verbose`
- **Full test suite:** `cd backend && npm test --verbose`
- **Test count verification:** Output shows `Tests: 96 passed, 96 total` (68 existing + 28 new)

## Deviations

- Plan estimated ≥15 new tests; delivered 28 tests (10+13+5) — exceeded target by providing more thorough coverage of edge cases (undefined binary path, binary not found on disk, DB failure resilience, view-only ABI fallback, score formula boundary conditions).

## Known Issues

None discovered.

## Files Created/Modified

- `backend/test/engine.service.spec.ts` — 10 unit tests for EngineService subprocess bridge (mocked spawnSync/fs)
- `backend/test/vibe-score.service.spec.ts` — 13 unit tests for VibeScoreService pipeline (mocked dependencies)
- `backend/test/vibe-score.controller.spec.ts` — 5 unit tests for VibeScoreController endpoint behavior
