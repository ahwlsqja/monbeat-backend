---
id: S03
parent: M001
milestone: M001
provides:
  - AuthModule with GitHub OAuth code-exchange, JWT signing/validation, JwtAuthGuard
  - AnalysisModule with GeminiService (model fallback + streaming), OptimizerService (6 detection patterns), RAG context loading
  - PaymasterModule with deploy-count gating (3 free), signed-tx relay via ethers.js
  - configuration.ts extended with github, jwt, gemini config keys
  - 34 new unit tests (9 auth + 13 analysis + 12 paymaster)
  - 5 monad-docs RAG context files
requires:
  - slice: S01
    provides: PrismaService (User model — githubId, deployCount), ConfigModule, AppModule structure
affects:
  - S05
key_files:
  - backend/src/auth/auth.module.ts
  - backend/src/auth/auth.service.ts
  - backend/src/auth/auth.controller.ts
  - backend/src/auth/jwt.strategy.ts
  - backend/src/auth/jwt-auth.guard.ts
  - backend/src/auth/github.strategy.ts
  - backend/src/analysis/analysis.module.ts
  - backend/src/analysis/analysis.service.ts
  - backend/src/analysis/analysis.controller.ts
  - backend/src/analysis/gemini.service.ts
  - backend/src/analysis/optimizer.service.ts
  - backend/src/analysis/prompt-templates.ts
  - backend/src/analysis/error-handler.ts
  - backend/src/paymaster/paymaster.module.ts
  - backend/src/paymaster/paymaster.service.ts
  - backend/src/paymaster/paymaster.controller.ts
  - backend/src/config/configuration.ts
  - backend/src/app.module.ts
  - backend/data/monad-docs/
key_decisions:
  - AnalysisController uses @Res() for both streaming and non-streaming paths to avoid TransformInterceptor conflict; non-streaming manually wraps in { success, data } envelope
  - GeminiService lazy-initializes GoogleGenerativeAI client on first call; returns null if API key not configured (graceful degradation)
  - PaymasterService creates fresh JsonRpcProvider per broadcastSignedTransaction call (stateless, appropriate for infrequent relay)
  - JwtStrategy secretOrKey uses || 'fallback-secret' as belt-and-suspenders for passport-jwt type requirement
patterns_established:
  - AuthGuard convenience classes (JwtAuthGuard, GithubAuthGuard) wrapping Passport strategies — JwtAuthGuard exported from AuthModule for cross-module use
  - Pure utility modules (prompt-templates.ts, error-handler.ts) exported as plain functions — no @Injectable(), no DI
  - Service pipeline pattern: parse → RAG → prompt → AI → parse → fallback → optimize → persist
  - Controller test pattern: .overrideGuard(JwtAuthGuard) with mock canActivate injecting test user into request
  - OptimizerService as stateless class — can be instantiated directly in tests without TestingModule
observability_surfaces:
  - GET /api/auth/me — returns current user profile from JWT (401 without)
  - GET /api/paymaster/status — returns { used, max: 3, remaining, canUseRelay } for authenticated user
  - POST /api/analysis/error — returns AI analysis + optimization result; ?stream=true for chunked streaming
  - AuthService logs user upsert (githubId, username, userId)
  - GeminiService logs model selection, primary/fallback attempts, and failures
  - PaymasterService logs deploy count checks, relay eligibility, and broadcast results/errors
drill_down_paths:
  - .gsd/milestones/M001/slices/S03/tasks/T01-SUMMARY.md
  - .gsd/milestones/M001/slices/S03/tasks/T02-SUMMARY.md
  - .gsd/milestones/M001/slices/S03/tasks/T03-SUMMARY.md
duration: 28m
verification_result: passed
completed_at: 2026-03-22
---

# S03: Auth + Analysis + Paymaster Module

**Built 3 NestJS modules delivering GitHub OAuth → JWT auth pipeline, Gemini RAG error analysis with streaming + rule-based optimizer fallback, and deploy-count gated paymaster with signed-tx relay; 34 new tests, 68 total passing.**

## What Happened

