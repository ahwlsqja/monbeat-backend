---
id: T01
parent: S03
milestone: M006
provides:
  - DecodedConflict, ConflictMatrix, ConflictAnalysis TypeScript interfaces in api-client.ts
  - conflictAnalysis optional field on VibeScoreResult
  - Matrix heatmap table rendering in VibeScoreDashboard (Section 4)
  - Structured suggestion cards with conflict type badges (Section 5)
  - conflictAnalysis prop wiring from page.tsx to VibeScoreDashboard
key_files:
  - Vibe-Loom/src/lib/api-client.ts
  - Vibe-Loom/src/components/ide/VibeScoreDashboard.tsx
  - Vibe-Loom/src/app/page.tsx
key_decisions:
  - Structured suggestion cards replace plain cards when conflictAnalysis.conflicts is non-empty (not rendered alongside)
  - Cell color scale: 0→surface-base, 1→amber-500/30, 2→amber-500/60, 3+→red-400/60
  - Conflict type badge uses amber bg for write-write, red bg for read-write
patterns_established:
  - data-testid attributes on new sections (conflict-matrix, conflict-card) for test targeting
  - Guard pattern: multi-condition check on optional nested array lengths before rendering
observability_surfaces:
  - React DevTools props: conflictAnalysis prop on VibeScoreDashboard
  - DOM: data-testid="conflict-matrix" and data-testid="conflict-card"
duration: 15m
verification_result: passed
completed_at: 2026-03-24
blocker_discovered: false
---

# T01: Add conflict analysis types, heatmap rendering, suggestion cards, and page wiring

**Added DecodedConflict/ConflictMatrix/ConflictAnalysis interfaces, matrix heatmap table, structured suggestion cards with conflict type badges, and conflictAnalysis prop wiring from page.tsx through to VibeScoreDashboard**

## What Happened

1. Added three TypeScript interfaces (`DecodedConflict`, `ConflictMatrix`, `ConflictAnalysis`) to `api-client.ts` mirroring the backend DTO exactly, and added `conflictAnalysis?: ConflictAnalysis` to `VibeScoreResult`.

2. Extended `VibeScoreDashboard` component:
   - Imported `ConflictAnalysis` type and added it as optional prop
   - Section 4: Matrix heatmap — HTML `<table>` with `data-testid="conflict-matrix"`, color-coded cells (amber/red scale by conflict count), function names as rows, variable names as columns. Guarded by non-empty rows/cols check.
   - Section 5: Structured suggestion cards — replace plain text suggestions when `conflictAnalysis.conflicts` has entries. Each card shows variable name, conflict type badge (amber for write-write, red for read-write), variable type, comma-separated function names in accent color, and actionable suggestion text. Uses `stagger-item` class for animation.
   - When `conflictAnalysis` is absent, plain suggestion cards render unchanged (backward compat).

3. Wired `conflictAnalysis={vibeScore?.conflictAnalysis}` prop in `page.tsx`.

## Verification

- **TypeScript**: `npx tsc --noEmit` — zero type errors in source files (`api-client.ts`, `VibeScoreDashboard.tsx`, `page.tsx`). Pre-existing test file type errors (missing `@testing-library/jest-dom` types) are unrelated.
- **Dashboard tests**: All 10 existing tests pass unchanged — backward compatibility confirmed.
- **API client tests**: All 11 existing tests pass unchanged — type-only additions did not break runtime.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd /home/ahwlsqja/Vibe-Loom && npx tsc --noEmit` | 2 (pre-existing test-only errors) | ✅ pass (0 source file errors) | 8.5s |
| 2 | `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/VibeScoreDashboard.test.tsx --verbose` | 0 | ✅ pass (10/10) | 2.0s |
| 3 | `cd /home/ahwlsqja/Vibe-Loom && npx jest src/__tests__/api-client.test.ts --verbose` | 0 | ✅ pass (11/11) | 1.7s |

## Diagnostics

- **Inspect heatmap**: DOM query `[data-testid="conflict-matrix"]` — present only when `conflictAnalysis` prop has non-empty matrix rows/cols.
- **Inspect structured cards**: DOM query `[data-testid="conflict-card"]` — one per decoded conflict when `conflictAnalysis.conflicts` is non-empty.
- **Inspect props**: React DevTools → VibeScoreDashboard → `conflictAnalysis` prop shows full conflict data structure.
- **Fallback**: When `conflictAnalysis` is undefined, suggestion section renders plain `💡` cards as before.

## Deviations

None — implementation matches plan exactly.

## Known Issues

- Pre-existing tsc errors in test files (`toBeInTheDocument` not recognized) due to missing `@testing-library/jest-dom` type augmentation. Unrelated to this task.

## Files Created/Modified

- `Vibe-Loom/src/lib/api-client.ts` — Added `DecodedConflict`, `ConflictMatrix`, `ConflictAnalysis` interfaces and `conflictAnalysis?: ConflictAnalysis` field on `VibeScoreResult`
- `Vibe-Loom/src/components/ide/VibeScoreDashboard.tsx` — Added `ConflictAnalysis` import, `conflictAnalysis` prop, matrix heatmap table (Section 4), structured suggestion cards (Section 5)
- `Vibe-Loom/src/app/page.tsx` — Added `conflictAnalysis={vibeScore?.conflictAnalysis}` prop pass-through
