---
id: S01
parent: M002
milestone: M002
provides:
  - MonacoEditor SSR-safe wrapper with Solidity completion provider (9 snippets, 50 keywords)
  - IDE 3-panel resizable layout (EditorPanel | SidebarPanel | ConsolePanel) with collapsible panels
  - Full-viewport IDE page composition with all existing API flows preserved (deploy, vibe-score, auth, wallet)
  - editorRef exposed for downstream S02 marker/value API usage
requires:
  - slice: none
    provides: first slice — no dependencies
affects:
  - S02 (consumes editorRef for inline error markers, ConsolePanel slot for TransactionConsole)
  - S03 (consumes SidebarPanel slot for contract interaction UI, Vibe-Score dashboard)
  - S04 (consumes entire IDE layout for responsive adaptation)
key_files:
  - frontend/src/components/ide/MonacoEditor.tsx — SSR-safe next/dynamic wrapper
  - frontend/src/components/ide/MonacoEditorInner.tsx — actual Monaco Editor with sol language + vs-dark theme
  - frontend/src/lib/solidity-language.ts — Solidity completion provider (9 snippets, 50 keywords)
  - frontend/src/components/ide/IDELayout.tsx — 3-panel resizable layout (Group/Panel/Separator)
  - frontend/src/components/ide/EditorPanel.tsx — editor panel with toolbar slot
  - frontend/src/components/ide/SidebarPanel.tsx — scrollable sidebar with title header
  - frontend/src/components/ide/ConsolePanel.tsx — console panel with fixed header
  - frontend/src/app/page.tsx — full rewrite composing IDELayout with existing state/handlers
  - frontend/src/app/globals.css — full-viewport CSS (overflow hidden)