**T01 (AuthModule):** Installed 8 npm dependencies for the entire slice. Extended `configuration.ts` with `github.clientId`, `github.clientSecret`, `jwt.secret`, and `gemini.apiKey` config keys. Built AuthModule with GitHub OAuth code-exchange flow (controller receives code → exchanges for access_token → fetches user info → Prisma upsert → JWT sign), JWT Passport strategy with Prisma user lookup, and JwtAuthGuard exported for cross-module use. Three endpoints: `GET /auth/github` (OAuth redirect), `GET /auth/github/callback` (token exchange), `GET /auth/me` (JWT-protected profile). 9 unit tests.

**T02 (AnalysisModule):** Copied 5 monad-docs RAG files. Ported prompt-templates.ts and error-handler.ts as pure utility modules. Created GeminiService wrapping `@google/generative-ai` with lazy client initialization, `gemini-2.5-flash` primary + `gemini-pro` fallback, and both synchronous and streaming content generation. Created OptimizerService with 6 detection patterns from Vibe-Loom (loop storage, SLOAD/SSTORE, mapping access, CEI violation, centralized bottleneck, block/tx dependency). Created AnalysisService orchestrating: error parsing → RAG context → prompt building → Gemini AI → JSON parsing (3-layer fallback) → heuristic fallback → optimizer scoring → optional DB persistence. AnalysisController supports JSON and `?stream=true` streaming via `@Res()`. 13 unit tests.

**T03 (PaymasterModule):** Created PaymasterService with deploy-count gating (3 free via `MAX_FREE_DEPLOYMENTS` constant), `getDeployStatus()` returning `{ used, max, remaining, canUseRelay }`, `incrementDeployCount()` via Prisma atomic increment, and `broadcastSignedTransaction()` via ethers.js `JsonRpcProvider.broadcastTransaction()`. PaymasterController exposes 2 JWT-protected endpoints: `GET /api/paymaster/status` and `POST /api/paymaster/relay-signed`. 12 unit tests.

All three modules registered in AppModule. Total test count: 68 (34 existing + 34 new), zero regressions.

## Verification

| # | Check | Result |
|---|-------|--------|
| 1 | `npm run build` — exit 0, zero TypeScript errors | ✅ pass |
| 2 | `npm test` — 68 tests pass across 11 suites | ✅ pass |
| 3 | `npm test -- --testPathPattern=auth` — 9 tests, 2 suites | ✅ pass |
| 4 | `npm test -- --testPathPattern="analysis\|optimizer"` — 13 tests, 2 suites | ✅ pass |
| 5 | `npm test -- --testPathPattern=paymaster` — 12 tests, 2 suites | ✅ pass |
| 6 | AuthModule registered in app.module.ts | ✅ confirmed |
| 7 | AnalysisModule registered in app.module.ts | ✅ confirmed |
| 8 | PaymasterModule registered in app.module.ts | ✅ confirmed |
| 9 | 5 monad-docs RAG files present in backend/data/monad-docs/ | ✅ confirmed |

## New Requirements Surfaced

- none

## Deviations

None. All three tasks executed as planned.

## Known Limitations

- **Anonymous deployment FK constraint:** The existing DeployService defaults to `'anonymous'` userId (discovered in S02, documented in KNOWLEDGE.md). This will fail at runtime with a Prisma FK violation. Deploy endpoints should require auth or userId should be made nullable — this must be resolved when S05 wires the frontend to the deploy flow.
- **GeminiService requires API key for AI analysis:** Without `GEMINI_API_KEY`, GeminiService returns null and AnalysisService falls back to heuristic-only analysis. This is by design (graceful degradation) but means the AI-powered fix suggestions won't work until the key is configured.
- **broadcastSignedTransaction creates a fresh provider per call:** No connection pooling. Acceptable for infrequent relay use but would need optimization if relay volume increases.

## Follow-ups

