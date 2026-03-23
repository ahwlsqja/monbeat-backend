---
estimated_steps: 5
estimated_files: 3
---

# T01: Install dependencies and create Monaco Editor wrapper with Solidity language support

**Slice:** S01 — Monaco Editor + IDE 레이아웃
**Milestone:** M002

## Description

Install `@monaco-editor/react` and `react-resizable-panels`, then create the Monaco Editor wrapper component with `next/dynamic` SSR-safe import and a Solidity language completion provider. This is the highest-risk task in the slice — if Monaco doesn't work with Next.js SSR / React 19, we catch it here before building anything on top of it.

The Monaco Editor wrapper must:
- Use `next/dynamic` with `ssr: false` to avoid window/document dependency crashes
- Use `defaultLanguage="sol"` (Monaco's built-in Solidity language identifier — NOT `"solidity"`)
- Use `theme="vs-dark"` for dark mode
- Expose the editor instance via an `onEditorReady` callback prop so S02 can access it for markers/getValue
- Register a Solidity completion provider in `beforeMount` for snippet completions (contract, function, modifier, event templates)
- Show a "Loading editor..." fallback while Monaco loads from CDN

## Steps

1. **Install dependencies:**
   ```bash
   cd frontend && npm install @monaco-editor/react react-resizable-panels --legacy-peer-deps
   ```
   Use `--legacy-peer-deps` because `@monaco-editor/react@4.7.0` declares React 18 max as peer dep, but works fine with React 19 in practice.

2. **Create `frontend/src/lib/solidity-language.ts`:**
   - Export a `registerSolidityLanguage(monaco)` function that:
     - Registers a `CompletionItemProvider` for language `"sol"` with snippets:
       - `contract` → full contract template
       - `function` → function with visibility + returns
       - `modifier` → modifier template
       - `event` → event declaration
       - `mapping` → mapping declaration
       - Common Solidity keywords as completion items (pragma, import, require, emit, revert, etc.)
     - Sets language configuration for `"sol"`:
       - Comments: `//` line, `/* */` block
       - Brackets: `{}`, `()`, `[]`
       - Auto-closing pairs: `{`, `(`, `[`, `"`, `'`
       - Surrounding pairs matching brackets and quotes
   - This is a pure utility — no React dependency, easy to verify in isolation.

3. **Create `frontend/src/components/ide/MonacoEditor.tsx`:**
   - Create the `ide/` directory under components
   - Build the wrapper as a **named export** component `MonacoEditorWrapper` that:
     - Accepts props: `value: string`, `onChange: (value: string) => void`, `onEditorReady?: (editor: any, monaco: any) => void`
     - Renders the `Editor` component from `@monaco-editor/react` with:
       - `height="100%"` (parent controls actual height via panel)
       - `defaultLanguage="sol"` — **critical: `sol` not `solidity`**
       - `theme="vs-dark"`
       - `value={value}` and `onChange` handler
       - `beforeMount` calls `registerSolidityLanguage(monaco)`
       - `onMount` calls `onEditorReady?.(editor, monaco)` to expose the editor instance
       - `loading` prop with a simple `<div className="flex items-center justify-center h-full text-gray-500">Loading editor...</div>` fallback
   - Export via `next/dynamic`:
     ```tsx
     const MonacoEditor = dynamic(
       () => import('./MonacoEditorInner').then(mod => ({ default: mod.MonacoEditorWrapper })),
       { ssr: false, loading: () => <div>Loading editor...</div> }
     );
     ```
     **Alternative approach** (simpler): Put the inner component and dynamic export in the same file. The component definition must be the non-default export, and the dynamic wrapper is the default export. The key constraint is that `next/dynamic` must wrap the import so the component never runs on the server.

4. **Verify build succeeds:**
   ```bash
   cd frontend && npm run build
   ```
   This proves SSR compatibility — Next.js static analysis pass won't fail on Monaco's window/document references because the component is dynamically imported with `ssr: false`.

5. **Verify Monaco chunk is separate:**
   After build, check that Monaco's ~2MB payload is in a separate lazy-loaded chunk, not in the main bundle. This confirms code splitting works correctly.

## Must-Haves

- [ ] `@monaco-editor/react` and `react-resizable-panels` installed in `frontend/package.json`
- [ ] `solidity-language.ts` exports `registerSolidityLanguage(monaco)` function
- [ ] Completion provider registered for `"sol"` language with at least 4 snippet templates
- [ ] `MonacoEditor.tsx` uses `next/dynamic` with `ssr: false`
- [ ] Editor uses `defaultLanguage="sol"` (NOT `"solidity"`)
- [ ] Editor uses `theme="vs-dark"`
- [ ] `onEditorReady` callback exposes editor + monaco instances
- [ ] `npm run build` succeeds with no errors

## Verification

- `cd frontend && npm run build` exits 0 with no errors
- `test -f frontend/src/components/ide/MonacoEditor.tsx` — file exists
- `test -f frontend/src/lib/solidity-language.ts` — file exists
- `grep -q "sol" frontend/src/components/ide/MonacoEditor.tsx` — uses sol language
- `grep -q "ssr.*false\|ssr: false" frontend/src/components/ide/MonacoEditor.tsx` — uses SSR-safe dynamic import
- `grep -q "registerSolidityLanguage" frontend/src/lib/solidity-language.ts` — function exists
- `grep -q "@monaco-editor/react" frontend/package.json` — dependency installed
- `grep -q "react-resizable-panels" frontend/package.json` — dependency installed

## Observability Impact

- **Monaco CDN loading:** When the editor mounts, Monaco loads its ~2 MB worker scripts from CDN. Network tab shows these as separate chunk requests — if they fail, the "Loading editor..." fallback stays visible indefinitely.
- **Solidity completion provider:** Registered via `beforeMount` callback. If registration fails, the editor still renders but autocompletion won't include Solidity snippets — inspectable via `monaco.languages.getLanguages()` in browser console.
- **Editor ready signal:** The `onEditorReady` callback fires after full mount. Downstream components (S02 marker API) can gate on this — if it doesn't fire, the editor instance is unavailable for programmatic access.
- **SSR safety:** The `next/dynamic` wrapper with `ssr: false` prevents the component from running on the server. If this wrapper is bypassed (e.g., direct import of `MonacoEditorInner`), the build will fail with `window is not defined` — this is a build-time signal, not runtime.

- `frontend/package.json` — current dependency list (React 19, Next.js 15, wagmi 3)
- `frontend/tsconfig.json` — TypeScript config (moduleResolution: bundler, paths: @/*)
- `frontend/next.config.js` — current Next.js config (empty)

## Expected Output

- `frontend/package.json` — updated with `@monaco-editor/react` and `react-resizable-panels` dependencies
- `frontend/src/lib/solidity-language.ts` — new Solidity completion provider utility
- `frontend/src/components/ide/MonacoEditor.tsx` — new Monaco Editor wrapper with dynamic import
