# S05: Frontend Integration — UAT

**Milestone:** M001
**Written:** 2026-03-22

## UAT Type

- UAT mode: mixed (artifact-driven for build/test verification, live-runtime for user flow)
- Why this mode is sufficient: Build and unit tests verify the API contract and type safety. Runtime testing against a live NestJS backend validates the complete user flow (deferred to S06 E2E, but manual UAT steps provided here for when backend is available).

## Preconditions

- `cd frontend && npm install` has been run
- For artifact-driven tests: no backend needed
- For runtime tests (marked with 🔴): NestJS backend must be running on `http://localhost:3000` with valid environment variables (DATABASE_URL, GITHUB_CLIENT_ID/SECRET, JWT_SECRET, GEMINI_API_KEY, MONAD_RPC_URL)
- `NEXT_PUBLIC_API_URL=http://localhost:3000` set in `frontend/.env.local`
- For WalletConnect tests: `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID` set (or MetaMask browser extension installed for injected wallet tests)

## Smoke Test

```bash
cd frontend && npm run build && npm test
```
Build exits 0 with zero TypeScript errors. 11 unit tests pass.

## Test Cases

### 1. Frontend builds without errors

1. `cd frontend && rm -rf .next`
2. `npm run build`
3. **Expected:** Exit code 0. Output shows "Generating static pages (4/4)" and route table with `/` page.

### 2. All API calls use environment-based base URL

1. `grep -r "NEXT_PUBLIC_API_URL" frontend/src/`
2. **Expected:** Matches in `api-client.ts` (2 occurrences) and `auth-context.tsx` (1 occurrence). No other files contain hardcoded API URLs.

### 3. No hardcoded /api/ fetch calls remain

1. `grep -rn 'fetch("/api/' frontend/src/`
2. **Expected:** No matches (exit code 1). All fetch calls go through api-client.ts.

### 4. API client envelope unwrapping works correctly

1. `cd frontend && npm test -- --testNamePattern "envelope"`
2. **Expected:** 3 tests pass: unwraps `{ success: true, data }`, throws on `{ success: false }`, throws on non-ok HTTP.

### 5. API client field mappings are correct

1. `cd frontend && npm test -- --testNamePattern "getContractSource|deployContract|getVibeScore|analyzeError|compileContract|getDeployStatus"`
2. **Expected:** 6 tests pass. Each test verifies the correct URL path and request body field names:
   - `getContractSource` → `GET /api/contracts/source?type=...`
   - `deployContract` → `POST /api/contracts/deploy` with `{ source }` (not `{ code }`)
   - `getVibeScore` → `POST /api/vibe-score` with `{ source }`, returns `vibeScore` (not `score`)
   - `analyzeError` → `POST /api/analysis/error` with `{ error, contractSource }`
   - `compileContract` → `POST /api/contracts/compile` with `{ source }`
   - `getDeployStatus` → `GET /api/paymaster/status` with auth header

### 6. Auth header injection from localStorage

1. `cd frontend && npm test -- --testNamePattern "auth header"`
2. **Expected:** 2 tests pass:
   - When `localStorage` has `vibe-loom-token`, fetch receives `Authorization: Bearer <token>` header
   - When `localStorage` is empty, no `Authorization` header is sent

### 7. Backend tests pass with OAuth redirect change

1. `cd backend && npm test`
2. **Expected:** 14 suites, 97 tests pass. Auth controller tests include redirect behavior and fallback URL.

### 8. 🔴 GitHub OAuth login flow (runtime)

1. Open `http://localhost:3001` in browser
2. Click "GitHub 로그인" button in header
3. **Expected:** Redirects to GitHub OAuth authorization page
4. Authorize the app on GitHub
5. **Expected:** Redirected back to `http://localhost:3001?token=<jwt>`. Page shows username in header. Login button changes to logout button.
6. Open DevTools → Application → Local Storage
7. **Expected:** `vibe-loom-token` key present with JWT value

### 9. 🔴 Contract source loading (runtime)

1. With backend running, open `http://localhost:3001`
2. Select a contract type from the dropdown (e.g., "FixedContract")
3. **Expected:** Contract source code loads and displays in the code area
4. Open DevTools → Network tab
5. **Expected:** `GET http://localhost:3000/api/contracts/source?type=FixedContract` returns 200 with `{ success: true, data: { source: "..." } }`

### 10. 🔴 Vibe-Score analysis (runtime)

