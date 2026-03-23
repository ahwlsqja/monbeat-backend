# S02: 컴파일·배포 UX + 트랜잭션 콘솔

**Goal:** Compile button triggers `compileContract()` API with error results shown as Monaco inline markers, deploy/compile events logged in a TransactionConsole inside the ConsolePanel, and AI fix suggestions displayed via Monaco DiffEditor replacing the text-based CodeDiffView.
**Demo:** User clicks "Compile" → compile errors appear as red squiggly markers in the editor at the correct line + column → error details appear in the TransactionConsole below the editor → AI fix suggestions render as a side-by-side Monaco diff with "Apply Fix" button.

## Must-Haves

- Compile button in the editor toolbar that calls `compileContract()` from api-client
- Solc error parser that extracts line/column from `formattedMessage` strings for Monaco markers
- `monaco.editor.setModelMarkers()` called with parsed error data on compile failure
- `useTransactionLog` hook managing an array of timestamped compile/deploy/error entries
- `TransactionConsole` component rendering log entries with color-coded severity inside ConsolePanel
- `AIDiffViewer` using Monaco `DiffEditor` via `next/dynamic({ ssr: false })` (same two-file SSR-safe pattern as MonacoEditor)
- `CodeDiffView` replaced by `AIDiffViewer` in page.tsx for AI fix suggestions
- Monaco namespace (`monaco`) stored alongside `editorInstance` in page.tsx state (needed for `setModelMarkers`)
- `CompileResult` interface updated with `contractName` field (for S03 consumption)
- All 7 existing API methods preserved — no API integration changes
- `npm run build` exits 0 (no SSR errors)
- All existing tests pass + new tests for `useTransactionLog` and `solc-error-parser`

## Proof Level

- This slice proves: contract (component-level behavior) + integration (compile→markers→console pipeline)
- Real runtime required: no (build + unit tests sufficient; live API optional)
- Human/UAT required: no

## Verification

- `cd frontend && npm run build` exits 0 — no SSR crashes from DiffEditor dynamic import
- `cd frontend && npm test` — all existing 11 tests pass + new tests for useTransactionLog hook and solc-error-parser
- `test -f frontend/src/hooks/useTransactionLog.ts` — hook file exists
- `test -f frontend/src/components/ide/TransactionConsole.tsx` — console component exists
- `test -f frontend/src/lib/solc-error-parser.ts` — parser utility exists
- `test -f frontend/src/components/ide/AIDiffViewer.tsx` — SSR-safe DiffEditor wrapper exists
- `test -f frontend/src/components/ide/AIDiffViewerInner.tsx` — DiffEditor implementation exists
- `grep -q "compileContract" frontend/src/app/page.tsx` — compile API wired into page
- `grep -q "setModelMarkers" frontend/src/app/page.tsx` — inline error markers implemented
- `grep -q "TransactionConsole" frontend/src/app/page.tsx` — console component used in page
- `grep -q "AIDiffViewer" frontend/src/app/page.tsx` — new diff viewer used in page
- `! grep -q "CodeDiffView" frontend/src/app/page.tsx` — old text diff replaced
- `grep -q "contractName" frontend/src/lib/api-client.ts` — CompileResult updated
- `grep -q 'ssr: false' frontend/src/components/ide/AIDiffViewer.tsx` — SSR-safe pattern used

## Observability / Diagnostics

- Runtime signals: TransactionConsole entries with timestamps and status (success/error/pending) — visible in browser
- Inspection surfaces: React DevTools shows `useTransactionLog` hook state; Console entries render compile/deploy history
- Failure visibility: Compile errors show in both inline markers (editor) and TransactionConsole entries (console panel) with full error message
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `frontend/src/components/ide/MonacoEditor.tsx` (editorRef + monaco namespace), `frontend/src/components/ide/ConsolePanel.tsx` (children slot), `frontend/src/lib/api-client.ts` (compileContract method)
- New wiring introduced in this slice: Compile button → compileContract API → solc-error-parser → setModelMarkers; useTransactionLog → TransactionConsole in ConsolePanel; AIDiffViewer replacing CodeDiffView in SidebarPanel
- What remains before the milestone is truly usable end-to-end: S03 (ABI-based contract interaction + VibeScore dashboard), S04 (responsive + deploy)

## Tasks

