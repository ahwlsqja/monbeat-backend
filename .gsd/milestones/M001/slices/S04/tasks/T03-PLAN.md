---
estimated_steps: 3
estimated_files: 3
---

# T03: Write unit tests for EngineService, VibeScoreService, and VibeScoreController

**Slice:** S04 — Engine Bridge + Vibe-Score
**Milestone:** M001

## Description

Write comprehensive unit tests for all new S04 NestJS services and controller. Tests mock `child_process.spawnSync` for EngineService, and mock all injected services for VibeScoreService and VibeScoreController. Target ≥15 new tests across 3 test suites. The existing backend test framework (Jest + ts-jest + @nestjs/testing) is already configured with `rootDir: "."` and `roots: ["src", "test"]`.

**Skills:** `test` skill may be helpful for test patterns. The existing test files in `backend/test/` provide patterns — especially `compile.service.spec.ts` and `deploy.service.spec.ts` for mocking patterns.

## Steps

1. **Create `backend/test/engine.service.spec.ts`** (~5 tests):
   - Mock `child_process` module: `jest.mock('child_process')`.
   - Mock `ConfigService` to return `ENGINE_BINARY_PATH`.
   - Test cases:
     a. `executeBlock() returns parsed JSON on successful CLI execution` — mock spawnSync returning `{ status: 0, stdout: JSON.stringify(validCliOutput), stderr: '' }` → assert result matches expected structure.
     b. `executeBlock() returns null when binary path is empty` — config returns `''` → assert null.
     c. `executeBlock() returns null when spawnSync times out` — mock spawnSync throwing ETIMEDOUT → assert null.
     d. `executeBlock() returns null when CLI returns non-zero status` — mock status 1, stderr 'error' → assert null.
     e. `executeBlock() returns null when stdout is not valid JSON` — mock stdout 'not json' → assert null.
   - Use `Test.createTestingModule` with `ConfigService` mock providing `configService.get('engine.binaryPath')`.

2. **Create `backend/test/vibe-score.service.spec.ts`** (~8 tests):
   - Mock all dependencies: `CompileService`, `EngineService`, `OptimizerService`, `PrismaService`.
   - `CompileService.compile` mock returns `{ contractName: 'FixedContract', abi: [{type:'function', name:'store', inputs:[{type:'uint256',name:'_value'}], outputs:[], stateMutability:'nonpayable'}, {type:'function', name:'retrieve', inputs:[], outputs:[{type:'uint256'}], stateMutability:'view'}], bytecode: '0x6080...' }`.
   - `EngineService.executeBlock` mock returns `{ results: [{success:true, gas_used:50000, output:'0x', error:null, logs_count:1}, ...], incarnations: [0, 0, 0, 0], stats: { total_gas:200000, num_transactions:4, num_conflicts:0, num_re_executions:0 } }` by default.
   - `PrismaService.vibeScore.create` mock returns resolved object.
   - Test cases:
     a. `returns engine-based score with engineBased=true when engine succeeds` — default mocks → assert result.engineBased === true, result.vibeScore > 0.
     b. `returns high score (≥80) when no conflicts detected` — incarnations all 0 → assert vibeScore ≥ 80.
     c. `returns lower score when conflicts detected` — mock engine returning incarnations [0, 2, 1, 3] with num_conflicts=3 → assert vibeScore < score from test (a).
     d. `falls back to heuristic when engine returns null` — mock `EngineService.executeBlock` returning null, mock `OptimizerService.calculateScore` returning `{ score: 75, deductions: [], suggestions: ['test'] }` → assert result.engineBased === false, result.vibeScore === 75.
     e. `filters view/pure functions from ABI` — mock ABI with only view functions → should still work (falls back or uses deploy-only block).
     f. `saves result to database` — assert `PrismaService.vibeScore.create` was called with correct data.
     g. `handles compilation error` — mock CompileService.compile throwing BadRequestException → assert error propagates.
     h. `calculates gasEfficiency correctly` — mock with 1 failed tx out of 4 → assert gasEfficiency = 75.

3. **Create `backend/test/vibe-score.controller.spec.ts`** (~4 tests):
   - Mock `VibeScoreService`.
   - Test cases:
     a. `controller is defined` — standard NestJS pattern.
     b. `POST /api/vibe-score calls analyzeContract with source` — assert service method called with dto.source.
     c. `returns VibeScoreResultDto shape` — mock service returns full result → verify response structure.
     d. `handles service errors` — mock service throwing → assert error propagates.

## Must-Haves

- [ ] ≥5 EngineService tests covering success, null returns, timeout, bad JSON
- [ ] ≥8 VibeScoreService tests covering engine path, heuristic fallback, scoring, DB persistence
- [ ] ≥4 VibeScoreController tests covering endpoint behavior
- [ ] All existing tests still pass (no regressions)
- [ ] Total new tests ≥ 15

## Verification

- `cd /home/ahwlsqja/monad-core/.gsd/worktrees/M001/backend && npm test` — all tests pass
- `cd /home/ahwlsqja/monad-core/.gsd/worktrees/M001/backend && npm test -- --testPathPattern=engine.service` — ≥5 tests pass
- `cd /home/ahwlsqja/monad-core/.gsd/worktrees/M001/backend && npm test -- --testPathPattern=vibe-score` — ≥12 tests pass (service + controller)

## Inputs

- `backend/src/engine/engine.service.ts` — EngineService to test (from T02)
- `backend/src/vibe-score/vibe-score.service.ts` — VibeScoreService to test (from T02)
- `backend/src/vibe-score/vibe-score.controller.ts` — VibeScoreController to test (from T02)
- `backend/src/vibe-score/dto/vibe-score-request.dto.ts` — request DTO (from T02)
- `backend/src/vibe-score/dto/vibe-score-result.dto.ts` — result interface (from T02)
- `backend/test/compile.service.spec.ts` — existing test patterns for reference
- `backend/test/deploy.service.spec.ts` — existing mock patterns for reference

## Expected Output

- `backend/test/engine.service.spec.ts` — EngineService unit tests (≥5)
- `backend/test/vibe-score.service.spec.ts` — VibeScoreService unit tests (≥8)
- `backend/test/vibe-score.controller.spec.ts` — VibeScoreController unit tests (≥4)
