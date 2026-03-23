---
id: T02
parent: S05
milestone: M001
provides:
  - Centralized API client with NestJS envelope unwrapping and JWT auth
  - React auth context with GitHub OAuth token lifecycle
  - All 5 API calls rewired from hardcoded /api/ to NestJS backend via NEXT_PUBLIC_API_URL
  - Backend OAuth callback redirects to frontend with token parameter
key_files:
  - frontend/src/lib/api-client.ts
  - frontend/src/lib/auth-context.tsx
  - frontend/src/app/page.tsx
  - frontend/src/app/layout.tsx
  - frontend/src/app/providers.tsx
  - frontend/src/components/VibeStatus.tsx
  - backend/src/auth/auth.controller.ts
  - backend/src/config/configuration.ts
  - backend/test/auth.controller.spec.ts
key_decisions:
  - Created providers.tsx client wrapper to keep layout.tsx as server component for metadata export
  - API client uses console.error logging with endpoint and status for observability
patterns_established:
  - All fetch calls go through api-client.ts typed methods — never direct fetch to /api/ paths
  - Auth token stored in localStorage key 'vibe-loom-token', injected via getAuthHeaders()
  - NestJS { success, data } envelope unwrapped by generic unwrapResponse<T> helper
  - OAuth callback flow: backend redirects to frontend?token=xxx → auth-context reads URL param → stores in localStorage
observability_surfaces:
  - console.error logs on API failures with endpoint name and HTTP status code
  - localStorage 'vibe-loom-token' key for JWT auth state inspection
  - auth-context exposes isAuthenticated and user state via useAuth() hook
duration: 11m
verification_result: passed
completed_at: 2026-03-22
blocker_discovered: false
---

# T02: Create API client + auth context and rewire all API calls to NestJS backend

**Created centralized api-client.ts with 5 typed endpoint methods, auth-context.tsx for JWT lifecycle, rewired all fetch calls in page.tsx and VibeStatus.tsx to NestJS backend, and modified backend OAuth callback to redirect to frontend with token.**

## What Happened

Created `frontend/src/lib/api-client.ts` with base URL from `NEXT_PUBLIC_API_URL`, JWT auth header injection from localStorage, and generic `unwrapResponse<T>` that handles the NestJS `{ success, data }` envelope. Implemented all 5 API methods with correct paths and field mappings: `getContractSource` (→ `/api/contracts/source`), `deployContract` (→ `/api/contracts/deploy`), `analyzeError` (→ `/api/analysis/error`), `getVibeScore` (→ `/api/vibe-score`), and `getDeployStatus` (→ `/api/paymaster/status`). Each method includes TypeScript interfaces for response types.

Created `frontend/src/lib/auth-context.tsx` with `AuthProvider` that reads `?token=xxx` from URL params (OAuth callback redirect), stores in localStorage, fetches user profile, and provides `login()`/`logout()` via `useAuth()` hook.

Created `frontend/src/app/providers.tsx` as a client component wrapper to keep `layout.tsx` as a server component (required for Next.js metadata export). Updated `layout.tsx` to wrap children with `<Providers>`.

Rewrote `page.tsx` — replaced all 3 fetch calls with api-client methods, mapped field names correctly (e.g., `data.vibeScore` instead of `data.score`, `analysis.fixedCode` instead of `fixedSnippet`, `analysis.explanation` instead of `summary`), added GitHub login/logout button in the header area showing username when authenticated.

Rewrote `VibeStatus.tsx` — removed `githubId` prop entirely, uses JWT-authenticated `getDeployStatus()` instead, shows "로그인 필요" when not authenticated. Removed `githubId` prop from the call site in `page.tsx`.

Modified `backend/src/auth/auth.controller.ts` — `githubCallback` now injects `ConfigService` and `@Res() res: Response`, redirecting to `${frontendUrl}?token=${accessToken}` instead of returning JSON. Updated `auth.controller.spec.ts` to provide mock `ConfigService` and mock `res.redirect` — test count went from 92 to 97.

