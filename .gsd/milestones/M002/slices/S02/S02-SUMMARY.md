---
id: S02
parent: M002
milestone: M002
provides:
  - Compile button in toolbar calling compileContract() API with Monaco inline error markers via setModelMarkers
  - solc-error-parser utility extracting line/column/severity from solc formattedMessage into MonacoMarker[]
  - useTransactionLog hook managing timestamped compile/deploy/call event log with addEntry/clearLog
  - TransactionConsole component rendering color-coded log entries in ConsolePanel
  - AIDiffViewer (SSR-safe Monaco DiffEditor) replacing text-based CodeDiffView for AI fix suggestions
  - CompileResult.contractName field for S03 contract interaction consumption
  - monacoInstance state in page.tsx enabling marker operations across compile/deploy flows
requires:
  - slice: S01
    provides: MonacoEditor wrapper (editorRef + monaco namespace), IDE 3-panel layout (ConsolePanel slot), api-client.ts (compileContract method)
affects:
  - S03
key_files:
  - frontend/src/hooks/useTransactionLog.ts
  - frontend/src/components/ide/TransactionConsole.tsx
  - frontend/src/lib/solc-error-parser.ts
  - frontend/src/components/ide/AIDiffViewer.tsx
  - frontend/src/components/ide/AIDiffViewerInner.tsx
  - frontend/src/app/page.tsx
  - frontend/src/lib/api-client.ts
key_decisions:
  - Entries prepend (newest first) for natural console ordering — most recent events visible without scrolling
  - Unparseable solc errors produce fallback markers at line 1 col 1 rather than being silently dropped
  - Keep compileContract as-is (throws on error) and catch in page.tsx — error.message passed to parseSolcErrors for marker extraction
  - Markers cleared on every contractSource change via useEffect (instant clear prevents stale markers)
  - Same two-file SSR-safe pattern (Inner + dynamic wrapper) used for AIDiffViewer as established by MonacoEditor
  - CodeDiffView.tsx file kept on disk (not deleted) — only removed from page.tsx imports
patterns_established:
  - Monaco marker integration: store monacoInstance alongside editorInstance via handleEditorReady, call monacoInstance.editor.setModelMarkers(model, owner, markers) for inline error decoration
  - Transaction logging: addEntry() at three points in each async flow (pending at start, success/error in try/catch branches)
  - SSR-safe DiffEditor: same two-file pattern as MonacoEditor (AIDiffViewerInner exports component, AIDiffViewer wraps with next/dynamic ssr:false)
observability_surfaces:
  - monacoInstance.editor.getModelMarkers({owner:'solc'}) in browser console shows active compile error markers
  - React DevTools: useTransactionLog entries array shows all compile/deploy events with timestamps and status
  - TransactionConsole: color-coded visual indicators (emerald/red/amber) for success/error/pending with type badges (blue/purple/cyan)
  - DiffEditor renders side-by-side original vs modified code with character-level highlighting in sidebar
drill_down_paths:
  - .gsd/milestones/M002/slices/S02/tasks/T01-SUMMARY.md
  - .gsd/milestones/M002/slices/S02/tasks/T02-SUMMARY.md
  - .gsd/milestones/M002/slices/S02/tasks/T03-SUMMARY.md
duration: 26m
verification_result: passed
completed_at: 2026-03-23
---

# S02: 컴파일·배포 UX + 트랜잭션 콘솔

**Compile button with Monaco inline error markers, TransactionConsole event log, and Monaco DiffEditor for AI fix suggestions — completing the compile→deploy→analyze UX pipeline**

## What Happened

Built the compile/deploy UX layer in three tasks across 26 minutes:

**T01 (12m)** laid the foundation with three pure units: `useTransactionLog` hook (manages timestamped entries with addEntry/clearLog, newest-first ordering), `TransactionConsole` component (color-coded list with emerald/red/amber status indicators, type badges, HH:MM:SS timestamps, expandable details), and `solc-error-parser` utility (regex extraction of line/column/severity from solc `formattedMessage` strings into `MonacoMarker[]`, with fallback markers at line 1 for unmatched strings). All three came with 19 tests.

**T02 (8m)** wired everything into page.tsx: stored the `monaco` namespace (previously discarded as `_monaco`) in `monacoInstance` state, added a "Compile" button to the toolbar calling `compileContract()`, set up Monaco inline markers via `setModelMarkers()` on compile failure with parsed solc errors, replaced the console placeholder with `<TransactionConsole>`, and logged both compile and deploy events through `useTransactionLog`. Added `contractName` to `CompileResult` for S03 consumption. Added `useEffect` to clear markers on source change.

**T03 (6m)** replaced the text-based `CodeDiffView` with a Monaco `DiffEditor` for AI fix suggestions. Created `AIDiffViewerInner.tsx` (Monaco DiffEditor with summary banner and "Apply Fix" button) and `AIDiffViewer.tsx` (SSR-safe `next/dynamic` wrapper), following the exact same two-file pattern established for `MonacoEditor`. Updated page.tsx to import `AIDiffViewer` and removed all `CodeDiffView` references.

