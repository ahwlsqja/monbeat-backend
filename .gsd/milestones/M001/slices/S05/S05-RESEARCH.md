# S05: Frontend Integration — Research

**Date:** 2026-03-22
**Depth:** Targeted (known technology — Next.js frontend migrating API calls to NestJS backend with known endpoints)

## Summary

S05 ports the existing Vibe-Loom Next.js frontend (`/tmp/vibe-loom/src/`) into the worktree as a `frontend/` sibling to `backend/`, then rewires all 5 API routes to call the NestJS backend instead of the local Next.js API routes. The frontend code is ~400 lines across 4 files (1 page, 3 components) plus layout/globals. The work is straightforward API URL remapping with minor request/response shape adjustments, plus adding a WalletConnect modal for the 3-deploy-exceeded flow (which doesn't exist yet in Vibe-Loom).

The NestJS backend wraps all responses in `{ success: true, data: {...} }` via TransformInterceptor (except AnalysisController which manually wraps). The Vibe-Loom frontend currently reads flat response shapes (`data.source`, `data.success`, `data.score`). The main integration work is:
1. Adjusting fetch URLs from `/api/*` to `${NEXT_PUBLIC_API_URL}/api/*`
2. Unwrapping the `{ success, data }` envelope from NestJS responses
3. Mapping field name differences (e.g., `{ code }` → `{ source }` for vibe-score)
4. Adding GitHub OAuth login flow and JWT token management
5. Adding WalletConnect wallet connection for 3+ deploys

## Recommendation

Copy the Vibe-Loom frontend into `frontend/` as a standalone Next.js project. Remove the 5 API routes (they now live in NestJS). Create an API client module that handles base URL, JWT auth headers, and response envelope unwrapping. Modify `page.tsx` to use the API client. Add a simple auth context for JWT token management and a WalletConnect integration for the paymaster flow.

**Do NOT redesign the UI** (per D007, R016). The scope is API rewiring + auth/wallet additions only.

## Implementation Landscape

### Key Files

**Source (copy from Vibe-Loom):**
- `/tmp/vibe-loom/src/app/page.tsx` — Main page component (~190 lines). All 5 API calls live here
- `/tmp/vibe-loom/src/app/layout.tsx` — Root layout (minimal, 16 lines)
- `/tmp/vibe-loom/src/app/globals.css` — Tailwind directives + body reset (9 lines)
- `/tmp/vibe-loom/src/components/VibeScoreGauge.tsx` — Score visualization (no API calls, copy as-is)
- `/tmp/vibe-loom/src/components/CodeDiffView.tsx` — Diff viewer (no API calls, copy as-is)
- `/tmp/vibe-loom/src/components/VibeStatus.tsx` — Deploy status badge (1 API call to rewrite)

**New files to create:**
- `frontend/src/lib/api-client.ts` — Centralized API client with base URL, JWT header injection, envelope unwrapping
- `frontend/src/lib/auth-context.tsx` — React context for JWT token + user state (localStorage persistence)
- `frontend/src/components/WalletConnectModal.tsx` — Modal for wallet connection when deploys > 3 (new feature)

**Backend file to modify (CORS):**
- `backend/src/main.ts` — CORS `origin: true` already set (permissive), no change needed

### API Mapping Table

| Vibe-Loom Frontend Call | Vibe-Loom Route | NestJS Endpoint | Request Diff | Response Diff |
|---|---|---|---|---|
| Load contract source | `GET /api/contract-source?type=X` | `GET /api/contracts/source?type=X` | Path change only | Vibe-Loom: `{ type, source }` → NestJS: `{ success, data: { contractType, source } }` — unwrap envelope, use `data.source` |
| Deploy contract | `POST /api/deploy` body: `{ contractType, contractSource }` | `POST /api/contracts/deploy` body: `{ source }` | Path + body shape: rename `contractSource` → `source`, drop `contractType` | Vibe-Loom: `{ success, address }` → NestJS: `{ success, data: { contractName, address, txHash, deploymentId } }` — unwrap envelope |
| Analyze error | `POST /api/analyze-deployment-error` body: `{ error, contractSource, contractCode }` | `POST /api/analysis/error` body: `{ error, contractSource }` | Path + body shape: rename `contractCode` → `contractSource`, drop duplicates | Vibe-Loom: `{ success, analysis: { originalSnippet, fixedSnippet, summary } }` → NestJS: `{ success, data: { analysis: { summary, fixedCode, explanation }, optimization } }` — field renames: `fixedSnippet` → `fixedCode`, `summary` comes from `explanation` |
| Vibe score | `POST /api/vibe-score` body: `{ code }` | `POST /api/vibe-score` body: `{ source }` | Body shape: rename `code` → `source` | Vibe-Loom: `{ success, score, suggestions }` → NestJS: `{ success, data: { vibeScore, suggestions, engineBased, ... } }` — unwrap + rename `score` → `vibeScore` |
| Deploy status | `GET /api/deploy-status?githubId=X` | `GET /api/paymaster/status` (JWT auth, no query param) | Path change, remove githubId param, add JWT Bearer header | Vibe-Loom: `{ used, max, remaining, canUseRelay }` → NestJS: `{ success, data: { used, max, remaining, canUseRelay } }` — unwrap envelope |
| (NEW) Login | — | `GET /api/auth/github` → callback → JWT | New flow | New: `{ accessToken, user }` |
| (NEW) User profile | — | `GET /api/auth/me` (JWT auth) | New flow | New: `{ success, data: { id, githubId, username, ... } }` |
| (NEW) Relay signed tx | — | `POST /api/paymaster/relay-signed` (JWT auth) body: `{ signedTransaction }` | New flow | New: `{ success, data: { txHash } }` |

### Build Order

1. **T01: Scaffold frontend/ and copy Vibe-Loom code** — Create Next.js project in `frontend/`, copy components, layout, globals.css, configure `NEXT_PUBLIC_API_URL` env var. Remove the 5 API route directories. This is pure scaffolding, no logic changes. Verify with `npm run build`.

2. **T02: API client + auth context + API rewiring** — Create `api-client.ts` with base URL handling and envelope unwrapping. Create `auth-context.tsx` for JWT token state. Rewrite all 5 fetch calls in `page.tsx` and `VibeStatus.tsx` to use the API client with correct paths, request bodies, and response parsing. Add a login button that redirects to GitHub OAuth. Verify with `npm run build` (no runtime deps on backend needed for build).

3. **T03: WalletConnect integration + deploy flow gating** — Add WalletConnect/wagmi/viem for wallet connection. Modify the deploy flow in `page.tsx`: check paymaster status first, if `canUseRelay=false` show WalletConnect modal, sign the deploy tx client-side, relay via `POST /api/paymaster/relay-signed`. This is the only new UI feature.

4. **T04: Unit tests + integration smoke test** — Write tests for the API client (envelope unwrapping, auth header injection) and any new components. Verify the complete frontend builds and the API mapping is correct.

### Verification Approach

- `cd frontend && npm run build` — zero TypeScript errors, zero build errors
- `cd frontend && npm test` — all unit tests pass
- `cd backend && npm test` — existing 96 tests still pass (no backend changes expected, or CORS-only)
- Manual check: `NEXT_PUBLIC_API_URL=http://localhost:3001 npm run dev` with backend running — page loads, contract source loads from NestJS

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| WalletConnect wallet connection | `@web3modal/wagmi` + `wagmi` + `viem` | Standard Web3 wallet modal. WalletConnect protocol handled by library. Don't implement raw WC protocol |
| JWT token storage | `localStorage` + React context | Simple, sufficient for this use case. No need for a state management library |
| API envelope unwrapping | Single `apiClient` wrapper function | One function handles all NestJS `{ success, data }` responses. Don't repeat unwrapping in every fetch call |

## Constraints

- **D007 (R016):** No UI redesign. Copy existing Vibe-Loom UI, change API calls only. Add only auth UI (login button) and WalletConnect modal (3+ deploys)
- **D008:** Frontend lives at `frontend/` sibling to `backend/` in the worktree
- **NestJS global prefix `api`:** All backend endpoints are at `/api/*`. Frontend `NEXT_PUBLIC_API_URL` should be the backend origin (e.g., `http://localhost:3001`) without `/api` suffix — the individual fetch calls include `/api/` in the path
- **TransformInterceptor envelope:** All NestJS responses (except AnalysisController streaming) are wrapped in `{ success: true, data: ... }`. The API client must unwrap this
- **AnalysisController @Res() pattern (D011):** The non-streaming analysis response is manually wrapped as `{ success: true, data: result }` by the controller itself (not by TransformInterceptor). The shape is the same from the frontend's perspective
- **Backend port:** NestJS defaults to port 3000. Frontend Next.js dev also defaults to 3000. Need to run them on different ports (backend:3001 or frontend:3001)

## Common Pitfalls

- **Envelope double-unwrap on AnalysisController** — AnalysisController manually wraps `{ success, data }` via `res.json()`. TransformInterceptor doesn't touch it because `@Res()` bypasses the interceptor. The API client should handle both paths identically since the shape is the same — but don't assume TransformInterceptor wraps analysis responses (it doesn't)
- **JWT token on deploy calls** — The Vibe-Loom deploy flow doesn't use auth. The NestJS deploy endpoint (`POST /api/contracts/deploy`) currently has no `@UseGuards(JwtAuthGuard)`, but the paymaster status check does. The frontend should still send JWT when available for user tracking, but deploy can work without auth currently (the anonymous userId FK issue from S02/S03 summaries applies)
- **`code` vs `source` field name** — Vibe-Loom vibe-score sends `{ code }`, NestJS expects `{ source }`. The DTO has `@IsNotEmpty()` validation — sending `code` will fail with 400. This is the most likely regression if missed
- **WalletConnect project ID** — `@web3modal/wagmi` requires a WalletConnect Cloud projectId. This is a free signup at cloud.walletconnect.com. Must be configured as `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID` env var

## Open Risks

- **Anonymous deploy FK constraint (from S02/S03):** DeployService defaults to `'anonymous'` userId which will fail with Prisma FK violation at runtime. The frontend deploy flow needs either (a) require login before deploy, or (b) the backend needs to handle this. Recommendation: require GitHub login before deploy — this is consistent with D004 (GitHub OAuth for identification) and resolves the FK issue cleanly
- **WalletConnect complexity for M001 scope:** Full WalletConnect integration (signing deploy transactions client-side) requires understanding the Monad network chain config, gas estimation, and transaction construction client-side. If this proves too complex, a simpler "connect wallet + show address" implementation with a note for S06 to complete the relay flow is acceptable

## Skills Discovered

No additional skills needed — this is standard Next.js + React work with Tailwind CSS. The `react-best-practices` and `frontend-design` skills are already available but the scope (API rewiring, no redesign) doesn't require them.

## Sources

- Vibe-Loom source code at `/tmp/vibe-loom/src/` — examined all API routes, components, and page
- NestJS backend at `backend/src/` — examined all controllers, DTOs, and TransformInterceptor
- S02, S03, S04 summaries — API shapes, known limitations, forward intelligence
