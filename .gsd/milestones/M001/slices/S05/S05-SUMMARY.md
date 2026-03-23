---
id: S05
parent: M001
milestone: M001
provides:
  - Independent Next.js 15 frontend project in frontend/ with Vibe-Loom UI ported
  - Centralized api-client.ts with NestJS envelope unwrapping, JWT auth, and 6 typed endpoint methods
  - React auth context (AuthProvider) with GitHub OAuth token lifecycle via localStorage
  - WalletConnect/wagmi integration with Monad Testnet chain (10143) and deploy gating
  - WalletConnectModal component for wallet-based deploys when canUseRelay=false
  - Backend OAuth callback redirect to frontend with token parameter
  - 11 API client unit tests covering envelope, auth, and field mapping contracts
requires:
  - slice: S02
    provides: ContractsModule endpoints (GET /api/contracts/source, POST /api/contracts/compile, POST /api/contracts/deploy)
  - slice: S03
    provides: AuthModule (GitHub OAuth + JWT), AnalysisModule (POST /api/analysis/error), PaymasterModule (GET /api/paymaster/status, POST /api/paymaster/relay-signed)
  - slice: S04
    provides: VibeScoreModule (POST /api/vibe-score)
affects:
  - S06
key_files:
  - frontend/src/lib/api-client.ts
  - frontend/src/lib/auth-context.tsx
  - frontend/src/lib/wagmi-config.ts
  - frontend/src/components/WalletConnectModal.tsx
  - frontend/src/app/page.tsx
  - frontend/src/app/layout.tsx
  - frontend/src/app/providers.tsx
  - frontend/src/__tests__/api-client.test.ts
  - backend/src/auth/auth.controller.ts
key_decisions:
  - Used wagmi v2 native connectors instead of deprecated @web3modal/wagmi (D016)
  - Created providers.tsx client wrapper to keep layout.tsx as server component for metadata export (D017)
  - API client uses console.error logging with endpoint and status for observability
patterns_established:
  - All fetch calls go through api-client.ts typed methods — never direct fetch to /api/ paths
  - Auth token stored in localStorage key 'vibe-loom-token', injected via getAuthHeaders()
  - NestJS { success, data } envelope unwrapped by generic unwrapResponse<T> helper
  - OAuth callback flow — backend redirects to frontend?token=xxx → auth-context reads URL param → stores in localStorage
  - Deploy gating pattern — check paymaster status first, show WalletConnectModal if canUseRelay=false
  - WalletConnect connector conditionally added only when NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID is set
observability_surfaces:
  - console.error logs on API failures with endpoint name and HTTP status code
  - localStorage 'vibe-loom-token' key for JWT auth state inspection
  - auth-context exposes isAuthenticated and user state via useAuth() hook
  - console.warn for missing WalletConnect project ID
  - npm test runs 11 API client contract tests as slice verification
drill_down_paths:
  - .gsd/milestones/M001/slices/S05/tasks/T01-SUMMARY.md
  - .gsd/milestones/M001/slices/S05/tasks/T02-SUMMARY.md
  - .gsd/milestones/M001/slices/S05/tasks/T03-SUMMARY.md
duration: 37m
verification_result: passed
completed_at: 2026-03-22
---

# S05: Frontend Integration

**Next.js 15 frontend ported from Vibe-Loom with all 5 API calls rewired to NestJS backend via centralized api-client, GitHub OAuth auth context, WalletConnect wallet integration for 3+ deploys, and 11 unit tests verifying the API contract.**

## What Happened

Built the frontend in three tasks. First, scaffolded an independent Next.js 15 project in `frontend/` with React 19 and Tailwind CSS v4, copying all 6 Vibe-Loom source files (page.tsx, layout.tsx, globals.css, 3 components). The project builds cleanly with zero TypeScript errors from the start.

