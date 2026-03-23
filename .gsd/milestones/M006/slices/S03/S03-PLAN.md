# S03: Vibe-Loom — 매트릭스 히트맵 + Suggestion 카드 UI

**Goal:** VibeScoreDashboard renders a function×variable conflict matrix heatmap and structured suggestion cards when `conflictAnalysis` data is present in the API response, while preserving all existing functionality when the field is absent.
**Demo:** Given a mock `conflictAnalysis` with 2 functions × 2 variables, the heatmap table renders colored cells (amber/red scale by conflict count), and each decoded conflict shows a structured card with variable name, function list, conflict type badge, and actionable suggestion text. When `conflictAnalysis` is undefined, the dashboard renders identically to the current version.

## Must-Haves

- `DecodedConflict`, `ConflictMatrix`, `ConflictAnalysis` interfaces in `api-client.ts` mirroring the backend DTO exactly
- `VibeScoreResult.conflictAnalysis?: ConflictAnalysis` field added
- `VibeScoreDashboard` accepts optional `conflictAnalysis` prop
- Matrix heatmap table renders below stats grid when `conflictAnalysis` is present, with color-coded cells using M004 design tokens (oklch surface/amber/red)
- Structured suggestion cards replace plain text suggestions when `conflictAnalysis.conflicts` exists, showing variable name, function list, conflict type, and actionable suggestion
- Empty matrix guard — no heatmap rendered when `matrix.rows` or `matrix.cols` is empty
- All 10 existing `VibeScoreDashboard` tests pass unchanged
- `page.tsx` passes `conflictAnalysis` prop from `vibeScore` state to the dashboard component

## Proof Level

- This slice proves: operational
- Real runtime required: no (unit tests with mock data sufficient; E2E deferred to S04)
- Human/UAT required: yes (heatmap visual clarity and suggestion usefulness require human review — deferred to S04)

## Verification

- `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/VibeScoreDashboard.test.tsx --verbose` — All 10 existing tests pass + 5+ new tests pass
- `cd /home/ahwlsqja/Vibe-Loom && npx tsc --noEmit` — TypeScript type-check passes with new interfaces
- `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/api-client.test.ts --verbose` — Existing 10 tests still pass (type-only changes must not break)
- Backward compat: test confirms heatmap section absent when `conflictAnalysis` prop is undefined

## Integration Closure

- Upstream surfaces consumed: `Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts` (DecodedConflict, ConflictMatrix, ConflictAnalysis interfaces — mirrored as frontend types)
- New wiring introduced in this slice: `page.tsx` passes `vibeScore.conflictAnalysis` → `VibeScoreDashboard` as new prop
- What remains before the milestone is truly usable end-to-end: S04 E2E test with real ParallelConflict contract through the full pipeline

## Tasks

- [x] **T01: Add conflict analysis types, heatmap rendering, suggestion cards, and page wiring** `est:45m`
  - Why: This is the entire implementation — types, component rendering, and prop wiring. Without this, the dashboard has no conflict visualization.
  - Files: `Vibe-Loom/src/lib/api-client.ts`, `Vibe-Loom/src/components/ide/VibeScoreDashboard.tsx`, `Vibe-Loom/src/app/page.tsx`
  - Do: (1) Add `DecodedConflict`, `ConflictMatrix`, `ConflictAnalysis` interfaces to `api-client.ts` mirroring backend DTO. Add `conflictAnalysis?: ConflictAnalysis` to `VibeScoreResult`. (2) Add `conflictAnalysis?: ConflictAnalysis` to `VibeScoreDashboardProps`. Render Section 4 (matrix heatmap as HTML table with oklch color scale) and Section 5 (structured suggestion cards with variable name, functions, conflict type badge). Guard with `{conflictAnalysis && ...}`. (3) Pass `vibeScore?.conflictAnalysis` prop in `page.tsx`. Use design tokens only — no hardcoded hex. Use `stagger-item` for card animations.
  - Verify: `cd /home/ahwlsqja/Vibe-Loom && npx tsc --noEmit` passes
  - Done when: TypeScript compiles cleanly, heatmap and suggestion card JSX exists in the component, prop is wired in page.tsx

- [ ] **T02: Add unit tests for heatmap, structured suggestion cards, and backward compatibility** `est:30m`
  - Why: Verifies all new rendering paths and confirms existing tests remain green. This is the slice's objective stopping condition.
  - Files: `Vibe-Loom/src/__tests__/VibeScoreDashboard.test.tsx`
  - Do: Add 5+ tests: (1) heatmap table renders when `conflictAnalysis` provided with mock data, (2) heatmap absent when `conflictAnalysis` undefined, (3) structured suggestion cards show variable names and function names, (4) conflict type badge rendered, (5) empty matrix guard — no table when rows/cols empty. All existing 10 tests must pass unchanged.
  - Verify: `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/VibeScoreDashboard.test.tsx --verbose` — all tests pass
  - Done when: 15+ total tests pass (10 existing + 5+ new), zero failures

## Observability / Diagnostics

- **Runtime signals:** When `conflictAnalysis` is present in the API response, the heatmap section and structured suggestion cards render. When absent, the component renders identically to the pre-S03 version — no console output, no side effects. All rendering is pure/presentational.
- **Inspection surface:** React DevTools → `VibeScoreDashboard` props → `conflictAnalysis` prop shows the full conflict matrix and decoded conflicts object. DOM inspection: `data-testid="conflict-matrix"` on the heatmap table, `data-testid="conflict-card"` on each structured card.
- **Failure visibility:** If `conflictAnalysis` contains an empty `matrix.rows` or `matrix.cols`, the heatmap section is suppressed (guard clause). If `conflicts` array is empty, plain suggestion cards render as fallback. No error boundary needed — the component is stateless.
- **Redaction:** No secrets or PII in conflict analysis data — all fields are contract-level (variable names, function names, slot identifiers).

## Verification (diagnostic/failure-path)

- Backward compatibility: rendering with `conflictAnalysis={undefined}` produces identical DOM to pre-S03 — verified by existing test suite passing unchanged.
- Empty matrix guard: rendering with `conflictAnalysis={{ conflicts: [], matrix: { rows: [], cols: [], cells: [] } }}` produces no heatmap table — verified by unit test.

## Files Likely Touched

- `Vibe-Loom/src/lib/api-client.ts`
- `Vibe-Loom/src/components/ide/VibeScoreDashboard.tsx`
- `Vibe-Loom/src/app/page.tsx`
- `Vibe-Loom/src/__tests__/VibeScoreDashboard.test.tsx`
