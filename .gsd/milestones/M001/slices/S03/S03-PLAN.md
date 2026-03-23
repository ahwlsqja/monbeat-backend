# S03: Auth + Analysis + Paymaster Module

**Goal:** GitHub OAuth login → JWT issuance → deploy status tracking (3-deploy limit), Gemini RAG error analysis API with streaming, WalletConnect signed-tx relay — all wired into AppModule with JwtAuthGuard protecting paymaster endpoints.
**Demo:** `POST /api/auth/github/callback` with GitHub code → returns JWT. `GET /api/paymaster/status` with JWT → returns `{ used, max: 3, remaining, canUseRelay }`. `POST /api/analysis/error` with error + contract source → returns AI-generated fix with explanation. `POST /api/paymaster/relay-signed` with signed tx hex → broadcasts to Monad testnet.

## Must-Haves

- GitHub OAuth code-exchange endpoint (`POST /api/auth/github/callback`) that upserts User and returns JWT
- JWT strategy + JwtAuthGuard protecting paymaster endpoints
- `GET /api/auth/me` returning current user profile from JWT
- `POST /api/analysis/error` with Gemini AI RAG analysis using monad-docs context, streaming support via `?stream=true`
- Rule-based optimizer (`calculateMonadParallelismScore`) as AI fallback
- `GET /api/paymaster/status` returning deploy count, remaining free deploys, relay eligibility
- `POST /api/paymaster/relay-signed` broadcasting user-signed transaction via ethers.js
- Configuration factory extended with `github.clientId`, `github.clientSecret`, `jwt.secret`, `gemini.apiKey`
- Unit tests for all three modules (≥15 new tests total)

## Proof Level

- This slice proves: contract (module boundaries, guards, service logic)
- Real runtime required: no (all external services mocked in tests; GitHub OAuth App + Gemini API key needed for runtime)
- Human/UAT required: no

## Verification

- `cd backend && npm run build` — exit 0, zero TypeScript errors
- `cd backend && npm test` — all suites pass (≥49 total: 34 existing + ≥15 new)
- `cd backend && npm test -- --testPathPattern=auth` — auth suite passes
- `cd backend && npm test -- --testPathPattern=analysis` — analysis suite passes
- `cd backend && npm test -- --testPathPattern=paymaster` — paymaster suite passes
- `grep -q "AuthModule" backend/src/app.module.ts` — AuthModule registered
- `grep -q "AnalysisModule" backend/src/app.module.ts` — AnalysisModule registered
- `grep -q "PaymasterModule" backend/src/app.module.ts` — PaymasterModule registered
- `test -d backend/data/monad-docs && ls backend/data/monad-docs/*.md | wc -l` — 5 RAG files present

## Observability / Diagnostics

- Runtime signals: AuthService logs user upsert (githubId, username); GeminiService logs model selection and fallback; PaymasterService logs deploy count checks and relay broadcasts; AnalysisService logs RAG context loading and AI call results
- Inspection surfaces: `GET /api/auth/me` (user profile from JWT), `GET /api/paymaster/status` (deploy count), User table (deployCount field), Analysis table (error analysis records)
- Failure visibility: 401 Unauthorized on missing/invalid JWT (JwtAuthGuard), GeminiService falls back to rule-based heuristics on API failure (logged), PaymasterService relay broadcast errors include RPC error message
- Redaction constraints: JWT_SECRET, GITHUB_CLIENT_SECRET, GEMINI_API_KEY never logged; GitHub access_token discarded after user info fetch

## Integration Closure

- Upstream surfaces consumed: `backend/src/prisma/prisma.service.ts` (User model — githubId, deployCount), `backend/src/config/configuration.ts` (extended with auth/gemini config keys), `backend/src/contracts/deploy.service.ts` (userId parameter for Paymaster relay)
- New wiring introduced in this slice: AuthModule, AnalysisModule, PaymasterModule registered in AppModule; JwtAuthGuard available as reusable guard; configuration.ts extended with 4 new config keys
- What remains before the milestone is truly usable end-to-end: S04 (engine bridge + vibe-score), S05 (frontend integration), S06 (Railway deploy)

