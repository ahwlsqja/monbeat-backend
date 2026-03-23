# S01 ("Monaco Editor + IDE 레이아웃") — Research

**Date:** 2026-03-23
**Depth:** Targeted — known technology (Monaco Editor, React panels) applied to existing codebase with clear requirements.

## Summary

This slice replaces the current textarea-based code editor (`page.tsx`) with a Monaco Editor inside an IDE-style 3-panel resizable layout (Editor | Sidebar | Console). The existing codebase has a flat single-page layout with a `<textarea>` for Solidity source input and inline buttons for deploy/analysis. All supporting infrastructure (api-client, auth-context, wagmi-config, providers) remains unchanged.

**Key discovery: Monaco Editor already ships with built-in Solidity support** via the `sol` language identifier in `monaco-languages`. No custom Monarch tokenizer is needed for basic syntax highlighting — `sol` provides keywords, types, comments, strings, numbers out of the box. We only need a custom completion provider for Solidity snippets/keywords to enhance the editing experience beyond basic highlighting.

The `@monaco-editor/react` wrapper (v4.7.0) loads Monaco from CDN by default, avoiding webpack configuration entirely. Combined with `next/dynamic` + `ssr: false`, this cleanly solves the SSR compatibility risk. The `react-resizable-panels` library (v4+) provides a production-grade `Group/Panel/Separator` API perfect for the IDE layout, with collapsible panels and imperative resize control.

## Recommendation

1. **Use `@monaco-editor/react`** (not `react-monaco-editor`) — 380k+ weekly downloads, no webpack plugin required, CDN-based loading, clean `useMonaco` hook for language registration.
2. **Use `react-resizable-panels`** (v4+, `Group/Panel/Separator` API) for the IDE layout — trust score 10/10, supports nested horizontal+vertical groups, collapsible panels, imperative API.
3. **Use Monaco's built-in `sol` language** for Solidity highlighting. Register a custom `CompletionItemProvider` for enhanced Solidity snippets (contract, function, modifier, event templates).
4. **Dynamic import the entire editor component** via `next/dynamic({ ssr: false })` — this is the established pattern for Monaco + Next.js per the library's own documentation.
5. **Preserve all existing lib/ files unchanged** — `api-client.ts`, `auth-context.tsx`, `wagmi-config.ts`. The `page.tsx` is the only file that gets a full rewrite.

## Implementation Landscape

### Key Files

**Keep unchanged:**
- `frontend/src/lib/api-client.ts` — 7 API methods (getContractSource, deployContract, compileContract, analyzeError, getVibeScore, getDeployStatus, getUserProfile). All consumed by the new IDE page.
- `frontend/src/lib/auth-context.tsx` — GitHub OAuth context (token, user, login, logout). Used by header/toolbar.
- `frontend/src/lib/wagmi-config.ts` — Monad Testnet chain (10143), WalletConnect + injected connectors. Used by WalletConnectModal.
- `frontend/src/app/providers.tsx` — WagmiProvider + QueryClientProvider + AuthProvider wrapper. No changes needed.
- `frontend/src/app/layout.tsx` — Server component with metadata + Providers. No changes needed.
- `frontend/src/components/WalletConnectModal.tsx` — Wallet connection modal for direct deploys. Keep as-is, will be triggered from new IDE.
- `frontend/src/components/VibeStatus.tsx` — Paymaster status badge. Keep as-is, mount in IDE toolbar.
- `frontend/src/app/globals.css` — Minimal Tailwind import. May need minor additions for panel sizing (e.g., `html, body { height: 100%; overflow: hidden; }` for full-viewport IDE).

**Rewrite / Replace:**
- `frontend/src/app/page.tsx` — Currently 200-line monolithic component with textarea. Full rewrite to IDE layout that composes the new panel components. All state management for deploy/analyze/vibe-score stays here but rendered inside panel slots.