- [x] **T01: Create useTransactionLog hook, TransactionConsole component, and solc-error-parser utility** `est:25m`
  - Why: These are the foundational building blocks — pure state management, pure UI, and pure utility — with no API wiring. Creating them first with tests ensures a solid base for T02/T03 integration.
  - Files: `frontend/src/hooks/useTransactionLog.ts`, `frontend/src/components/ide/TransactionConsole.tsx`, `frontend/src/lib/solc-error-parser.ts`, `frontend/src/__tests__/useTransactionLog.test.ts`, `frontend/src/__tests__/solc-error-parser.test.ts`
  - Do: (1) Create `useTransactionLog` hook with `TransactionLogEntry` type (type: compile|deploy|call, timestamp, status: success|error|pending, message, details: {address?, txHash?, errors?}), `addEntry()`, `clearLog()`, `entries` state. (2) Create `TransactionConsole` accepting entries array, rendering color-coded list (green=success, red=error, amber=pending) with timestamps and expandable details. (3) Create `solc-error-parser.ts` with regex `/^([^:]+):(\d+):(\d+):\s*(Warning|Error|Info):\s*(.+)$/m` that parses solc `formattedMessage` strings to `{ line, column, severity, message }` for Monaco markers. (4) Write tests for hook state transitions and parser regex edge cases.
  - Verify: `cd frontend && npm test -- --testPathPattern="useTransactionLog|solc-error-parser"` passes
  - Done when: Hook manages entries correctly, console renders them, parser extracts line/column from solc error strings, all new tests pass

- [x] **T02: Wire Compile button + inline error markers + transaction logging into page.tsx** `est:30m`
  - Why: This is the core compile UX flow — the Compile button, Monaco inline markers, and TransactionConsole integration. Requires storing the `monaco` namespace (currently ignored as `_monaco`) and updating `CompileResult` with `contractName`.
  - Files: `frontend/src/app/page.tsx`, `frontend/src/lib/api-client.ts`
  - Do: (1) Update `CompileResult` interface in api-client.ts to add `contractName: string`. (2) In page.tsx: store `monacoInstance` ref alongside `editorInstance` by changing `handleEditorReady` to save both editor and monaco. (3) Add `useTransactionLog` hook and render `TransactionConsole` in the console slot (replacing placeholder). (4) Add "Compile" button to toolbar that calls `compileContract(contractSource)`, on success stores ABI/bytecode in state and logs success entry, on error parses the error message with `solc-error-parser` and calls `monaco.editor.setModelMarkers(model, 'solc', markers)` for inline markers plus logs error entry. (5) Clear markers when source changes. (6) Log deploy events (success/error) to transaction log alongside existing deploy flow.
  - Verify: `cd frontend && npm run build` exits 0 AND `grep -q "compileContract" src/app/page.tsx` AND `grep -q "setModelMarkers" src/app/page.tsx` AND `grep -q "TransactionConsole" src/app/page.tsx`
  - Done when: Compile button visible in toolbar, compile errors produce inline markers and console entries, deploy events logged, build passes

- [x] **T03: Create AIDiffViewer with Monaco DiffEditor and replace CodeDiffView in page.tsx** `est:25m`
  - Why: The current `CodeDiffView` is a simple text-based side-by-side diff. Replacing it with Monaco's `DiffEditor` provides proper syntax-highlighted diff with character-level change highlighting. Uses the same two-file SSR-safe pattern as MonacoEditor.
  - Files: `frontend/src/components/ide/AIDiffViewer.tsx`, `frontend/src/components/ide/AIDiffViewerInner.tsx`, `frontend/src/app/page.tsx`
  - Do: (1) Create `AIDiffViewerInner.tsx` with Monaco `DiffEditor` from `@monaco-editor/react` — props: `original`, `modified`, `language="sol"`, `theme="vs-dark"`, `onApplyFix` callback, height="300px", options: `{ readOnly: true, renderSideBySide: true }`. Include summary banner and "Apply Fix" button below diff. (2) Create `AIDiffViewer.tsx` as SSR-safe wrapper using `next/dynamic(() => import('./AIDiffViewerInner'), { ssr: false })` — same pattern as `MonacoEditor.tsx`. (3) In page.tsx: replace `CodeDiffView` import with `AIDiffViewer`, update JSX to pass `original={errorDiff.original}` `modified={errorDiff.fixed}` `onApplyFix={handleApplyFix}` `summary={errorDiff.summary}`. Remove `CodeDiffView` import. (4) Verify build passes with no SSR errors.
  - Verify: `cd frontend && npm run build` exits 0 AND `grep -q "AIDiffViewer" src/app/page.tsx` AND `! grep -q "CodeDiffView" src/app/page.tsx` AND `grep -q "ssr: false" src/components/ide/AIDiffViewer.tsx`
  - Done when: AI fix suggestions render via Monaco DiffEditor, Apply Fix works, old CodeDiffView no longer used in page.tsx, build passes

## Files Likely Touched

- `frontend/src/hooks/useTransactionLog.ts` — new: transaction log state hook
- `frontend/src/components/ide/TransactionConsole.tsx` — new: console log renderer
- `frontend/src/lib/solc-error-parser.ts` — new: solc error message parser
- `frontend/src/components/ide/AIDiffViewer.tsx` — new: SSR-safe DiffEditor wrapper
- `frontend/src/components/ide/AIDiffViewerInner.tsx` — new: DiffEditor implementation
- `frontend/src/app/page.tsx` — modified: compile button, markers, TransactionConsole, AIDiffViewer
- `frontend/src/lib/api-client.ts` — modified: CompileResult.contractName added
- `frontend/src/__tests__/useTransactionLog.test.ts` — new: hook tests
- `frontend/src/__tests__/solc-error-parser.test.ts` — new: parser tests
