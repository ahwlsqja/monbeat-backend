---
id: S04
parent: M002
milestone: M002
provides:
  - useIsMobile SSR-safe viewport detection hook
  - Mobile tab-based IDE layout (Editor/Results/Console tabs) with Monaco kept alive
  - Responsive toolbar with adaptive sizing for mobile/desktop
  - Dead code removal (VibeScoreGauge.tsx, CodeDiffView.tsx)
  - Production next.config.js with unoptimized images for Vercel
  - Layout metadata updated to "Monad Vibe-Loom IDE"
  - Dark mode visual polish (panel transitions, focus rings, font smoothing, gradient headers)
  - Global CSS utilities (panel-transition, tabular-nums, focus-visible amber outline)
requires:
  - slice: S03
    provides: Complete IDE component set (MonacoEditor, EditorPanel, SidebarPanel, ConsolePanel, TransactionConsole, ContractInteraction, VibeScoreDashboard, AIDiffViewer), page.tsx full wiring
affects: []
key_files:
  - frontend/src/hooks/useIsMobile.ts
  - frontend/src/components/ide/IDELayout.tsx
  - frontend/src/app/page.tsx
  - frontend/src/app/layout.tsx
  - frontend/next.config.js
  - frontend/src/app/globals.css
  - frontend/src/components/ide/EditorPanel.tsx
  - frontend/src/components/ide/SidebarPanel.tsx
  - frontend/src/components/ide/ConsolePanel.tsx
  - frontend/src/components/ide/TransactionConsole.tsx
key_decisions:
  - Used absolute-positioned hidden/block panels instead of conditional rendering to keep Monaco editor DOM alive across mobile tab switches — avoids full remount of ~630KB Monaco payload
  - Used custom CSS class panel-transition with specific properties (bg, border, shadow) and cubic-bezier easing instead of Tailwind transition-all, per make-interfaces-feel-better guidance
  - Applied amber focus-visible outline globally for accessibility, consistent with IDE amber accent theme
patterns_established:
  - SSR-safe media query hook pattern (useState(false) + useEffect + matchMedia listener + cleanup)
  - Mobile tab UI with absolute-positioned content panels preserving heavy component DOM
  - Global panel-transition utility class for consistent IDE panel color transitions
  - Alternating row opacity pattern (100%/90%) for list readability in dark theme
observability_surfaces:
  - React DevTools: isMobile state on IDELayout, activeTab state for mobile tabs
  - Browser console: window.matchMedia('(max-width: 767px)').matches for breakpoint verification
  - Browser DevTools responsive mode (375px vs 1280px) as primary inspection tool
  - Build success: npm run build exit code 0 confirms no import breakage
drill_down_paths:
  - .gsd/milestones/M002/slices/S04/tasks/T01-SUMMARY.md
  - .gsd/milestones/M002/slices/S04/tasks/T02-SUMMARY.md
duration: 20m
verification_result: passed
completed_at: 2026-03-23
---

# S04: 반응형 + 폴리싱 + Vercel 배포

**Added mobile-responsive tab-based IDE layout, dark mode visual polish across all panels, removed dead code, and configured production build for Vercel deployment readiness**

## What Happened

This slice finalized the M002 frontend milestone with two parallel tracks: responsive layout (T01) and polish/cleanup/production config (T02).

**T01 — Responsive Layout:** Created a `useIsMobile` hook using native `window.matchMedia('(max-width: 767px)')` with SSR-safe default (`false`) and proper event listener cleanup. Refactored `IDELayout` to conditionally render based on viewport width: on mobile (<768px), it displays a 3-tab bar (Editor / Results / Console) with amber active-tab styling. The key architectural choice was using absolute-positioned panels with hidden/block toggling rather than conditional rendering — this keeps the Monaco editor DOM alive across tab switches, avoiding the costly ~630KB remount penalty. On desktop (≥768px), the existing `react-resizable-panels` 3-panel layout renders unchanged. The toolbar in `page.tsx` was updated with Tailwind responsive prefixes (`sm:`) for padding, text size, and button scaling — contract selector buttons shrink to `text-[10px]` on mobile, the GitHub label hides on small screens, and usernames truncate.

**T02 — Polish + Cleanup + Production Config:** Deleted two dead code files (`VibeScoreGauge.tsx` replaced by VibeScoreDashboard in S03, `CodeDiffView.tsx` replaced by AIDiffViewer in S02). Updated `layout.tsx` metadata title to "Monad Vibe-Loom IDE" with production-appropriate description. Added `images: { unoptimized: true }` to `next.config.js` for static Vercel deployment. Applied visual refinements across four IDE panel components: `globals.css` gained font-smoothing, amber focus-visible outlines, tabular-nums utility, and a `panel-transition` class with `cubic-bezier(0.2, 0, 0, 1)` easing on specific properties (background-color, border-color, box-shadow). `SidebarPanel` got a gradient header with amber inset shadow. `ConsolePanel` got uppercase tracking on labels. `TransactionConsole` got softened borders, alternating row opacity, and tabular-nums on timestamps.

