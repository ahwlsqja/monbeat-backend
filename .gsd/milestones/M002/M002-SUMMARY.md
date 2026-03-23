---
id: M002
provides:
  - Monaco Editor-based Monad smart contract IDE with Solidity syntax highlighting + 59-item completion provider
  - IDE 3-panel resizable layout (Editor | Sidebar | Console) with collapsible panels via react-resizable-panels v4
  - Compile button with Monaco inline error markers (solc error parser → setModelMarkers)
  - TransactionConsole with color-coded event log (compile/deploy/call history)
  - Monaco DiffEditor for AI fix suggestions (AIDiffViewer, SSR-safe two-file pattern)
  - ABI-driven contract interaction UI (read via viem, write via wagmi) in sidebar
  - Vibe-Score dashboard with SVG circular gauge + 3-stat grid (conflicts, reExecutions, gasEfficiency) + suggestion cards
  - Mobile-responsive tab-based layout (Editor/Results/Console tabs) with Monaco DOM kept alive
  - Dark mode visual polish (panel transitions, focus rings, font smoothing, gradient headers)
  - Production-ready Next.js build (229 kB first load, unoptimized images for Vercel)
  - All 7 existing API methods preserved without modification
key_decisions:
  - D019: @monaco-editor/react + react-resizable-panels v4 (CDN loading, SSR-safe, 8KB layout lib)
  - D020: SSR-safe Monaco DiffEditor via two-file pattern (Inner + dynamic wrapper)
  - D021: Transaction log newest-first ordering (prepend) for console UX
  - D022: solc error parse failure → fallback marker at line 1 (never silent drop)
  - D023: Text input for uint/int ABI fields to support BigInt-scale values
  - D024: SVG circle gauge for Vibe-Score (no chart library, 229 kB maintained)
  - D025: Full VibeScoreResult object stored in state (no double-mapping)
  - D026: Absolute-positioned hidden/block panels for mobile tabs (Monaco DOM alive)
  - D027: Custom panel-transition CSS class (specific properties + cubic-bezier)
patterns_established:
  - Two-file SSR-safe pattern for browser-only components (Wrapper + Inner with next/dynamic ssr:false)
  - Language providers registered in Monaco beforeMount callback (runs once before editor init)
  - Panel components as pure composition wrappers (accept children/toolbar via props)
  - Monaco marker integration via monacoInstance.editor.setModelMarkers(model, owner, markers)
  - Transaction logging at three points in async flows (pending → success/error in try/catch)
  - Defensive ABI parsing with try/catch fallback to empty arrays
  - onCallResult callback emitting TransactionLogEntry-compatible objects
  - SSR-safe viewport detection hook (useState(false) + useEffect + matchMedia)
observability_surfaces:
  - monacoInstance.editor.getModelMarkers({owner:'solc'}) — active compile error markers
  - React DevTools → useTransactionLog entries — full compile/deploy/call event history
  - TransactionConsole — color-coded visual indicators (emerald/red/amber)
  - SVG gauge strokeDashoffset — directly reflects rendered Vibe-Score value
  - Browser DevTools responsive mode (375px vs 1280px) — mobile/desktop layout verification
  - npm run build exit 0 + 229 kB first load — production build health
  - npm test 57/57 pass — unit test suite health
requirement_outcomes:
  - id: R016
    from_status: active
    to_status: active
    proof: "Monaco Editor + IDE 3-panel layout + inline error markers + contract interaction UI + Vibe-Score dashboard + responsive layout all implemented and build-verified. R016 remains active — future enhancements possible but core IDE redesign delivered."
  - id: R009
    from_status: active
    to_status: active
    proof: "All 7 API methods (getContractSource, deployContract, compileContract, analyzeError, getVibeScore, getDeployStatus, getUserProfile) preserved in api-client.ts and wired into new IDE layout. No API endpoint modifications."
duration: 114m
verification_result: passed
completed_at: 2026-03-23
---

# M002: Vibe-Loom Frontend — Monad IDE 리디자인

