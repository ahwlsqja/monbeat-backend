---
id: T02
parent: S04
milestone: M001
provides:
  - EngineService subprocess bridge to Rust CLI binary with timeout/error handling
  - VibeScoreService orchestrator (compile → block construction → engine → scoring pipeline)
  - POST /api/vibe-score endpoint with validation
  - Heuristic fallback via OptimizerService when engine unavailable
  - VibeScoreModule and EngineModule wired into AppModule
key_files:
  - backend/src/engine/engine.service.ts
  - backend/src/engine/engine.module.ts
  - backend/src/vibe-score/vibe-score.service.ts
  - backend/src/vibe-score/vibe-score.controller.ts
  - backend/src/vibe-score/vibe-score.module.ts
  - backend/src/vibe-score/dto/vibe-score-request.dto.ts
  - backend/src/vibe-score/dto/vibe-score-result.dto.ts
  - backend/src/config/configuration.ts
  - backend/src/app.module.ts
key_decisions:
  - EngineService returns null on any failure (missing binary, timeout, parse error) rather than throwing — callers decide fallback behavior
  - Transaction block construction uses 8 rotating sender addresses (0xE1..0xE8) matching CLI pre-funded accounts
  - Deploy address computed client-side via ethers.getCreateAddress() for deterministic tx construction
  - DB write failures in VibeScoreService are logged but don't crash the scoring pipeline
patterns_established:
  - CliOutput/TxResult/CliStats TypeScript interfaces match Rust CliOutput serde types exactly
  - ABI filtering pattern for state-changing functions (exclude view/pure/constructor)
  - Default argument generation for ABI parameter types (uint→0, address→sender, bool→false, etc.)
  - Score formula with capped penalties for conflicts (max 40), re-executions (max 30), failures (max 20)
observability_surfaces:
  - EngineService logs CLI spawn/completion/timeout/error with duration in ms
  - VibeScoreService logs each pipeline phase (compile, block construction, engine, scoring) with timing
  - POST /api/vibe-score response includes engineBased boolean flag
  - VibeScore DB records persist engineBased, conflicts, reExecutions, gasEfficiency fields
duration: 8 minutes
verification_result: passed
completed_at: 2026-03-22
blocker_discovered: false
---

# T02: Build NestJS EngineService, VibeScoreService, Controller, and module wiring

**Added EngineService subprocess bridge, VibeScoreService orchestrator with ABI-based tx block construction and engine-backed scoring, POST /api/vibe-score endpoint, and module wiring into AppModule**

## What Happened

Executed all 7 steps from the task plan:

1. Extended `configuration.ts` with `engine: { binaryPath: process.env.ENGINE_BINARY_PATH || '' }` config entry.

2. Created `EngineService` with `executeBlock(transactions, blockEnv)` method that spawns the Rust CLI binary via `child_process.spawnSync`, pipes JSON to stdin, parses JSON from stdout, and returns null on any error (missing binary, binary not found on disk, timeout, non-zero exit, JSON parse failure). All failure modes log detailed context including duration and stderr content.

3. Created `EngineModule` providing and exporting `EngineService`.

4. Created DTOs: `VibeScoreRequestDto` (class with `@IsString()` + `@IsNotEmpty()` validators) and `VibeScoreResultDto` (interface with vibeScore, conflicts, reExecutions, gasEfficiency, engineBased, suggestions, traceResults fields).

5. Created `VibeScoreService` with the full pipeline:
   - Phase 1: Compile source via CompileService → ABI + bytecode
   - Phase 2: Filter ABI for state-changing functions (exclude view/pure)
   - Phase 3: Construct transaction block — deploy tx (to=null, data=bytecode) + call txs from 8 rotating senders with ABI-encoded function data via `ethers.Interface.encodeFunctionData()`
   - Phase 4: Execute through EngineService
   - Phase 5: Calculate score from engine results (conflict penalty + re-execution penalty + failure penalty)
   - Phase 6: Persist to database
   - Heuristic fallback via OptimizerService.calculateScore() when engine returns null

6. Created `VibeScoreController` with `@Post()` handler at `vibe-score` route (maps to `/api/vibe-score` with global prefix).

7. Created `VibeScoreModule` importing ContractsModule, EngineModule, AnalysisModule. Verified AnalysisModule already exports OptimizerService. Registered both EngineModule and VibeScoreModule in AppModule imports.

## Verification

- `npm run build` — zero TypeScript errors, clean compilation
- `grep EngineModule backend/src/app.module.ts` — registered in AppModule imports
- `grep VibeScoreModule backend/src/app.module.ts` — registered in AppModule imports
- `cargo build -p monad-cli` — Rust CLI binary compiles (T01 still intact)
- `cargo test -p monad-cli` — 8 CLI tests pass
- `npm test` — all 68 existing tests pass (no regressions)

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd backend && npm run build` | 0 | ✅ pass | 3.5s |
| 2 | `grep -q "EngineModule" backend/src/app.module.ts` | 0 | ✅ pass | <0.1s |
| 3 | `grep -q "VibeScoreModule" backend/src/app.module.ts` | 0 | ✅ pass | <0.1s |
| 4 | `cargo build -p monad-cli` | 0 | ✅ pass | 0.2s |
| 5 | `cargo test -p monad-cli` | 0 | ✅ pass | 0.04s |
| 6 | `cd backend && npm test` | 0 | ✅ pass | 8.3s |

## Diagnostics

- **Inspect engine config:** Check `ENGINE_BINARY_PATH` env var is set; EngineService logs a warning if path is empty or binary not found
- **Test endpoint:** `curl -X POST http://localhost:3000/api/vibe-score -H 'Content-Type: application/json' -d '{"source":"// SPDX-License-Identifier: MIT\npragma solidity ^0.8.20;\ncontract Counter { uint256 public count; function increment() external { count += 1; } }"}'`
- **Check engineBased flag:** Response includes `"engineBased": true` (engine available) or `"engineBased": false` (heuristic fallback)
- **DB inspection:** Query `VibeScore` table for persisted results with `engineBased`, `conflicts`, `reExecutions`, `gasEfficiency` columns
- **Pipeline logging:** VibeScoreService logs each phase with timing; EngineService logs CLI spawn/completion with duration

## Deviations

None — all steps executed as planned.

## Known Issues

None discovered.

## Files Created/Modified

- `backend/src/config/configuration.ts` — Added `engine.binaryPath` config entry from `ENGINE_BINARY_PATH` env var
- `backend/src/engine/engine.service.ts` — EngineService subprocess bridge with spawnSync, timeout, error handling, and observability logging
- `backend/src/engine/engine.module.ts` — NestJS module providing and exporting EngineService
- `backend/src/vibe-score/dto/vibe-score-request.dto.ts` — Request DTO with class-validator decorators
- `backend/src/vibe-score/dto/vibe-score-result.dto.ts` — Result interface with scoring and metadata fields
- `backend/src/vibe-score/vibe-score.service.ts` — Orchestrator service with compile→block→engine→score pipeline and heuristic fallback
- `backend/src/vibe-score/vibe-score.controller.ts` — POST /api/vibe-score controller
- `backend/src/vibe-score/vibe-score.module.ts` — Module importing ContractsModule, EngineModule, AnalysisModule
- `backend/src/app.module.ts` — Added EngineModule and VibeScoreModule imports
