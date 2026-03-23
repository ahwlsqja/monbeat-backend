---
estimated_steps: 5
estimated_files: 3
---

# T01: Add conflict analysis types, heatmap rendering, suggestion cards, and page wiring

**Slice:** S03 — Vibe-Loom — 매트릭스 히트맵 + Suggestion 카드 UI
**Milestone:** M006

## Description

Add the complete frontend implementation for conflict analysis visualization: TypeScript interfaces mirroring the backend DTO, a function×variable matrix heatmap table with color-coded cells, structured suggestion cards replacing plain text when decoded conflicts are available, and prop wiring from the page to the dashboard component. This task produces all user-facing rendering — the next task adds test coverage.

**Relevant skills:** `react-best-practices`, `frontend-design`, `make-interfaces-feel-better`

## Steps

1. **Add TypeScript interfaces to `Vibe-Loom/src/lib/api-client.ts`:**
   - Add `DecodedConflict` interface: `{ variableName: string, variableType: string, slot: string, functions: string[], conflictType: string, suggestion: string }`
   - Add `ConflictMatrix` interface: `{ rows: string[], cols: string[], cells: number[][] }`
   - Add `ConflictAnalysis` interface: `{ conflicts: DecodedConflict[], matrix: ConflictMatrix }`
   - Add `conflictAnalysis?: ConflictAnalysis` to the existing `VibeScoreResult` interface
   - Place new interfaces above `VibeScoreResult` so they can be referenced

2. **Extend `VibeScoreDashboardProps` in `Vibe-Loom/src/components/ide/VibeScoreDashboard.tsx`:**
   - Import `ConflictAnalysis` from `@/lib/api-client`
   - Add `conflictAnalysis?: ConflictAnalysis` to the props interface
   - Destructure `conflictAnalysis` in the component function with no default (optional)

3. **Render Section 4 — Matrix heatmap table** (after Section 2 stats grid, before Section 3 suggestions):
   - Guard with `{conflictAnalysis && conflictAnalysis.matrix.rows.length > 0 && conflictAnalysis.matrix.cols.length > 0 && (...)}`
   - Render an HTML `<table>` with:
     - Header row: empty corner cell + one `<th>` per `matrix.cols` (variable names) — use `text-xs text-text-secondary truncate max-w-[120px]`
     - Body rows: one per `matrix.rows` (function names) — first cell is function name label, rest are colored cells
     - Cell color logic: `cells[i][j] === 0` → `bg-surface-base`, `=== 1` → `bg-amber-500/30`, `=== 2` → `bg-amber-500/60`, `>= 3` → `bg-red-400/60`
     - Each non-zero cell displays the conflict count number in `text-xs font-medium`
     - Section header: "Conflict Matrix" with `text-text-secondary text-xs font-semibold`
   - Use only M004 design tokens — `bg-surface-base`, `bg-surface-raised`, `border-border-subtle`, `text-text-primary`, `text-text-secondary`, `text-text-muted`
   - Table should have `border-collapse` and `rounded-lg overflow-hidden`

4. **Render Section 5 — Structured suggestion cards** (replace plain suggestions when conflict data exists):
   - When `conflictAnalysis?.conflicts` exists and has length > 0, render structured cards INSTEAD of (not alongside) the plain `suggestions.map(...)` cards
   - Each structured card: `border-l-2 border-amber-500 bg-surface-base rounded-lg p-2.5` (same pattern as existing)
   - Card content:
     - Top line: variable name in `font-medium text-text-primary` + conflict type badge (`text-[10px] px-1.5 py-0.5 rounded` — amber bg for "write-write", red bg for "read-write")
     - Variable type in `text-xs text-text-muted` (e.g. "mapping(address => uint256)")
     - Functions list: `text-xs text-text-secondary` comma-separated function names with `text-accent` color
     - Suggestion text: `text-sm text-text-primary` — the actionable English text
   - When `conflictAnalysis` is absent, keep existing plain suggestion cards unchanged
   - Apply `stagger-item` class on each card for fade-in animation

5. **Wire prop in `Vibe-Loom/src/app/page.tsx`:**
   - In the `<VibeScoreDashboard>` JSX, add: `conflictAnalysis={vibeScore?.conflictAnalysis}`
   - This is a one-line addition to the existing prop spread

## Must-Haves

- [ ] `DecodedConflict`, `ConflictMatrix`, `ConflictAnalysis` interfaces in `api-client.ts` match backend DTO exactly
- [ ] `VibeScoreResult` has `conflictAnalysis?: ConflictAnalysis`
- [ ] Heatmap table renders only when `conflictAnalysis` is present with non-empty matrix
- [ ] Color scale uses design tokens — no hardcoded hex colors outside the token system
- [ ] Structured suggestion cards show variable name, type, functions, conflict type badge, and suggestion
- [ ] Plain suggestion cards still render when `conflictAnalysis` is absent
- [ ] `page.tsx` passes `conflictAnalysis` prop

## Verification

- `cd /home/ahwlsqja/Vibe-Loom && npx tsc --noEmit` — zero type errors
- `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/VibeScoreDashboard.test.tsx --verbose` — existing 10 tests still pass
- `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/api-client.test.ts --verbose` — existing 10 tests still pass

## Observability Impact

- **New test IDs:** `data-testid="conflict-matrix"` on the heatmap `<table>`, `data-testid="conflict-card"` on each structured suggestion card. These enable both test selectors and agent DOM inspection.
- **Inspect via:** React DevTools props panel for `VibeScoreDashboard` — `conflictAnalysis` prop is the single source of truth for conflict visualization state.
- **Failure state visible:** When `conflictAnalysis` is undefined or has empty matrix, the heatmap section is absent from the DOM entirely. When `conflicts` array is empty, plain suggestion cards render instead of structured ones.
- **No new console output:** Component is pure presentational — no `console.log` or error boundary additions.

## Inputs

- `Vibe-Loom/src/lib/api-client.ts` — existing `VibeScoreResult` interface to extend
- `Vibe-Loom/src/components/ide/VibeScoreDashboard.tsx` — existing component to add sections to
- `Vibe-Loom/src/app/page.tsx` — existing page that renders the dashboard component
- `Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts` — authoritative backend DTO interfaces to mirror (DO NOT modify this file — read only for reference)

## Expected Output

- `Vibe-Loom/src/lib/api-client.ts` — extended with `DecodedConflict`, `ConflictMatrix`, `ConflictAnalysis` interfaces and `VibeScoreResult.conflictAnalysis` field
- `Vibe-Loom/src/components/ide/VibeScoreDashboard.tsx` — extended with matrix heatmap table (Section 4) and structured suggestion cards (Section 5)
- `Vibe-Loom/src/app/page.tsx` — extended with `conflictAnalysis` prop pass-through