## Verification

| # | Check | Result |
|---|-------|--------|
| 1 | `npm run build` exits 0 | ✅ pass (48s) |
| 2 | `npm test` — 57 tests pass, 0 failures | ✅ pass (5s) |
| 3 | `useIsMobile` hook file exists | ✅ pass |
| 4 | `isMobile` logic present in IDELayout | ✅ pass |
| 5 | `matchMedia` used in hook | ✅ pass |
| 6 | `VibeScoreGauge.tsx` deleted | ✅ pass |
| 7 | `CodeDiffView.tsx` deleted | ✅ pass |
| 8 | `"Monad Vibe-Loom"` in layout.tsx | ✅ pass |
| 9 | `"unoptimized"` in next.config.js | ✅ pass |

Build output: 229 kB first load JS for `/` route (unchanged from S03 — no bundle regression from responsive additions).

## New Requirements Surfaced

- none

## Deviations

None — both tasks followed their plans exactly.

## Known Limitations

- **Vercel deployment not executed:** Production build is ready (`npm run build` passes, next.config.js configured) but actual Vercel deployment with env vars and domain DNS is a manual platform step outside slice scope.
- **Tablet breakpoint:** Only two breakpoints exist (mobile <768px, desktop ≥768px). There is no dedicated tablet layout (e.g., 2-panel) — tablets get the desktop 3-panel view, which works but may feel cramped on smaller tablets.
- **Monaco height in mobile tab:** The absolute-positioned panel approach requires the parent container to have explicit dimensions. If CSS containment changes, Monaco may render with zero height in the mobile Editor tab.

## Follow-ups

- **Vercel deployment:** Deploy to vibe-loom.xyz with `NEXT_PUBLIC_API_URL` pointing to the Railway backend. Backend must deploy first (NEXT_PUBLIC_ vars are build-time inlined).
- **Tablet 2-panel layout:** If user feedback indicates tablet UX issues, add an intermediate breakpoint (768px–1024px) with a 2-panel view.

## Files Created/Modified

- `frontend/src/hooks/useIsMobile.ts` — new: SSR-safe viewport detection hook using matchMedia
- `frontend/src/components/ide/IDELayout.tsx` — refactored: conditional mobile tab UI / desktop 3-panel layout
- `frontend/src/app/page.tsx` — modified: toolbar responsive sizing with sm: breakpoints
- `frontend/src/components/VibeScoreGauge.tsx` — deleted: dead code replaced by VibeScoreDashboard
- `frontend/src/components/CodeDiffView.tsx` — deleted: dead code replaced by AIDiffViewer
- `frontend/src/app/layout.tsx` — modified: title → "Monad Vibe-Loom IDE", description updated
- `frontend/next.config.js` — modified: added images.unoptimized for Vercel static export
- `frontend/src/app/globals.css` — modified: font smoothing, focus-visible outline, tabular-nums, panel-transition utility
- `frontend/src/components/ide/EditorPanel.tsx` — modified: added panel-transition class
- `frontend/src/components/ide/SidebarPanel.tsx` — modified: gradient header, amber inset shadow, letter-spacing, panel-transition
- `frontend/src/components/ide/ConsolePanel.tsx` — modified: header shadow, uppercase tracking, panel-transition
- `frontend/src/components/ide/TransactionConsole.tsx` — modified: softer borders, alternating opacity, shadow-sm, tabular-nums, panel-transition

## Forward Intelligence

### What the next slice should know
- The full IDE component set is complete and production-build-verified. All S01–S04 components coexist without import conflicts or bundle regressions (229 kB first load maintained).
- `NEXT_PUBLIC_API_URL` must be set at Vercel build time — it's inlined by webpack. Deploy backend first, then trigger frontend build with the backend URL.
- `--legacy-peer-deps` is required for all `npm install` in this project due to @monaco-editor/react's React 19 peer dep gap.

### What's fragile
- **Monaco in mobile tab layout** — the absolute-positioned panel depends on `relative h-full` container chain. Any CSS change to IDELayout or its parents that breaks this chain will cause Monaco to render at 0 height with no error signal.
- **ContractInteraction conditional rendering** — depends on `compileResult?.abi && deployResult?.address` shape. State shape changes silently hide the UI with no error.

### Authoritative diagnostics
- `npm run build` exit 0 — confirms all imports resolve, no TypeScript errors, no dead references after file deletions.
- `npm test` 57 pass — covers api-client, abi-utils, useTransactionLog, VibeScoreDashboard, ContractInteraction components.
- Browser DevTools responsive mode at 375px/1280px — primary tool for visual verification of mobile/desktop layout switching.

### What assumptions changed
- No assumptions changed — S04 was a low-risk polish slice and executed cleanly per plan.
