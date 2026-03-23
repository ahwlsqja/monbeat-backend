# S01: Monaco Editor + IDE 레이아웃 — UAT

**Milestone:** M002
**Written:** 2026-03-23

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S01 is infrastructure scaffolding (editor wrapper + layout components). Build success proves SSR compatibility, file existence proves component creation, grep checks prove API integration preserved. Runtime visual verification deferred to S04 (final polish slice).

## Preconditions

- `cd frontend && npm install --legacy-peer-deps` has been run (dependencies installed)
- No dev server required for artifact-driven checks
- For optional visual checks: `cd frontend && npm run dev` on localhost:3000

## Smoke Test

Run `cd frontend && npm run build` — must exit 0 with no SSR errors. If this passes, Monaco dynamic import and all component composition is valid.

## Test Cases

### 1. Build succeeds with no SSR errors

1. Run `cd frontend && npm run build`
2. **Expected:** Exit code 0. Output shows route table with `○ /` at ~64KB first load JS. No "window is not defined" or "document is not defined" errors.

### 2. All existing tests pass

1. Run `cd frontend && npm test -- --passWithNoTests`
2. **Expected:** 11 tests pass (api-client tests). 0 failures. No new test regressions.

### 3. Monaco Editor component exists with SSR-safe dynamic import

1. Run `test -f frontend/src/components/ide/MonacoEditor.tsx`
2. Run `grep -q "next/dynamic" frontend/src/components/ide/MonacoEditor.tsx`
3. Run `grep -q "ssr.*false" frontend/src/components/ide/MonacoEditor.tsx`
4. **Expected:** All exit 0. MonacoEditor.tsx exists and uses `next/dynamic` with `ssr: false`.

### 4. Solidity language support registered

1. Run `test -f frontend/src/lib/solidity-language.ts`
2. Run `grep -q "registerSolidityLanguage" frontend/src/lib/solidity-language.ts`
3. Run `grep -q "sol" frontend/src/components/ide/MonacoEditor.tsx`
4. **Expected:** All exit 0. Solidity language util exists, exports registration function, and MonacoEditor uses "sol" language identifier.

### 5. IDE 3-panel layout components exist with resize support

1. Run `test -f frontend/src/components/ide/IDELayout.tsx`
2. Run `test -f frontend/src/components/ide/EditorPanel.tsx`
3. Run `test -f frontend/src/components/ide/SidebarPanel.tsx`
4. Run `test -f frontend/src/components/ide/ConsolePanel.tsx`
5. Run `grep -q "Group" frontend/src/components/ide/IDELayout.tsx`
6. Run `grep -q "collapsible" frontend/src/components/ide/IDELayout.tsx`
7. **Expected:** All exit 0. Four panel components exist, IDELayout uses react-resizable-panels Group with collapsible panels.

### 6. page.tsx composes IDE layout with existing features

1. Run `grep -q "IDELayout" frontend/src/app/page.tsx`
2. Run `grep -q "MonacoEditor" frontend/src/app/page.tsx`
3. Run `! grep -q "<textarea" frontend/src/app/page.tsx`
4. Run `grep -q "deployContract" frontend/src/app/page.tsx`
5. Run `grep -q "getVibeScore" frontend/src/app/page.tsx`
6. Run `grep -q "useAuth" frontend/src/app/page.tsx`
7. **Expected:** All exit 0. page.tsx uses IDELayout + MonacoEditor (no textarea), and preserves deployContract, getVibeScore, and useAuth integrations.

### 7. Full-viewport CSS applied

1. Run `grep -q "overflow.*hidden\|overflow: hidden" frontend/src/app/globals.css`
2. **Expected:** Exit 0. globals.css has overflow hidden for full-viewport IDE layout.

### 8. Monaco chunk is lazily loaded (not in main bundle)

1. Run `cd frontend && npm run build 2>&1 | grep "First Load JS"`
2. **Expected:** The `○ /` route shows ~64KB page JS and ~184KB first load JS total. Monaco's ~630KB is NOT included in these numbers (loaded dynamically on demand).

## Edge Cases

### Monaco CDN failure simulation

1. In browser DevTools, block `cdn.jsdelivr.net` (Network tab → Block request URL)
2. Load localhost:3000
3. **Expected:** "Loading editor..." fallback text is visible. No crash or white screen. Other IDE panels (sidebar, console) still render.

### Panel collapse behavior

1. Load localhost:3000 in browser
2. Drag the sidebar separator all the way to the right
3. **Expected:** Sidebar collapses. Editor area expands to fill the space. No layout breakage.
4. Drag the console separator all the way down
5. **Expected:** Console collapses. Editor area expands vertically. No layout breakage.

### Contract selector API integration

1. Load localhost:3000 in browser (dev server must be running)
2. Click a contract selector button (e.g., "SimpleStorage")
3. **Expected:** Monaco editor loads the contract source code. No errors in browser console related to API calls.

## Failure Signals

- `npm run build` fails with "window is not defined" → Monaco SSR bypass broken
- `npm run build` fails with "cannot find module" → dependency installation incomplete
- Page shows blank white screen → IDELayout composition error
- "Loading editor..." persists forever on normal network → MonacoEditorInner import path broken
- Existing deploy/analyze buttons don't appear → page.tsx toolbar composition broken
- Page has scrollbar → globals.css overflow hidden not applied
- React hydration mismatch warnings in console → client/server render mismatch

## Not Proven By This UAT

- Monaco editor actually renders Solidity with colored syntax (requires visual/runtime check)
- Completion provider suggestions appear on typing (requires interactive runtime check)
- Resize drag handles respond to mouse drag (requires interactive runtime check)
- Deploy/Vibe-Score API calls succeed end-to-end (requires running backend)
- Mobile/tablet responsive behavior (deferred to S04)

## Notes for Tester

- Pre-existing wagmi connector warnings during build (porto, coinbaseWallet, metaMask) are expected and unrelated to S01 changes — ignore them.
- The ConsolePanel shows placeholder text "Console output will appear here..." — this is intentional, S02 will replace it with TransactionConsole.
- `--legacy-peer-deps` is required for all `npm install` commands due to @monaco-editor/react React 18 peer dep declaration vs React 19.