- S05 must decide how to handle the anonymous userId FK constraint — either require auth on deploy endpoints, seed an anonymous user, or make userId nullable.
- S06 must ensure `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `JWT_SECRET`, and `GEMINI_API_KEY` are configured in Railway environment variables.

## Files Created/Modified

- `backend/package.json` — 8 npm dependencies added (6 prod + 2 dev)
- `backend/src/config/configuration.ts` — extended with github, jwt, gemini config sections
- `backend/src/app.module.ts` — added AuthModule, AnalysisModule, PaymasterModule imports
- `backend/src/auth/auth.module.ts` — AuthModule with PassportModule + JwtModule
- `backend/src/auth/auth.service.ts` — validateOrCreateUser + login
- `backend/src/auth/auth.controller.ts` — 3 auth endpoints
- `backend/src/auth/jwt.strategy.ts` — JWT Passport strategy
- `backend/src/auth/jwt-auth.guard.ts` — reusable JWT guard (exported)
- `backend/src/auth/github.strategy.ts` — GitHub OAuth Passport strategy
- `backend/src/auth/github-auth.guard.ts` — GitHub guard
- `backend/src/auth/dto/auth-response.dto.ts` — auth DTOs
- `backend/src/analysis/analysis.module.ts` — AnalysisModule
- `backend/src/analysis/analysis.service.ts` — orchestration service
- `backend/src/analysis/analysis.controller.ts` — JSON + streaming controller
- `backend/src/analysis/gemini.service.ts` — Gemini AI wrapper
- `backend/src/analysis/optimizer.service.ts` — 6-pattern Monad parallelism scorer
- `backend/src/analysis/prompt-templates.ts` — prompt builder functions
- `backend/src/analysis/error-handler.ts` — error parsing utilities
- `backend/src/analysis/dto/analysis-request.dto.ts` — request DTO
- `backend/src/analysis/dto/analysis-response.dto.ts` — response interface
- `backend/src/paymaster/paymaster.module.ts` — PaymasterModule
- `backend/src/paymaster/paymaster.service.ts` — deploy gating + tx relay
- `backend/src/paymaster/paymaster.controller.ts` — 2 JWT-protected endpoints
- `backend/src/paymaster/dto/deploy-status.dto.ts` — deploy status DTO
- `backend/src/paymaster/dto/relay-signed.dto.ts` — relay request DTO
- `backend/data/monad-docs/00-index.md` — RAG context
- `backend/data/monad-docs/01-parallel-execution.md` — RAG context
- `backend/data/monad-docs/02-rpc-and-gas.md` — RAG context
- `backend/data/monad-docs/03-evm-and-pectra.md` — RAG context
- `backend/data/monad-docs/04-consensus-and-mempool.md` — RAG context
- `backend/test/auth.service.spec.ts` — 5 tests
- `backend/test/auth.controller.spec.ts` — 4 tests
- `backend/test/analysis.service.spec.ts` — 7 tests
- `backend/test/optimizer.service.spec.ts` — 6 tests
- `backend/test/paymaster.service.spec.ts` — 10 tests
- `backend/test/paymaster.controller.spec.ts` — 2 tests

## Forward Intelligence

### What the next slice should know
- JwtAuthGuard is exported from AuthModule and imported by PaymasterModule — any module needing auth protection should import AuthModule and use `@UseGuards(JwtAuthGuard)`.
- The JWT payload contains `{ sub: userId, githubId, username }`. Access via `req.user.id` (sub mapped to id by JwtStrategy), `req.user.githubId`, `req.user.username`.
- `configuration.ts` now has 4 config sections: `github`, `jwt`, `gemini`, `monad`. Access via `configService.get('github.clientId')` etc.
- PaymasterService.canUseRelay() and incrementDeployCount() are exported — S05 deploy flow should call canUseRelay before server-wallet deploy, and incrementDeployCount after successful deploy.
- AnalysisService.analyzeError() accepts `{ error, contractSource, errorCode?, userId? }` — the userId is optional and used only for DB persistence.

### What's fragile
- **@Res() in AnalysisController** — bypasses NestJS TransformInterceptor. The non-streaming path manually wraps in `{ success, data }` envelope. If the response format changes globally, this controller won't pick it up automatically.
- **GeminiService AI JSON parsing** — has 3-layer fallback (code-block → regex → heuristic) but relies on the Gemini model returning JSON in a specific structure. Model updates could break parsing.

### Authoritative diagnostics
- `GET /api/auth/me` with valid JWT — confirms auth pipeline works end-to-end (JWT signing + validation + Prisma user lookup)
- `GET /api/paymaster/status` with valid JWT — confirms paymaster gating logic and user deploy count
- `npm test -- --testPathPattern="auth|analysis|optimizer|paymaster"` — 34 tests covering all S03 services and controllers

### What assumptions changed
- No assumptions changed — all three modules built as planned with no blockers or surprises.
