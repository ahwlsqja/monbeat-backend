---
estimated_steps: 4
estimated_files: 3
---

# T01: Add responsive layout with mobile tab switching and toolbar collapse

**Slice:** S04 — 반응형 + 폴리싱 + Vercel 배포
**Milestone:** M002

## Description

The IDE currently uses a fixed 3-panel resizable layout (`react-resizable-panels`) that is unusable on mobile — drag handles require desktop precision and panels can't be resized on touch devices. This task creates a `useIsMobile` hook and refactors `IDELayout` to switch between tab-based UI on mobile and the existing 3-panel layout on desktop. The toolbar in `page.tsx` is also adjusted to wrap gracefully on narrow viewports.

**Key constraints:**
- `window.matchMedia` is not available during SSR — the hook must default to `false` (desktop) and update on client mount
- Monaco Editor requires explicit height — the tab layout container must fill the viewport minus the tab bar
- `react-resizable-panels` components stay imported (already in bundle from S01) — don't try to lazy-load them
- No new npm dependencies — use native `matchMedia` API
- The existing desktop layout must not change at all — this is additive

**Relevant skills:** `react-best-practices`, `frontend-design`

## Steps

1. **Create `useIsMobile` hook** at `frontend/src/hooks/useIsMobile.ts`:
   - `useState(false)` default (SSR-safe, avoids hydration mismatch)
   - `useEffect` with `window.matchMedia('(max-width: 767px)')` listener
   - Return `isMobile` boolean
   - Clean up event listener on unmount

2. **Refactor `IDELayout.tsx`** to support responsive rendering:
   - Import and use `useIsMobile` hook
   - When `isMobile === false`: render the current `Group/Panel/Separator` layout exactly as-is (no changes)
   - When `isMobile === true`: render a tab-based layout with 3 tabs: "Editor", "Results", "Console"
   - Tab state: `useState<'editor' | 'sidebar' | 'console'>('editor')`
   - Tab bar: fixed at top with 3 buttons, active tab highlighted with amber styling
   - Content area: `div` with `flex-1 overflow-hidden` showing only the active tab's content
   - The outer container keeps `h-screen w-screen overflow-hidden bg-gray-900`

3. **Adjust toolbar in `page.tsx`** for mobile:
   - The `editorToolbar` already uses `flex-wrap` which handles basic wrapping
   - Add responsive text sizing: contract selector buttons use `text-[10px]` on very small screens (via Tailwind responsive prefix or conditional class)
   - Ensure the title, buttons, and auth section don't overflow

4. **Verify build and tests pass**:
   - Run `npm run build` — must exit 0
   - Run `npm test` — all 57 tests must pass
   - Verify `useIsMobile.ts` exists
   - Verify `IDELayout.tsx` contains `isMobile` logic

## Must-Haves

- [ ] `useIsMobile` hook exists at `frontend/src/hooks/useIsMobile.ts` with SSR-safe default
- [ ] Mobile layout (<768px) shows tab-based UI with Editor/Results/Console tabs
- [ ] Desktop layout (≥768px) renders unchanged 3-panel resizable layout
- [ ] Monaco Editor fills container height in tab layout (no collapsed/invisible editor)
- [ ] `npm run build` exits 0
- [ ] `npm test` passes (57 tests, 0 failures)

## Verification

- `cd frontend && npm run build` exits 0
- `cd frontend && npm test` — 57 tests pass
- `grep -q "useIsMobile" frontend/src/hooks/useIsMobile.ts`
- `grep -q "isMobile" frontend/src/components/ide/IDELayout.tsx`
- `grep -q "matchMedia" frontend/src/hooks/useIsMobile.ts`

## Inputs

- `frontend/src/components/ide/IDELayout.tsx` — current 3-panel layout (33 lines, Group/Panel/Separator)
- `frontend/src/app/page.tsx` — toolbar and IDELayout usage (394 lines)
- `frontend/src/components/ide/EditorPanel.tsx` — editor container with toolbar slot
- `frontend/src/components/ide/SidebarPanel.tsx` — sidebar container with title
- `frontend/src/components/ide/ConsolePanel.tsx` — console container

## Expected Output

- `frontend/src/hooks/useIsMobile.ts` — new: viewport detection hook
- `frontend/src/components/ide/IDELayout.tsx` — modified: responsive conditional rendering (tab vs panels)
- `frontend/src/app/page.tsx` — modified: toolbar responsive adjustments

## Observability Impact

- **New signals:** `useIsMobile` hook state (`isMobile: boolean`) observable in React DevTools on IDELayout. Active mobile tab (`activeTab: 'editor' | 'sidebar' | 'console'`) also visible in IDELayout component state.
- **Inspection:** Toggle browser DevTools device toolbar to <768px to verify tab UI appears. Evaluate `window.matchMedia('(max-width: 767px)').matches` in console to confirm media query match.
- **Failure states:** Hydration mismatch warnings in browser console indicate SSR default doesn't match client — the hook defaults to `false` (desktop) to prevent this. If Monaco editor is invisible in mobile tab view, the container height chain (`h-full` / `flex-1`) is broken.