**Replaced the textarea prototype frontend with a full Monaco Editor-based smart contract IDE featuring Solidity syntax highlighting, inline compile error markers, ABI-driven contract interaction, Vibe-Score dashboard with SVG gauge, AI diff suggestions via Monaco DiffEditor, and mobile-responsive tab layout — all in 229 kB first load with 57 passing tests.**

## What Happened

Four slices executed sequentially over ~114 minutes, each building on the prior to transform the Vibe-Loom frontend from a basic textarea+button prototype into a Remix-style IDE experience.

**S01 (33m)** laid the foundation: installed `@monaco-editor/react` and `react-resizable-panels`, created a two-file SSR-safe Monaco wrapper architecture (MonacoEditor.tsx thin dynamic wrapper + MonacoEditorInner.tsx actual editor), registered a custom Solidity completion provider with 9 snippet templates and 50 keywords, and built the IDE 3-panel resizable layout (EditorPanel 75% | SidebarPanel 25% horizontal, EditorArea 70% | ConsolePanel 30% vertical). Fully rewrote `page.tsx` to compose IDELayout with all existing state management, replacing the `<textarea>` with Monaco while preserving every API integration. Monaco loads as a separate ~630KB lazy chunk via genuine `import()` code splitting.

**S02 (26m)** built the compile/deploy UX layer: a `solc-error-parser` utility that extracts line/column/severity from solc error messages into Monaco markers (with fallback to line-1 for unparseable errors), a `useTransactionLog` hook managing newest-first timestamped event entries, a `TransactionConsole` component with color-coded status indicators, and a Monaco DiffEditor (`AIDiffViewer`) for AI fix suggestions — all using the same two-file SSR-safe pattern. The Compile button now sets inline error markers via `monacoInstance.editor.setModelMarkers()`, and both compile and deploy events flow through the transaction log.

**S03 (35m)** completed the functional feature set: an `abi-utils` library for parsing ABI functions and converting Solidity types to form inputs (with BigInt support for uint256), a `ContractInteraction` component with separate ReadFunctionCard (viem) and WriteFunctionCard (wagmi) sub-components, and a `VibeScoreDashboard` with an SVG circular gauge, 3-column stats grid (conflicts/reExecutions/gasEfficiency with emoji icons), and numbered suggestion cards. All contract call results route through `onCallResult → addEntry` to the TransactionConsole.

**S04 (20m)** polished and prepared for production: created a `useIsMobile` hook with SSR-safe `matchMedia`, refactored IDELayout to render a 3-tab mobile UI (Editor/Results/Console) using absolute-positioned panels that keep Monaco's DOM alive across tab switches, applied dark mode visual refinements (gradient headers, panel-transition cubic-bezier, focus-visible amber outlines, font smoothing), deleted dead code (VibeScoreGauge.tsx, CodeDiffView.tsx), updated layout metadata to "Monad Vibe-Loom IDE", and configured `next.config.js` with `unoptimized: true` for Vercel static deployment.

## Cross-Slice Verification

Each success criterion from the M002 roadmap was verified:

| Success Criterion | Status | Evidence |
|---|---|---|
| Monaco Editor에서 Solidity 구문 하이라이팅 동작 | ✅ | `defaultLanguage="sol"` in MonacoEditorInner.tsx, `theme="vs-dark"`, build passes |
| 컴파일 에러가 에디터 인라인 마커로 표시 | ✅ | `setModelMarkers` called 3 times in page.tsx, `solc-error-parser` with 12 tests |
| Deploy 결과가 트랜잭션 콘솔에 기록 | ✅ | TransactionConsole in page.tsx, `addEntry` calls in deploy flow, 7 hook tests |
| 배포된 컨트랙트 ABI 기반 read/write 함수 호출 | ✅ | ContractInteraction.tsx with ReadFunctionCard (viem) + WriteFunctionCard (wagmi), 17 abi-utils tests |
| Vibe-Score 시각적 대시보드(게이지 + 제안 카드) | ✅ | VibeScoreDashboard.tsx with SVG gauge + stats grid + suggestion cards, 10 render tests |
| AI 수정 제안이 Monaco diff editor로 표시 | ✅ | AIDiffViewer.tsx (SSR-safe DiffEditor), replaces CodeDiffView in page.tsx |
| IDE 스타일 패널 리사이즈 가능 | ✅ | IDELayout.tsx with react-resizable-panels v4 Group/Panel/Separator |
| 모바일 레이아웃 동작 | ✅ | useIsMobile hook + mobile 3-tab layout in IDELayout with absolute-positioned panels |

