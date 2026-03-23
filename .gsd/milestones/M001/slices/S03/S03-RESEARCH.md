# S03: Auth + Analysis + Paymaster Module — Research

**Date:** 2026-03-22
**Depth:** Targeted

## Summary

S03 adds three NestJS modules — **AuthModule** (GitHub OAuth → JWT), **AnalysisModule** (Gemini AI RAG error analysis), and **PaymasterModule** (3 free deploys + WalletConnect relay) — to the existing backend scaffold from S01. All three are well-understood patterns: NestJS passport auth, Gemini API call with RAG context, and deploy-count gating with DB storage. The primary complexity is integration wiring, not novel technology.

The Vibe-Loom codebase (`/tmp/vibe-loom/src/lib/`) provides complete reference implementations for all three areas: `ai.ts` (Gemini client), `prompt-templates.ts` (error analysis + RAG prompts), `deployment-error-handler.ts` (error parsing), `optimizer.ts` (rule-based parallelism scoring), `paymaster.ts` (deploy count logic), and `deploy-count-store.ts` (file-based storage → now Prisma). The migration is mostly restructuring existing logic into NestJS services and controllers.

Key architectural choice: GitHub OAuth will use a **code-exchange pattern** (frontend redirects to GitHub, gets code, sends to `POST /api/auth/github/callback` with the code, backend exchanges it for access_token and user info via HTTP, upserts User, returns JWT). This is simpler than passport redirect-based flow for a separate SPA frontend and avoids server-rendered redirect complexity. However, we will also provide a `GET /api/auth/github` redirect endpoint for flexibility. We'll use `@nestjs/passport` + `@nestjs/jwt` + `passport-github2` + `passport-jwt` — standard NestJS authentication stack.

## Recommendation

Build the three modules in this order: **Auth → Paymaster → Analysis**. Auth is foundational (JwtAuthGuard is used by Paymaster endpoints). Paymaster depends on Auth (needs userId from JWT for deploy-count tracking). Analysis is independent of Auth/Paymaster and can be built last. Each module is self-contained with its own controller, service(s), DTOs, and tests.

Use `passport-github2` for the GitHub OAuth strategy and `passport-jwt` for JWT validation. For Gemini AI, use `@google/generative-ai` (same as Vibe-Loom). For WalletConnect relay, no server-side WalletConnect SDK is needed — the frontend signs the transaction, the backend receives the raw signed transaction hex and broadcasts it via `ethers.js` `provider.broadcastTransaction()`.

The monad-docs RAG files (~5KB total, 5 .md files) should be copied into the backend project at `backend/data/monad-docs/` and loaded at service initialization.

## Implementation Landscape

### Key Files — Existing (to consume)

- `backend/src/prisma/prisma.service.ts` — @Global PrismaService, inject anywhere. User model has `githubId` (unique), `username`, `email`, `avatarUrl`, `deployCount` (int, default 0)
- `backend/src/config/configuration.ts` — Currently exports `port`, `database.url`, `nodeEnv`, `monad.rpcUrl`, `monad.privateKey`. **Needs extension** for `github.clientId`, `github.clientSecret`, `jwt.secret`, `gemini.apiKey`
- `backend/src/contracts/deploy.service.ts` — `deploy(source, userId?)` with `userId` defaulting to `'anonymous'`. S03 Paymaster will call this with real userId from JWT. The KNOWLEDGE.md notes `userId FK constraint blocks anonymous deployments` — S03 must handle this (either seed anonymous user or require auth for deploy)
- `backend/src/app.module.ts` — Must import AuthModule, AnalysisModule, PaymasterModule
- `backend/src/main.ts` — Global pipes/filters/interceptors already registered. TransformInterceptor auto-wraps responses

### Key Files — Vibe-Loom Source (to port)