key_decisions:
  - Two-file Monaco architecture (MonacoEditor.tsx wrapper + MonacoEditorInner.tsx impl) for real webpack code splitting — single-file next/dynamic with Promise.resolve() does not split
  - Used `defaultLanguage="sol"` (Monaco's built-in Solidity identifier), not "solidity"
  - Used react-resizable-panels v4 `orientation` prop (not `direction` — renamed in v4)
  - WalletConnectModal rendered as sibling of IDELayout via Fragment, not inside any panel, to avoid z-index/overflow clipping
  - Page-level state stays in page.tsx; panel components are pure composition slots
  - Used --legacy-peer-deps for npm install (@monaco-editor/react declares React 18 max but works with React 19)
patterns_established:
  - SSR-unsafe components use next/dynamic with ssr:false in a separate wrapper file that re-exports the type interface
  - Language providers registered in beforeMount callback (runs once before editor init)
  - Panel components are pure composition wrappers — accept children/toolbar via props, handle only styling and layout
  - Overlays (modals) render outside IDELayout as siblings to avoid overflow clipping
observability_surfaces:
  - Monaco ~630KB chunk loads lazily via Network tab (separate from 64KB main page bundle)
  - "Loading editor..." fallback visible while Monaco CDN loads
  - React DevTools: VibeLoomPage > IDELayout > Group > Panel > EditorPanel/SidebarPanel/ConsolePanel
  - All existing API console.error patterns in api-client preserved
  - Full-viewport layout — no page scrollbar (document.documentElement.scrollHeight === clientHeight)
drill_down_paths:
  - .gsd/milestones/M002/slices/S01/tasks/T01-SUMMARY.md
  - .gsd/milestones/M002/slices/S01/tasks/T02-SUMMARY.md
  - .gsd/milestones/M002/slices/S01/tasks/T03-SUMMARY.md
duration: 33m
verification_result: passed
completed_at: 2026-03-23
---

# S01: Monaco Editor + IDE 레이아웃

**Replaced textarea editor with SSR-safe Monaco Editor (Solidity syntax highlighting + 59-item completion provider) inside a 3-panel resizable IDE layout, preserving all existing deploy/vibe-score/auth/wallet API flows — build passes, Monaco loads as separate 630KB lazy chunk.**

## What Happened

Three tasks executed sequentially, each building on the prior:

**T01 (15m)** installed `@monaco-editor/react@4.7.0` and `react-resizable-panels@4.7.5` (with `--legacy-peer-deps` for React 19 compat), then created the Monaco wrapper as a two-file architecture. `MonacoEditorInner.tsx` contains the actual `<Editor>` with `defaultLanguage="sol"`, `theme="vs-dark"`, and `onEditorReady` callback. `MonacoEditor.tsx` is a thin `next/dynamic({ ssr: false })` wrapper that lazy-imports the inner file — this split is necessary because `next/dynamic` with `Promise.resolve()` doesn't trigger real webpack code splitting. `solidity-language.ts` provides a `registerSolidityLanguage(monaco)` function with 9 snippet templates (contract, function, modifier, event, mapping, constructor, error, struct, enum) and 50 Solidity keyword completions, registered via `beforeMount`.

**T02 (8m)** created four panel components using react-resizable-panels v4 Group/Panel/Separator API. `IDELayout.tsx` composes a horizontal Group (editor area 75% | sidebar 25% collapsible) with a vertical Group inside the editor area (editor 70% | console 30% collapsible). `EditorPanel`, `SidebarPanel`, and `ConsolePanel` are pure composition wrappers accepting children via props. Hit the v4 `orientation` vs `direction` rename — fixed and documented in KNOWLEDGE.md.

**T03 (10m)** fully rewrote `page.tsx` to compose `IDELayout` with existing state management. The `<textarea>` is replaced by `MonacoEditor` bound to `contractSource` state. EditorPanel toolbar contains contract selector, deploy/analyze buttons, VibeStatus, and auth controls. SidebarPanel shows deploy results, errors with AI fix suggestions (CodeDiffView), and VibeScoreGauge. ConsolePanel has placeholder text for S02's TransactionConsole. `WalletConnectModal` renders as a Fragment sibling of IDELayout. Updated `globals.css` with `html, body { height: 100%; overflow: hidden }` for full-viewport IDE.

## Verification

All 16 slice-level verification checks pass:

| Check | Result |
|-------|--------|
| `npm run build` exits 0 | ✅ (36s, no SSR errors) |
| `npm test` — all tests pass | ✅ (11/11, 0.85s) |
| MonacoEditor.tsx exists | ✅ |
| IDELayout.tsx exists | ✅ |
| solidity-language.ts exists | ✅ |
| EditorPanel.tsx, SidebarPanel.tsx, ConsolePanel.tsx exist | ✅ |
| MonacoEditor uses "sol" language identifier | ✅ |
| MonacoEditor uses next/dynamic | ✅ |
| IDELayout uses Group (react-resizable-panels) | ✅ |
| page.tsx uses IDELayout + MonacoEditor | ✅ |
| No `<textarea>` in page.tsx | ✅ |
| deployContract, getVibeScore, useAuth preserved in page.tsx | ✅ |
| globals.css has overflow hidden | ✅ |
| Monaco chunk lazily loaded (630KB separate from 64KB page) | ✅ |

## Requirements Advanced

- **R016** (프론트엔드 IDE 리디자인) — Monaco Editor + IDE 3-panel layout foundation established. Solidity syntax highlighting, resizable panels, and all existing API integrations preserved. R016 remains active — S02-S04 needed for inline markers, contract interaction, vibe-score dashboard, and responsive polish.
- **R009** (프론트엔드 API 엔드포인트 전환) — All 7 existing API methods (getContractSource, deployContract, getVibeScore, etc.) preserved in the new IDE layout without modification.

## New Requirements Surfaced

- none

## Deviations

- **Two-file Monaco split** — T01 used MonacoEditor.tsx + MonacoEditorInner.tsx instead of a single file because `next/dynamic` with `Promise.resolve()` doesn't perform real code splitting. The dynamic `import()` call is necessary for webpack/turbopack to create a separate chunk.
- **react-resizable-panels v4 API** — T02 used `orientation` prop instead of `direction` (renamed in v4). Already documented in KNOWLEDGE.md.

## Known Limitations

- **Monaco CDN dependency** — Monaco loads from CDN via @monaco-editor/react. If CDN is blocked/slow, the "Loading editor..." fallback persists indefinitely. No offline fallback.
- **Pre-existing wagmi warnings** — Build produces porto/coinbaseWallet/metaMask/baseAccount module-not-found warnings. These predate S01 and don't affect functionality.
- **ConsolePanel is placeholder** — Shows static text "Console output will appear here..." until S02 implements TransactionConsole.
- **SidebarPanel content is sparse** — Shows deploy results and vibe-score when available, but S03's contract interaction UI and vibe-score dashboard will fill it out.

## Follow-ups

- S02 should consume `editorRef` (exposed via `onEditorReady` callback in page.tsx's `editorInstance` state) to set inline error markers via `monaco.editor.setModelMarkers()`.
- S02 should replace ConsolePanel placeholder with TransactionConsole component.
- S04 should address the pre-existing wagmi connector warnings during responsive/polish pass.

## Files Created/Modified

- `frontend/package.json` — added @monaco-editor/react@4.7.0, react-resizable-panels@4.7.5
- `frontend/src/lib/solidity-language.ts` — new: Solidity language config + completion provider (9 snippets, 50 keywords)
- `frontend/src/components/ide/MonacoEditor.tsx` — new: SSR-safe next/dynamic wrapper
- `frontend/src/components/ide/MonacoEditorInner.tsx` — new: actual Monaco Editor component (sol language, vs-dark theme, onEditorReady)
- `frontend/src/components/ide/IDELayout.tsx` — new: 3-panel resizable layout (Group/Panel/Separator)
- `frontend/src/components/ide/EditorPanel.tsx` — new: editor panel with optional toolbar slot
- `frontend/src/components/ide/SidebarPanel.tsx` — new: scrollable sidebar with optional title header
- `frontend/src/components/ide/ConsolePanel.tsx` — new: console panel with fixed header + scrollable area
- `frontend/src/app/page.tsx` — full rewrite: IDELayout + MonacoEditor composition, all existing state/handlers preserved
- `frontend/src/app/globals.css` — updated: full-viewport CSS (html, body height 100%, overflow hidden)

## Forward Intelligence

### What the next slice should know
- `editorInstance` state in page.tsx holds the Monaco editor instance after mount. Access it for `monaco.editor.setModelMarkers(model, owner, markers)` to set inline error markers (S02).
- The `onEditorReady(editor, monaco)` callback fires once. Both the editor instance and monaco namespace are available.
- ConsolePanel accepts `children` — drop the TransactionConsole component in as `console` prop of IDELayout.
- SidebarPanel accepts `children` and `title` props — expand with contract interaction UI in S03.

### What's fragile
- **Monaco CDN loading** — if the CDN fails, no editor renders and no fallback editor exists. The only signal is the "Loading editor..." text remaining visible.
- **react-resizable-panels percentage-based sizes** — changing the Panel `defaultSize` values affects the initial layout ratio. The current 75/25 horizontal and 70/30 vertical split works for desktop; S04 will need to adjust for mobile.
- **`--legacy-peer-deps` install** — @monaco-editor/react@4.7.0 declares React 18 max peer dep. Works with React 19 in practice, but npm won't auto-resolve it. All future `npm install` in this project should use `--legacy-peer-deps`.

### Authoritative diagnostics
- **Build output** — `npm run build` shows page size (64KB / 184KB first load). Monaco chunk is separate and not listed in the route table (it's a dynamic import, loaded on demand).
- **Network tab** — first page load shows the Monaco chunk (~630KB) loading separately after the page shell renders.
- **React DevTools** — component tree shows `VibeLoomPage > IDELayout > Group > Panel > EditorPanel/SidebarPanel/ConsolePanel`.

### What assumptions changed
- **Monarch tokenizer not needed** — the plan mentioned "커스텀 monarch tokenizer 등록" but Monaco has a built-in `sol` language with syntax highlighting. Only the completion provider needed custom registration.
- **Bundle size concern resolved** — Monaco's ~2MB was a stated risk, but `next/dynamic({ ssr: false })` with the two-file split confirmed lazy loading. The main page bundle is only 64KB.