## Tasks

- [x] **T01: Build AuthModule with GitHub OAuth, JWT strategies, and JwtAuthGuard** `est:45m`
  - Why: Auth is foundational — JwtAuthGuard is consumed by Paymaster endpoints and all future protected routes. Configuration extension (github, jwt, gemini keys) is shared infrastructure for the entire slice.
  - Files: `backend/src/auth/auth.module.ts`, `backend/src/auth/auth.service.ts`, `backend/src/auth/auth.controller.ts`, `backend/src/auth/github.strategy.ts`, `backend/src/auth/jwt.strategy.ts`, `backend/src/auth/jwt-auth.guard.ts`, `backend/src/auth/dto/auth-response.dto.ts`, `backend/src/config/configuration.ts`, `backend/src/app.module.ts`, `backend/test/auth.service.spec.ts`, `backend/test/auth.controller.spec.ts`
  - Do: Install all S03 npm deps (`@nestjs/passport`, `@nestjs/jwt`, `passport`, `passport-github2`, `passport-jwt`, `@google/generative-ai`, `@types/passport-github2`, `@types/passport-jwt`). Extend `configuration.ts` with `github: { clientId, clientSecret }`, `jwt: { secret }`, `gemini: { apiKey }`. Implement GitHub OAuth code-exchange flow (controller receives code, exchanges via HTTP for access_token + user info, upserts User in Prisma, signs JWT). Implement JWT strategy extracting Bearer token. Create JwtAuthGuard. Expose `GET /api/auth/github` (redirect), `GET /api/auth/github/callback?code=...` (exchange → JWT), `GET /api/auth/me` (JWT-protected). Register AuthModule in AppModule.
  - Verify: `cd backend && npm run build && npm test -- --testPathPattern=auth`
  - Done when: AuthModule registered in AppModule, `npm run build` clean, auth tests pass with ≥6 tests

- [x] **T02: Build AnalysisModule with Gemini RAG error analysis, optimizer, and streaming** `est:45m`
  - Why: R004 — the core AI-powered error analysis pipeline. Ports Vibe-Loom's analysis logic (ai.ts, prompt-templates.ts, deployment-error-handler.ts, optimizer.ts) into NestJS services with Gemini RAG using monad-docs context.
  - Files: `backend/src/analysis/analysis.module.ts`, `backend/src/analysis/analysis.service.ts`, `backend/src/analysis/analysis.controller.ts`, `backend/src/analysis/gemini.service.ts`, `backend/src/analysis/optimizer.service.ts`, `backend/src/analysis/prompt-templates.ts`, `backend/src/analysis/error-handler.ts`, `backend/src/analysis/dto/analysis-request.dto.ts`, `backend/src/analysis/dto/analysis-response.dto.ts`, `backend/data/monad-docs/`, `backend/test/analysis.service.spec.ts`, `backend/test/optimizer.service.spec.ts`
  - Do: Copy 5 monad-docs .md files from `/tmp/vibe-loom/data/monad-docs/` to `backend/data/monad-docs/`. Port `prompt-templates.ts` (prompts + builder functions as pure module). Port `error-handler.ts` (parseProviderError, handleDeploymentError). Create GeminiService wrapping `@google/generative-ai` with ConfigService injection, model fallback, generateContent + generateContentStream. Create OptimizerService porting `calculateMonadParallelismScore()`. Create AnalysisService orchestrating: load RAG context → build prompt → call Gemini → parse JSON → fallback heuristics → return analysis with optimizer result. Create AnalysisController with `POST /api/analysis/error` (accepts `{ error, contractSource, errorCode }`). Support streaming via `?stream=true` using `@Res()` with chunked transfer. Register AnalysisModule in AppModule.
  - Verify: `cd backend && npm run build && npm test -- --testPathPattern="analysis|optimizer"`
  - Done when: AnalysisModule registered in AppModule, `npm run build` clean, analysis + optimizer tests pass with ≥6 tests, monad-docs files present in `backend/data/monad-docs/`

