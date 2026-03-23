---
estimated_steps: 5
estimated_files: 2
---

# T02: Wire Compile button + inline error markers + transaction logging into page.tsx

**Slice:** S02 — 컴파일·배포 UX + 트랜잭션 콘솔
**Milestone:** M002

## Description

Wire the Compile UX flow end-to-end in `page.tsx`: add a "Compile" button to the toolbar, call `compileContract()` from api-client, parse errors with `solc-error-parser`, set Monaco inline markers via `setModelMarkers`, and log all compile/deploy events to `TransactionConsole` in the ConsolePanel slot.

**Critical fix:** The current `handleEditorReady` callback ignores the `monaco` namespace (`_monaco` parameter). This task must store both `editor` and `monaco` — the `monaco` namespace is required for `monaco.editor.setModelMarkers(model, owner, markers)`.

Also updates `CompileResult` interface in api-client.ts to include `contractName` (backend returns it, S03 will need it for contract interaction UI).

**Relevant patterns from S01:**
- `editorInstance` state already exists in page.tsx (set by `handleEditorReady`)
- `MonacoEditor` component fires `onEditorReady(editor, monaco)` once on mount
- All API calls use the existing api-client pattern (async/await with try/catch)
- `--legacy-peer-deps` required for any npm install

## Steps

1. Update `frontend/src/lib/api-client.ts`:
   - Add `contractName: string` to `CompileResult` interface
   - Also create an enhanced compile function that captures raw error response body on failure. The current `unwrapResponse` throws a generic error message — for compile errors, we need the detailed error text. Add a `compileContractWithErrors` wrapper that: catches the error, attempts to extract any structured error messages from the response, and returns `{ success: false, errors: string[] }` or `{ success: true, data: CompileResult }`. Actually, simpler approach: keep `compileContract` as-is (it throws on error), and in page.tsx catch the error and pass `error.message` to `parseSolcErrors`. The error message from the API contains whatever the backend sends.

2. In `page.tsx`, store the `monaco` namespace:
   - Add `const [monacoInstance, setMonacoInstance] = useState<any>(null);`
   - Change `handleEditorReady` from `(editor: any, _monaco: any)` to `(editor: any, monaco: any)` and call `setMonacoInstance(monaco)`
   - Both `editorInstance` and `monacoInstance` are needed for marker operations

3. In `page.tsx`, add `useTransactionLog` and `TransactionConsole`:
   - Import `useTransactionLog` from `@/hooks/useTransactionLog`
   - Import `TransactionConsole` from `@/components/ide/TransactionConsole`
   - Call `const { entries, addEntry, clearLog } = useTransactionLog()`
   - Replace the console placeholder (`<div className="p-4 text-gray-500...">Console output will appear here...</div>`) with `<TransactionConsole entries={entries} />`
   - Wrap the TransactionConsole in a `<ConsolePanel>` — the IDELayout console slot already accepts ReactNode

4. In `page.tsx`, add compile state and Compile button:
   - Add compile state: `const [isCompiling, setIsCompiling] = useState(false)`, `const [compileResult, setCompileResult] = useState<CompileResult | null>(null)`
   - Import `compileContract` and `CompileResult` from api-client
   - Import `parseSolcErrors` from `@/lib/solc-error-parser`
   - Create `handleCompile` function:
     ```
     async function handleCompile() {
       if (!contractSource.trim()) return;
       setIsCompiling(true);
       // Clear previous markers
       if (editorInstance && monacoInstance) {
         monacoInstance.editor.setModelMarkers(editorInstance.getModel(), 'solc', []);
       }
       addEntry({ type: 'compile', status: 'pending', message: 'Compiling...' });
       try {
         const result = await compileContract(contractSource);
         setCompileResult(result);
         addEntry({ type: 'compile', status: 'success', message: `Compiled: ${result.contractName || 'Contract'}`, details: { contractName: result.contractName, abi: result.abi } });
       } catch (err) {
         const msg = (err as Error).message || 'Compilation failed';
         // Parse for inline markers
         const markers = parseSolcErrors(msg);
         if (editorInstance && monacoInstance) {
           monacoInstance.editor.setModelMarkers(editorInstance.getModel(), 'solc', markers);
         }
         addEntry({ type: 'compile', status: 'error', message: msg, details: { errors: [msg] } });
       } finally {
         setIsCompiling(false);
       }
     }
     ```
   - Add Compile button to toolbar (between contract selector and Deploy button):
     ```jsx
     <button onClick={handleCompile} disabled={isCompiling || !contractSource.trim()} className="bg-blue-600 hover:bg-blue-500 ...">
       {isCompiling ? "컴파일 중..." : "Compile"}
     </button>
     ```
   - Clear markers when contractSource changes: add a `useEffect` that clears markers when `contractSource` changes (debounce not needed — just clear on change)

