---
id: T02
parent: S02
milestone: M002
provides:
  - Compile button in toolbar that calls compileContract() API
  - Monaco inline error markers via setModelMarkers on compile failure
  - TransactionConsole rendering in ConsolePanel (replaces placeholder)
  - Deploy and compile events logged to useTransactionLog
  - monacoInstance state stored for marker operations
  - CompileResult.contractName field for downstream S03 consumption
key_files:
  - frontend/src/app/page.tsx
  - frontend/src/lib/api-client.ts
key_decisions:
  - Keep compileContract as-is (throws on error) and catch in page.tsx — error.message passed to parseSolcErrors for marker extraction rather than creating a separate compile-with-errors wrapper
  - Markers cleared on every contractSource change via useEffect (no debounce needed — instant clear prevents stale markers from persisting)
patterns_established:
  - Monaco marker integration pattern: store monacoInstance alongside editorInstance via handleEditorReady, then call monacoInstance.editor.setModelMarkers(model, owner, markers) for inline error decoration
  - Transaction logging pattern: addEntry() called at three points in each async flow (pending at start, success/error in try/catch branches)
observability_surfaces:
  - monacoInstance.editor.getModelMarkers({owner:'solc'}) in browser console shows active compile error markers
  - useTransactionLog entries array in React DevTools shows full compile/deploy event history with timestamps and status
  - TransactionConsole visually renders color-coded entries (blue/compile, purple/deploy) with expandable details
duration: 8m
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T02: Wire Compile button + inline error markers + transaction logging into page.tsx

**Wired Compile button, Monaco inline error markers via setModelMarkers, and TransactionConsole into page.tsx with deploy/compile event logging**

## What Happened

Modified two files to complete the compile UX pipeline:

1. **`api-client.ts`** — Added `contractName: string` field to `CompileResult` interface (backend returns it, S03 needs it for contract interaction UI).

2. **`page.tsx`** — Comprehensive wiring of the compile/deploy UX:
   - Fixed `handleEditorReady` to store the `monaco` namespace (was ignored as `_monaco`) in new `monacoInstance` state — required for `setModelMarkers()`.
   - Added `handleCompile` async function: calls `compileContract()`, on success stores result and logs success entry, on error parses the error message with `parseSolcErrors()` and sets Monaco inline markers plus logs error entry.
   - Added "Compile" button (blue, `bg-blue-600`) to toolbar between contract selector and Deploy button with Korean loading text ("컴파일 중...").
   - Replaced console placeholder (`"Console output will appear here..."`) with `<TransactionConsole entries={entries} />` using the `useTransactionLog` hook.
   - Added `useEffect` to clear Monaco markers when `contractSource` changes — prevents stale error markers from lingering after edits.
   - Wired deploy event logging into `handleDeploy`: pending entry at start, success entry with address/txHash on success, error entry with message on failure.

## Verification

- `npm run build` exits 0 — no SSR crashes, no type errors
- `npm test` — all 30 tests pass (11 api-client + 7 useTransactionLog + 12 solc-error-parser)
- All 7 T02-scoped grep checks pass (compileContract, setModelMarkers, TransactionConsole, monacoInstance, contractName, placeholder removed)
- 5 T03-scoped slice checks expected to fail (AIDiffViewer not yet created, CodeDiffView not yet replaced)

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd frontend && npm run build` | 0 | ✅ pass | 60s |
| 2 | `cd frontend && npm test` | 0 | ✅ pass | 1.7s |
| 3 | `grep -q "compileContract" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 4 | `grep -q "setModelMarkers" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 5 | `grep -q "TransactionConsole" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 6 | `grep -q "monacoInstance\|setMonacoInstance" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 7 | `grep -q "contractName" frontend/src/lib/api-client.ts` | 0 | ✅ pass | <1s |
| 8 | `! grep -q "Console output will appear here" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 9 | `test -f frontend/src/hooks/useTransactionLog.ts` | 0 | ✅ pass | <1s |
| 10 | `test -f frontend/src/components/ide/TransactionConsole.tsx` | 0 | ✅ pass | <1s |
| 11 | `test -f frontend/src/lib/solc-error-parser.ts` | 0 | ✅ pass | <1s |
| 12 | `test -f frontend/src/components/ide/AIDiffViewer.tsx` | 1 | ⏳ skip (T03) | <1s |
| 13 | `test -f frontend/src/components/ide/AIDiffViewerInner.tsx` | 1 | ⏳ skip (T03) | <1s |
| 14 | `grep -q "AIDiffViewer" frontend/src/app/page.tsx` | 1 | ⏳ skip (T03) | <1s |
| 15 | `! grep -q "CodeDiffView" frontend/src/app/page.tsx` | 1 | ⏳ skip (T03) | <1s |

## Diagnostics

- **Monaco markers**: In browser console, run `monaco.editor.getModelMarkers({owner:'solc'})` to inspect active error markers after a failed compile.
- **Transaction log state**: React DevTools → inspect any component using `useTransactionLog` → entries array shows all compile/deploy events with id, type, timestamp, status, message, and details.
- **TransactionConsole rendering**: Visual inspection — entries render inside ConsolePanel with color-coded borders (emerald=success, red=error, amber=pending) and type badges (blue=compile, purple=deploy).
- **Compile button state**: Button shows "컴파일 중..." while compiling, disabled when source is empty or compile is in progress.

## Deviations

None — implementation followed the task plan exactly.

## Known Issues

- `CodeDiffView` is still imported and used in page.tsx — will be replaced by `AIDiffViewer` in T03.
- The `compileResult` state is set but not consumed elsewhere in page.tsx yet — S03 will use it for contract interaction UI (ABI-based function calls).
- Pre-existing wagmi connector warnings in build output (`porto/internal`, `@metamask/sdk`, etc.) are unrelated to this task.

## Files Created/Modified

- `frontend/src/app/page.tsx` — modified: added Compile button, monacoInstance state, handleCompile with setModelMarkers, useTransactionLog + TransactionConsole wiring, deploy event logging, marker-clear-on-source-change useEffect
- `frontend/src/lib/api-client.ts` — modified: added contractName to CompileResult interface
- `.gsd/milestones/M002/slices/S02/tasks/T02-PLAN.md` — modified: added Observability Impact section (pre-flight fix)