## Verification

- **Build**: `npm run build` exits 0 — no SSR crashes from DiffEditor dynamic import
- **Tests**: 30/30 pass (11 existing api-client + 7 useTransactionLog + 12 solc-error-parser)
- **File existence**: All 5 new files verified (useTransactionLog.ts, TransactionConsole.tsx, solc-error-parser.ts, AIDiffViewer.tsx, AIDiffViewerInner.tsx)
- **Integration checks**: compileContract in page.tsx ✅, setModelMarkers in page.tsx ✅, TransactionConsole in page.tsx ✅, AIDiffViewer in page.tsx ✅, CodeDiffView removed from page.tsx ✅, contractName in api-client.ts ✅, ssr:false in AIDiffViewer.tsx ✅

## New Requirements Surfaced

- none

## Deviations

- Installed `@testing-library/dom` as a missing peer dependency for `@testing-library/react` v16 — required for hook tests to run. Not in the plan but necessary. Already documented in KNOWLEDGE.md.

## Known Limitations

- `CodeDiffView.tsx` file still exists on disk — only removed from page.tsx imports. Could be deleted in a cleanup pass.
- `compileResult` state is set but not yet consumed by contract interaction UI — S03 will wire it to ABI-based function call forms.
- Pre-existing wagmi connector type warnings in build output remain (unrelated to S02).

## Follow-ups

- S03 consumes `compileResult` (ABI, bytecode, contractName) for contract interaction UI
- S03 consumes `useTransactionLog.addEntry()` for logging contract call results
- Consider deleting `CodeDiffView.tsx` file entirely in S04 cleanup pass

## Files Created/Modified

- `frontend/src/hooks/useTransactionLog.ts` — new: transaction log state hook (addEntry/clearLog, newest-first ordering)
- `frontend/src/components/ide/TransactionConsole.tsx` — new: color-coded console log renderer with expandable details
- `frontend/src/lib/solc-error-parser.ts` — new: solc formattedMessage → MonacoMarker[] parser with fallback
- `frontend/src/components/ide/AIDiffViewer.tsx` — new: SSR-safe next/dynamic wrapper for DiffEditor
- `frontend/src/components/ide/AIDiffViewerInner.tsx` — new: Monaco DiffEditor with summary banner and Apply Fix button
- `frontend/src/app/page.tsx` — modified: Compile button, monacoInstance state, setModelMarkers, TransactionConsole, AIDiffViewer (replacing CodeDiffView), deploy event logging
- `frontend/src/lib/api-client.ts` — modified: CompileResult.contractName added
- `frontend/src/__tests__/useTransactionLog.test.ts` — new: 7 hook state transition tests
- `frontend/src/__tests__/solc-error-parser.test.ts` — new: 12 parser regex + edge case tests
- `frontend/package.json` — modified: added @testing-library/dom dev dependency

## Forward Intelligence

### What the next slice should know
- `compileResult` state in page.tsx holds `{ abi, bytecode, contractName }` after successful compile — S03 reads this directly for contract interaction UI.
- `useTransactionLog` hook is already instantiated in page.tsx with `{ entries, addEntry, clearLog }` — S03 just calls `addEntry()` to log contract call results.
- `monacoInstance` is stored in page.tsx state — if S03 needs marker operations for contract call errors, the same `setModelMarkers` pattern applies.
- The `AIDiffViewer` component accepts `original`, `modified`, `summary`, and `onApplyFix` props — same interface could be reused for future diff scenarios.

### What's fragile
- `solc-error-parser` relies on regex matching of solc's `formattedMessage` format (`file:line:col: Severity: msg`). If the backend changes how it formats error messages (e.g., wrapping them in an error object instead of passing raw formattedMessage), the regex will miss and fall back to line-1 markers.
- The `monacoInstance` state must be set before `handleCompile` is called — if the editor hasn't mounted yet (loading state), `setModelMarkers` will silently skip. There's a guard (`if (monacoInstance && editorInstance)`) but no user feedback when markers can't be set.

### Authoritative diagnostics
- `monacoInstance.editor.getModelMarkers({owner:'solc'})` in browser console — shows exactly what markers are active and their positions. This is the ground truth for compile error display.
- React DevTools → page component → `entries` state from `useTransactionLog` — shows full event history with timestamps, status, and details.

### What assumptions changed
- Plan assumed the `monaco` namespace was not accessible — it was already passed to `handleEditorReady` but ignored as `_monaco`. Simple rename to store it in state, no API change needed.
- DiffEditor shares Monaco's core instance with the main editor — no additional ~2MB bundle cost. The `@monaco-editor/react` package handles instance sharing automatically via CDN loader.
