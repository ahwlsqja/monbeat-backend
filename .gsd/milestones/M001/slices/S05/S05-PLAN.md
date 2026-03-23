# S05: Frontend Integration

**Goal:** Next.js 프론트엔드가 NestJS 백엔드 API만을 호출하며, 기존 Vibe-Loom과 동일한 UX를 제공한다. GitHub OAuth 로그인 + WalletConnect 모달 포함.
**Demo:** `cd frontend && npm run build` 성공, 브라우저에서 페이지 로드 → 컨트랙트 선택 → 소스 표시 → Vibe-Score 분석 → 배포 → 에러 시 AI 수정 제안. 로그인 버튼 → GitHub OAuth → JWT 발급. 3회 초과 배포 시 WalletConnect 지갑 연결 모달 표시.

## Must-Haves

- `frontend/` 디렉토리에 독립 Next.js 프로젝트 존재 (별도 package.json)
- 5개 API 호출이 모두 NestJS 백엔드를 가리킴 (`NEXT_PUBLIC_API_URL` 기반)
- NestJS `{ success, data }` 응답 envelope을 API 클라이언트가 자동 unwrap
- 필드명 매핑 정확: `code` → `source`, `score` → `vibeScore`, `fixedSnippet` → `fixedCode`
- GitHub OAuth 로그인 버튼 + JWT 토큰 관리 (localStorage)
- WalletConnect 지갑 연결 모달 (3회 초과 배포 시)
- `npm run build` 타입 에러 0개, `npm test` 통과

## Proof Level

- This slice proves: integration
- Real runtime required: no (build + unit tests verify contract; runtime needs backend running)
- Human/UAT required: yes (S06에서 전체 플로우 수동 검증)

## Verification

- `cd frontend && npm run build` — zero TypeScript/build errors
- `cd frontend && npm test` — API client unit tests pass (envelope unwrapping, auth header injection, field mapping)
- `cd backend && npm test` — existing 96 tests still pass (backend changes are minimal)
- `grep -r "NEXT_PUBLIC_API_URL" frontend/src/` — all API calls use environment variable-based base URL
- `! grep -rn "fetch(\"/api/" frontend/src/` — no hardcoded `/api/` fetch calls remain (all go through api-client)

## Observability / Diagnostics

- Runtime signals: API client logs failed requests to console.error with status code and endpoint
- Inspection surfaces: Browser DevTools Network tab shows all `${NEXT_PUBLIC_API_URL}/api/*` calls; auth state visible in localStorage `vibe-loom-token` key
- Failure visibility: API client throws typed errors with endpoint name and HTTP status; auth context exposes `isAuthenticated` and `user` state
- Redaction constraints: JWT token stored in localStorage — never logged or displayed in UI

## Integration Closure

- Upstream surfaces consumed: `backend/src/contracts/contracts.controller.ts` (3 endpoints), `backend/src/analysis/analysis.controller.ts` (1 endpoint), `backend/src/vibe-score/vibe-score.controller.ts` (1 endpoint), `backend/src/paymaster/paymaster.controller.ts` (2 endpoints), `backend/src/auth/auth.controller.ts` (3 endpoints)
- New wiring introduced in this slice: `frontend/src/lib/api-client.ts` connects to all NestJS endpoints; `backend/src/auth/auth.controller.ts` modified to redirect to frontend after OAuth callback
- What remains before the milestone is truly usable end-to-end: S06 — Railway deployment, Dockerfile with Rust CLI binary, E2E tests

## Tasks

- [x] **T01: Scaffold frontend/ Next.js project with Vibe-Loom code** `est:30m`
  - Why: Establishes the frontend project structure. Without a buildable Next.js project, no API rewiring or feature work can begin.
  - Files: `frontend/package.json`, `frontend/tsconfig.json`, `frontend/next.config.js`, `frontend/tailwind.config.ts`, `frontend/postcss.config.js`, `frontend/src/app/page.tsx`, `frontend/src/app/layout.tsx`, `frontend/src/app/globals.css`, `frontend/src/components/VibeScoreGauge.tsx`, `frontend/src/components/CodeDiffView.tsx`, `frontend/src/components/VibeStatus.tsx`
  - Do: Create Next.js 15 project in `frontend/`. Copy Vibe-Loom's page.tsx, layout.tsx, globals.css, and 3 components. Configure Tailwind CSS. Set `NEXT_PUBLIC_API_URL` env var pattern. Do NOT modify any component logic — just make it build. The existing `/api/*` fetch paths will fail at runtime but TypeScript compilation must succeed.
  - Verify: `cd frontend && npm install && npm run build` exits 0
  - Done when: `frontend/` contains a complete Next.js project that builds without TypeScript errors