Added `frontend: { url: process.env.FRONTEND_URL || 'http://localhost:3001' }` to `backend/src/config/configuration.ts`.

## Verification

- `cd frontend && npm run build` — exits 0, compiled in 3.1s with zero TypeScript errors
- `cd backend && npm run build` — exits 0
- `cd backend && npm test` — 14 suites, 97 tests pass (up from 92 — auth controller test now includes redirect and fallback URL tests)
- `grep -r "NEXT_PUBLIC_API_URL" frontend/src/` — confirms env var usage in api-client.ts and auth-context.tsx
- `grep -rn 'fetch("/api/' frontend/src/` — no matches, all hardcoded fetch calls removed

### Slice-level verification (T02 of 3):
- ✅ `cd frontend && npm run build` — zero build errors
- ⏳ `cd frontend && npm test` — test placeholder (API client unit tests created in T03)
- ✅ `cd backend && npm test` — 97 tests pass (14 suites)
- ✅ `grep -r "NEXT_PUBLIC_API_URL" frontend/src/` — 3 matches in api-client.ts and auth-context.tsx
- ✅ `! grep -rn "fetch(\"/api/" frontend/src/` — no hardcoded fetch calls remain

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd frontend && npm run build` | 0 | ✅ pass | 23.2s |
| 2 | `cd backend && npm run build` | 0 | ✅ pass | 4.7s |
| 3 | `cd backend && npm test` | 0 | ✅ pass | 13.2s |
| 4 | `grep -r "NEXT_PUBLIC_API_URL" frontend/src/lib/api-client.ts` | 0 | ✅ pass | <1s |
| 5 | `grep -rn 'fetch("/api/' frontend/src/` (expect no matches) | 1 | ✅ pass | <1s |

## Diagnostics

- **API client health:** Check `frontend/src/lib/api-client.ts` for all 5 endpoint definitions and their TypeScript interfaces
- **Auth state inspection:** `localStorage.getItem('vibe-loom-token')` in browser DevTools shows JWT if logged in
- **Failed request visibility:** API client logs to `console.error` with `[api-client] /api/endpoint failed: HTTP <status>` format
- **Auth context state:** `useAuth()` hook exposes `isAuthenticated`, `user`, `token`, `login()`, `logout()`
- **Backend OAuth flow:** `auth.controller.ts` redirects to `${FRONTEND_URL}?token=<jwt>` after GitHub callback

## Deviations

- Created `providers.tsx` as a separate client component wrapper instead of making `layout.tsx` a client component. This is necessary because Next.js requires `layout.tsx` to be a server component when exporting `metadata`. Not a change to what was delivered — just an implementation detail for the "wrap layout with AuthProvider" step.
- Updated `backend/test/auth.controller.spec.ts` to match new `githubCallback` signature (mock ConfigService + mock res.redirect). This was expected given the controller change.

## Known Issues

- Frontend `npm test` still uses placeholder script (`echo + exit 0`). API client unit tests are created in T03.
- The `NEXT_PUBLIC_API_URL` is duplicated in both `api-client.ts` and `auth-context.tsx` — could be extracted to a shared constant but keeping it simple for now.

## Files Created/Modified

- `frontend/src/lib/api-client.ts` — Centralized API client with 5 typed endpoint methods, JWT auth, and envelope unwrapping
- `frontend/src/lib/auth-context.tsx` — React auth context managing JWT lifecycle (localStorage, URL params, profile fetch)
- `frontend/src/app/providers.tsx` — Client component wrapper for AuthProvider (keeps layout as server component)
- `frontend/src/app/layout.tsx` — Updated to wrap children with Providers
- `frontend/src/app/page.tsx` — Rewired all 3 fetch calls to api-client methods, added login/logout button
- `frontend/src/components/VibeStatus.tsx` — Rewired to use JWT-authenticated getDeployStatus(), removed githubId prop
- `backend/src/auth/auth.controller.ts` — OAuth callback redirects to frontend with token param
- `backend/src/config/configuration.ts` — Added frontend.url config entry
- `backend/test/auth.controller.spec.ts` — Updated test for new githubCallback signature with mock ConfigService and Response
