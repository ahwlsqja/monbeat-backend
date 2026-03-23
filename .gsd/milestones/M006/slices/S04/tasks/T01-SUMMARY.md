---
id: T01
parent: S04
milestone: M006
provides:
  - NestJS E2E tests for conflict analysis API response shape (ParallelConflict + backward compat)
key_files:
  - Vibe-Room-Backend/test/app.e2e-spec.ts
key_decisions:
  - Separate describe block with own TestingModule avoids EngineService mock leaking to existing tests
  - CompileService left real (not mocked) so storageLayout comes from actual solc compilation
  - Mock conflict address uses non-coinbase address to avoid buildConflictAnalysis coinbase filtering
patterns_established:
  - E2E tests requiring different provider overrides use separate describe blocks with independent beforeAll/afterAll
observability_surfaces:
  - Jest test output shows pass/fail for 3 new tests (ParallelConflict analysis, backward compat, isolation check)
  - Pipeline phase logs (Phase 1–5b) emitted to stderr during E2E runs for pipeline tracing
duration: 15m
verification_result: passed
completed_at: 2026-03-24
blocker_discovered: false
---

# T01: NestJS E2E — conflict analysis API 응답 형태 검증

**Added 3 E2E tests validating Phase 5b conflict analysis pipeline: ParallelConflict source returns decoded conflicts with variableName/matrix/suggestions, Simple source omits conflictAnalysis for backward compat, and EngineService mock isolation is verified.**

## What Happened

Added a new `describe('Conflict Analysis E2E')` block to `app.e2e-spec.ts` with its own TestingModule that overrides both PrismaService (to prevent DB connections) and EngineService (to return controlled `conflict_details`). CompileService is left real so the actual solc compiler produces `storageLayout`, which the Phase 5b pipeline (`buildConflictAnalysis`) uses to decode storage slots into variable names.

Test 1 (ParallelConflict): Sends inline ParallelConflict Solidity source to `POST /api/vibe-score`. The mocked EngineService returns `conflict_details` with a write-write conflict on slot `0x0`. Asserts that `conflictAnalysis.conflicts[0].variableName === 'counter'`, matrix rows/cols are present, functions array is non-empty, suggestion is truthy, and existing fields (vibeScore, engineBased, suggestions) remain intact.

Test 2 (backward compat): Sends a simple contract with EngineService returning no `conflict_details`. Asserts that `conflictAnalysis` is undefined and existing fields (vibeScore 0–100, suggestions array) are valid.

Test 3 (isolation): Verifies that the EngineService mock is a jest.fn() and was called, confirming mock scope is limited to this describe block.

## Verification

Ran `npx jest --config ./test/jest-e2e.json test/app.e2e-spec.ts --forceExit` — all 15 tests pass (12 existing + 3 new). Ran `npx jest --forceExit` for the full suite — 118 passed, 1 pre-existing failure in `deploy.service.spec.ts` (unrelated, confirmed failing on main before any M006 changes).

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd Vibe-Room-Backend && npx jest --config ./test/jest-e2e.json test/app.e2e-spec.ts --forceExit` | 0 | ✅ pass | 12.5s |
| 2 | `cd Vibe-Room-Backend && npx jest --forceExit` | 1 | ⚠️ partial (1 pre-existing failure in deploy.service.spec.ts) | 12.9s |

## Diagnostics

- Run `npx jest --config ./test/jest-e2e.json test/app.e2e-spec.ts --forceExit --verbose` to see per-test timing and phase logs
- The `Conflict Analysis E2E` block's stderr output includes Phase 1–5b timing logs from VibeScoreService
- On assertion failure, Jest shows full response body diff for easy debugging
- Mock data (mockConflictDetails) is inline in the test file for self-contained diagnosis

## Deviations

- Slice plan verification command `npx jest test/app.e2e-spec.ts --forceExit` doesn't work with the default jest config (testRegex doesn't match `.e2e-spec.ts`). Must use `--config ./test/jest-e2e.json` instead. This is a pre-existing project convention, not a change I introduced.
- Added a 3rd test (EngineService mock isolation check) beyond the 2 described in the plan, for observability.

## Known Issues

- `deploy.service.spec.ts` has 1 pre-existing test failure on `main` (userId 'anonymous' vs null assertion mismatch) — unrelated to conflict analysis work.

## Files Created/Modified

- `Vibe-Room-Backend/test/app.e2e-spec.ts` — Added `Conflict Analysis E2E` describe block with 3 tests (ParallelConflict analysis, backward compat, isolation check) and EngineService import
