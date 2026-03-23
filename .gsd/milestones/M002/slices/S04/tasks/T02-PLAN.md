---
estimated_steps: 5
estimated_files: 8
---

# T02: Dark mode polish, dead code cleanup, and production build config

**Slice:** S04 — 반응형 + 폴리싱 + Vercel 배포
**Milestone:** M002

## Description

This task handles three independent cleanup/polish tracks that complete the milestone:

1. **Dead code removal** — delete `VibeScoreGauge.tsx` (replaced by VibeScoreDashboard in S03) and `CodeDiffView.tsx` (replaced by AIDiffViewer in S02). Both files are confirmed unreferenced from any import.

2. **Dark mode visual polish** — enhance existing components with focus rings for accessibility, subtle shadow refinements, smoother transitions, and minor visual improvements. The app is already fully dark-themed; this is incremental polish, not a redesign.

3. **Production build config** — update `next.config.js` for Vercel deployment, update `layout.tsx` metadata to reflect the IDE branding ("Monad Vibe-Loom IDE"), and verify the build succeeds.

**Key constraints:**
- No new npm dependencies
- No structural/behavioral changes to components — CSS/Tailwind class additions only for polish
- The `VibeScoreGauge` component is ONLY referenced in its own file and in a comment in VibeScoreDashboard — safe to delete
- The `CodeDiffView` component is ONLY referenced in its own file — safe to delete
- Tailwind CSS v4 with `@tailwindcss/postcss` — no `tailwind.config.js`, CSS-based config

**Relevant skills:** `frontend-design`, `make-interfaces-feel-better`

## Steps

1. **Delete dead code files:**
   - `rm frontend/src/components/VibeScoreGauge.tsx`
   - `rm frontend/src/components/CodeDiffView.tsx`
   - Verify no imports break: `npm run build`

2. **Update production metadata in `layout.tsx`:**
   - Change `title` from "Vibe-Check AI" to "Monad Vibe-Loom IDE"
   - Update `description` to match the IDE product identity
   - Keep `lang="ko"` and Providers wrapper

3. **Update `next.config.js` for Vercel:**
   - Add `images: { unoptimized: true }` (no external image optimization needed)
   - Keep module as CommonJS (`module.exports`)

4. **Apply dark mode polish to panel components:**
   - `EditorPanel.tsx`: add `transition-colors` to container
   - `SidebarPanel.tsx`: add subtle header styling (border glow or gradient accent)
   - `ConsolePanel.tsx`: add header styling consistency
   - `TransactionConsole.tsx`: add alternating row opacity for readability, softer shadows on status cards
   - Add CSS custom properties or utility classes in `globals.css` if needed (e.g., smooth focus transitions)

5. **Final verification:**
   - `npm run build` exits 0
   - `npm test` passes (57 tests, 0 failures)
   - Confirm dead code files are gone
   - Confirm metadata updated

## Must-Haves

- [ ] `VibeScoreGauge.tsx` deleted
- [ ] `CodeDiffView.tsx` deleted
- [ ] `layout.tsx` title updated to include "Monad Vibe-Loom"
- [ ] `next.config.js` has `images: { unoptimized: true }`
- [ ] At least 3 components have visual polish improvements (focus rings, transitions, shadows)
- [ ] `npm run build` exits 0
- [ ] `npm test` passes (57 tests, 0 failures)

## Verification

- `cd frontend && npm run build` exits 0
- `cd frontend && npm test` — 57 tests pass
- `! test -f frontend/src/components/VibeScoreGauge.tsx` — dead code removed
- `! test -f frontend/src/components/CodeDiffView.tsx` — dead code removed
- `grep -q "Monad Vibe-Loom" frontend/src/app/layout.tsx` — metadata updated
- `grep -q "unoptimized" frontend/next.config.js` — image config present

## Inputs

- `frontend/src/components/VibeScoreGauge.tsx` — dead code to delete
- `frontend/src/components/CodeDiffView.tsx` — dead code to delete
- `frontend/src/app/layout.tsx` — metadata to update (currently "Vibe-Check AI")
- `frontend/next.config.js` — empty config to enhance
- `frontend/src/app/globals.css` — minimal CSS to extend
- `frontend/src/components/ide/EditorPanel.tsx` — polish target
- `frontend/src/components/ide/SidebarPanel.tsx` — polish target
- `frontend/src/components/ide/ConsolePanel.tsx` — polish target
- `frontend/src/components/ide/TransactionConsole.tsx` — polish target

## Expected Output

- `frontend/src/components/VibeScoreGauge.tsx` — deleted
- `frontend/src/components/CodeDiffView.tsx` — deleted
- `frontend/src/app/layout.tsx` — modified: updated metadata
- `frontend/next.config.js` — modified: production config
- `frontend/src/app/globals.css` — modified: transition utilities
- `frontend/src/components/ide/EditorPanel.tsx` — modified: visual polish
- `frontend/src/components/ide/SidebarPanel.tsx` — modified: visual polish
- `frontend/src/components/ide/ConsolePanel.tsx` — modified: visual polish
- `frontend/src/components/ide/TransactionConsole.tsx` — modified: visual polish

## Observability Impact

- **Build signal:** `npm run build` exit code 0 confirms dead code deletion caused no import breakage and all visual polish compiles cleanly.
- **Test signal:** `npm test` 57 passes confirms no behavioral regressions from CSS-only changes.
- **Metadata inspection:** `grep "Monad Vibe-Loom" frontend/src/app/layout.tsx` verifies branding is updated.
- **Visual inspection:** Browser DevTools computed styles on any IDE panel should show `transition-property: background-color, border-color, box-shadow`. Focus any interactive element to see amber `focus-visible` ring. Inspect `<body>` for `-webkit-font-smoothing: antialiased`.
- **Dead code absence:** `! test -f frontend/src/components/VibeScoreGauge.tsx && ! test -f frontend/src/components/CodeDiffView.tsx` — both must return exit 0.
