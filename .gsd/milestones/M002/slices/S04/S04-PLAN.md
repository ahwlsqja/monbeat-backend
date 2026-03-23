# S04: 반응형 + 폴리싱 + Vercel 배포

**Goal:** 모바일/태블릿에서 IDE 레이아웃이 반응형으로 동작하고, 다크모드 비주얼이 폴리싱되며, 프로덕션 빌드가 성공하여 Vercel 배포 준비가 완료된 상태
**Demo:** 브라우저 DevTools에서 375px 모바일 뷰포트로 전환 시 탭 기반 UI(Editor/Results/Console)로 전환되고, 데스크톱(1280px)에서는 기존 3패널 리사이즈 레이아웃 유지. `npm run build` 성공, dead code 제거 완료.

## Must-Haves

- `useIsMobile` 훅으로 모바일(<768px) 감지, SSR-safe (기본값 false)
- 모바일: 탭 전환 UI (Editor / Results / Console 탭), `react-resizable-panels` 미사용
- 데스크톱: 기존 3패널 리사이즈 레이아웃 유지 (변경 없음)
- 툴바가 모바일에서 오버플로 없이 축소/랩핑
- Dead code 파일 삭제 (`VibeScoreGauge.tsx`, `CodeDiffView.tsx`)
- 다크모드 비주얼 폴리싱 (포커스 링, 그라디언트, 트랜지션)
- `next.config.js` 프로덕션 설정 + layout.tsx 메타데이터 업데이트
- `npm run build` 0 에러, `npm test` 57 테스트 전체 통과

## Proof Level

- This slice proves: operational
- Real runtime required: no (build + test verification)
- Human/UAT required: yes (반응형 레이아웃 시각 확인은 브라우저에서)

## Verification

- `cd frontend && npm run build` exits 0
- `cd frontend && npm test` — 57+ tests pass, 0 failures
- `grep -q "useIsMobile" frontend/src/hooks/useIsMobile.ts` — hook exists
- `grep -q "isMobile" frontend/src/components/ide/IDELayout.tsx` — responsive logic present
- `! test -f frontend/src/components/VibeScoreGauge.tsx` — dead code removed
- `! test -f frontend/src/components/CodeDiffView.tsx` — dead code removed
- `grep -q "Monad Vibe-Loom" frontend/src/app/layout.tsx` — metadata updated

## Observability / Diagnostics

- **Runtime signals:** `useIsMobile` hook logs nothing by default — viewport state is visible via React DevTools (`isMobile` state). Tab switch state in IDELayout is observable via React DevTools component state.
- **Inspection surfaces:** Browser DevTools responsive mode (375px vs 1280px) is the primary inspection tool. `window.matchMedia('(max-width: 767px)').matches` can be evaluated in console to verify breakpoint detection.
- **Failure visibility:** Build failures surface via `npm run build` exit code and stderr. Test failures surface via `npm test` output. Hydration mismatches (if SSR default is wrong) appear as React warnings in browser console.
- **Redaction constraints:** No secrets or sensitive data involved in this slice.

## Integration Closure

- Upstream surfaces consumed: All S01–S03 components (MonacoEditor, EditorPanel, SidebarPanel, ConsolePanel, TransactionConsole, ContractInteraction, VibeScoreDashboard, AIDiffViewer), page.tsx full wiring
- New wiring introduced in this slice: `useIsMobile` hook consumed by IDELayout for conditional rendering, `next.config.js` production config
- What remains before the milestone is truly usable end-to-end: Vercel deployment with env vars (manual platform step), domain DNS configuration

## Tasks

- [x] **T01: Add responsive layout with mobile tab switching and toolbar collapse** `est:45m`
  - Why: IDELayout currently renders fixed 3-panel resizable layout that's unusable on mobile. This task creates a `useIsMobile` hook and refactors IDELayout to show tab-based UI on mobile (<768px) while preserving the desktop 3-panel layout.
  - Files: `frontend/src/hooks/useIsMobile.ts`, `frontend/src/components/ide/IDELayout.tsx`, `frontend/src/app/page.tsx`
  - Do: (1) Create `useIsMobile` hook using `window.matchMedia('(max-width: 767px)')` with SSR-safe default `false`. (2) Refactor `IDELayout` to accept `useIsMobile` result and conditionally render: mobile → tab switcher with 3 tabs (Editor/Results/Console), only active tab's content visible; desktop → current Group/Panel/Separator layout unchanged. (3) Adjust toolbar in page.tsx to wrap gracefully on mobile — contract selector buttons use smaller text, action buttons stack. (4) Ensure Monaco Editor container in tab layout has explicit height (`h-full` + parent flex). Skills: `react-best-practices`, `frontend-design`.
  - Verify: `cd frontend && npm run build && npm test`
  - Done when: `npm run build` passes, IDELayout.tsx contains `isMobile` conditional rendering, useIsMobile hook file exists

- [x] **T02: Dark mode polish, dead code cleanup, and production build config** `est:30m`
  - Why: Removes dead code from prior slices, adds visual refinements to the dark theme, and configures next.config.js + metadata for production/Vercel deployment readiness.
  - Files: `frontend/src/components/VibeScoreGauge.tsx`, `frontend/src/components/CodeDiffView.tsx`, `frontend/src/app/layout.tsx`, `frontend/next.config.js`, `frontend/src/app/globals.css`, `frontend/src/components/ide/EditorPanel.tsx`, `frontend/src/components/ide/SidebarPanel.tsx`, `frontend/src/components/ide/ConsolePanel.tsx`, `frontend/src/components/ide/TransactionConsole.tsx`
  - Do: (1) Delete `VibeScoreGauge.tsx` and `CodeDiffView.tsx`. (2) Update `layout.tsx` title to "Monad Vibe-Loom IDE" and description. (3) Update `next.config.js` with `images: { unoptimized: true }`. (4) Add focus ring utilities and transition improvements to interactive elements across EditorPanel, SidebarPanel, ConsolePanel, TransactionConsole. (5) Add subtle CSS transitions/animations in globals.css if needed. Skills: `frontend-design`, `make-interfaces-feel-better`.
  - Verify: `cd frontend && npm run build && npm test && ! test -f src/components/VibeScoreGauge.tsx && ! test -f src/components/CodeDiffView.tsx`
  - Done when: Dead code files deleted, build passes, metadata updated, visual polish applied to panel components

## Files Likely Touched

- `frontend/src/hooks/useIsMobile.ts` (new)
- `frontend/src/components/ide/IDELayout.tsx` (major refactor)
- `frontend/src/app/page.tsx` (toolbar responsive adjustments)
- `frontend/src/components/VibeScoreGauge.tsx` (delete)
- `frontend/src/components/CodeDiffView.tsx` (delete)
- `frontend/src/app/layout.tsx` (metadata update)
- `frontend/next.config.js` (production config)
- `frontend/src/app/globals.css` (transition utilities)
- `frontend/src/components/ide/EditorPanel.tsx` (polish)
- `frontend/src/components/ide/SidebarPanel.tsx` (polish)
- `frontend/src/components/ide/ConsolePanel.tsx` (polish)
- `frontend/src/components/ide/TransactionConsole.tsx` (polish)