Second, created the core integration layer: `api-client.ts` provides 6 typed endpoint methods with `NEXT_PUBLIC_API_URL` base URL, JWT `Authorization: Bearer` header injection from localStorage, and a generic `unwrapResponse<T>` that strips the NestJS `{ success, data }` envelope. Field name mappings are correct: `source` (not `code`), `vibeScore` (not `score`), `fixedCode` (not `fixedSnippet`). Created `auth-context.tsx` with `AuthProvider` that handles the full OAuth lifecycle — reads `?token=xxx` from URL after GitHub callback redirect, stores in localStorage, fetches user profile via `/api/auth/me`. Added `providers.tsx` as a client component wrapper (keeping `layout.tsx` as a server component for Next.js metadata export). Rewired all fetch calls in `page.tsx` and `VibeStatus.tsx` to use api-client methods. Modified backend `auth.controller.ts` to redirect to `${FRONTEND_URL}?token=<jwt>` after OAuth callback instead of returning JSON. Backend tests grew from 92 to 97.

Third, integrated WalletConnect via wagmi v2 with native `injected` + `walletConnect` connectors (avoiding the deprecated `@web3modal/wagmi` package). Created `wagmi-config.ts` defining Monad Testnet chain (chainId 10143, MON currency). Created `WalletConnectModal.tsx` with four states: connector selection → connected address → deploying → success. Modified `page.tsx` deploy flow to check paymaster status before deploying — when `canUseRelay` is false, the WalletConnectModal opens instead of server-side deploy. Added `compileContract` API method for wallet-based bytecode deployment. Set up Jest with `next/jest` preset and wrote 11 unit tests covering envelope unwrapping (success, error, non-ok HTTP), correct URL/path for each endpoint, field mappings, auth header injection, and auth header absence.

## Verification

All 5 slice-level verification checks pass:

| # | Check | Result |
|---|-------|--------|
| 1 | `cd frontend && npm run build` — zero TypeScript/build errors | ✅ pass |
| 2 | `cd frontend && npm test` — 11 API client unit tests pass | ✅ pass |
| 3 | `cd backend && npm test` — 97 tests pass (14 suites, no regressions) | ✅ pass |
| 4 | `grep -r "NEXT_PUBLIC_API_URL" frontend/src/` — 3 matches in api-client.ts, auth-context.tsx | ✅ pass |
| 5 | `! grep -rn 'fetch("/api/' frontend/src/` — no hardcoded fetch calls remain | ✅ pass |

## New Requirements Surfaced

- none

## Deviations

- Used wagmi v2 native `walletConnect` connector instead of `@web3modal/wagmi` specified in the task plan. The `@web3modal/wagmi` package is deprecated and rebranded as `@reown/appkit`. Wagmi's native connector provides identical functionality.
- Created `providers.tsx` instead of making `layout.tsx` a client component. Next.js requires server components for `metadata` export, so a separate client wrapper was necessary.
- Cast `walletConnect()` connector to `CreateConnectorFn` to resolve TypeScript generic type parameter mismatch between `injected()` and `walletConnect()` storage types.

## Known Limitations

- CORS configuration is not yet applied on the NestJS backend — needed for cross-origin frontend→backend communication at runtime (S06 scope).
- WalletConnect QR modal requires a valid `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID` from WalletConnect Cloud — without it, only injected wallets (MetaMask) are available.
- Frontend runtime verification requires the NestJS backend running — build + unit tests verify the API contract, but full integration is validated in S06 E2E tests.
- `NEXT_PUBLIC_API_URL` constant is duplicated in `api-client.ts` and `auth-context.tsx` — could be extracted to a shared module.

## Follow-ups

- S06 must configure CORS on the NestJS backend to allow the frontend origin.
- S06 E2E tests should validate the complete frontend→backend flow at runtime.
- S06 Dockerfile must serve the frontend (or deploy it separately) alongside the NestJS backend.

## Files Created/Modified

