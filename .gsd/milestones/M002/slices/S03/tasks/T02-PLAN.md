---
estimated_steps: 4
estimated_files: 3
---

# T02: VibeScoreResult 타입 확장 + VibeScoreDashboard 컴포넌트 구현

**Slice:** S03 — 컨트랙트 인터랙션 + Vibe-Score 대시보드
**Milestone:** M002

## Description

Replace the existing `VibeScoreGauge` with a richer `VibeScoreDashboard` that shows a circular score gauge, conflict/reExecution/gasEfficiency stats, and suggestion cards. First expand the `VibeScoreResult` type in `api-client.ts` to include the additional fields the backend already returns but weren't typed.

This is pure UI work — no wallet, no chain interaction. The existing `VibeScoreGauge` component (`frontend/src/components/VibeScoreGauge.tsx`) has the score→color logic that can be reused.

**Relevant skills:** react-best-practices, frontend-design

## Steps

1. **Expand `VibeScoreResult` in `frontend/src/lib/api-client.ts`**:
   - Add optional fields to the existing `VibeScoreResult` interface:
     ```typescript
     export interface VibeScoreResult {
       vibeScore: number;
       suggestions: string[];
       engineBased?: boolean;
       // New fields — backend already returns these, just not typed
       conflicts?: number;
       reExecutions?: number;
       gasEfficiency?: number;
     }
     ```
   - This is backward compatible — existing code only reads `vibeScore` and `suggestions`

2. **Create `frontend/src/components/ide/VibeScoreDashboard.tsx`**:
   - Props interface:
     ```typescript
     interface VibeScoreDashboardProps {
       score: number;
       suggestions?: string[];
       conflicts?: number;
       reExecutions?: number;
       gasEfficiency?: number;
       engineBased?: boolean;
       loading?: boolean;
     }
     ```
   - **Loading state**: Skeleton with pulsing circle + stat boxes + card placeholders (similar to VibeScoreGauge loading but richer)
   - **Section 1 — Circular gauge**: Large circular score display with SVG ring. Color coding: ≥80 emerald, ≥60 amber, <60 red. Score number centered. Brief text description below
   - **Section 2 — Stats grid**: 3-column grid showing:
     - Conflicts: number with collision icon, label "Conflicts"
     - Re-Executions: number with retry icon, label "Re-Executions"
     - Gas Efficiency: percentage with gas icon, label "Gas Efficiency"
     - Each stat as a compact card with `bg-gray-900` background
     - Show "—" when value is undefined (data not available)
   - **Section 3 — Suggestion cards**: Each suggestion as a numbered card with amber accent. Icon per card (lightbulb or similar via emoji). Truncate long suggestions with `line-clamp-2`
   - **Engine badge**: If `engineBased` is true, show a small "Engine-Based" badge near the title
   - Container: `bg-gray-800 rounded-xl border border-gray-700 p-4` consistent with sidebar style
   - Export as named export: `export function VibeScoreDashboard(...)`

3. **Create `frontend/src/__tests__/VibeScoreDashboard.test.tsx`** — 5+ render tests:
   - Renders score in the gauge (text "85" visible when score=85)
   - Renders suggestions as cards (check each suggestion text appears)
   - Renders stats grid with conflict/reExecution/gasEfficiency values
   - Shows loading skeleton when `loading=true` (check for "분석 중..." text)
   - Shows "—" for undefined stats (render with no conflicts/reExecutions/gasEfficiency)
   - Shows "Engine-Based" badge when `engineBased=true`
   - Use `@testing-library/react` `render` + `screen.getByText` / `screen.queryByText` patterns
   - Mock nothing — this is a pure presentational component

4. **Verify**:
   - Run `cd frontend && npm test -- --testPathPatterns VibeScoreDashboard` — all tests pass
   - Run `cd frontend && npm test -- --testPathPatterns api-client` — existing api-client tests still pass (backward compat)
   - Check file exists: `test -f frontend/src/components/ide/VibeScoreDashboard.tsx`

## Must-Haves

- [ ] `VibeScoreResult` in api-client.ts has `conflicts?`, `reExecutions?`, `gasEfficiency?` optional fields
- [ ] VibeScoreDashboard renders circular gauge with color-coded score
- [ ] VibeScoreDashboard renders 3-stat grid (conflicts, reExecutions, gasEfficiency)
- [ ] VibeScoreDashboard renders suggestion cards
- [ ] Loading skeleton renders when `loading=true`
- [ ] Undefined stats show "—" fallback (not 0, not crash)
- [ ] 5+ render tests pass
- [ ] Existing api-client tests unaffected

## Verification

- `cd frontend && npm test -- --testPathPatterns VibeScoreDashboard` — all tests pass
- `cd frontend && npm test -- --testPathPatterns api-client` — existing 11 tests still pass
- `test -f frontend/src/components/ide/VibeScoreDashboard.tsx` — file exists
- `grep -q "conflicts" frontend/src/lib/api-client.ts` — type expanded

## Inputs

- `frontend/src/lib/api-client.ts` — existing VibeScoreResult type to expand
- `frontend/src/components/VibeScoreGauge.tsx` — reference for score→color logic and existing UI patterns

## Expected Output

- `frontend/src/lib/api-client.ts` — modified: VibeScoreResult with 3 new optional fields
- `frontend/src/components/ide/VibeScoreDashboard.tsx` — new: rich dashboard component
- `frontend/src/__tests__/VibeScoreDashboard.test.tsx` — new: 5+ render tests
