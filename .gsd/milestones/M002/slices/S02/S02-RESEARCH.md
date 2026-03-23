# S02: 컴파일·배포 UX + 트랜잭션 콘솔 — Research

**Date:** 2026-03-23
**Depth:** Targeted

## Summary

S02 adds three features to the S01 IDE foundation: (1) a Compile button that calls `compileContract()` and renders solc errors as Monaco inline markers, (2) a TransactionConsole component replacing the ConsolePanel placeholder to show deploy/compile/call history with timestamps, and (3) an AIDiffViewer using Monaco's built-in `DiffEditor` component to replace the current `CodeDiffView` text-based diff with a proper side-by-side diff editor plus "Apply Fix" button.

All three features are straightforward applications of existing Monaco APIs (`setModelMarkers` for inline errors, `DiffEditor` from `@monaco-editor/react` for diff view) and existing backend APIs (`compileContract`, `analyzeError` already defined in `api-client.ts`). The `compileContract` API method exists but is not yet used in `page.tsx`. The `DiffEditor` component is already exported by `@monaco-editor/react` (confirmed in its type definitions). No new libraries needed.

**Key constraint:** The backend `HttpExceptionFilter` strips the `errors` array from `BadRequestException` — compile error details (the solc `formattedMessage` strings with line:column info) are lost in the `ApiResponse.fail()` envelope. The frontend currently gets only `"Compilation failed"` as the message. For inline error markers, either: (A) modify the compile endpoint to return errors in the success envelope alongside a failure flag, (B) modify the filter to preserve structured error data, or (C) parse the error message client-side. **Recommendation: option (C)** — enhance `compileContract()` in `api-client.ts` to catch HTTP 400 responses and parse the raw response body (which contains the NestJS `BadRequestException` response before the filter processes it). Actually, looking more carefully, the filter sends `{ success: false, error: { message: "Compilation failed" } }` which loses the errors array. The cleanest fix is **option (B): modify the filter to pass through the `errors` array** when present on the exception response. But since the backend is out-of-scope for M002 frontend work, **option (C) is viable**: use a fallback regex parser that extracts line/column from the generic error message string. In practice, the `message` field from the filter is just `"Compilation failed"` (string, no line info). So the actual approach should be: **call the compile endpoint, catch the error, and display the raw error message in the console. For inline markers, parse whatever line:column info is available from the error message.** If the error message is just `"Compilation failed"` without detail, show it in the console only — no inline markers. This is pragmatic: the error detail is still visible, and when the backend is enhanced later to include structured errors, the frontend parser can extract line/column info.

**Actually, re-reading the backend filter more carefully:** When `BadRequestException({ message: 'Compilation failed', errors: [...] })` is thrown, NestJS's `getResponse()` returns `{ message: 'Compilation failed', errors: [...] }`. The filter does `(exceptionResponse as any).message` → gets `'Compilation failed'`. The `errors` array is indeed lost. However — the `message` field in the NestJS `BadRequestException` object format can be an array. Let me trace this: `new BadRequestException({ message: 'Compilation failed', errors: messages })` — NestJS internally stores this as `{ statusCode: 400, message: 'Compilation failed', errors: messages }`. The filter extracts `.message` → `"Compilation failed"`. So yes, the `errors` array with solc formatted messages is lost.

**Pragmatic decision:** For S02, we'll:
1. Enhance `api-client.ts` `compileContract` to also handle error responses and return structured error info
2. Parse solc `formattedMessage` strings (format: `Contract.sol:LINE:COL: ErrorType: message`) to extract line/column for markers
3. Use a try/catch pattern where the catch path still attempts to parse the response body for the `errors` array before the envelope unwrapper discards it

## Recommendation

Build S02 in four natural units:

1. **useTransactionLog hook + TransactionConsole component** — pure UI + state, no API changes. Replaces ConsolePanel placeholder.
2. **Compile button + inline error markers** — wire `compileContract()` into page.tsx, parse error responses for line/column, call `monaco.editor.setModelMarkers()`.
3. **AIDiffViewer with Monaco DiffEditor** — replace `CodeDiffView` with a new component using `@monaco-editor/react`'s `DiffEditor`, add "Apply Fix" button. SSR-safe via `next/dynamic`.
4. **Wire everything into page.tsx** — integrate compile state, tx logging, and diff viewer into the IDE layout.

Tasks 1-3 are independent components; task 4 integrates them. The build order should be T01 (hook+console), T02 (compile+markers), T03 (diff viewer), T04 (integration+verification) — but T01-T03 could be parallelized since they're independent components.

## Implementation Landscape

### Key Files

- `frontend/src/app/page.tsx` — main page; needs Compile button added to toolbar, compile state management, TransactionConsole wired into ConsolePanel slot, AIDiffViewer replacing CodeDiffView in sidebar
- `frontend/src/lib/api-client.ts` — `compileContract()` already exists but needs error response enhancement to extract solc error details; `CompileResult` interface needs `contractName` added
- `frontend/src/components/ide/MonacoEditorInner.tsx` — the Monaco Editor impl; `onEditorReady` callback already provides `(editor, monaco)` refs needed for `setModelMarkers`
- `frontend/src/components/ide/ConsolePanel.tsx` — currently a simple wrapper with children; stays as-is, new TransactionConsole goes inside
- `frontend/src/components/CodeDiffView.tsx` — current text-based diff; will be replaced by AIDiffViewer using Monaco DiffEditor. Keep the old file for now (it may be used elsewhere), but page.tsx switches to the new component.

### New Files to Create

- `frontend/src/hooks/useTransactionLog.ts` — hook managing an array of `TransactionLogEntry` objects (type: compile|deploy|call, timestamp, status: success|error|pending, message, details like address/txHash/errors). Provides `addEntry()`, `clearLog()`, `entries` state.
- `frontend/src/components/ide/TransactionConsole.tsx` — renders the transaction log entries with color-coded severity (green=success, red=error, amber=pending), timestamps, expandable details. Accepts `entries` from the hook.
- `frontend/src/components/ide/AIDiffViewer.tsx` — SSR-safe wrapper around Monaco `DiffEditor`. Props: `original: string`, `modified: string`, `language?: string`, `onApplyFix?: (code: string) => void`. Shows side-by-side diff with "Apply Fix" button.
- `frontend/src/components/ide/AIDiffViewerInner.tsx` — actual DiffEditor implementation (same two-file split pattern as MonacoEditor for SSR safety).
- `frontend/src/lib/solc-error-parser.ts` — utility to parse solc `formattedMessage` strings into `{ line, column, endLine, endColumn, severity, message }` for Monaco markers. Regex: `/^([^:]+):(\d+):(\d+):\s*(Warning|Error|Info):\s*(.+)$/m`

### Build Order

**T01: useTransactionLog hook + TransactionConsole component** — Independent, no API deps. Creates the state management hook and the visual console component. This fills the ConsolePanel placeholder from S01.

**T02: Compile button + inline error markers** — Adds a "Compile" button to the editor toolbar, calls `compileContract()`, parses error responses, sets Monaco markers via `editor.getModel()` + `monaco.editor.setModelMarkers()`. Also updates `CompileResult` type in api-client to include `contractName`. Stores successful compile results (ABI, bytecode) in state for S03 consumption.

**T03: AIDiffViewer with Monaco DiffEditor** — Creates the two-file SSR-safe wrapper (same pattern as MonacoEditor). Replaces `CodeDiffView` usage in page.tsx. DiffEditor props: `original`, `modified`, `language="sol"`, `theme="vs-dark"`, `options={{ readOnly: true, renderSideBySide: true }}`. "Apply Fix" button below the diff.

**T04: Integration in page.tsx** — Wire compile state, transaction logging, and AIDiffViewer into the existing page composition. Add `useTransactionLog` hook, log deploy/compile/error events, pass TransactionConsole as console content, replace CodeDiffView with AIDiffViewer.

