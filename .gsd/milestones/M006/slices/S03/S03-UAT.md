# S03 UAT: Vibe-Loom — 매트릭스 히트맵 + Suggestion 카드 UI

## Preconditions

- Vibe-Loom project at `/home/ahwlsqja/Vibe-Loom`
- Node.js installed, `npm install` completed
- Jest test runner available (`npx jest`)
- TypeScript compiler available (`npx tsc`)

## Test Cases

### TC-01: Matrix Heatmap Renders with Conflict Data

**Goal:** Verify heatmap table appears when `conflictAnalysis` prop contains valid data.

1. Run: `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/VibeScoreDashboard.test.tsx -t "renders heatmap table" --verbose`
2. **Expected:** Test passes. DOM contains element with `data-testid="conflict-matrix"`.
3. **Expected:** Heatmap heading "Conflict Matrix" is rendered.
4. **Expected:** Function names appear as row headers (e.g., "transfer", "approve", "increment").
5. **Expected:** Variable names appear as column headers (e.g., "balances", "counter").

### TC-02: Heatmap Absent Without Conflict Data (Backward Compatibility)

**Goal:** Verify no heatmap renders when `conflictAnalysis` is undefined.

1. Run: `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/VibeScoreDashboard.test.tsx -t "does not render heatmap when conflictAnalysis is undefined" --verbose`
2. **Expected:** Test passes. No `data-testid="conflict-matrix"` element in DOM.
3. **Expected:** All 10 original VibeScoreDashboard tests still pass (no regression).

### TC-03: Structured Suggestion Cards Display Conflict Details

**Goal:** Verify structured cards show variable names, function names, types, and suggestions.

1. Run: `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/VibeScoreDashboard.test.tsx -t "renders structured suggestion cards" --verbose`
2. **Expected:** Test passes. Each `data-testid="conflict-card"` contains:
   - Variable name (e.g., "balances", "counter")
   - Function names in accent color (e.g., "transfer, approve")
   - Variable type (e.g., "mapping(address => uint256)")
   - Actionable suggestion text (e.g., "Consider separating into different mappings")

### TC-04: Conflict Type Badges Render Correctly

**Goal:** Verify write-write and read-write badges render with correct styling.

1. Run: `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/VibeScoreDashboard.test.tsx -t "renders conflict type badges" --verbose`
2. **Expected:** Test passes. "write-write" badge present with amber background styling.
3. **Expected:** "read-write" badge present with red background styling.

### TC-05: Empty Matrix Guard

**Goal:** Verify heatmap table is suppressed when matrix has empty rows/cols, but suggestion cards still render.

1. Run: `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/VibeScoreDashboard.test.tsx -t "does not render heatmap with empty matrix" --verbose`
2. **Expected:** Test passes. No `data-testid="conflict-matrix"` element.
3. **Expected:** `data-testid="conflict-card"` elements still present (conflicts array is non-empty).

### TC-06: Plain Suggestions Fallback

**Goal:** Verify plain 💡-prefixed suggestion cards render when `conflictAnalysis` is absent.

1. Run: `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/VibeScoreDashboard.test.tsx -t "renders plain suggestions when conflictAnalysis is absent" --verbose`
2. **Expected:** Test passes. Plain numbered suggestions visible (e.g., "1. Reduce state...").
3. **Expected:** No `data-testid="conflict-card"` or `data-testid="conflict-matrix"` elements present.

### TC-07: Full Test Suite Regression Check

**Goal:** Confirm all 16 tests pass together and no interference between old and new tests.

1. Run: `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/VibeScoreDashboard.test.tsx --verbose`
2. **Expected:** 16/16 tests pass (10 in `VibeScoreDashboard` describe + 6 in `Conflict Analysis` describe).
3. **Expected:** Zero failures, zero skips.

### TC-08: API Client Types Not Broken

**Goal:** Confirm type additions to `api-client.ts` don't break existing API client tests.

1. Run: `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/api-client.test.ts --verbose`
2. **Expected:** 11/11 tests pass. No regressions from new interface additions.

### TC-09: TypeScript Compilation — Source Files Clean

**Goal:** Verify zero type errors in modified source files.

1. Run: `cd /home/ahwlsqja/Vibe-Loom && npx tsc --noEmit 2>&1 | grep -v '__tests__' | grep -v 'node_modules'`
2. **Expected:** No output (zero errors in source files).
3. **Note:** Pre-existing test file errors (`toBeInTheDocument` type augmentation) are expected and unrelated.

### TC-10: Heatmap Color Scale Verification (Manual/Visual)

**Goal:** Verify heatmap cells use correct color scale based on conflict count.

1. **Precondition:** Run Vibe-Loom dev server with mock data containing `conflictAnalysis`.
2. Open VibeScoreDashboard with conflict data in browser.
3. **Expected:** Cells with count 0 → neutral surface color.
4. **Expected:** Cells with count 1 → light amber.
5. **Expected:** Cells with count 2 → medium amber.
6. **Expected:** Cells with count 3+ → red.
7. **Note:** This is a visual verification deferred to S04 E2E or human review.

## Edge Cases

| Case | Expected Behavior |
|------|-------------------|
| `conflictAnalysis` is `undefined` | No heatmap, no structured cards; plain suggestions render |
| `conflictAnalysis.matrix.rows` is empty | No heatmap table; suggestion cards still render if conflicts non-empty |
| `conflictAnalysis.conflicts` is empty | No structured cards; plain suggestions render as fallback |
| Both matrix and conflicts empty | No heatmap, no structured cards; plain suggestions render |
| Single function × single variable | 1×1 heatmap table renders correctly |
| Large matrix (10×10) | Table renders without overflow (scrollable if needed) |

## Pass Criteria

- All 16 unit tests pass (TC-07)
- All 11 API client tests pass (TC-08)
- Zero source file TypeScript errors (TC-09)
- Each individual test case (TC-01 through TC-06) passes independently
