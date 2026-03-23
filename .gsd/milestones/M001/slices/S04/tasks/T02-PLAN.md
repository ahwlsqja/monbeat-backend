---
estimated_steps: 7
estimated_files: 10
---

# T02: Build NestJS EngineService, VibeScoreService, Controller, and module wiring

**Slice:** S04 — Engine Bridge + Vibe-Score
**Milestone:** M001

## Description

Create the NestJS side of the engine bridge: `EngineService` (subprocess manager for Rust CLI), `VibeScoreService` (orchestrator: compile → block construction → engine → scoring), `VibeScoreController` (`POST /api/vibe-score`), and module wiring into AppModule. This task builds on T01's Rust CLI binary and S02's `CompileService`.

The VibeScoreService must construct meaningful transaction blocks from compiled bytecode + ABI: a deploy tx (to=null, data=bytecode) followed by multiple call txs from different senders encoding non-view/non-pure functions via `ethers.Interface.encodeFunctionData()`. The deploy address is computed client-side via `ethers.getCreateAddress({ from, nonce })`.

When the engine binary is unavailable, the service falls back to S03's existing `OptimizerService.calculateScore()` for heuristic scoring.

**Skills:** None needed (standard NestJS patterns from S01-S03 apply).

## Steps

1. **Extend configuration.ts** — add `engine: { binaryPath: process.env.ENGINE_BINARY_PATH || '' }` to the config factory. The `.env.example` already documents `ENGINE_BINARY_PATH`.

2. **Create EngineService** (`backend/src/engine/engine.service.ts`):
   - Inject `ConfigService`.
   - Method `executeBlock(transactions: any[], blockEnv: any): CliOutput | null`.
   - Get `binaryPath` from config (`configService.get<string>('engine.binaryPath')`).
   - If binaryPath is empty or file doesn't exist → log warning, return null.
   - Use `child_process.spawnSync(binaryPath, [], { input: JSON.stringify({ transactions, block_env: blockEnv }), encoding: 'utf-8', timeout: 30000 })`.
   - If status !== 0 or stderr → log error with stderr content, return null.
   - Parse stdout as JSON → return parsed `CliOutput`.
   - If JSON parse fails → log error, return null.
   - Define TypeScript interfaces matching T01's CLI output: `CliOutput { results: TxResult[], incarnations: number[], stats: { total_gas, num_transactions, num_conflicts, num_re_executions } }`.

3. **Create EngineModule** (`backend/src/engine/engine.module.ts`):
   - Simple NestJS module providing and exporting `EngineService`.

4. **Create DTOs** for vibe-score:
   - `backend/src/vibe-score/dto/vibe-score-request.dto.ts`: `{ source: string }` with `@IsString()` and `@IsNotEmpty()` validators.
   - `backend/src/vibe-score/dto/vibe-score-result.dto.ts`: Interface `VibeScoreResultDto { vibeScore: number, conflicts: number, reExecutions: number, gasEfficiency: number, engineBased: boolean, suggestions: string[], traceResults?: any[] }`.

5. **Create VibeScoreService** (`backend/src/vibe-score/vibe-score.service.ts`):
   - Inject: `CompileService`, `EngineService`, `OptimizerService`, `PrismaService`, `Logger`.
   - Main method `analyzeContract(source: string, userId?: string): Promise<VibeScoreResultDto>`.
   - Pipeline:
     a. `CompileService.compile(source)` → `{ contractName, abi, bytecode }`.
     b. Filter ABI for non-view, non-pure, non-constructor functions (state-changing methods).
     c. If no state-changing functions → use heuristic fallback.
     d. Define 8 sender addresses (hex strings: `0x000...00E1` through `0x000...00E8`).
     e. Compute deploy address: `ethers.getCreateAddress({ from: senders[0], nonce: 0 })`.
     f. Construct transaction block:
        - tx0: deploy tx `{ sender: senders[0], to: null, data: bytecode (strip 0x prefix for Rust), value: "0", gas_limit: 2000000, nonce: 0, gas_price: "1000000000" }`.
        - For each state-changing function, create call txs from senders[1..N] with `new ethers.Interface(abi).encodeFunctionData(fnName, defaultArgs)`.
        - Default args: use `0` for uint types, empty for others. For simplicity, handle `uint256` → `0`, `address` → sender address, `bool` → `false`, `string` → `""`, `bytes` → `"0x"`.
     g. Construct blockEnv: `{ number: 1, coinbase: "0x000...00C0", timestamp: currentUnixSeconds, gas_limit: 30000000, base_fee: "0", difficulty: "0" }`.
     h. Call `EngineService.executeBlock(txBlock, blockEnv)`.
     i. If engine returns null → fall back to `OptimizerService.calculateScore(source)` and return with `engineBased: false`.
     j. If engine returns results → calculate score:
        - `conflictCount = stats.num_conflicts`
        - `reExecutionCount = stats.num_re_executions`
        - `totalTxs = results.length`
        - `conflictRatio = conflictCount / Math.max(totalTxs - 1, 1)` (exclude deploy tx)
        - `conflictPenalty = Math.min(40, Math.round(conflictRatio * 50))`
        - `reExecPenalty = Math.min(30, reExecutionCount * 5)`
        - `gasTotal = results.reduce((sum, r) => sum + r.gas_used, 0)`
        - `failedTxs = results.filter(r => !r.success).length`
        - `failurePenalty = Math.min(20, failedTxs * 10)`
        - `vibeScore = Math.max(0, Math.min(100, 100 - conflictPenalty - reExecPenalty - failurePenalty))`
        - `gasEfficiency = Math.round((1 - failedTxs / Math.max(totalTxs, 1)) * 100)`
     k. Build suggestions array based on score components.
     l. Save to DB: `prisma.vibeScore.create({ data: { userId, contractSource: source, score: vibeScore, engineBased: true, conflicts: String(conflictCount), reExecutions: String(reExecutionCount), gasEfficiency: String(gasEfficiency), suggestions } })`.
     m. Return `VibeScoreResultDto`.