1. With contract source loaded, click "Vibe-Score 분석" button
2. **Expected:** Vibe-Score gauge shows a numeric score. Analysis details appear.
3. Check Network tab
4. **Expected:** `POST http://localhost:3000/api/vibe-score` with `{ source: "..." }` body. Response contains `vibeScore` field.

### 11. 🔴 Deploy with paymaster (runtime, authenticated)

1. Log in via GitHub OAuth
2. Click "배포" (Deploy) button
3. **Expected:** If deployCount < 3, server-side deploy proceeds. Network shows `POST /api/contracts/deploy`.
4. **Expected:** If deployCount >= 3, WalletConnectModal opens instead of deploying.

### 12. 🔴 WalletConnect modal appears on 3+ deploys (runtime)

1. Log in and deploy 3 contracts successfully (or mock paymaster status)
2. Click "배포" on 4th attempt
3. **Expected:** WalletConnectModal opens showing available wallet connectors (MetaMask, WalletConnect QR)
4. Click a connector
5. **Expected:** Wallet connection prompt appears. After connecting, modal shows truncated wallet address and "Deploy with Wallet" button.

### 13. 🔴 Error analysis with AI suggestion (runtime)

1. Deploy a contract that will fail (e.g., invalid Solidity)
2. **Expected:** Error message appears with AI-suggested fix
3. Check Network tab
4. **Expected:** `POST http://localhost:3000/api/analysis/error` with error details. Response contains `fixedCode` and `explanation`.

## Edge Cases

### Missing NEXT_PUBLIC_API_URL

1. Remove `NEXT_PUBLIC_API_URL` from `.env.local`
2. `npm run build && npm start`
3. Try loading a contract
4. **Expected:** API calls fall back to `http://localhost:3000` (default in api-client.ts). Works if backend is on that port.

### Missing WalletConnect Project ID

1. Ensure `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID` is NOT set
2. Build and open the WalletConnectModal
3. **Expected:** Only injected wallet connectors shown (MetaMask, etc.). `console.warn` message: `[wagmi-config] No WalletConnect project ID...`. No crash.

### Unauthenticated deploy status check

1. Without logging in, trigger deploy flow
2. **Expected:** `getDeployStatus()` is skipped or returns error (no JWT token). Deploy proceeds as server-side (paymaster assumes free tier without auth).

### Auth token expiry

1. Set an invalid/expired JWT in localStorage key `vibe-loom-token`
2. Reload page
3. **Expected:** `GET /api/auth/me` returns 401. Auth context clears token and shows logged-out state. No infinite retry loop.

### Network failure during API call

1. Stop the NestJS backend
2. Try loading a contract source
3. **Expected:** `console.error('[api-client] /api/contracts/source failed: ...')` logged. UI shows error state, not blank/frozen page.

## Failure Signals

- `npm run build` exits non-zero — TypeScript compilation errors or missing dependencies
- `npm test` fails — API contract mismatch between frontend expectations and backend response shape
- Network tab shows requests to wrong URLs (e.g., `/api/` without base URL prefix)
- `fetch("/api/` appears in source — hardcoded paths bypassing api-client
- WalletConnectModal crashes on open — wagmi config or provider wrapping issue
- Login button click does nothing — auth-context or OAuth redirect misconfigured
- After OAuth callback, token not stored — `auth-context.tsx` URL param parsing broken

## Not Proven By This UAT

- Full end-to-end flow on Railway deployment (S06 scope)
- CORS behavior between separate frontend/backend origins (S06 CORS config)
- Real Monad testnet deployment via WalletConnect wallet (requires funded wallet + testnet access)
- Gemini AI streaming responses in the error analysis UI (backend streaming is tested in S03, but frontend SSE/streaming display is not yet wired)
- Mobile responsive layout (R016 is out of scope)

## Notes for Tester

- Artifact-driven tests (1-7) can be run entirely without a backend — they verify build, types, and API client contracts via Jest mocks.
- Runtime tests (8-13, marked 🔴) require both `backend` running on port 3000 and `frontend` on port 3001.
- The WalletConnect QR code test requires a mobile wallet app. If testing in development, MetaMask browser extension is sufficient for the injected wallet path.
- Backend test count increased from 92 (S04) to 97 (S05) due to new auth controller redirect tests — this is expected, not a regression.
- The `console.error` and `console.warn` messages are intentional observability surfaces, not bugs.
