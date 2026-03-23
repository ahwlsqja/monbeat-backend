---
id: T02
parent: S01
milestone: M002
provides:
  - IDE 3-panel resizable layout (IDELayout with horizontal+vertical Groups)
  - EditorPanel, SidebarPanel, ConsolePanel composition wrappers
key_files:
  - frontend/src/components/ide/IDELayout.tsx
  - frontend/src/components/ide/EditorPanel.tsx
  - frontend/src/components/ide/SidebarPanel.tsx
  - frontend/src/components/ide/ConsolePanel.tsx
key_decisions:
  - Used `orientation` prop (not `direction`) for react-resizable-panels v4 Group API — v4 renamed this from earlier versions
  - Renamed destructured `console` prop to `consolePanelContent` inside IDELayout to avoid shadowing the global `console` object
patterns_established:
  - Panel wrappers are pure composition components — they accept children/toolbar via props and handle only styling and layout, no state
  - All IDE panel components marked "use client" since they'll be composed inside client-side page.tsx
observability_surfaces:
  - React DevTools shows component tree: IDELayout > Group > Panel > EditorPanel/SidebarPanel/ConsolePanel
  - Panel resize state is managed internally by react-resizable-panels; drag handles highlight amber on hover for user affordance
  - Collapsible panels (sidebar, console) can be collapsed via drag — collapsed state is visible as panel width/height reaching zero
duration: 8m
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T02: Create IDE 3-panel resizable layout components

**Created 4 IDE layout components using react-resizable-panels v4 Group/Panel/Separator API: IDELayout (3-panel composition), EditorPanel (toolbar+content), SidebarPanel (scrollable sidebar), ConsolePanel (monospace log area), all with dark theme styling and collapsible panels.**

## What Happened

Created four `"use client"` components in `frontend/src/components/ide/`:

1. **EditorPanel** — flex-column layout with optional `toolbar` slot at top and `children` filling remaining space. Uses `overflow-hidden` so Monaco can manage its own scrolling.
2. **SidebarPanel** — scrollable container with optional `title` header, `border-l` separator styling, and `bg-gray-800` dark theme.
3. **ConsolePanel** — fixed "Console" header bar (`font-mono text-xs`) with scrollable content area below, `border-t` separator styling.
4. **IDELayout** — top-level composition using `react-resizable-panels` v4 API. Horizontal Group splits editor area (75%, min 40%) from sidebar (25%, min 15%, collapsible). Vertical Group inside editor area splits editor (70%, min 30%) from console (30%, min 10%, collapsible). Separator elements are styled as thin gray lines with amber hover highlight.

Initial build failed because the task plan specified `direction` prop but v4 uses `orientation`. Fixed to `orientation="horizontal"` / `orientation="vertical"` and rebuild succeeded.

## Verification

All task-level and slice-level verification checks pass:
- `npm run build` exits 0 (no SSR errors, only pre-existing wagmi warnings)
- `npm test` — all 11 tests pass
- All 4 component files exist
- IDELayout uses `Group`, `Separator`, and `collapsible` props
- All components include `"use client"` directive

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd frontend && npm run build` | 0 | ✅ pass | 36s |
| 2 | `cd frontend && npm test -- --passWithNoTests` | 0 | ✅ pass | 1s |
| 3 | `test -f frontend/src/components/ide/IDELayout.tsx` | 0 | ✅ pass | <1s |
| 4 | `test -f frontend/src/components/ide/EditorPanel.tsx` | 0 | ✅ pass | <1s |
| 5 | `test -f frontend/src/components/ide/SidebarPanel.tsx` | 0 | ✅ pass | <1s |
| 6 | `test -f frontend/src/components/ide/ConsolePanel.tsx` | 0 | ✅ pass | <1s |
| 7 | `grep -q "Group" frontend/src/components/ide/IDELayout.tsx` | 0 | ✅ pass | <1s |
| 8 | `grep -q "Separator" frontend/src/components/ide/IDELayout.tsx` | 0 | ✅ pass | <1s |
| 9 | `grep -q "collapsible" frontend/src/components/ide/IDELayout.tsx` | 0 | ✅ pass | <1s |

## Diagnostics

- **Panel structure:** React DevTools shows `IDELayout > Group > Panel > EditorPanel/SidebarPanel/ConsolePanel` hierarchy.
- **Resize behavior:** Drag the amber-highlighted separators to resize. Console and sidebar panels are collapsible — drag to minimum to collapse.
- **Composition API:** Each panel accepts `children` (and EditorPanel accepts `toolbar`, SidebarPanel accepts `title`). T03 will slot in Monaco editor, deploy/vibe content, and console output.

## Deviations

- **`orientation` instead of `direction`:** The task plan specified `direction="horizontal"` but react-resizable-panels v4 uses `orientation` prop. Fixed during implementation.
- **`console` prop shadowing:** Renamed the destructured `console` prop to `consolePanelContent` to avoid shadowing `window.console`. The TypeScript interface still uses `console` as the prop name for API clarity.

## Known Issues

- Pre-existing wagmi connector warnings during build (porto, coinbaseWallet, metaMask, baseAccount modules not found) — unrelated to IDE layout changes, present before this task.

## Files Created/Modified

- `frontend/src/components/ide/EditorPanel.tsx` — new: editor panel wrapper with optional toolbar slot
- `frontend/src/components/ide/SidebarPanel.tsx` — new: scrollable sidebar panel with optional title header
- `frontend/src/components/ide/ConsolePanel.tsx` — new: console panel with fixed header and scrollable log area
- `frontend/src/components/ide/IDELayout.tsx` — new: 3-panel resizable layout using Group/Panel/Separator