**Definition of Done verification:**

| Criterion | Status |
|---|---|
| 모든 4개 슬라이스 완료 | ✅ S01✅ S02✅ S03✅ S04✅ |
| 전체 플로우 동작 (편집→컴파일→배포→인터랙션) | ✅ All components wired in page.tsx |
| Vibe-Score 대시보드 + AI 수정 제안 동작 | ✅ VibeScoreDashboard + AIDiffViewer in page.tsx |
| vibe-loom.xyz 라이브 배포 | ⚠️ Build ready, Vercel deploy pending env config |
| 반응형 레이아웃 모바일/데스크톱 동작 | ✅ useIsMobile + mobile tabs + desktop 3-panel |
| `npm run build` 성공 | ✅ Exit 0, 229 kB first load |
| `npm test` 전 테스트 통과 | ✅ 57/57 pass, 5 suites |
| 모든 API 메서드 보존 | ✅ 7 methods in api-client.ts unchanged |
| 16 key files exist | ✅ All verified |
| Dead code removed | ✅ VibeScoreGauge.tsx + CodeDiffView.tsx deleted |

**Note:** Vercel deployment to vibe-loom.xyz is production-build-ready (`npm run build` passes, `next.config.js` configured with `unoptimized: true`) but the actual deployment requires setting `NEXT_PUBLIC_API_URL` environment variable on Vercel pointing to the Railway backend — this is a platform configuration step outside code scope.

## Requirement Changes

- **R016** (프론트엔드 IDE 리디자인): active → active — Core IDE redesign fully delivered (Monaco Editor, 3-panel layout, inline markers, contract interaction, Vibe-Score dashboard, responsive layout). Remains active for potential future enhancements but the M002 scope is complete.
- **R009** (프론트엔드 API 엔드포인트 전환): active → active — All 7 API methods preserved without modification in the new IDE layout. The API layer was untouched; only the UI consuming it changed.

No requirement status transitions to validate — both R016 and R009 remain active as their validation requires live deployment verification which is pending Vercel env configuration.

## Forward Intelligence

### What the next milestone should know
- The frontend IDE is feature-complete in code. All components (MonacoEditor, IDELayout, TransactionConsole, ContractInteraction, VibeScoreDashboard, AIDiffViewer) coexist at 229 kB first load. Monaco loads as a separate ~630KB lazy chunk.
- `NEXT_PUBLIC_API_URL` must be set at Vercel **build time** — it's inlined by webpack. Deploy backend first, then trigger frontend build with the backend URL.
- `--legacy-peer-deps` is required for **all** `npm install` commands due to @monaco-editor/react declaring React 18 max peer dep (works with React 19).
- The project has three independent build systems in one repo: `cargo build` (root), `npm run build` (backend/), `npm run build` (frontend/). Each must be handled separately.

### What's fragile
- **Monaco CDN dependency** — Monaco loads from CDN via @monaco-editor/react. If CDN is blocked/slow, the "Loading editor..." fallback persists indefinitely with no offline fallback.
- **Monaco in mobile tab layout** — The absolute-positioned panel approach depends on a `relative h-full` container chain. Any CSS change breaking this chain causes Monaco to render at 0 height with no error signal.
- **ContractInteraction conditional rendering** — Renders only when `compileResult?.abi && deployResult?.address` exist. State shape changes silently hide the UI with no console warning.
- **solc-error-parser regex** — Relies on solc's `formattedMessage` format (`file:line:col: Severity: msg`). Backend error format changes will cause fallback to line-1 markers (graceful degradation, not crash).

### Authoritative diagnostics
- `npm run build` exit 0 with 229 kB first load — the single most trustworthy signal that all imports resolve and TypeScript is clean.
- `npm test` 57/57 pass — covers api-client (11), solc-error-parser (12), useTransactionLog (7), VibeScoreDashboard (10), abi-utils (17).
- `monacoInstance.editor.getModelMarkers({owner:'solc'})` in browser console — ground truth for compile error markers.
- React DevTools → `entries` state on TransactionConsole — full event history.
- Browser DevTools responsive mode at 375px/1280px — primary tool for layout verification.

