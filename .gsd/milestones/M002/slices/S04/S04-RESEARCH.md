# S04: 반응형 + 폴리싱 + Vercel 배포 — Research

**Date:** 2026-03-23
**Depth:** Light — well-understood work on established codebase

## Summary

S04 is the final polish slice. All functional components are complete (S01–S03). The work is three independent tracks: (1) responsive layout for mobile/tablet, (2) dark mode visual polish, and (3) Vercel deployment to vibe-loom.xyz.

The IDE currently uses `react-resizable-panels` v4.7.5 with a fixed horizontal 3-panel layout (`IDELayout.tsx`). On mobile, resizable panels are unusable — drag handles require desktop precision. The standard pattern is to detect viewport width and switch to a tab-based layout on mobile (editor/sidebar/console as tabs) while keeping the desktop panel layout. Tailwind CSS v4 is already configured with `@tailwindcss/postcss`.

The app is already fully dark-themed (gray-800/900 backgrounds, gray text colors). Polish means adding visual refinements — subtle gradients, improved shadows, better transitions, and minor UX touches. No light mode is needed.

Vercel deployment requires `next.config.js` configuration, `NEXT_PUBLIC_*` env vars set at build time, and domain setup (vibe-loom.xyz).

## Recommendation

1. **Responsive layout**: Create a `useIsMobile` hook (CSS media query via `matchMedia`). In `IDELayout.tsx`, conditionally render: mobile → tab switcher (Editor/Results/Console tabs), tablet → 2-panel vertical stack, desktop → current 3-panel resizable. This avoids a library — a simple `useState` + `useEffect` + `matchMedia` is sufficient.

2. **Dark mode polish**: Enhance existing components with gradient backgrounds, ring/glow on focus states, subtle box shadows, and smooth transitions. Touch the toolbar, panel headers, buttons, and separators. No structural changes.

3. **Vercel deployment**: Configure `next.config.js` for production, clean up dead code files, verify build succeeds, then deploy with proper env vars.

4. **Dead code cleanup**: Delete `VibeScoreGauge.tsx` and `CodeDiffView.tsx` (both unreferenced from page.tsx, confirmed by S03 summary).

## Implementation Landscape

### Key Files

- `frontend/src/components/ide/IDELayout.tsx` — **primary change**: responsive layout switching (currently 33 lines, desktop-only 3-panel)
- `frontend/src/app/page.tsx` — toolbar needs responsive adjustments (flex-wrap already present, needs mobile collapse)
- `frontend/src/app/globals.css` — add custom CSS for transitions/animations if needed (currently 6 lines)
- `frontend/src/app/layout.tsx` — metadata update (title: "Monad Vibe-Loom IDE")
- `frontend/next.config.js` — Vercel deployment config (currently empty)
- `frontend/src/components/VibeScoreGauge.tsx` — **delete** (dead code since S03)
- `frontend/src/components/CodeDiffView.tsx` — **delete** (dead code since S02, replaced by AIDiffViewer)

### Responsive Design Specifics

Current `IDELayout.tsx` structure:
```
Group(horizontal) → Panel(75%) [Group(vertical) → Panel(editor 70%) | Panel(console 30%)] | Panel(sidebar 25%)
```

Target breakpoints:
- **Desktop (≥1024px)**: Current 3-panel resizable layout — no change
- **Tablet (≥768px, <1024px)**: 2-panel — editor+console stacked vertically (full width), sidebar below or as overlay. Or keep 3-panel with adjusted ratios.
- **Mobile (<768px)**: Tab-based — 3 tabs (Editor / Results / Console), only one visible at a time. No react-resizable-panels at all on mobile.

Implementation pattern:
```tsx
// useIsMobile hook — simple matchMedia listener
const [isMobile, setIsMobile] = useState(false);
useEffect(() => {
  const mq = window.matchMedia('(max-width: 767px)');
  setIsMobile(mq.matches);
  const handler = (e: MediaQueryListEvent) => setIsMobile(e.matches);
  mq.addEventListener('change', handler);
  return () => mq.removeEventListener('change', handler);
}, []);
```