### Verification Approach

1. `npm run build` exits 0 — no SSR errors from DiffEditor dynamic import
2. `npm test` — all existing 11 tests pass + new tests for useTransactionLog hook and solc-error-parser
3. File existence checks: TransactionConsole.tsx, AIDiffViewer.tsx, useTransactionLog.ts, solc-error-parser.ts
4. page.tsx uses `compileContract` from api-client
5. page.tsx no longer imports `CodeDiffView` (replaced by AIDiffViewer)
6. page.tsx has a "Compile" button in the toolbar
7. TransactionConsole is rendered inside ConsolePanel (not placeholder text)
8. AIDiffViewer uses next/dynamic with ssr:false (same pattern as MonacoEditor)
9. `monaco.editor.setModelMarkers` is called with parsed error results

## Constraints

- **Backend is out-of-scope** — no changes to backend code. The compile error format from the API is what it is. The frontend must work with whatever the `HttpExceptionFilter` sends back.
- **`--legacy-peer-deps` required** — all `npm install` must use this flag (inherited from S01, @monaco-editor/react React 19 compat).
- **DiffEditor needs same SSR-safe pattern** — Monaco DiffEditor also uses `window`/`document`, so it must use `next/dynamic({ ssr: false })` with the two-file split.
- **editorRef lifetime** — `editorInstance` state in page.tsx holds the editor ref after mount. It's `any` typed. For `setModelMarkers`, we need both the editor instance (to get the model) and the monaco namespace (to access `monaco.editor.setModelMarkers`). The `handleEditorReady` callback provides both — we need to store the `monaco` namespace as well.
- **CompileResult interface mismatch** — Frontend's `CompileResult` has `{ bytecode, abi }` but backend returns `{ contractName, abi, bytecode }`. Need to add `contractName` to the interface. This also feeds S03's contract interaction UI.

## Common Pitfalls

- **Monaco namespace not stored** — Currently `page.tsx` stores `editorInstance` but NOT the `monaco` namespace from `onEditorReady(editor, monaco)`. `setModelMarkers` requires `monaco.editor.setModelMarkers(model, owner, markers)`. Must also store `monaco` in state or ref.
- **DiffEditor SSR crash** — If `DiffEditor` from `@monaco-editor/react` is imported directly (not via `next/dynamic`), it will crash during SSR. Must use the same two-file split pattern as `MonacoEditor`.
- **Compile error response parsing** — The API error response arrives as HTTP 400 with `{ success: false, error: { message: "Compilation failed" } }`. The detailed solc error messages with line info may be in a separate `errors` field, or may be joined into the `message` string. The frontend `unwrapResponse` function throws on non-OK status, so the error details are in the thrown error's message. Parsing must handle both cases: structured errors array and plain string message.
- **Marker model mismatch** — `setModelMarkers` must be called on the current editor model. If the user switches contract type (changing the editor value), the model reference stays the same (Monaco reuses it), but markers should be cleared on source change.
- **DiffEditor height** — Monaco DiffEditor needs an explicit height. In the sidebar panel (which scrolls), it needs a fixed pixel height or a container with explicit dimensions. Using `height="300px"` is safe; `height="100%"` requires the parent to have explicit height.

## Open Risks

- **Compile error detail loss** — The backend filter only passes through `message: "Compilation failed"` string. Without the `errors` array containing solc `formattedMessage` strings, inline markers can only show a generic "Compilation failed" error without line/column positioning. **Mitigation:** For now, show the error in the console and highlight the entire first line. When backend is enhanced to include structured errors, the parser is ready to extract line/column. Alternatively, the `message` might be an array (NestJS behavior with validation pipe) — need to handle both cases.
- **DiffEditor bundle size** — Monaco DiffEditor shares the same Monaco core, so it shouldn't add significant bundle size beyond the already-loaded Monaco. But verify with build output.