- [x] **T02: Create API client + auth context and rewire all API calls to NestJS backend** `est:45m`
  - Why: This is the core R009 delivery — all 5 API calls in the frontend must point to NestJS with correct field mappings, and GitHub OAuth login must work end-to-end.
  - Files: `frontend/src/lib/api-client.ts`, `frontend/src/lib/auth-context.tsx`, `frontend/src/app/page.tsx`, `frontend/src/app/layout.tsx`, `frontend/src/components/VibeStatus.tsx`, `backend/src/auth/auth.controller.ts`
  - Do: (1) Create `api-client.ts` with base URL from `NEXT_PUBLIC_API_URL`, JWT `Authorization: Bearer` header injection, `{ success, data }` envelope unwrapping, and typed methods for all 5 API calls with correct path/field mappings. (2) Create `auth-context.tsx` React context with JWT token in localStorage, login redirect, token extraction from URL params after OAuth callback, user profile fetch via GET /api/auth/me. (3) Rewrite all fetch calls in `page.tsx` to use api-client methods. (4) Rewrite VibeStatus.tsx to use api-client with JWT auth. (5) Add login/logout button in page header. (6) Modify backend `auth.controller.ts` githubCallback to redirect to frontend URL with token param instead of returning JSON. (7) Wrap layout.tsx with AuthProvider.
  - Verify: `cd frontend && npm run build` exits 0 and `cd backend && npm run build && npm test` — 96 tests pass
  - Done when: All API calls use api-client with NestJS paths/fields, auth context manages JWT lifecycle, backend OAuth callback redirects to frontend

- [x] **T03: Add WalletConnect integration + unit tests** `est:45m`
  - Why: Delivers R013 frontend support (wallet connection for 3+ deploys) and provides unit test verification for the entire slice.
  - Files: `frontend/src/lib/wagmi-config.ts`, `frontend/src/components/WalletConnectModal.tsx`, `frontend/src/app/page.tsx`, `frontend/src/app/layout.tsx`, `frontend/jest.config.js`, `frontend/src/__tests__/api-client.test.ts`
  - Do: (1) Install wagmi, viem, @web3modal/wagmi. (2) Create `wagmi-config.ts` with Monad testnet chain definition (chainId 10143, RPC from env). (3) Create `WalletConnectModal.tsx` — modal showing "Connect Wallet" button, displays connected address, provides deploy-via-wallet action using wagmi's sendTransaction. (4) Modify deploy flow in `page.tsx`: before deploy, check paymaster status; if `canUseRelay=false`, show WalletConnectModal instead of server deploy. (5) Wrap layout.tsx with WagmiProvider + Web3Modal. (6) Set up Jest + React Testing Library. (7) Write unit tests for api-client (envelope unwrapping, auth headers, field mapping for each endpoint).
  - Verify: `cd frontend && npm run build && npm test` — build succeeds and all tests pass
  - Done when: WalletConnect modal appears when deploys > 3, wallet connection works, unit tests cover API client envelope/auth/mapping logic

## Files Likely Touched

- `frontend/package.json`
- `frontend/tsconfig.json`
- `frontend/next.config.js`
- `frontend/tailwind.config.ts`
- `frontend/postcss.config.js`
- `frontend/src/app/page.tsx`
- `frontend/src/app/layout.tsx`
- `frontend/src/app/globals.css`
- `frontend/src/components/VibeScoreGauge.tsx`
- `frontend/src/components/CodeDiffView.tsx`
- `frontend/src/components/VibeStatus.tsx`
- `frontend/src/lib/api-client.ts`
- `frontend/src/lib/auth-context.tsx`
- `frontend/src/lib/wagmi-config.ts`
- `frontend/src/components/WalletConnectModal.tsx`
- `frontend/jest.config.js`
- `frontend/src/__tests__/api-client.test.ts`
- `backend/src/auth/auth.controller.ts`
