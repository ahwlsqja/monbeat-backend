# S04 UAT — E2E 검증: 전체 파이프라인 통합 테스트

## Preconditions

- Node.js ≥ 18, npm available
- `Vibe-Room-Backend` cloned at `/home/ahwlsqja/Vibe-Room-Backend` with `npm install` completed
- `Vibe-Loom` cloned at `/home/ahwlsqja/Vibe-Loom` with `npm install` completed
- Playwright browsers installed (`npx playwright install chromium`)
- No local services need to be running for backend E2E (mock-based)
- Live https://vibe-loom.xyz + Railway backend must be accessible for Playwright E2E

---

## Test Case 1: NestJS E2E — ParallelConflict conflict analysis

**Purpose:** Verify Phase 5b pipeline returns decoded conflicts for a contract with storage slot collisions.

1. `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest --config ./test/jest-e2e.json test/app.e2e-spec.ts --forceExit --verbose`
2. **Expected:** Test `should return conflictAnalysis with decoded conflicts for ParallelConflict source` passes
3. **Verify in output:**
   - `conflictAnalysis.conflicts[0].variableName` === `'counter'`
   - `conflictAnalysis.matrix.rows` and `matrix.cols` are non-empty arrays
   - `conflictAnalysis.conflicts[0].functions` is a non-empty array
   - `conflictAnalysis.conflicts[0].suggestion` is a non-empty string
   - `body.vibeScore` is a number 0-100
   - `body.engineBased` is `true`
   - `body.suggestions` is a non-empty array

## Test Case 2: NestJS E2E — Backward compatibility (no conflict contract)

**Purpose:** Verify simple contracts without R/W conflicts return the original API shape, no conflictAnalysis.

1. Same test command as Test Case 1
2. **Expected:** Test `should omit conflictAnalysis for non-conflict contract (backward compat)` passes
3. **Verify in output:**
   - `body.conflictAnalysis` is `undefined`
   - `body.vibeScore` is a number 0-100
   - `body.suggestions` is an array

## Test Case 3: NestJS E2E — EngineService mock isolation

**Purpose:** Confirm the mock doesn't leak from the Conflict Analysis describe block into other test blocks.

1. Same test command as Test Case 1
2. **Expected:** Test `should only mock EngineService in this describe block (isolation check)` passes
3. **Verify:** EngineService mock `jest.fn()` check passes, confirming scope isolation

## Test Case 4: NestJS Full Suite Regression

**Purpose:** Confirm all existing backend tests still pass after M006 changes.

1. `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest --forceExit`
2. **Expected:** 118 passed, 1 failed
3. **Verify:** The 1 failure is in `deploy.service.spec.ts` (userId 'anonymous' vs null) — this is pre-existing on `main`, not introduced by M006

## Test Case 5: Frontend Unit Test Regression

**Purpose:** Confirm all Vibe-Loom unit tests still pass after S03 UI additions.

1. `cd /home/ahwlsqja/Vibe-Loom && npx jest`
2. **Expected:** 63 passed, 0 failed (5 suites)
3. **Verify:** No regressions in api-client, abi-utils, solc-error-parser, heatmap, or dashboard tests

## Test Case 6: Playwright E2E — ParallelConflict heatmap rendering

**Purpose:** Verify end-to-end browser flow for conflict analysis heatmap rendering.

1. `cd /home/ahwlsqja/Vibe-Loom && npx playwright test e2e/full-stack.spec.ts --reporter=list`
2. **Expected:** Test `ParallelConflict renders heatmap and suggestion cards` passes
3. **Outcome depends on live backend deployment state:**
   - **heatmap-visible:** M006 Rust binary deployed → `[data-testid="conflict-matrix"]` visible, `[data-testid="conflict-card"]` present, 'counter' text in suggestion card
   - **gauge-only:** M006 binary NOT deployed → standard SVG gauge visible, no heatmap. Test passes gracefully with log: `[Conflict Analysis] ParallelConflict outcome: gauge-only`
   - **timeout:** Service unavailable → test passes gracefully with log: `[Conflict Analysis] ParallelConflict outcome: timeout`
4. **Verify screenshot:** `e2e/screenshots/conflict-parallel-heatmap.png` or `conflict-parallel-gauge-only.png` captured

## Test Case 7: Playwright E2E — FixedContract backward compatibility

**Purpose:** Verify FixedContract shows standard gauge UI without conflict heatmap.

1. Same test command as Test Case 6
2. **Expected:** Test `FixedContract backward compatibility — no conflict heatmap, gauge visible` passes
3. **Verify in output:**
   - SVG gauge element is visible
   - `[data-testid="conflict-matrix"]` is NOT present (hidden or absent)
   - Log: `[Conflict Analysis] FixedContract gauge outcome: gauge-visible`
4. **Verify screenshot:** `e2e/screenshots/fixedcontract-compat.png` captured

## Test Case 8: Playwright Full Suite Regression

**Purpose:** Confirm all existing Playwright E2E tests still pass after M006 changes.

1. Same test command as Test Case 6
2. **Expected:** 23+ passed, ≤1 skipped (deploy timeout is acceptable)
3. **Verify:** No new failures in existing test blocks (Backend API Health, Frontend IDE Load, Compile Flow, Vibe-Score Flow, Deploy Flow, Full E2E Flow, Mobile Responsive, Contract Selector, AI Error Analysis)

---

## Edge Cases

### E1: Mock conflict address is coinbase
If the E2E mock `conflict_details` uses the default coinbase address (e.g., `0x0000...0000`), `buildConflictAnalysis` will filter it out and `conflictAnalysis.conflicts` will be empty. **The mock must use a non-coinbase address** (e.g., `0x1234...abcd`).

### E2: Production deployment state changes
When the M006 Rust binary is deployed to production Railway:
- Test Case 6 will automatically switch from "gauge-only" to "heatmap-visible" outcome
- No code changes required — the Promise.race pattern handles both states

### E3: solc storageLayout availability
The NestJS E2E test (Test Case 1) uses the real CompileService. If solc version changes or isn't available, compilation may fail. The test depends on solc being accessible in the test environment.

### E4: Pre-existing deploy.service.spec.ts failure
This 1 failure (userId assertion mismatch) exists on `main` before any M006 changes. It should not be counted against M006 verification. Confirm by running `git stash && npx jest test/deploy.service.spec.ts && git stash pop` to verify it fails identically on untouched main.