- `/tmp/vibe-loom/src/lib/ai.ts` → `backend/src/analysis/gemini.service.ts` — Gemini client singleton, model selection with fallback
- `/tmp/vibe-loom/src/lib/prompt-templates.ts` → `backend/src/analysis/prompt-templates.ts` — System prompts (DEPLOYMENT_ERROR_ANALYSIS_PROMPT, RAG_FIX_REQUEST_PROMPT), keyword arrays, `isLikelyMonadSpecificError()`, `buildRagFixPrompt()`, `buildErrorAnalysisPrompt()`
- `/tmp/vibe-loom/src/lib/deployment-error-handler.ts` → `backend/src/analysis/error-handler.ts` — `parseProviderError()`, `handleDeploymentError()`
- `/tmp/vibe-loom/src/lib/optimizer.ts` → `backend/src/analysis/optimizer.service.ts` — `calculateMonadParallelismScore()` (rule-based, ~200 lines of regex pattern matching)
- `/tmp/vibe-loom/src/lib/paymaster.ts` → `backend/src/paymaster/paymaster.service.ts` — `MAX_FREE_DEPLOYMENTS=3`, deploy-count check logic. File-based storage replaced by Prisma `User.deployCount`
- `/tmp/vibe-loom/src/app/api/analyze-deployment-error/route.ts` → `backend/src/analysis/analysis.controller.ts` — Full RAG pipeline: load monad-docs → build prompt → call Gemini → parse JSON → fallback heuristics → return analysis
- `/tmp/vibe-loom/src/app/api/deploy-status/route.ts` → `backend/src/paymaster/paymaster.controller.ts` — Deploy count status endpoint
- `/tmp/vibe-loom/data/monad-docs/*.md` → `backend/data/monad-docs/` — 5 RAG context files (~5KB total)

### Key Files — To Create

**Auth Module** (`backend/src/auth/`):
- `auth.module.ts` — Imports PassportModule, JwtModule.registerAsync, provides strategies + service
- `auth.service.ts` — `validateOrCreateUser(profile)` → upsert User in Prisma, `login(user)` → sign JWT
- `auth.controller.ts` — `GET /api/auth/github` (redirect), `GET /api/auth/github/callback` (exchange code → JWT), `GET /api/auth/me` (JWT-protected, return user)
- `github.strategy.ts` — PassportStrategy(Strategy) from `passport-github2`, validates GitHub callback
- `jwt.strategy.ts` — PassportStrategy(Strategy) from `passport-jwt`, extracts Bearer token
- `jwt-auth.guard.ts` — `AuthGuard('jwt')` convenience class
- `github-auth.guard.ts` — `AuthGuard('github')` convenience class
- `dto/auth-response.dto.ts` — `{ accessToken, user: { id, githubId, username, avatarUrl, deployCount } }`

**Analysis Module** (`backend/src/analysis/`):
- `analysis.module.ts` — Provides GeminiService, AnalysisService, OptimizerService
- `analysis.controller.ts` — `POST /api/analysis/error` (accepts `{ error, contractSource, errorCode }`, supports `?stream=true`)
- `analysis.service.ts` — Orchestrates: load RAG context → build prompt → call Gemini → parse response → fallback heuristics
- `gemini.service.ts` — Wraps `@google/generative-ai`, model init with fallback, `generateContent()`, `generateContentStream()`
- `optimizer.service.ts` — Port of `calculateMonadParallelismScore()` rule-based scoring
- `prompt-templates.ts` — Prompt constants + builder functions (pure functions, no class needed)
- `error-handler.ts` — `parseProviderError()`, `handleDeploymentError()` (pure functions)
- `dto/analysis-request.dto.ts` — `{ error: object, contractSource: string, errorCode?: string }`
- `dto/analysis-response.dto.ts` — `{ analysis: { summary, fixedCode, explanation, ... }, optimization: { score, deductions, suggestions } }`

**Paymaster Module** (`backend/src/paymaster/`):
- `paymaster.module.ts` — Provides PaymasterService, imports ContractsModule (for DeployService)
- `paymaster.controller.ts` — `GET /api/paymaster/status` (JWT-protected, returns deploy count), `POST /api/paymaster/relay-signed` (receives signed tx hex, broadcasts)
- `paymaster.service.ts` — `getDeployStatus(userId)` → query User.deployCount, `canUseRelay(userId)` → deployCount < 3, `incrementDeployCount(userId)`, `broadcastSignedTransaction(signedTxHex)`
- `dto/deploy-status.dto.ts` — `{ used, max, remaining, canUseRelay }`
- `dto/relay-signed.dto.ts` — `{ signedTransaction: string }` (hex-encoded signed tx)