6. **Create VibeScoreController** (`backend/src/vibe-score/vibe-score.controller.ts`):
   - `@Controller('vibe-score')` → routes to `/api/vibe-score` (with global prefix).
   - `@Post()` handler accepting `@Body() dto: VibeScoreRequestDto`, returns `VibeScoreResultDto`.
   - Inject `VibeScoreService`, call `analyzeContract(dto.source)`.

7. **Create VibeScoreModule and wire into AppModule**:
   - `VibeScoreModule` imports `ContractsModule` (for CompileService), `EngineModule`, `AnalysisModule` (for OptimizerService — must be exported from AnalysisModule).
   - Verify `AnalysisModule` exports `OptimizerService` — if not, add it.
   - Register `EngineModule` and `VibeScoreModule` in `backend/src/app.module.ts` imports.

## Must-Haves

- [ ] `EngineService.executeBlock()` spawns subprocess, pipes JSON, returns parsed output or null
- [ ] EngineService handles: missing binary → null, timeout → null, bad JSON → null
- [ ] `VibeScoreService.analyzeContract()` orchestrates compile → block construction → engine → scoring
- [ ] Transaction block includes deploy tx + call txs with ABI-encoded function data
- [ ] Score formula penalizes conflicts, re-executions, and failures
- [ ] Heuristic fallback via OptimizerService when engine unavailable
- [ ] `POST /api/vibe-score` endpoint registered and functional
- [ ] Both modules registered in AppModule
- [ ] `npm run build` succeeds with zero errors

## Verification

- `cd /home/ahwlsqja/monad-core/.gsd/worktrees/M001/backend && npm run build` — zero TypeScript errors
- `grep -q "EngineModule" backend/src/app.module.ts` — registered
- `grep -q "VibeScoreModule" backend/src/app.module.ts` — registered

## Observability Impact

- **New runtime signals:** EngineService logs CLI spawn/completion/timeout/error with duration (ms); VibeScoreService logs each pipeline phase (compile, block construction, engine, scoring) with timing
- **Inspection surfaces:** `POST /api/vibe-score` response includes `engineBased: boolean` flag; VibeScore DB records include `engineBased`, `conflicts`, `reExecutions`, `gasEfficiency` fields
- **Failure visibility:** EngineService logs CLI stderr on subprocess failure, timeout errors include duration; VibeScoreService logs fallback reason when engine unavailable; DB write failures are logged but don't crash the scoring pipeline
- **How to inspect:** Call `POST /api/vibe-score` with Solidity source and check the `engineBased` field in the response; query the `VibeScore` table for persisted results

## Inputs

- `backend/src/contracts/compile.service.ts` — CompileService.compile() returns { contractName, abi, bytecode }
- `backend/src/contracts/contracts.module.ts` — exports CompileService
- `backend/src/analysis/optimizer.service.ts` — OptimizerService.calculateScore() for heuristic fallback
- `backend/src/config/configuration.ts` — config factory to extend with engine.binaryPath
- `backend/src/app.module.ts` — AppModule to register new modules
- `crates/cli/src/main.rs` — CLI output JSON schema (from T01)

## Expected Output

- `backend/src/config/configuration.ts` — extended with engine.binaryPath
- `backend/src/engine/engine.service.ts` — subprocess bridge to Rust CLI
- `backend/src/engine/engine.module.ts` — NestJS module for EngineService
- `backend/src/vibe-score/vibe-score.service.ts` — orchestrator service
- `backend/src/vibe-score/vibe-score.controller.ts` — POST /api/vibe-score endpoint
- `backend/src/vibe-score/vibe-score.module.ts` — NestJS module
- `backend/src/vibe-score/dto/vibe-score-request.dto.ts` — request DTO
- `backend/src/vibe-score/dto/vibe-score-result.dto.ts` — result interface
- `backend/src/app.module.ts` — updated with EngineModule + VibeScoreModule imports