Mobile tab UI renders the same child components (`editor`, `sidebar`, `console` props) but in a `div` with tab switching instead of `<Group>/<Panel>/<Separator>`.

### Toolbar Responsive

The `editorToolbar` in page.tsx already uses `flex-wrap`. For mobile:
- Hide contract selector buttons (show as dropdown or abbreviate)
- Stack action buttons below title
- Shrink auth section

### Dark Mode Polish Targets

Already dark. Polish opportunities:
- Toolbar: subtle bottom gradient border or glow effect
- Panel separators: slightly more visible hover state (already has `hover:bg-amber-500`)
- Buttons: add `ring` on focus for accessibility
- Deploy success/error cards: softer shadow (`shadow-lg shadow-emerald-500/10`)
- TransactionConsole: alternating row opacity for readability
- VibeScoreDashboard gauge: add subtle glow/shadow behind SVG circle
- Global: smooth `transition-colors` on interactive elements (many already have this)

### Vercel Deployment

`next.config.js` needs:
```js
const nextConfig = {
  output: 'standalone',  // optional for Vercel but good practice
  images: { unoptimized: true },  // no external image optimization needed
};
```

Environment variables for Vercel (build-time):
- `NEXT_PUBLIC_API_URL` — `https://vibe-room-backend-production.up.railway.app`
- `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID` — WalletConnect project ID
- `NEXT_PUBLIC_MONAD_RPC_URL` — `https://testnet-rpc.monad.xyz` (or omit, defaults in code)

### Build Order

1. **T01: Responsive layout** — `IDELayout.tsx` refactor + `useIsMobile` hook + toolbar responsive. This is the only task with meaningful code changes. Verify with `npm run build` + manual resize check.
2. **T02: Dark mode polish + dead code cleanup** — CSS/Tailwind class enhancements across components + delete dead files. Independent of T01. Verify with `npm run build`.
3. **T03: Vercel deployment** — `next.config.js` update, env var setup, deploy command. Depends on T01+T02 being complete. Verify with `npm run build` + deployment URL check.

### Verification Approach

- `npm run build` passes with 0 errors after each task
- `npm test` — 57 tests still pass (no functional changes)
- Build output size stays ≤ 230 kB first load (no new dependencies)
- Dead code files removed (VibeScoreGauge.tsx, CodeDiffView.tsx)
- Mobile layout: browser dev tools responsive mode at 375px shows tab UI
- Desktop layout: unchanged 3-panel behavior at 1280px

## Constraints

- Tailwind CSS v4 with `@tailwindcss/postcss` — no `tailwind.config.js` file, config is CSS-based (`@import "tailwindcss"`)
- `NEXT_PUBLIC_*` env vars are inlined at build time — must be set before `next build` on Vercel
- No new npm dependencies allowed — responsive detection uses native `matchMedia` API
- `react-resizable-panels` v4 uses `orientation` (not `direction`) prop — KNOWLEDGE entry
- `@monaco-editor/react` requires `--legacy-peer-deps` with React 19 — KNOWLEDGE entry

## Common Pitfalls

- **SSR and matchMedia** — `window.matchMedia` is not available during SSR. The `useIsMobile` hook must default to `false` (desktop) and update on mount. Since the page is `"use client"`, this is safe but initial render will flash desktop layout.
- **Monaco Editor height in tab layout** — Monaco requires explicit height (`height="100%"`) and a parent with defined height. Tab-based mobile layout must ensure the editor container fills the viewport minus tab bar height.
- **react-resizable-panels import on mobile** — Even if not rendered, the `Group/Panel/Separator` components are imported. This is fine (they're already in the bundle from S01). Don't try to lazy-load them.