### Build Order

1. **Auth Module first** — JwtAuthGuard is a dependency for Paymaster endpoints. Auth is self-contained: config extension → GitHub strategy → JWT strategy → service → controller → guard → tests. Once JwtAuthGuard exists, Paymaster can use it.

2. **Paymaster Module second** — Uses JwtAuthGuard from Auth. Needs Prisma User.deployCount. Integrates with existing DeployService (S02). The relay-signed endpoint uses ethers.js `broadcastTransaction()` which is already a dependency.

3. **Analysis Module third** — Independent of Auth/Paymaster. Requires `@google/generative-ai` npm package. Ports the largest amount of Vibe-Loom logic but it's straightforward restructuring. Streaming support via NestJS `StreamableFile` or raw `Response` with chunked transfer encoding.

4. **Wire up** — Register all three modules in AppModule. Extend configuration.ts with auth/gemini config keys. Copy monad-docs RAG data files.

### Verification Approach

**Auth:**
- Unit tests: `AuthService.validateOrCreateUser()` with mock PrismaService, `AuthService.login()` returns valid JWT
- Unit tests: `JwtStrategy.validate()` extracts userId from token payload
- Integration check: `GET /api/auth/github` returns redirect URL (302)
- `GET /api/auth/me` without Bearer → 401, with valid JWT → user data

**Paymaster:**
- Unit tests: `PaymasterService.canUseRelay()` returns true when deployCount < 3, false when >= 3
- Unit tests: `PaymasterService.incrementDeployCount()` updates User record
- `GET /api/paymaster/status` without JWT → 401
- `GET /api/paymaster/status` with JWT → `{ used, max: 3, remaining, canUseRelay }`

**Analysis:**
- Unit tests: `AnalysisService.analyzeError()` with mock GeminiService returns analysis
- Unit tests: `OptimizerService.calculateScore()` with known contract patterns returns expected scores
- `POST /api/analysis/error` with `{ error: { message: 'gas' }, contractSource: '...' }` → returns analysis with fixedCode
- Streaming: `POST /api/analysis/error?stream=true` → chunked text response

**Build check:** `cd backend && npm run build` exit 0, `cd backend && npm test` all suites pass.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| GitHub OAuth | `passport-github2` + `@nestjs/passport` | Standard NestJS pattern, handles OAuth2 flow, tested community package |
| JWT signing/verification | `@nestjs/jwt` + `passport-jwt` | Official NestJS JWT package, integrates with Passport guards |
| Gemini AI calls | `@google/generative-ai` | Same library as Vibe-Loom, supports generateContent + streaming |
| Request validation | `class-validator` + `class-transformer` | Already installed (S01), ValidationPipe registered globally |
| Deploy count storage | Prisma `User.deployCount` | Already modeled in schema (S01), replaces Vibe-Loom's file-based store |

## Constraints

- **Config keys must use dot-notation** — `configService.get<string>('github.clientId')`. The `configuration.ts` factory returns nested objects. S03 must extend this factory, not add flat env var reads.
- **TransformInterceptor wraps all responses** — Controllers return raw data; don't manually wrap in `{ success: true, data }`. Exception: streaming responses bypass interceptors (use `@Res()` or `StreamableFile`).
- **PrismaModule is @Global** — No need to import it in Auth/Analysis/Paymaster modules. Just inject `PrismaService`.
- **Deployment.userId FK** — KNOWLEDGE.md notes that `userId: 'anonymous'` will fail at runtime (FK constraint). S03 should either: (a) require auth for deploy endpoints via JwtAuthGuard, or (b) make userId nullable in schema. Recommendation: **(a) require auth** — this is the right architectural choice since Paymaster needs user identity anyway.
- **Monad-docs total size ~5KB** — Small enough to load entirely into each prompt. No chunking or vector DB needed for RAG.
- **`@nestjs/passport` v11+ requires `passport` v0.7+** — Both are current. Type definitions: `@types/passport-github2` and `@types/passport-jwt` needed as devDependencies.