**Create new:**
- `frontend/src/components/ide/MonacoEditor.tsx` — Monaco Editor wrapper. Uses `next/dynamic` for SSR-safe import of `@monaco-editor/react`. Registers Solidity completion provider via `useMonaco` hook. Exposes `editorRef` for S02 to set markers and read values.
- `frontend/src/components/ide/IDELayout.tsx` — Top-level IDE layout using `react-resizable-panels`. Horizontal Group: [EditorPanel | SidebarPanel]. Vertical nested Group in EditorPanel area: [Editor | ConsolePanel].
- `frontend/src/components/ide/EditorPanel.tsx` — Contains the Monaco editor + toolbar (contract selector, deploy/analyze buttons).
- `frontend/src/components/ide/SidebarPanel.tsx` — Right sidebar. For S01: deploy result display, contract selector, vibe-score area (placeholder). S02/S03 will add more.
- `frontend/src/components/ide/ConsolePanel.tsx` — Bottom console. For S01: simple log output area (placeholder). S02 will add TransactionConsole.
- `frontend/src/lib/solidity-language.ts` — Solidity completion provider registration function. Called from MonacoEditor's `beforeMount`. Adds keyword/snippet completions and language configuration (brackets, auto-closing pairs, comments).

**Components to deprecate (not delete in S01 — S02/S03 replace them):**
- `frontend/src/components/CodeDiffView.tsx` — Will be replaced by Monaco DiffEditor in S02.
- `frontend/src/components/VibeScoreGauge.tsx` — Will be replaced by VibeScoreDashboard in S03.

### Build Order

1. **Install dependencies** — `@monaco-editor/react` + `react-resizable-panels`. Verify `npm install` succeeds and `npm run build` still works (no breaking peer deps with React 19 / Next.js 15).

2. **Create `solidity-language.ts`** — Register Solidity completion provider (contract/function/modifier/event snippets, Solidity keyword completions). Set language configuration (comments: `//` + `/* */`, brackets, auto-closing pairs `{`, `(`, `[`, `"`, `'`). This is a pure utility with no React dependency — easy to test and verify in isolation.

3. **Create `MonacoEditor.tsx`** — Dynamic import wrapper. This is the riskiest piece (SSR compat). Must confirm it renders without hydration errors. Exposes `editorRef` via `useImperativeHandle` or callback ref for downstream slices. Uses `defaultLanguage="sol"` and `theme="vs-dark"`. Calls Solidity language registration in `beforeMount`.

4. **Create `IDELayout.tsx` + panel components** — Build the 3-panel layout shell. Horizontal: [Editor area (flex grow) | Sidebar (20%, collapsible, min 200px)]. Vertical within editor area: [Monaco editor (70%) | Console (30%, collapsible, min 100px)]. Wire up resize handles.

5. **Rewrite `page.tsx`** — Compose IDELayout with existing state management. Move contract selector, deploy button, analyze button into EditorPanel toolbar. Move deploy result, vibe-score display into SidebarPanel. Console stays as placeholder. Keep all API call logic from existing page.tsx.

6. **Update `globals.css`** — Add full-viewport constraints: `html, body, #__next { height: 100vh; overflow: hidden; }` so the IDE fills the screen without scrolling.

### Verification Approach

1. **Build check:** `cd frontend && npm run build` must succeed with no errors. This proves SSR compatibility (Next.js static analysis pass).
2. **Bundle size check:** After build, check `.next/static` output for Monaco chunk. Expected: ~2MB separate chunk loaded lazily. Confirm it's NOT in the main bundle.
3. **Dev server visual check:** `npm run dev`, open browser, verify:
   - Monaco editor renders with Solidity syntax highlighting (keywords like `contract`, `function`, `pragma` are colored)
   - 3-panel layout is visible (editor, sidebar, console)
   - Resize handles work (drag between panels)
   - Console panel is collapsible
   - Contract selector loads source via API
   - Deploy button triggers existing API flow
   - Auth (login/logout) still works in toolbar
4. **No hydration errors:** Browser console must be free of React hydration mismatch warnings.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Monaco + React integration | `@monaco-editor/react` v4.7.0 | CDN-based loading, no webpack config, `useMonaco` hook, handles lifecycle/disposal. 1500+ dependents |
| Resizable panel layout | `react-resizable-panels` v4+ | Production-grade, trust score 10/10, mouse/touch/keyboard support, collapsible panels, imperative API, tiny bundle (~8KB) |
| Solidity syntax highlighting | Monaco built-in `sol` language | Already in `monaco-languages`. No custom Monarch tokenizer needed for basic highlighting |
| SSR-safe dynamic import | `next/dynamic` with `ssr: false` | Next.js built-in, proven pattern documented by @monaco-editor/react itself |

