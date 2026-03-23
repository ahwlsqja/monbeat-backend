---
id: T03
parent: S02
milestone: M002
provides:
  - AIDiffViewerInner component wrapping Monaco DiffEditor with syntax-highlighted side-by-side diff
  - AIDiffViewer SSR-safe dynamic wrapper using next/dynamic({ ssr: false })
  - page.tsx updated to use AIDiffViewer instead of CodeDiffView for AI fix suggestions
key_files:
  - frontend/src/components/ide/AIDiffViewerInner.tsx
  - frontend/src/components/ide/AIDiffViewer.tsx
  - frontend/src/app/page.tsx
key_decisions:
  - Used same two-file SSR-safe pattern as MonacoEditor (Inner + dynamic wrapper) for consistency and proven reliability
  - Kept CodeDiffView.tsx file on disk (not deleted) to avoid breaking any other imports — only removed from page.tsx
patterns_established:
  - SSR-safe DiffEditor pattern: AIDiffViewerInner exports the component, AIDiffViewer wraps with next/dynamic({ ssr: false }) — identical to MonacoEditor pattern
observability_surfaces:
  - DiffEditor renders side-by-side original vs modified code with character-level highlighting — visible in browser sidebar when AI fix is suggested
  - Apply Fix button calls onApplyFix(modified) to replace editor source — verifiable via React DevTools contractSource state change
  - Summary banner displays explanation text in amber background when provided
duration: 6m
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T03: Create AIDiffViewer with Monaco DiffEditor and replace CodeDiffView in page.tsx

**Replaced text-based CodeDiffView with Monaco DiffEditor (AIDiffViewer) for syntax-highlighted side-by-side AI fix suggestions**

## What Happened

Created two new components and updated page.tsx to complete the AI fix suggestion UX upgrade:

1. **`AIDiffViewerInner.tsx`** — Imports `DiffEditor` from `@monaco-editor/react` and renders it with `height="300px"`, `language="sol"`, `theme="vs-dark"`, read-only side-by-side mode, and minimap disabled. Includes an optional amber summary banner at top and a conditional "Apply Fix" button (emerald-600) that calls `onApplyFix(modified)` when the original and modified code differ. Loading fallback displays "Loading diff..." centered in the 300px area.

2. **`AIDiffViewer.tsx`** — Thin `next/dynamic({ ssr: false })` wrapper following the exact same pattern as `MonacoEditor.tsx`. Re-exports the `AIDiffViewerProps` type for consumer imports. Loading state shows a centered "Loading diff..." placeholder.

3. **`page.tsx`** — Replaced `import { CodeDiffView } from "../components/CodeDiffView"` with `import AIDiffViewer from "@/components/ide/AIDiffViewer"`. Swapped `<CodeDiffView>` JSX for `<AIDiffViewer>` with matching props (`original`, `modified` mapped from `errorDiff.fixed`, `summary`, `onApplyFix`). The existing `handleApplyFix` callback works unchanged.

The DiffEditor shares Monaco's core instance already loaded by MonacoEditor, so no additional bundle cost.

## Verification

- `npm run build` exits 0 — no SSR crashes from DiffEditor dynamic import
- `npm test` — all 30 tests pass (11 api-client + 7 useTransactionLog + 12 solc-error-parser)
- All file existence and grep checks pass — both new files exist, AIDiffViewer referenced in page.tsx, CodeDiffView removed from page.tsx, ssr:false present in wrapper, DiffEditor present in inner component
- All 14 slice-level verification checks pass (this is the final task of S02)

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd frontend && npm run build` | 0 | ✅ pass | 52s |
| 2 | `cd frontend && npm test` | 0 | ✅ pass | 1.5s |
| 3 | `test -f frontend/src/components/ide/AIDiffViewer.tsx` | 0 | ✅ pass | <1s |
| 4 | `test -f frontend/src/components/ide/AIDiffViewerInner.tsx` | 0 | ✅ pass | <1s |
| 5 | `grep -q "AIDiffViewer" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 6 | `! grep -q "CodeDiffView" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 7 | `grep -q "ssr: false" frontend/src/components/ide/AIDiffViewer.tsx` | 0 | ✅ pass | <1s |
| 8 | `grep -q "DiffEditor" frontend/src/components/ide/AIDiffViewerInner.tsx` | 0 | ✅ pass | <1s |
| 9 | `grep -q "compileContract" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 10 | `grep -q "setModelMarkers" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 11 | `grep -q "TransactionConsole" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 12 | `grep -q "contractName" frontend/src/lib/api-client.ts` | 0 | ✅ pass | <1s |
| 13 | `test -f frontend/src/hooks/useTransactionLog.ts` | 0 | ✅ pass | <1s |
| 14 | `test -f frontend/src/lib/solc-error-parser.ts` | 0 | ✅ pass | <1s |

## Diagnostics

- **DiffEditor rendering**: In browser, trigger a deploy error on FailingContract → AI fix suggestion renders as Monaco DiffEditor in sidebar with syntax-highlighted side-by-side diff.
- **Apply Fix**: Clicking the "Apply Fix" button calls `onApplyFix(modified)` which sets `contractSource` — verifiable via React DevTools state inspection.
- **SSR safety**: Build confirms no SSR crashes; the `next/dynamic({ ssr: false })` wrapper prevents `window`/`document` access during server-side rendering.

## Deviations

None — implementation followed the task plan exactly.

## Known Issues

- `CodeDiffView.tsx` file remains on disk (intentionally not deleted per plan) — it is no longer imported by page.tsx but may be referenced elsewhere.
- Pre-existing wagmi connector warnings in build output are unrelated to this task.

## Files Created/Modified

- `frontend/src/components/ide/AIDiffViewerInner.tsx` — new: Monaco DiffEditor implementation with summary banner and Apply Fix button
- `frontend/src/components/ide/AIDiffViewer.tsx` — new: SSR-safe next/dynamic wrapper for AIDiffViewerInner
- `frontend/src/app/page.tsx` — modified: replaced CodeDiffView import and usage with AIDiffViewer