- [x] **T03: Build PaymasterModule with deploy-count gating and signed-tx relay** `est:30m`
  - Why: R008 + R013 — deploy count tracking per user (3 free via server wallet relay, then WalletConnect), and POST /api/paymaster/relay-signed for broadcasting user-signed transactions. Completes the Paymaster flow.
  - Files: `backend/src/paymaster/paymaster.module.ts`, `backend/src/paymaster/paymaster.service.ts`, `backend/src/paymaster/paymaster.controller.ts`, `backend/src/paymaster/dto/deploy-status.dto.ts`, `backend/src/paymaster/dto/relay-signed.dto.ts`, `backend/test/paymaster.service.spec.ts`, `backend/test/paymaster.controller.spec.ts`
  - Do: Create PaymasterService with: `getDeployStatus(userId)` querying User.deployCount from Prisma, `canUseRelay(userId)` returning deployCount < 3, `incrementDeployCount(userId)` updating User record, `broadcastSignedTransaction(signedTxHex)` using ethers.js `JsonRpcProvider.broadcastTransaction()`. Create PaymasterController with: `GET /api/paymaster/status` (JWT-protected via JwtAuthGuard, returns `{ used, max, remaining, canUseRelay }`), `POST /api/paymaster/relay-signed` (JWT-protected, receives `{ signedTransaction }` hex string, broadcasts via PaymasterService). Create DTOs with class-validator. Register PaymasterModule in AppModule.
  - Verify: `cd backend && npm run build && npm test -- --testPathPattern=paymaster`
  - Done when: PaymasterModule registered in AppModule, `npm run build` clean, paymaster tests pass with ≥5 tests, `GET /api/paymaster/status` requires JWT (401 without)

## Files Likely Touched

- `backend/package.json` — new npm dependencies
- `backend/src/config/configuration.ts` — extended with github, jwt, gemini config keys
- `backend/src/app.module.ts` — import AuthModule, AnalysisModule, PaymasterModule
- `backend/src/auth/auth.module.ts`
- `backend/src/auth/auth.service.ts`
- `backend/src/auth/auth.controller.ts`
- `backend/src/auth/github.strategy.ts`
- `backend/src/auth/jwt.strategy.ts`
- `backend/src/auth/jwt-auth.guard.ts`
- `backend/src/auth/dto/auth-response.dto.ts`
- `backend/src/analysis/analysis.module.ts`
- `backend/src/analysis/analysis.service.ts`
- `backend/src/analysis/analysis.controller.ts`
- `backend/src/analysis/gemini.service.ts`
- `backend/src/analysis/optimizer.service.ts`
- `backend/src/analysis/prompt-templates.ts`
- `backend/src/analysis/error-handler.ts`
- `backend/src/analysis/dto/analysis-request.dto.ts`
- `backend/src/analysis/dto/analysis-response.dto.ts`
- `backend/src/paymaster/paymaster.module.ts`
- `backend/src/paymaster/paymaster.service.ts`
- `backend/src/paymaster/paymaster.controller.ts`
- `backend/src/paymaster/dto/deploy-status.dto.ts`
- `backend/src/paymaster/dto/relay-signed.dto.ts`
- `backend/data/monad-docs/*.md`
- `backend/test/auth.service.spec.ts`
- `backend/test/auth.controller.spec.ts`
- `backend/test/analysis.service.spec.ts`
- `backend/test/optimizer.service.spec.ts`
- `backend/test/paymaster.service.spec.ts`
- `backend/test/paymaster.controller.spec.ts`
