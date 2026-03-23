---
id: T03
parent: S01
milestone: M002
provides:
  - Fully composed IDE page with MonacoEditor, 3-panel resizable layout, and all existing features
  - Full-viewport CSS for IDE layout (no page scroll)
key_files:
  - frontend/src/app/page.tsx
  - frontend/src/app/globals.css
key_decisions:
  - Wrapped MonacoEditor onChange to pass `(value: string) => void` since the inner component already normalizes undefined; avoids double-wrapping
  - Used Fragment wrapper (`<>...</>`) instead of div to render IDELayout + WalletConnectModal as siblings, keeping the modal outside the layout
  - Toolbar uses compact sizing (text-xs, px-2 py-1) to maximize editor space inside the fixed-height IDE
patterns_established:
  - Page-level state stays in page.tsx; panel components are pure composition slots receiving JSX via props
  - WalletConnectModal (and other overlays) render as siblings of IDELayout, not inside any panel, to avoid z-index and overflow clipping issues
observability_surfaces:
  - React DevTools shows VibeLoomPage > IDELayout > Group > Panel > EditorPanel/SidebarPanel/ConsolePanel hierarchy
  - Full-viewport CSS (overflow:hidden on html/body) removes page scrollbar — visible absence of scroll affordance
  - Monaco ~630KB chunk loads lazily via Network tab (separate from 64KB main page bundle)
  - All existing API console.error patterns in api-client preserved — deploy/vibe-score/auth failures log to console
duration: 10m
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T03: Rewrite page.tsx to compose IDE layout with existing features and update globals.css

**Rewrote page.tsx to compose IDELayout with MonacoEditor replacing textarea, distributed existing features into 3 resizable panels (editor+toolbar, sidebar with deploy/vibe results, console placeholder), updated globals.css for full-viewport IDE — all existing API flows preserved, build succeeds.**

## What Happened

Updated `globals.css` to add `height: 100%` and `overflow: hidden` on both `html` and `body`, ensuring the IDE layout fills the entire viewport without page scrolling.

Fully rewrote `page.tsx` while preserving every existing state variable and handler function. The `<textarea>` is replaced by the `MonacoEditor` wrapper (from T01), and the entire return JSX is replaced by an `IDELayout` (from T02) composition:

- **EditorPanel** — toolbar contains the app title ("Monad Vibe-Loom"), contract selector buttons, Deploy/Vibe Score action buttons, VibeStatus badge, and auth controls (GitHub login/logout). Children contain the MonacoEditor bound to `contractSource` state.
- **SidebarPanel** — titled "Results", contains deploy success display, error display with AI fix suggestions (CodeDiffView), and VibeScoreGauge. All conditional rendering logic is identical to before.
- **ConsolePanel** — placeholder text "Console output will appear here..." for S02's TransactionConsole.
- **WalletConnectModal** — rendered outside IDELayout as a sibling (Fragment wrapper) to avoid overflow clipping from panel containers.

Added `editorInstance` state with `handleEditorReady` callback, exposing the Monaco editor instance for downstream slice use (S02 marker API).

## Verification

All 13 verification checks pass (7 task-level + 6 slice-level). Build exits 0, all 11 existing tests pass. Monaco chunk confirmed as lazily loaded (630KB separate from 64KB main bundle).

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd frontend && npm run build` | 0 | ✅ pass | 43s |
| 2 | `cd frontend && npm test -- --passWithNoTests` | 0 | ✅ pass | 1s |
| 3 | `grep -q "IDELayout" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 4 | `grep -q "MonacoEditor" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 5 | `! grep -q "<textarea" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 6 | `grep -q "deployContract" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 7 | `grep -q "getVibeScore" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 8 | `grep -q "useAuth" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 9 | `grep -q "overflow.*hidden" frontend/src/app/globals.css` | 0 | ✅ pass | <1s |
| 10 | `test -f frontend/src/components/ide/MonacoEditor.tsx` | 0 | ✅ pass | <1s |
| 11 | `test -f frontend/src/components/ide/IDELayout.tsx` | 0 | ✅ pass | <1s |
| 12 | `test -f frontend/src/lib/solidity-language.ts` | 0 | ✅ pass | <1s |
| 13 | `grep -q "Group" frontend/src/components/ide/IDELayout.tsx` | 0 | ✅ pass | <1s |

## Diagnostics

- **Component tree:** React DevTools → Components tab → `VibeLoomPage > IDELayout > Group > Panel > EditorPanel / SidebarPanel / ConsolePanel`. Each panel's children are the slotted JSX from page.tsx.
- **Monaco lazy load:** Network tab on first page load should show a separate ~630KB JS chunk (the Monaco editor payload). If this chunk fails to load, the "Loading editor..." placeholder persists indefinitely.
- **API flows:** All deploy/vibe-score/contract-source API calls still go through `api-client.ts` which logs failures to `console.error`. Inspect via browser DevTools Console tab.
- **Full-viewport layout:** The page should have no scrollbar. `document.documentElement.scrollHeight === document.documentElement.clientHeight` should be true.
- **Editor ref:** `editorInstance` state is set once Monaco mounts. Downstream tasks (S02) can consume it for marker API and value access.

## Deviations

None. Implementation follows the task plan exactly.

## Known Issues

- Pre-existing wagmi connector warnings during build (porto, coinbaseWallet, metaMask, baseAccount modules not found) — unrelated to this task, present since before S01.

## Files Created/Modified

- `frontend/src/app/page.tsx` — full rewrite: composes IDELayout with MonacoEditor, EditorPanel toolbar, SidebarPanel results, ConsolePanel placeholder; all existing state/handlers preserved
- `frontend/src/app/globals.css` — updated: added `html, body { height: 100%; overflow: hidden }` for full-viewport IDE layout
