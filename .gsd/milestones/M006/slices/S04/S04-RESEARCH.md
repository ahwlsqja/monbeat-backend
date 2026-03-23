# S04 Research: E2E 검증 — 전체 파이프라인 통합 테스트

**Calibration: Light research** — well-understood work using established test patterns already in the codebase. All three layers (S01/S02/S03) are complete with passing unit tests. S04 writes integration/E2E tests following existing patterns.

## Summary

S04 verifies the M006 pipeline end-to-end: ParallelConflict → Rust CLI conflict_details → NestJS storage layout decoding → Vibe-Loom heatmap + suggestion cards. Two verification axes: (1) ParallelConflict produces `conflictAnalysis` with decoded variable names, matrix, and suggestions, (2) FixedContract (no conflict) retains full backward compatibility with no `conflictAnalysis`.

**Requirements coverage:** R017 (병렬 실행 최적화 제안), R018 (R/W Set 충돌 시각화), R006 (Vibe Score 강화), R010 (E2E 통합 테스트).

## Recommendation

Three test files across two repos, following existing patterns exactly:

### Test 1: NestJS Backend E2E — ParallelConflict + FixedContract API (supertest)

**File:** `Vibe-Room-Backend/test/app.e2e-spec.ts` — extend existing E2E suite  
**Pattern:** Already has `describe('POST /api/vibe-score')` with supertest. Add two new tests.

The existing E2E boots a real NestJS app with mocked PrismaService. **Key constraint:** No Rust engine binary in test env → `engineService.executeBlock()` returns `null` → heuristic fallback → no `conflict_details`. Two options:

- **Option A (recommended):** Add a dedicated `describe` block that provides a mock `EngineService` returning realistic `CliOutput` with `conflict_details` for ParallelConflict. This tests the Phase 5b pipeline (compile → storageLayout extraction → conflict decoding → suggestion generation → API response shape) as an integration test.
- **Option B:** Test against live Railway backend (like Playwright E2E). Fragile — depends on deployed binary.

Option A is robust and tests the NestJS integration layer end-to-end (compile → decode → respond). Two tests:
1. `POST /api/vibe-score` with ParallelConflict source → 201, `conflictAnalysis.conflicts[0].variableName === 'counter'`, matrix has rows/cols, suggestions non-empty
2. `POST /api/vibe-score` with FixedContract source → 201, `conflictAnalysis` is `undefined`, existing fields intact

**Mock data shape** (from S02's test fixtures — reuse `mockConflictDetails` and `mockStorageLayout` from `test/vibe-score.service.spec.ts`):
```typescript
conflict_details: {
  per_tx: [],
  conflicts: [{
    location: { location_type: 'Storage', address: '0xDeployedAddr', slot: '0x0' },
    tx_a: 1, tx_b: 2, conflict_type: 'write-write'
  }]
}
```

### Test 2: Vibe-Loom Playwright E2E — Live Pipeline (against production)

**File:** `Vibe-Loom/e2e/full-stack.spec.ts` — extend existing Playwright suite  
**Pattern:** Existing `test.describe('Vibe-Score Flow')` navigates to live site, sets contract, clicks Vibe Score. Add a new `test.describe('Conflict Analysis E2E')` block.

Two tests following the existing defensive patterns (KNOWLEDGE.md: Promise.race, timeout guards, screenshot evidence):

1. **ParallelConflict pipeline:** Load page → select ParallelConflict from contract selector → click Vibe Score → wait for dashboard → assert `[data-testid="conflict-matrix"]` visible + `[data-testid="conflict-card"]` present + heatmap has 'counter' text
2. **FixedContract backward compat:** Select FixedContract → Vibe Score → dashboard appears (SVG gauge visible) → `[data-testid="conflict-matrix"]` NOT present → plain suggestions visible

**Critical guard:** The live backend may not have the updated engine binary yet. Tests should use `test.skip` guard or `Promise.race` pattern if `conflictAnalysis` is absent — the test verifies the pipeline works when data flows, but doesn't hard-fail if backend hasn't deployed M006 changes yet.

### Test 3: Rust CLI Integration Test (cargo test)

**File:** `crates/cli/tests/integration_test.rs` (new file, or extend existing)  
**Pattern:** Build CLI, pipe ParallelConflict bytecode as JSON stdin, parse JSON stdout, verify `conflict_details` field present with Storage conflicts on the counter slot.

**Actually:** The S01 unit tests in `conflict.rs` already verify `detect_conflicts()` thoroughly (6 tests). A true CLI integration test would require compiling ParallelConflict.sol → getting bytecode → constructing JSON input → piping through `cargo run`. This is heavy and largely redundant with S01's unit tests + the NestJS E2E. **Skip this** — the NestJS test already calls the real compile pipeline.

## Implementation Landscape

| What | Where | Pattern |
|------|-------|---------|
| NestJS E2E additions | `Vibe-Room-Backend/test/app.e2e-spec.ts` | Extend existing `describe` blocks with new tests; mock EngineService for conflict data |
| Playwright E2E additions | `Vibe-Loom/e2e/full-stack.spec.ts` | New `test.describe('Conflict Analysis E2E')` block with defensive assertions |
| Test contracts | Already exist: `contracts/test/ParallelConflict.sol`, `contracts/FixedContract.sol` | No new contracts needed |
| Frontend test-ids | `[data-testid="conflict-matrix"]`, `[data-testid="conflict-card"]` | Already added in S03 |
| API response shape | `{ success: true, data: { vibeScore, conflictAnalysis?: { conflicts, matrix } } }` | S02 DTO defines this |

## Constraints

- **3-repo rule** (KNOWLEDGE.md): Backend tests go to `Vibe-Room-Backend/`, frontend tests go to `Vibe-Loom/`. No cross-repo test files in monad-core.
- **Playwright tests against live site** (KNOWLEDGE.md): Use defensive patterns — `Promise.race`, `test.skip()` guard if service state prevents assertion, screenshot evidence at each step.
- **No engine binary in NestJS test env:** Must mock `EngineService.executeBlock()` return value for the conflict analysis path. The existing test already does this pattern in `vibe-score.service.spec.ts`.
- **app.e2e-spec.ts** already overrides PrismaService and boots a real NestJS app. To also override EngineService for specific tests, either use a separate `describe` block with a different TestingModule setup, or use `jest.spyOn` on the existing module's service.

## Verification

| Check | Command | Expected |
|-------|---------|----------|
| NestJS E2E tests pass | `cd Vibe-Room-Backend && npx jest test/app.e2e-spec.ts --forceExit` | New conflict analysis tests pass alongside existing tests |
| Playwright E2E (optional — live) | `cd Vibe-Loom && npx playwright test e2e/full-stack.spec.ts` | New conflict tests pass or skip gracefully |
| Existing tests unbroken | `cd Vibe-Room-Backend && npx jest --forceExit` | All 43+ backend tests pass |
| Frontend unit tests | `cd Vibe-Loom && npx jest` | All 16 dashboard + 11 API client tests pass |

## Task Decomposition Suggestion

- **T01:** NestJS E2E — add 2-3 tests to `app.e2e-spec.ts` verifying conflict analysis API response shape for ParallelConflict (with mocked engine) + FixedContract backward compat
- **T02:** Playwright E2E — add 2 tests to `full-stack.spec.ts` for ParallelConflict heatmap rendering + FixedContract backward compat (with defensive guards for live service state)