## Common Pitfalls

- **Streaming responses bypass TransformInterceptor** — When using `?stream=true` for Gemini streaming, the controller must use `@Res() res: Response` to write chunks directly. The `@Res()` decorator tells NestJS to skip the interceptor pipeline. Don't forget to set the correct Content-Type header (`text/plain; charset=utf-8` or `text/event-stream`).
- **Passport GitHub strategy callbackURL must match GitHub OAuth App settings** — The callback URL registered in the GitHub OAuth App must exactly match what's configured in the strategy. For local dev this is typically `http://localhost:3000/api/auth/github/callback`. For production it changes to the Railway domain.
- **JWT payload size** — Don't put the entire User object in the JWT. Include only `sub` (userId), `githubId`, `username`. The JwtStrategy.validate() fetches the full user from DB if needed.
- **Gemini model name** — Vibe-Loom uses `gemini-2.5-flash` as default with `gemini-pro` fallback. Model availability can change. The service should handle model-not-found errors gracefully.
- **DeployService.deploy() userId default** — Currently defaults to `'anonymous'`, which will cause Prisma FK error at runtime. After S03, the deploy endpoint should be behind JwtAuthGuard, and the controller should extract userId from `req.user` and pass it to `deploy()`.

## Open Risks

- **GitHub OAuth App setup** — Requires a GitHub OAuth App to be registered at github.com/settings/applications/new with correct callback URL. This is a manual setup step that must happen before the auth flow can be tested end-to-end. Unit tests mock the OAuth flow, so this doesn't block development.
- **Gemini API key availability** — `@google/generative-ai` requires a valid API key. Without it, the analysis service falls back to rule-based heuristics (which is the designed behavior from Vibe-Loom).
- **WalletConnect relay-signed endpoint** — The `POST /api/paymaster/relay-signed` endpoint receives a hex-encoded signed transaction and broadcasts it. This assumes the frontend properly constructs and signs the transaction targeting the correct chain (Monad testnet). If the signed tx targets the wrong chain or has invalid nonce, the broadcast will fail with an opaque RPC error. Error handling for broadcast failures should return useful diagnostics.

## New Dependencies (npm packages to add)

```
# Production
@nestjs/passport       # ^11.0.0 — Passport integration for NestJS
@nestjs/jwt            # ^11.0.0 — JWT utilities for NestJS
passport               # ^0.7.0 — Authentication middleware
passport-github2       # ^0.1.12 — GitHub OAuth2 strategy
passport-jwt           # ^4.0.1 — JWT strategy for Passport
@google/generative-ai  # ^0.24.0 — Gemini AI SDK

# Dev
@types/passport-github2  # — TypeScript types for passport-github2
@types/passport-jwt      # — TypeScript types for passport-jwt
```

## API Endpoints Summary

| Method | Path | Auth | Module | Description |
|--------|------|------|--------|-------------|
| GET | /api/auth/github | none | Auth | Redirect to GitHub OAuth |
| GET | /api/auth/github/callback | GitHub OAuth | Auth | Exchange code → JWT |
| GET | /api/auth/me | JWT | Auth | Current user profile |
| POST | /api/analysis/error | none | Analysis | Analyze deploy error with Gemini RAG |
| POST | /api/analysis/error?stream=true | none | Analysis | Streaming analysis response |
| GET | /api/paymaster/status | JWT | Paymaster | Deploy count + relay eligibility |
| POST | /api/paymaster/relay-signed | JWT | Paymaster | Broadcast user-signed transaction |

## Sources

- NestJS Passport docs (Context7: /nestjs/docs.nestjs.com — authentication/passport recipe)
- NestJS JWT docs (Context7: /nestjs/jwt — sign/verify API)
- Vibe-Loom source code (`/tmp/vibe-loom/src/lib/`, `/tmp/vibe-loom/src/app/api/`)