- `frontend/package.json` — Next.js 15 project manifest with runtime (next, react, wagmi, viem) and dev (jest, testing-library) dependencies
- `frontend/tsconfig.json` — TypeScript config with Next.js defaults and @/* path alias
- `frontend/next.config.js` — Minimal Next.js configuration
- `frontend/postcss.config.js` — PostCSS config with @tailwindcss/postcss plugin (Tailwind v4)
- `frontend/src/app/page.tsx` — Main page rewired to api-client methods with deploy gating on canUseRelay
- `frontend/src/app/layout.tsx` — Root layout wrapped with Providers
- `frontend/src/app/providers.tsx` — Client component wrapping AuthProvider + WagmiProvider + QueryClientProvider
- `frontend/src/app/globals.css` — Tailwind v4 styles with @import directive
- `frontend/src/lib/api-client.ts` — Centralized API client with 6 typed methods, JWT auth, envelope unwrapping
- `frontend/src/lib/auth-context.tsx` — React auth context managing JWT lifecycle (localStorage, URL params, profile fetch)
- `frontend/src/lib/wagmi-config.ts` — Wagmi configuration with Monad Testnet chain (10143), injected + WalletConnect connectors
- `frontend/src/components/VibeScoreGauge.tsx` — Score gauge component (ported from Vibe-Loom)
- `frontend/src/components/CodeDiffView.tsx` — Diff viewer component (ported from Vibe-Loom)
- `frontend/src/components/VibeStatus.tsx` — Deploy status component rewired to JWT-authenticated getDeployStatus()
- `frontend/src/components/WalletConnectModal.tsx` — Modal for wallet connection and direct wallet deploys
- `frontend/jest.config.js` — Jest configuration with next/jest preset and jsdom environment
- `frontend/jest.setup.js` — Jest setup importing @testing-library/jest-dom
- `frontend/src/__tests__/api-client.test.ts` — 11 unit tests for API client (envelope, auth, field mapping)
- `frontend/.env.local.example` — Env var documentation for NEXT_PUBLIC_API_URL
- `backend/src/auth/auth.controller.ts` — OAuth callback redirects to frontend with token param
- `backend/src/config/configuration.ts` — Added frontend.url config entry
- `backend/test/auth.controller.spec.ts` — Updated test for new githubCallback redirect signature

## Forward Intelligence

### What the next slice should know
- The frontend is a completely independent Next.js project in `frontend/` with its own `package.json`. It is NOT part of the NestJS monorepo. S06 Dockerfile must handle two separate build contexts or serve the frontend separately.
- `NEXT_PUBLIC_API_URL` must be set at build time (Next.js inlines `NEXT_PUBLIC_*` vars during build). For Railway, this means the frontend must be built after the backend URL is known.
- The backend OAuth callback redirects to `FRONTEND_URL?token=<jwt>`. S06 must ensure `FRONTEND_URL` is set correctly in Railway env vars.
- Backend test count is now 97 (up from 92 in S04) due to auth controller redirect tests.

### What's fragile
- `api-client.ts` field mappings (`source`, `vibeScore`, `fixedCode`, `explanation`) — if any NestJS controller changes its response shape, the frontend silently receives undefined values. The 11 unit tests mock fetch responses; there's no integration test that validates both sides match.
- `wagmi-config.ts` conditionally includes WalletConnect connector based on env var — if `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID` is unset, wallet connect silently degrades to injected-only.

### Authoritative diagnostics
- `cd frontend && npm run build` — exit code 0 proves TypeScript compilation and all API type contracts
- `cd frontend && npm test` — 11 tests verify API client envelope unwrapping, auth header injection, and field mapping against the NestJS contract
- `cd backend && npm test` — 97 tests (14 suites) confirm backend stability after OAuth redirect change
- Browser DevTools Network tab → all API calls should target `${NEXT_PUBLIC_API_URL}/api/*`

### What assumptions changed
- Plan assumed `@web3modal/wagmi` for WalletConnect UI — actually deprecated, used wagmi native connectors with custom modal instead
- Plan assumed `layout.tsx` would wrap with AuthProvider directly — Next.js metadata export requires server component, so `providers.tsx` intermediary was necessary
