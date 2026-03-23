---
estimated_steps: 7
estimated_files: 7
---

# T02: Create API client + auth context and rewire all API calls to NestJS backend

**Slice:** S05 — Frontend Integration
**Milestone:** M001

## Description

This is the core R009 delivery. Create a centralized API client that handles base URL, JWT auth headers, and NestJS response envelope unwrapping. Create a React auth context for JWT token lifecycle (GitHub OAuth redirect → callback → token storage). Rewrite all 5 fetch calls in page.tsx and VibeStatus.tsx to use the API client with correct NestJS paths and field name mappings. Modify the backend OAuth callback to redirect to the frontend with a token parameter instead of returning JSON.

**Critical field mappings (from S05-RESEARCH API Mapping Table):**
- Contract source: `GET /api/contract-source?type=X` → `GET /api/contracts/source?type=X`. Response: unwrap `data.source` from `{ success, data: { contractType, source } }`
- Deploy: `POST /api/deploy` body `{ contractType, contractSource }` → `POST /api/contracts/deploy` body `{ source }`. Response: unwrap `data.address` from `{ success, data: { contractName, address, txHash, deploymentId } }`
- Error analysis: `POST /api/analyze-deployment-error` body `{ error, contractSource, contractCode }` → `POST /api/analysis/error` body `{ error, contractSource }`. Response: unwrap from `{ success, data: { analysis: { summary, fixedCode, explanation }, optimization } }` — map `fixedCode` → where the old code used `fixedSnippet`, `explanation` → where old code used `summary`
- Vibe-score: `POST /api/vibe-score` body `{ code }` → `POST /api/vibe-score` body `{ source }`. Response: unwrap `data.vibeScore` from `{ success, data: { vibeScore, suggestions, engineBased, ... } }`
- Deploy status: `GET /api/deploy-status?githubId=X` → `GET /api/paymaster/status` (JWT auth, no query param). Response: unwrap from `{ success, data: { used, max, remaining, canUseRelay } }`

## Steps

1. Create `frontend/src/lib/api-client.ts`:
   - `const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000'`
   - Helper `getAuthHeaders()`: reads JWT token from localStorage key `vibe-loom-token`, returns `{ Authorization: 'Bearer <token>' }` if present, empty object otherwise
   - Helper `unwrapResponse<T>(res: Response): Promise<T>`: checks `res.ok`, parses JSON, unwraps `{ success, data }` envelope, throws on `!success` or `!res.ok` with error message
   - `getContractSource(type: string): Promise<{ contractType: string; source: string }>` — GET `/api/contracts/source?type=${type}`
   - `deployContract(source: string): Promise<{ contractName: string; address: string; txHash: string; deploymentId: string }>` — POST `/api/contracts/deploy` body `{ source }`, includes auth headers
   - `analyzeError(error: any, contractSource: string): Promise<AnalysisResult>` — POST `/api/analysis/error` body `{ error: String(error), contractSource }`, returns the analysis result shape
   - `getVibeScore(source: string): Promise<VibeScoreResult>` — POST `/api/vibe-score` body `{ source }`
   - `getDeployStatus(): Promise<DeployStatus>` — GET `/api/paymaster/status` with auth headers
   - `getUserProfile(): Promise<UserProfile>` — GET `/api/auth/me` with auth headers
   - Export TypeScript interfaces for all response types

2. Create `frontend/src/lib/auth-context.tsx`:
   - `AuthProvider` React context wrapping children
   - State: `token: string | null`, `user: { id, githubId, username } | null`, `isAuthenticated: boolean`
   - On mount: read `vibe-loom-token` from localStorage; if present, call `getUserProfile()` to validate and load user
   - On mount: check URL params for `?token=xxx` (from OAuth callback redirect); if present, save to localStorage, remove from URL, load user profile
   - `login()`: redirect to `${API_BASE}/api/auth/github`
   - `logout()`: remove token from localStorage, clear user state
   - Export `useAuth()` hook

3. Modify `frontend/src/app/layout.tsx`:
   - Import and wrap children with `<AuthProvider>`
   - Keep existing metadata (title, description)