5. Log existing deploy events to transaction log:
   - In `handleDeploy`, add `addEntry({ type: 'deploy', status: 'pending', message: 'Deploying...' })` at the start
   - On deploy success: `addEntry({ type: 'deploy', status: 'success', message: 'Deployed', details: { address: data.address, txHash: data.txHash } })`
   - On deploy error: `addEntry({ type: 'deploy', status: 'error', message: errObj?.message || 'Deploy failed' })`

## Must-Haves

- [ ] `CompileResult` interface has `contractName: string` field
- [ ] `monacoInstance` stored in page.tsx state (not ignored as `_monaco`)
- [ ] "Compile" button in toolbar calls `compileContract()` 
- [ ] Compile errors produce inline Monaco markers via `setModelMarkers`
- [ ] `TransactionConsole` renders in ConsolePanel (replaces placeholder text)
- [ ] Deploy and compile events logged to transaction log
- [ ] Markers cleared when source code changes
- [ ] `npm run build` exits 0

## Verification

- `cd frontend && npm run build` exits 0
- `grep -q "compileContract" frontend/src/app/page.tsx` — compile API imported and used
- `grep -q "setModelMarkers" frontend/src/app/page.tsx` — inline markers implemented
- `grep -q "TransactionConsole" frontend/src/app/page.tsx` — console component wired in
- `grep -q "monacoInstance\|setMonacoInstance" frontend/src/app/page.tsx` — monaco namespace stored
- `grep -q "contractName" frontend/src/lib/api-client.ts` — CompileResult updated
- `! grep -q "Console output will appear here" frontend/src/app/page.tsx` — placeholder replaced
- `cd frontend && npm test` — all tests still pass

## Observability Impact

- **New runtime signals:** Compile and deploy events are logged as `TransactionLogEntry` objects in `useTransactionLog` state — each entry has type, status, timestamp, and optional details. Visible in React DevTools and rendered in the TransactionConsole component.
- **Inline markers:** Compile errors produce Monaco model markers (via `setModelMarkers`) — visible as red squiggly underlines in the editor at the exact error line/column. Markers clear automatically when source code changes.
- **Inspection:** Future agents can inspect `monacoInstance.editor.getModelMarkers({owner:'solc'})` in browser console to see active error markers. `useTransactionLog` entries array is inspectable via React DevTools on any component using the hook.
- **Failure visibility:** Both compile and deploy errors appear in two places: (1) inline Monaco markers in the editor, (2) red-bordered entries in TransactionConsole with expandable details containing the raw error message.

## Inputs

- `frontend/src/app/page.tsx` — current page with editor, deploy flow, CodeDiffView
- `frontend/src/lib/api-client.ts` — existing API client with `compileContract` method and `CompileResult` type
- `frontend/src/hooks/useTransactionLog.ts` — hook created by T01
- `frontend/src/components/ide/TransactionConsole.tsx` — component created by T01
- `frontend/src/lib/solc-error-parser.ts` — parser created by T01

## Expected Output

- `frontend/src/app/page.tsx` — modified: compile button, markers, TransactionConsole, monacoInstance state
- `frontend/src/lib/api-client.ts` — modified: CompileResult.contractName added
