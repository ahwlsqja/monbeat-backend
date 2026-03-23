---
id: T01
parent: S01
milestone: M002
provides:
  - MonacoEditor SSR-safe wrapper component with dynamic import
  - Solidity language completion provider (9 snippet templates + 50 keywords)
  - @monaco-editor/react and react-resizable-panels dependencies
key_files:
  - frontend/src/components/ide/MonacoEditor.tsx
  - frontend/src/components/ide/MonacoEditorInner.tsx
  - frontend/src/lib/solidity-language.ts
  - frontend/package.json
key_decisions:
  - Split Monaco into two files (MonacoEditor.tsx wrapper + MonacoEditorInner.tsx implementation) so next/dynamic performs real lazy import and code-splitting
  - Used `defaultLanguage="sol"` per Monaco's built-in Solidity identifier, not "solidity"
  - Used `--legacy-peer-deps` for npm install since @monaco-editor/react@4.7.0 declares React 18 max but works with React 19
patterns_established:
  - SSR-unsafe components use next/dynamic with ssr:false in a wrapper file that re-exports the type interface
  - Language providers are registered in beforeMount callback (runs once before editor init)
observability_surfaces:
  - Monaco CDN chunk loading visible in Network tab (separate from main bundle)
  - "Loading editor..." fallback visible while Monaco loads or if CDN fails
  - onEditorReady callback signals editor availability to downstream consumers
duration: 15m
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T01: Install dependencies and create Monaco Editor wrapper with Solidity language support

**Installed @monaco-editor/react and react-resizable-panels, created SSR-safe Monaco Editor wrapper with Solidity completion provider (9 snippets, 50 keywords), verified Next.js build succeeds with no SSR errors.**

## What Happened

Installed `@monaco-editor/react@4.7.0` and `react-resizable-panels@4.7.5` with `--legacy-peer-deps` (React 19 peer dep conflict, but works in practice). Created `solidity-language.ts` with a `registerSolidityLanguage(monaco)` function that registers language configuration (comments, brackets, auto-closing pairs) and a completion provider with 9 snippet templates (contract, function, modifier, event, mapping, constructor, error, struct, enum) plus 50 Solidity keyword completions.

Created the Monaco wrapper as a two-file architecture: `MonacoEditorInner.tsx` contains the actual `<Editor>` component with `defaultLanguage="sol"`, `theme="vs-dark"`, and `onEditorReady` callback; `MonacoEditor.tsx` is a thin `next/dynamic` wrapper with `ssr: false` that lazy-imports the inner component. This ensures real code splitting — Monaco's ~2 MB payload loads as a separate chunk, not in the main bundle.

Build verified twice: once after initial implementation, once after refactoring to the two-file split. Both builds succeeded with exit 0, no SSR errors.

## Verification

All 8 task-level verification checks pass. All T01-relevant slice checks pass. IDELayout checks (T02 scope) expectedly not yet passing.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd frontend && npm run build` | 0 | ✅ pass | 36s |
| 2 | `test -f frontend/src/components/ide/MonacoEditor.tsx` | 0 | ✅ pass | <1s |
| 3 | `test -f frontend/src/lib/solidity-language.ts` | 0 | ✅ pass | <1s |
| 4 | `grep -q "sol" frontend/src/components/ide/MonacoEditor.tsx` | 0 | ✅ pass | <1s |
| 5 | `grep -q "ssr.*false" frontend/src/components/ide/MonacoEditor.tsx` | 0 | ✅ pass | <1s |
| 6 | `grep -q "registerSolidityLanguage" frontend/src/lib/solidity-language.ts` | 0 | ✅ pass | <1s |
| 7 | `grep -q "@monaco-editor/react" frontend/package.json` | 0 | ✅ pass | <1s |
| 8 | `grep -q "react-resizable-panels" frontend/package.json` | 0 | ✅ pass | <1s |
| 9 | `grep -q "next/dynamic" frontend/src/components/ide/MonacoEditor.tsx` | 0 | ✅ pass | <1s |
| 10 | `cd frontend && npm test -- --passWithNoTests` | 0 | ✅ pass | 1s |

## Diagnostics

- **Monaco loading state:** If Monaco CDN is slow or blocked, the "Loading editor..." fallback renders indefinitely — visible in the UI and React DevTools.
- **Completion provider:** Inspect registration via browser console: `monaco.languages.getLanguages()` should include an entry with `id: "sol"`.
- **Editor instance:** Downstream consumers receive the editor via `onEditorReady(editor, monaco)` — if this doesn't fire, check browser console for Monaco initialization errors.
- **SSR safety:** Direct import of `MonacoEditorInner` (bypassing the wrapper) will cause `window is not defined` errors during build — this is caught at build time, not runtime.

## Deviations

- **Two-file split instead of single file:** The task plan suggested a single-file approach as an alternative, but I used the two-file split (MonacoEditor.tsx + MonacoEditorInner.tsx) because `next/dynamic` with `Promise.resolve()` doesn't perform real code splitting. The `import()` call in the dynamic wrapper is necessary for webpack/turbopack to create a separate chunk.

## Known Issues

- Pre-existing wagmi connector warnings during build (porto, coinbaseWallet, metaMask modules not found) — unrelated to Monaco changes, present before this task.

## Files Created/Modified

- `frontend/package.json` — added @monaco-editor/react and react-resizable-panels dependencies
- `frontend/src/lib/solidity-language.ts` — new: Solidity language configuration and completion provider
- `frontend/src/components/ide/MonacoEditor.tsx` — new: SSR-safe dynamic import wrapper
- `frontend/src/components/ide/MonacoEditorInner.tsx` — new: actual Monaco Editor component with sol language and vs-dark theme