4. Rewrite `frontend/src/app/page.tsx`:
   - Import `{ getContractSource, deployContract, analyzeError, getVibeScore }` from `@/lib/api-client`
   - Import `{ useAuth }` from `@/lib/auth-context`
   - Replace `loadContractSource` fetch: use `getContractSource(type)` → set `data.source`
   - Replace `handleDeploy` fetch: use `deployContract(contractSource)` → set address from response. Error analysis: use `analyzeError(error, contractSource)` → map `analysis.fixedCode` to `errorDiff.fixed`, `analysis.explanation` to `errorDiff.summary`, `contractSource` to `errorDiff.original`
   - Replace `handleAnalyzeVibeScore` fetch: use `getVibeScore(contractSource)` → set score from `data.vibeScore`, suggestions from `data.suggestions`
   - Add login/logout button in the header area: show username if authenticated, login button if not

5. Rewrite `frontend/src/components/VibeStatus.tsx`:
   - Import `{ getDeployStatus }` from `@/lib/api-client` and `{ useAuth }` from `@/lib/auth-context`
   - Remove `githubId` prop entirely — deploy status comes from JWT-authenticated endpoint
   - If not authenticated: show "로그인 필요" message
   - If authenticated: call `getDeployStatus()` on mount → set status
   - Remove `githubId` prop from the parent call site in `page.tsx`

6. Modify `backend/src/auth/auth.controller.ts` — change `githubCallback` method:
   - Instead of returning `this.authService.login(req.user)` as JSON, redirect to the frontend:
   ```typescript
   @Get('github/callback')
   @UseGuards(GithubAuthGuard)
   async githubCallback(@Req() req: any, @Res() res: Response) {
     const { accessToken } = await this.authService.login(req.user);
     const frontendUrl = this.configService.get<string>('frontend.url') || 'http://localhost:3001';
     res.redirect(`${frontendUrl}?token=${accessToken}`);
   }
   ```
   - Add `ConfigService` injection to the controller constructor
   - Add `@Res()` import and `Response` type from express

7. Add `frontend.url` to `backend/src/config/configuration.ts`:
   ```typescript
   frontend: { url: process.env.FRONTEND_URL || 'http://localhost:3001' }
   ```

## Must-Haves

- [ ] `api-client.ts` handles base URL, auth headers, and envelope unwrapping
- [ ] All 5 API calls use correct NestJS paths and field names
- [ ] `auth-context.tsx` manages JWT token lifecycle (localStorage, URL params, profile fetch)
- [ ] Login/logout button visible in page header
- [ ] VibeStatus no longer takes githubId prop — uses JWT auth
- [ ] Backend OAuth callback redirects to frontend with token
- [ ] `cd frontend && npm run build` exits 0
- [ ] `cd backend && npm run build` exits 0

## Verification

- `cd frontend && npm run build` exits 0
- `cd backend && npm run build && npm test` — build succeeds and existing tests pass
- `grep -r "NEXT_PUBLIC_API_URL" frontend/src/lib/api-client.ts` — confirms env var usage
- `! grep -rn "fetch(\"/api/" frontend/src/` — no direct `/api/` fetch calls remain in page.tsx or components

## Observability Impact

- Signals added/changed: API client logs failed requests to `console.error` with endpoint and status code
- How a future agent inspects this: Check `frontend/src/lib/api-client.ts` for all endpoint definitions; check `localStorage.getItem('vibe-loom-token')` in browser for auth state
- Failure state exposed: `unwrapResponse` throws descriptive error with HTTP status and endpoint name

## Inputs

- `frontend/src/app/page.tsx` — page component to rewrite API calls (from T01)
- `frontend/src/app/layout.tsx` — layout to wrap with AuthProvider (from T01)
- `frontend/src/components/VibeStatus.tsx` — component to rewrite API call (from T01)
- `backend/src/auth/auth.controller.ts` — OAuth callback to modify for redirect
- `backend/src/config/configuration.ts` — config to extend with frontend.url

## Expected Output

- `frontend/src/lib/api-client.ts` — centralized API client with all 5 endpoint methods
- `frontend/src/lib/auth-context.tsx` — React auth context with JWT management
- `frontend/src/app/page.tsx` — rewritten with api-client calls and login button
- `frontend/src/app/layout.tsx` — wrapped with AuthProvider
- `frontend/src/components/VibeStatus.tsx` — rewritten with api-client and auth
- `backend/src/auth/auth.controller.ts` — OAuth callback redirects to frontend
- `backend/src/config/configuration.ts` — extended with frontend.url config
