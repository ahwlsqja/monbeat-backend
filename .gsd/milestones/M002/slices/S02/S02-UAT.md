# S02: 컴파일·배포 UX + 트랜잭션 콘솔 — UAT

**Milestone:** M002
**Written:** 2026-03-23

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All deliverables are build-verified components and unit-tested utilities. The compile→markers→console pipeline is wired at component level. No live backend or blockchain connection required to verify the UX flow — build success + test pass + file/grep checks cover the contract.

## Preconditions

- `cd frontend && npm install --legacy-peer-deps` completed successfully
- `npm run build` exits 0 (confirms no SSR crashes)
- `npm test` — 30/30 tests pass

## Smoke Test

Run `cd frontend && npm run build && npm test` — build exits 0, all 30 tests pass. This confirms SSR safety of both dynamic imports (MonacoEditor + AIDiffViewer) and correctness of hook/parser logic.

## Test Cases

### 1. useTransactionLog hook manages entries correctly

1. Run `cd frontend && npx jest --testPathPatterns="useTransactionLog" --verbose`
2. **Expected:** 7 tests pass covering: initial empty state, addEntry adds entries, entries are newest-first, multiple entries maintain order, clearLog empties entries, entry has auto-generated id and timestamp, supports all three types (compile/deploy/call)

### 2. solc-error-parser extracts line/column from solc error strings

1. Run `cd frontend && npx jest --testPathPatterns="solc-error-parser" --verbose`
2. **Expected:** 12 tests pass covering: standard Error format, Warning severity, TypeError/ParserError/DeclarationError, multi-line error messages, unparseable strings produce fallback marker at line 1, empty input returns empty array, multiple errors in one string are all extracted

### 3. Compile button exists and calls compileContract

1. Run `grep -n "compileContract" frontend/src/app/page.tsx`
2. **Expected:** Multiple matches showing import from api-client and usage in handleCompile function
3. Run `grep -n "handleCompile" frontend/src/app/page.tsx`
4. **Expected:** Function definition and button onClick binding

### 4. Monaco inline error markers are set on compile failure

1. Run `grep -n "setModelMarkers" frontend/src/app/page.tsx`
2. **Expected:** Call to `monacoInstance.editor.setModelMarkers()` with parsed solc errors and `'solc'` owner string
3. Run `grep -n "parseSolcErrors" frontend/src/app/page.tsx`
4. **Expected:** Import from solc-error-parser and usage in handleCompile error path

### 5. Markers are cleared when source changes

1. Run `grep -A5 "useEffect.*contractSource" frontend/src/app/page.tsx` or `grep -B2 -A5 "setModelMarkers.*\[\]" frontend/src/app/page.tsx`
2. **Expected:** useEffect with contractSource dependency that calls `setModelMarkers(model, 'solc', [])` to clear markers

### 6. TransactionConsole renders in ConsolePanel

1. Run `grep -n "TransactionConsole" frontend/src/app/page.tsx`
2. **Expected:** Import from components/ide/TransactionConsole and JSX usage with `entries={entries}` prop
3. Run `! grep -q "Console output will appear here" frontend/src/app/page.tsx && echo "✅ placeholder removed"`
4. **Expected:** Placeholder text no longer present

### 7. AIDiffViewer replaces CodeDiffView with SSR-safe pattern

1. Run `grep -n "AIDiffViewer" frontend/src/app/page.tsx`
2. **Expected:** Import and JSX usage with original/modified/summary/onApplyFix props
3. Run `! grep -q "CodeDiffView" frontend/src/app/page.tsx && echo "✅ CodeDiffView removed"`
4. **Expected:** No CodeDiffView references in page.tsx
5. Run `grep "ssr: false" frontend/src/components/ide/AIDiffViewer.tsx`
6. **Expected:** `ssr: false` present in dynamic import options

### 8. AIDiffViewerInner uses Monaco DiffEditor

1. Run `grep -n "DiffEditor" frontend/src/components/ide/AIDiffViewerInner.tsx`
2. **Expected:** Import from `@monaco-editor/react` and JSX usage with language="sol", theme="vs-dark", readOnly options

### 9. CompileResult includes contractName for S03

1. Run `grep -A3 "CompileResult" frontend/src/lib/api-client.ts`
2. **Expected:** Interface includes `contractName: string` field alongside existing abi/bytecode fields

### 10. Deploy events are logged to TransactionConsole

1. Run `grep -B2 -A3 "addEntry.*deploy" frontend/src/app/page.tsx`
2. **Expected:** addEntry calls with type 'deploy' at pending, success, and error points in the deploy flow

## Edge Cases

### Unparseable solc error strings

1. Run `cd frontend && npx jest --testPathPatterns="solc-error-parser" --verbose -t "fallback"`
2. **Expected:** Test passes confirming that unrecognized error formats produce a fallback marker at line 1, column 1 with the raw message — no silent drops

### Empty contract source prevents compile

1. Run `grep -A5 "disabled.*compile\|Compile.*disabled" frontend/src/app/page.tsx`
2. **Expected:** Compile button is disabled when contractSource is empty or isCompiling is true

### Full test suite regression

1. Run `cd frontend && npm test`
2. **Expected:** All 30 tests pass (11 api-client + 7 useTransactionLog + 12 solc-error-parser), zero failures

## Failure Signals

- `npm run build` fails with SSR error referencing `window`, `document`, or `monaco` — indicates DiffEditor dynamic import broke
- `npm test` shows fewer than 30 tests — missing test file or broken import
- `grep -q "CodeDiffView" frontend/src/app/page.tsx` returns 0 — old diff view not fully replaced
- `grep -q "Console output will appear here" frontend/src/app/page.tsx` returns 0 — console placeholder not replaced with TransactionConsole
- Build warnings about `monacoInstance` being unused — indicates handleEditorReady not storing monaco namespace

## Not Proven By This UAT

- Live compile against a running backend — requires backend API at NEXT_PUBLIC_API_URL
- Actual Monaco inline marker visual rendering — requires browser runtime with Monaco loaded
- DiffEditor visual rendering quality — requires browser with AI fix suggestion triggered
- Deploy event logging with real blockchain transaction — requires wallet connection and Monad testnet
- TransactionConsole scroll behavior and expandable details UX — requires browser interaction

## Notes for Tester

- Pre-existing wagmi connector warnings in build output (`porto/internal`, `@metamask/sdk`) are unrelated and harmless.
- `CodeDiffView.tsx` file still exists on disk (intentionally) — only its usage in page.tsx was removed. This is not a bug.
- The `--legacy-peer-deps` flag is required for all npm install commands due to @monaco-editor/react peer dep mismatch with React 19.
- Jest 30 uses `--testPathPatterns` (plural) instead of `--testPathPattern`. Direct CLI invocations must use the new flag.