### What assumptions changed
- **Monarch tokenizer not needed** — Plan mentioned "커스텀 monarch tokenizer 등록" but Monaco has a built-in `sol` language with syntax highlighting. Only the completion provider needed custom work.
- **Bundle size concern resolved** — Monaco's ~2MB was a stated risk, but the two-file split with `next/dynamic({ ssr: false })` confirmed lazy loading. Main page bundle is only 101 kB (229 kB first load shared).
- **DiffEditor has zero marginal bundle cost** — Monaco DiffEditor shares the core instance via CDN loader. Adding AIDiffViewer alongside MonacoEditor added no bundle size.
- **Vercel deployment is a platform config step, not a code step** — Build passes, config is ready, but actual deployment requires env var setup on Vercel which is outside code scope.

## Files Created/Modified

### New files (16 components + 4 test files + 1 hook)
- `frontend/src/components/ide/MonacoEditor.tsx` — SSR-safe next/dynamic wrapper for Monaco Editor
- `frontend/src/components/ide/MonacoEditorInner.tsx` — Actual Monaco Editor with sol language + vs-dark theme
- `frontend/src/components/ide/IDELayout.tsx` — 3-panel resizable layout (desktop) + 3-tab layout (mobile)
- `frontend/src/components/ide/EditorPanel.tsx` — Editor panel with optional toolbar slot
- `frontend/src/components/ide/SidebarPanel.tsx` — Scrollable sidebar with gradient header
- `frontend/src/components/ide/ConsolePanel.tsx` — Console panel with fixed header
- `frontend/src/components/ide/TransactionConsole.tsx` — Color-coded compile/deploy/call event log
- `frontend/src/components/ide/ContractInteraction.tsx` — ABI-driven read (viem) / write (wagmi) function call UI
- `frontend/src/components/ide/VibeScoreDashboard.tsx` — SVG gauge + stats grid + suggestion cards
- `frontend/src/components/ide/AIDiffViewer.tsx` — SSR-safe next/dynamic wrapper for DiffEditor
- `frontend/src/components/ide/AIDiffViewerInner.tsx` — Monaco DiffEditor with summary banner + Apply Fix
- `frontend/src/lib/solidity-language.ts` — Solidity completion provider (9 snippets, 50 keywords)
- `frontend/src/lib/solc-error-parser.ts` — solc formattedMessage → MonacoMarker[] parser
- `frontend/src/lib/abi-utils.ts` — ABI parsing utility (4 exported functions with BigInt support)
- `frontend/src/hooks/useTransactionLog.ts` — Transaction log state hook (addEntry/clearLog)
- `frontend/src/hooks/useIsMobile.ts` — SSR-safe viewport detection hook via matchMedia
- `frontend/src/__tests__/solc-error-parser.test.ts` — 12 parser tests
- `frontend/src/__tests__/useTransactionLog.test.ts` — 7 hook tests
- `frontend/src/__tests__/VibeScoreDashboard.test.tsx` — 10 render tests
- `frontend/src/__tests__/abi-utils.test.ts` — 17 ABI utility tests

### Modified files
- `frontend/src/app/page.tsx` — Full rewrite: IDELayout + MonacoEditor composition with all state/handlers
- `frontend/src/app/globals.css` — Full-viewport CSS, font smoothing, focus-visible, panel-transition
- `frontend/src/app/layout.tsx` — Title → "Monad Vibe-Loom IDE"
- `frontend/src/lib/api-client.ts` — CompileResult.contractName + VibeScoreResult extended fields
- `frontend/next.config.js` — images.unoptimized for Vercel
- `frontend/package.json` — Added @monaco-editor/react, react-resizable-panels, @testing-library/dom

### Deleted files
- `frontend/src/components/VibeScoreGauge.tsx` — Replaced by VibeScoreDashboard
- `frontend/src/components/CodeDiffView.tsx` — Replaced by AIDiffViewer
