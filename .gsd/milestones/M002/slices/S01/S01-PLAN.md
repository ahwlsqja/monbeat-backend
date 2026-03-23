# S01: Monaco Editor + IDE 레이아웃

**Goal:** Replace the textarea-based editor with Monaco Editor inside an IDE-style 3-panel resizable layout (Editor | Sidebar | Console), with Solidity syntax highlighting and existing API integration preserved.
**Demo:** Monaco Editor renders Solidity code with syntax highlighting. IDE has 3 resizable panels (editor, sidebar, console). Contract selector loads source via API. Deploy and Vibe-Score buttons work as before. `npm run build` succeeds with no SSR errors.

## Must-Haves

- Monaco Editor renders with Solidity (`sol`) syntax highlighting (keywords `contract`, `function`, `pragma` are colored)
- Solidity completion provider registered (contract/function/modifier/event snippets)
- IDE 3-panel layout: Editor (main) | Sidebar (right, collapsible) | Console (bottom, collapsible)
- Panels are resizable via drag handles
- `editorRef` exposed for downstream slices (S02 marker API, S02 value read/write)
- Contract selector loads source via existing `getContractSource()` API
- Deploy button triggers existing `deployContract()` flow
- Vibe-Score analysis triggers existing `getVibeScore()` flow
- Auth (login/logout) works in toolbar
- WalletConnect modal triggers on paymaster quota exceeded
- `npm run build` succeeds (SSR compatibility proven)
- No React hydration mismatch warnings in browser console

## Proof Level

- This slice proves: contract (Monaco SSR compat + panel layout) + integration (existing API flow preserved)
- Real runtime required: yes (dev server visual check for Monaco rendering)
- Human/UAT required: no (build success + component render tests sufficient)

## Verification

- `cd frontend && npm run build` — exits 0, no SSR errors
- `cd frontend && npm test -- --passWithNoTests` — all tests pass
- `test -f frontend/src/components/ide/MonacoEditor.tsx` — Monaco wrapper exists
- `test -f frontend/src/components/ide/IDELayout.tsx` — IDE layout exists
- `test -f frontend/src/lib/solidity-language.ts` — Solidity language util exists
- `grep -q "sol" frontend/src/components/ide/MonacoEditor.tsx` — uses `sol` language identifier
- `grep -q "next/dynamic" frontend/src/components/ide/MonacoEditor.tsx` — uses dynamic import
- `grep -q "Group" frontend/src/components/ide/IDELayout.tsx` — uses react-resizable-panels Group
- Build output check: Monaco chunk is lazily loaded (not in main bundle)

## Observability / Diagnostics

- Runtime signals: console.error on API call failures (existing api-client pattern), Monaco loading state visible via `loading` prop fallback
- Inspection surfaces: browser DevTools Network tab shows Monaco CDN chunk loading separately; React DevTools shows component tree with IDELayout > EditorPanel/SidebarPanel/ConsolePanel
- Failure visibility: Monaco CDN load failure shows "Loading editor..." fallback indefinitely; hydration mismatches appear in browser console as React warnings
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `frontend/src/lib/api-client.ts` (all 7 API methods), `frontend/src/lib/auth-context.tsx` (useAuth hook), `frontend/src/lib/wagmi-config.ts` (wagmi config), `frontend/src/app/providers.tsx` (provider hierarchy), existing components (`VibeStatus`, `VibeScoreGauge`, `CodeDiffView`, `WalletConnectModal`)
- New wiring introduced in this slice: `page.tsx` rewritten to compose `IDELayout` with `MonacoEditor`, panel components, and existing state management; `globals.css` updated for full-viewport IDE
- What remains before the milestone is truly usable end-to-end: S02 (compile errors → inline markers, transaction console, AI diff viewer), S03 (contract interaction, Vibe-Score dashboard), S04 (responsive + polish + deploy)

## Tasks

- [x] **T01: Install dependencies and create Monaco Editor wrapper with Solidity language support** `est:1h`
  - Why: This is the highest-risk piece — Monaco requires browser APIs and must be dynamically imported to avoid SSR failures. Getting this working first de-risks the entire slice. Also creates the Solidity completion provider that enhances the editing experience.
  - Files: `frontend/package.json`, `frontend/src/lib/solidity-language.ts`, `frontend/src/components/ide/MonacoEditor.tsx`
  - Do: Install `@monaco-editor/react` and `react-resizable-panels` (use `--legacy-peer-deps` if React 19 peer dep conflicts). Create `solidity-language.ts` with Solidity completion provider (contract/function/modifier/event snippets) and language configuration (comments, brackets, auto-closing pairs). Create `MonacoEditor.tsx` as a dynamic-import wrapper using `next/dynamic({ ssr: false })` — it must expose `editorRef` via callback prop for downstream slices, use `defaultLanguage="sol"` and `theme="vs-dark"`, and call Solidity language registration in `beforeMount`. Relevant skills: `react-best-practices`.
  - Verify: `cd frontend && npm run build` exits 0; `test -f frontend/src/components/ide/MonacoEditor.tsx && test -f frontend/src/lib/solidity-language.ts`
  - Done when: `npm run build` succeeds, MonacoEditor component exists with dynamic import and Solidity language registration

