# S04: 반응형 + 폴리싱 + Vercel 배포 — UAT

**Milestone:** M002
**Written:** 2026-03-23

## UAT Type

- UAT mode: mixed (artifact-driven build verification + human-experience visual checks)
- Why this mode is sufficient: Build/test verification proves code correctness and import integrity. Visual verification of responsive layout switching and dark mode polish requires human eyes in a real browser — these are inherently subjective and spatial.

## Preconditions

- `cd frontend && npm install --legacy-peer-deps` completed
- `cd frontend && npm run build` exits 0
- `cd frontend && npm run dev` running on localhost:3000
- Browser with DevTools available (Chrome/Edge recommended for responsive mode)

## Smoke Test

Open `http://localhost:3000` in a desktop browser (≥768px width). Verify the 3-panel IDE layout renders with resizable panels (editor | sidebar | console). Toggle DevTools responsive mode to 375px width — verify the layout switches to a tab-based UI with Editor/Results/Console tabs.

## Test Cases

### 1. Desktop 3-Panel Layout Preserved

1. Open `http://localhost:3000` in a browser window ≥1280px wide
2. Verify the editor panel is on the left with Monaco editor loaded
3. Verify the sidebar panel is in the middle (contract interaction, vibe-score sections)
4. Verify the console panel is on the right (transaction console)
5. Drag a panel separator handle
6. **Expected:** Panels resize smoothly. All three panels remain visible. Monaco editor content does not disappear during resize.

### 2. Mobile Tab UI Activation

1. Open DevTools → Toggle Device Toolbar → set viewport to 375×812 (iPhone SE/X size)
2. **Expected:** The 3-panel layout is replaced by a tab bar with three tabs: "Editor", "Results", "Console"
3. The "Editor" tab is active by default showing the Monaco editor
4. Tap "Results" tab
5. **Expected:** The sidebar content (contract interaction, vibe-score) is visible. The tab switches with amber highlight on the active tab.
6. Tap "Console" tab
7. **Expected:** The transaction console is visible.
8. Tap "Editor" tab again
9. **Expected:** Monaco editor is visible immediately — no loading spinner, no content loss. The editor preserved its content because the DOM was kept alive via absolute positioning.

### 3. Toolbar Responsive Sizing

1. At desktop width (≥768px), inspect the toolbar
2. **Expected:** Contract selector buttons show full text, GitHub label visible, normal padding
3. Switch to mobile width (<768px)
4. **Expected:** Contract selector buttons shrink to smaller text, GitHub label hidden (only icon visible), username truncated, reduced padding/gaps between elements

### 4. Breakpoint Boundary Behavior

1. Set viewport width to exactly 768px
2. **Expected:** Desktop 3-panel layout renders (768px is ≥768px threshold)
3. Set viewport width to 767px
4. **Expected:** Mobile tab layout renders (767px is <768px threshold)
5. Slowly resize between 767px and 768px
6. **Expected:** Layout switches cleanly between tab/panel modes. No flicker, no intermediate broken state.

### 5. Dark Mode Visual Polish

1. At desktop width, inspect any IDE panel (editor, sidebar, console)
2. **Expected:** Panels have smooth transitions when theme state changes — check computed styles for `transition-property: background-color, border-color, box-shadow`
3. Focus any interactive element (button, input) using Tab key
4. **Expected:** Amber focus-visible ring appears around the focused element
5. Inspect the SidebarPanel header
6. **Expected:** Subtle gradient background (from-gray-800 to-gray-800/90) and letter-spacing visible
7. Inspect the ConsolePanel header label
8. **Expected:** "Console" label is uppercase with letter-spacing
9. Check TransactionConsole entries (deploy or call some contracts first)
10. **Expected:** Alternating row opacity visible (100%/90%), timestamps use tabular-nums (monospaced digits)

### 6. Metadata and Production Config

1. View page source or inspect `<head>` in DevTools
2. **Expected:** `<title>` contains "Monad Vibe-Loom IDE"
3. Check `frontend/next.config.js` content
4. **Expected:** `images: { unoptimized: true }` present

### 7. Dead Code Removal Verification

1. In the project directory, check for old files:
   - `ls frontend/src/components/VibeScoreGauge.tsx`
   - `ls frontend/src/components/CodeDiffView.tsx`
2. **Expected:** Both files do not exist (command returns "No such file or directory")
3. Run `cd frontend && npm run build`
4. **Expected:** Build succeeds with 0 errors — no broken imports referencing deleted files

### 8. Full Build and Test Suite

1. Run `cd frontend && npm run build`
2. **Expected:** Exits 0. Route output shows `/` at ~229 kB first load JS
3. Run `cd frontend && npm test -- --watchAll=false`
4. **Expected:** 57 tests pass, 0 failures, 5 test suites

## Edge Cases

### Monaco Height in Mobile Tab

1. At mobile viewport (375px), switch to Editor tab
2. **Expected:** Monaco editor fills the available height (not collapsed to 0px)
3. Type some Solidity code
4. **Expected:** Code is visible and editor is scrollable

### Rapid Tab Switching on Mobile

1. At mobile viewport, rapidly tap between Editor → Results → Console → Editor
2. **Expected:** Each panel renders immediately. No blank panels, no loading states, no content loss. Monaco editor retains its code.

### Window Resize During Interaction

1. Start at desktop width with code in the editor
2. Resize browser to mobile width (<768px)
3. **Expected:** Tab UI appears with Editor tab active showing the same code
4. Resize back to desktop width (≥768px)
5. **Expected:** 3-panel layout restores with the same code in the editor

## Failure Signals

- **Layout broken at mobile:** If the 3-panel `react-resizable-panels` layout renders at <768px, the `useIsMobile` hook isn't working — check that `matchMedia` is being called and the breakpoint is correct (767px, not 768px)
- **Monaco invisible in mobile tab:** If editor tab shows blank, check CSS chain: the editor's parent must have `absolute inset-0` and the tab content container must be `relative h-full`
- **Build failure after dead code deletion:** If `npm run build` fails with "Cannot find module" referencing VibeScoreGauge or CodeDiffView, there's a lingering import somewhere
- **Focus ring not visible:** If amber focus-visible ring doesn't appear on Tab navigation, check that `globals.css` has the `*:focus-visible` rule and the browser supports `:focus-visible` pseudo-class
- **Hydration mismatch warning:** If React console shows hydration warnings about `isMobile`, the SSR default (false) is producing desktop HTML but the client is detecting mobile — this is expected for SSR and should resolve after hydration

## Not Proven By This UAT

- **Actual Vercel deployment:** Production build readiness is proven but live deployment to vibe-loom.xyz requires manual Vercel platform configuration (env vars, domain DNS)
- **Real API integration on mobile:** This UAT tests layout responsiveness, not whether API calls work on mobile — that's covered by the end-to-end flow verified in S01–S03
- **Tablet-specific layout:** No dedicated tablet breakpoint exists — tablets use desktop layout, which may be suboptimal on 768–1024px screens

## Notes for Tester

- **Legacy peer deps:** If you need to `npm install` anything, always use `--legacy-peer-deps` due to @monaco-editor/react React 19 compatibility
- **Build warnings are expected:** The build emits wagmi connector warnings — these are pre-existing and unrelated to S04 changes
- **Console errors during test:** The `api-client.test.ts` produces console.error output during test runs — this is intentional (testing error paths) and the tests pass
- **Monaco CDN dependency:** The Monaco editor loads from CDN at runtime. If offline or CDN is blocked, the editor won't render — this is by design and not a bug