## Constraints

- **React 19 compatibility** — Both `@monaco-editor/react` and `react-resizable-panels` must work with React 19. `@monaco-editor/react@4.7.0` uses peer dep `react: ^16.8.0 || ^17.0.0 || ^18.0.0` — may show a peer dep warning with React 19 but functions correctly (Monaco doesn't use removed React APIs). Use `--legacy-peer-deps` if npm install rejects.
- **SSR: Monaco requires browser environment** — `window`, `document`, `navigator` are all required. Must use `next/dynamic` with `ssr: false`. The `useMonaco` hook also needs browser context — only call it from the dynamically imported component.
- **Full viewport IDE layout** — The IDE must fill 100vh. Current `page.tsx` uses `min-h-screen` with scroll. Must switch to `h-screen overflow-hidden` to prevent the IDE itself from scrolling (panels handle their own overflow).
- **`sol` not `solidity`** — Monaco's built-in Solidity language identifier is `sol`, not `solidity`. Using `solidity` silently falls back to plaintext with no highlighting.

## Common Pitfalls

- **`useMonaco()` returns null on first render** — The hook is async; Monaco loads from CDN. Must guard `useEffect` with `if (monaco)` check before registering languages/providers. The library docs show this pattern explicitly.
- **Monaco CDN loading stuck** — In restricted network environments, CDN loading may hang. The `@monaco-editor/react` `loading` prop shows a fallback. For production, consider configuring `loader.config({ paths: { vs: '/monaco-editor/min/vs' } })` to self-host, but CDN is fine for initial implementation.
- **react-resizable-panels v4 API rename** — v4 renamed `PanelGroup` → `Group`, `PanelResizeHandle` → `Separator`. Import from `react-resizable-panels` using `{ Group, Panel, Separator }`. Old examples using `PanelGroup` will cause import errors.
- **Monaco editor height must be explicit** — Monaco needs an explicit height (not `auto`). Use `height="100%"` with a parent that has a defined height from the panel layout. Without this, Monaco renders as 0px tall.
- **Hydration mismatch with dynamic import** — The `next/dynamic` loading component renders on server, Monaco renders on client. Use a simple `<div>Loading editor...</div>` as loading fallback — complex JSX in the loading state can cause hydration mismatches if it differs from what the client renders during replacement.

## Open Risks

- **React 19 peer dependency** — `@monaco-editor/react@4.7.0` declares React 18 as peer dep max. If `npm install` strictly rejects, use `--legacy-peer-deps`. Functionally verified to work with React 19 in practice but no official support yet. If a v5 is released with React 19 support, prefer that.
- **Monaco CDN availability** — Default CDN is `cdn.jsdelivr.net`. If this is slow/blocked in Korea, consider self-hosting Monaco assets under `public/monaco-editor/` and configuring `loader.config()`. This is a runtime concern, not a build concern.

## Boundary Outputs for S02/S03

This slice produces the following interfaces consumed by downstream slices:

1. **`editorRef`** — React ref to the Monaco editor instance. S02 uses `monaco.editor.setModelMarkers(model, owner, markers)` for inline error markers. S02 uses `editor.getValue()` to read current source.
2. **`IDELayout` ConsolePanel slot** — S02 mounts `TransactionConsole` here.
3. **`IDELayout` SidebarPanel slot** — S03 mounts `ContractInteraction` and `VibeScoreDashboard` here.
4. **Monaco instance** — S02 uses `monaco.editor.createDiffEditor()` or `<DiffEditor>` from `@monaco-editor/react` for AI fix suggestions.

## Sources

- @monaco-editor/react Next.js SSR integration (source: [Context7 docs](/suren-atoyan/monaco-react))
- react-resizable-panels IDE layout patterns (source: [Context7 docs](/bvaughn/react-resizable-panels))
- Monaco Solidity `sol` language support confirmed in monaco-languages PR #20 (source: [monaco-editor changelog](https://github.com/microsoft/monaco-editor/blob/main/CHANGELOG.md))
- Monaco Monarch tokenizer custom language API (source: [Context7 docs](/microsoft/monaco-editor))