- [x] **T02: Create IDE 3-panel resizable layout components** `est:45m`
  - Why: The IDE layout is the structural foundation — all content (editor, sidebar, console) is rendered inside these panels. Using `react-resizable-panels` (Group/Panel/Separator API) gives production-grade resize handling with collapsible panels.
  - Files: `frontend/src/components/ide/IDELayout.tsx`, `frontend/src/components/ide/EditorPanel.tsx`, `frontend/src/components/ide/SidebarPanel.tsx`, `frontend/src/components/ide/ConsolePanel.tsx`
  - Do: Create `IDELayout.tsx` as the top-level layout using `react-resizable-panels` v4 API (`import { Group, Panel, Separator } from "react-resizable-panels"`). Structure: outer horizontal Group [EditorArea | Separator | SidebarPanel], inner vertical Group in EditorArea [EditorPanel | Separator | ConsolePanel]. EditorPanel takes `children` (for Monaco editor + toolbar). SidebarPanel takes `children` (for deploy results, vibe-score). ConsolePanel takes `children` (placeholder for S02 transaction console). Sidebar: `defaultSize="25%"`, `minSize={200}`, `collapsible`. Console: `defaultSize="30%"`, `minSize={100}`, `collapsible`. All panels accept `children` as props for composition. Relevant skills: `react-best-practices`.
  - Verify: `cd frontend && npm run build` exits 0; `grep -q "Group" frontend/src/components/ide/IDELayout.tsx`
  - Done when: All 4 panel components exist, IDELayout composes them with resize handles, build succeeds

- [x] **T03: Rewrite page.tsx to compose IDE layout with existing features and update globals.css** `est:1h30m`
  - Why: This is the integration closure task — it wires the Monaco editor and IDE layout into the actual page, preserving all existing functionality (contract selector, deploy, analyze, vibe-score, auth, wallet connect). Without this, the new components are unused scaffolding.
  - Files: `frontend/src/app/page.tsx`, `frontend/src/app/globals.css`
  - Do: Full rewrite of `page.tsx` to compose `IDELayout` → `EditorPanel` (Monaco editor + toolbar with contract selector, deploy/analyze buttons, auth controls) → `SidebarPanel` (deploy result, vibe-score gauge, error display, AI fix suggestions) → `ConsolePanel` (placeholder log output). All existing state management (`handleDeploy`, `handleAnalyzeVibeScore`, `handleApplyFix`, `handleWalletDeploySuccess`) stays in `page.tsx` but renders inside panel slots. Import `MonacoEditor` and pass value/onChange to sync with `contractSource` state. Import existing components (`VibeStatus`, `VibeScoreGauge`, `CodeDiffView`, `WalletConnectModal`) into their new panel positions. Update `globals.css` to add `html, body { height: 100vh; overflow: hidden; }` for full-viewport IDE layout. The page should use `h-screen overflow-hidden` instead of `min-h-screen`. Relevant skills: `react-best-practices`, `frontend-design`.
  - Verify: `cd frontend && npm run build` exits 0; `grep -q "IDELayout" frontend/src/app/page.tsx && grep -q "MonacoEditor" frontend/src/app/page.tsx`
  - Done when: `npm run build` succeeds, page.tsx uses IDELayout + MonacoEditor, all existing features (contract selector, deploy, analyze, vibe-score, auth, wallet) are wired into panel slots

## Files Likely Touched

- `frontend/package.json` — add `@monaco-editor/react`, `react-resizable-panels`
- `frontend/src/lib/solidity-language.ts` — new: Solidity completion provider + language config
- `frontend/src/components/ide/MonacoEditor.tsx` — new: dynamic-import Monaco wrapper
- `frontend/src/components/ide/IDELayout.tsx` — new: 3-panel resizable layout
- `frontend/src/components/ide/EditorPanel.tsx` — new: editor area panel
- `frontend/src/components/ide/SidebarPanel.tsx` — new: right sidebar panel
- `frontend/src/components/ide/ConsolePanel.tsx` — new: bottom console panel
- `frontend/src/app/page.tsx` — rewrite: compose IDE layout with existing state
- `frontend/src/app/globals.css` — update: full-viewport constraints
