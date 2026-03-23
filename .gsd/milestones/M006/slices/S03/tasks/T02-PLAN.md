---
estimated_steps: 3
estimated_files: 1
---

# T02: Add unit tests for heatmap, structured suggestion cards, and backward compatibility

**Slice:** S03 — Vibe-Loom — 매트릭스 히트맵 + Suggestion 카드 UI
**Milestone:** M006

## Description

Add comprehensive unit tests that verify the new conflict analysis visualization works correctly: heatmap renders with proper structure, structured suggestion cards display all required fields, backward compatibility is preserved (existing rendering unchanged when `conflictAnalysis` is absent), and edge cases are handled (empty matrix). This task is the slice's objective stopping condition — all tests must pass.

**Relevant skills:** `test`

## Steps

1. **Create mock test data at the top of the test file `Vibe-Loom/src/__tests__/VibeScoreDashboard.test.tsx`:**
   - Define a `mockConflictAnalysis` object:
     ```typescript
     const mockConflictAnalysis = {
       conflicts: [
         {
           variableName: 'balances',
           variableType: 'mapping(address => uint256)',
           slot: '0x3',
           functions: ['transfer', 'approve'],
           conflictType: 'write-write',
           suggestion: 'Consider separating the balances mapping into per-function mappings',
         },
         {
           variableName: 'counter',
           variableType: 'uint256',
           slot: '0x0',
           functions: ['increment', 'decrement'],
           conflictType: 'read-write',
           suggestion: 'Use function-specific counters to avoid cross-function conflicts',
         },
       ],
       matrix: {
         rows: ['transfer', 'approve', 'increment'],
         cols: ['balances', 'counter'],
         cells: [[2, 0], [1, 0], [0, 3]],
       },
     };
     ```
   - Define a `mockEmptyMatrix` variant with `rows: [], cols: [], cells: []` for edge case testing

2. **Add 5+ new test cases in a new `describe('Conflict Analysis')` block:**
   - **Test 1: "renders heatmap table when conflictAnalysis is provided"** — Render with `conflictAnalysis={mockConflictAnalysis}`. Assert: "Conflict Matrix" heading text visible, function names ("transfer", "approve", "increment") visible in table, variable names ("balances", "counter") visible in table header.
   - **Test 2: "does not render heatmap when conflictAnalysis is undefined"** — Render without `conflictAnalysis` prop. Assert: "Conflict Matrix" text NOT present (`queryByText` returns null).
   - **Test 3: "renders structured suggestion cards with variable names and functions"** — Render with `conflictAnalysis={mockConflictAnalysis}`. Assert: "balances" text visible, "transfer" text visible, "approve" text visible, "write-write" badge text visible, suggestion text visible.
   - **Test 4: "renders conflict type badges"** — Render with mock. Assert: both "write-write" and "read-write" badge texts visible.
   - **Test 5: "does not render heatmap with empty matrix"** — Render with `conflictAnalysis={ conflicts: mockConflictAnalysis.conflicts, matrix: { rows: [], cols: [], cells: [] } }`. Assert: "Conflict Matrix" text NOT present (heatmap hidden), but structured suggestion cards still render.
   - **Test 6 (optional): "renders plain suggestions when conflictAnalysis is absent"** — Render with `suggestions={['Some suggestion']}` and NO `conflictAnalysis`. Assert: plain suggestion text visible, confirming backward compat for non-conflict suggestion display.

3. **Run the full test suite and verify:**
   - All 10 existing tests pass unchanged
   - All 5+ new tests pass
   - Run `npx tsc --noEmit` to confirm type-check still clean
   - Run `npx jest src/__tests__/api-client.test.ts` to confirm type changes didn't break API client tests

## Must-Haves

- [ ] 5+ new tests covering heatmap rendering, structured cards, conflict type badges, backward compat, and empty matrix guard
- [ ] All 10 existing VibeScoreDashboard tests pass unchanged
- [ ] All api-client tests pass unchanged
- [ ] TypeScript type-check passes (`tsc --noEmit`)

## Verification

- `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/VibeScoreDashboard.test.tsx --verbose` — 15+ tests, all pass
- `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/api-client.test.ts --verbose` — 10 tests, all pass
- `cd /home/ahwlsqja/Vibe-Loom && npx tsc --noEmit` — zero errors

## Inputs

- `Vibe-Loom/src/__tests__/VibeScoreDashboard.test.tsx` — existing test file with 10 tests to preserve
- `Vibe-Loom/src/components/ide/VibeScoreDashboard.tsx` — T01 output: component with heatmap and structured suggestion card rendering
- `Vibe-Loom/src/lib/api-client.ts` — T01 output: `ConflictAnalysis` and related interfaces

## Expected Output

- `Vibe-Loom/src/__tests__/VibeScoreDashboard.test.tsx` — extended with 5+ new tests in a "Conflict Analysis" describe block
