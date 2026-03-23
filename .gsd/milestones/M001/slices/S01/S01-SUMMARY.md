---
id: S01
parent: M001
milestone: M001
provides:
  - NestJS backend project scaffold in backend/ with TypeScript strict mode, decorators, ES2021 target
  - ConfigModule (isGlobal) with typed configuration factory — port, database.url, nodeEnv
  - Prisma schema (PostgreSQL) with User, Deployment, Analysis, VibeScore models — all with cuid IDs, timestamps, proper indexes
  - PrismaService extending PrismaClient with OnModuleInit ($connect) in @Global() PrismaModule
  - HealthModule with /api/health (liveness) and /api/health/readiness (DB connectivity) using @nestjs/terminus + PrismaHealthIndicator
  - ApiResponse<T> DTO with ok/fail factory methods — standard API response envelope
  - HttpExceptionFilter — global catch-all returning { success: false, error: { statusCode, message, timestamp, path } }
  - TransformInterceptor — wraps all successful responses as { success: true, data }
  - Global ValidationPipe (whitelist + transform) registered in main.ts
  - main.ts bootstrap with /api global prefix, CORS (origin: true, credentials: true), shutdown hooks
  - 5 unit tests (2 suites) covering PrismaService and HealthController
requires:
  - slice: none
    provides: first slice — no upstream dependencies
affects:
  - S02
  - S03
  - S04
key_files:
  - backend/package.json
  - backend/tsconfig.json
  - backend/nest-cli.json
  - backend/src/main.ts
  - backend/src/app.module.ts
  - backend/src/config/configuration.ts
  - backend/prisma/schema.prisma
  - backend/src/prisma/prisma.service.ts
  - backend/src/prisma/prisma.module.ts
  - backend/src/health/health.controller.ts
  - backend/src/health/health.module.ts
  - backend/src/health/prisma-health.indicator.ts
  - backend/src/common/dto/api-response.dto.ts
  - backend/src/common/filters/http-exception.filter.ts
  - backend/src/common/interceptors/transform.interceptor.ts
  - backend/.env.example
  - test/health.controller.spec.ts
  - test/prisma.service.spec.ts
key_decisions:
  - "Used tsc builder instead of webpack in nest-cli.json — avoids ts-loader dependency and deleteOutDir/incremental conflict"
  - "Set incremental: false in tsconfig.json — deleteOutDir: true causes silent no-emit when tsc cache thinks nothing changed"
  - "Changed Jest rootDir from 'src' to '.' with roots ['src', 'test'] — ensures tests in test/ directory are discovered"
  - "Backend lives in backend/ subdirectory (D008) — avoids Cargo/npm conflict in worktree root, maps to D001 separate repo target"
patterns_established:
  - "NestJS project lives in backend/ — all npm commands require cd backend first"
  - "ConfigModule is isGlobal: true — inject ConfigService anywhere without importing ConfigModule"
  - "PrismaModule is @Global() — inject PrismaService anywhere without importing PrismaModule"
  - "All Prisma models use cuid() IDs, createdAt, updatedAt; text-heavy fields use @db.Text"
  - "All API errors: { success: false, error: { statusCode, message, timestamp, path } } via HttpExceptionFilter"
  - "All API successes: { success: true, data: ... } via TransformInterceptor"
  - "Health endpoints: /api/health (liveness), /api/health/readiness (DB check) — use for Railway probes"
  - "Unit tests use @nestjs/testing Test.createTestingModule with mock providers — no real DB needed"
observability_surfaces:
  - "GET /api/health → { status: 'ok', info: { database: { status: 'up' } } } when DB connected"
  - "GET /api/health/readiness → same check, for Railway/K8s readiness probes"
  - "NestJS Logger on bootstrap: 'Listening on port ${port}'"
  - "PrismaService logs: 'Connecting to database...' / 'Database connection established'"
  - "HttpExceptionFilter logs unhandled (non-HTTP) exceptions with stack trace — client sees 500 'Internal server error'"
  - "npm run build exit 0 = clean compile; npm test = 2 suites, 5 tests; npx prisma generate = schema valid"
drill_down_paths:
  - .gsd/milestones/M001/slices/S01/tasks/T01-SUMMARY.md
  - .gsd/milestones/M001/slices/S01/tasks/T02-SUMMARY.md
duration: ~25m
verification_result: passed
completed_at: 2026-03-22
---

# S01: NestJS Foundation + Database

**Complete NestJS backend scaffold with ConfigModule, Prisma ORM (4 PostgreSQL models), HealthModule with DB probes, and standardized API response patterns — foundation ready for S02–S06**

## What Happened

**T01** created the `backend/` NestJS project from scratch. Wrote `package.json` with all NestJS core + Prisma + dev dependencies, configured TypeScript in strict mode with decorator support, and set up `nest-cli.json` with the tsc builder (avoiding the default webpack builder that requires ts-loader). The `main.ts` bootstrap sets the `/api` global prefix, enables CORS with credentials, and registers shutdown hooks. `AppModule` imports `ConfigModule.forRoot({ isGlobal: true })` with a typed config factory (`port`, `database.url`, `nodeEnv`) and a `@Global()` `PrismaModule`. The Prisma schema defines 4 models targeting PostgreSQL — `User` (GitHub identity, deploy count), `Deployment` (contract deploy records with status tracking), `Analysis` (Gemini AI error analysis results), and `VibeScore` (engine-based parallel execution scores) — all with cuid IDs, timestamps, proper indexes, and `@db.Text` for large fields.

Hit one notable build issue: `deleteOutDir: true` in nest-cli.json conflicts with `incremental: true` in tsconfig — tsc's cache sees "nothing changed" after dist is deleted and emits zero files. Fixed by setting `incremental: false`.

**T02** added the runtime infrastructure layer. Created `HealthModule` with `@nestjs/terminus` — `PrismaHealthIndicator` runs `SELECT 1` against the database, and `HealthController` exposes `/api/health` (liveness) and `/api/health/readiness` (DB connectivity check) endpoints. Built three common API patterns: `ApiResponse<T>` DTO with `ok()`/`fail()` factory methods, `HttpExceptionFilter` that catches all exceptions and returns standardized error responses (with stack traces logged server-side only), and `TransformInterceptor` that wraps successful responses as `{ success: true, data }`. Registered all three globally in `main.ts` alongside `ValidationPipe` (whitelist + transform). Wrote 5 unit tests across 2 suites: `PrismaService` (defined + onModuleInit calls $connect) and `HealthController` (defined + check + checkReadiness), using `@nestjs/testing` with mock providers.

## Verification

All 7 slice-level verification checks pass:

| # | Check | Result |
|---|-------|--------|
| 1 | `cd backend && npm run build` — zero TypeScript errors | ✅ pass |
| 2 | `cd backend && npm test` — 2 suites, 5 tests, all pass | ✅ pass |
| 3 | `cd backend && npx prisma generate` — Prisma client generates | ✅ pass |
| 4 | `npx ts-node -e "import('./src/config/configuration')..."` — config loads | ✅ pass |
| 5 | `grep -q "setGlobalPrefix" backend/src/main.ts` — /api prefix set | ✅ pass |
| 6 | `grep -q "enableCors" backend/src/main.ts` — CORS enabled | ✅ pass |
| 7 | `grep -q "enableShutdownHooks" backend/src/main.ts` — shutdown hooks enabled | ✅ pass |

## New Requirements Surfaced

- none

## Deviations

- Set `incremental: false` in tsconfig.json — not in the original plan, but required to fix silent build failure when combined with `deleteOutDir: true`
- Added `"builder": "tsc"` to nest-cli.json — the plan didn't specify the build tool, but the default webpack builder requires ts-loader which wasn't in the dependency list
- Changed Jest `rootDir` from `"src"` to `"."` with explicit roots — T01's initial config only scanned src/ which would miss T02's tests in test/

## Known Limitations

- **No real DB connection tested** — PrismaService unit test mocks $connect. Runtime DB connectivity is only verifiable with a real DATABASE_URL (which becomes available when Railway PostgreSQL is provisioned in S06)
- **No migration files** — `prisma migrate dev` requires a live database; migration will happen on first Railway deploy or when a local PostgreSQL is set up for development
- **CORS is fully open** — `origin: true` allows all origins. Should be restricted to the frontend domain in production (S06)

## Follow-ups

- S02: Import PrismaModule (already global) and ConfigModule to build ContractsModule with HardhatService
- S03: Import PrismaModule for User/DeployCount storage, ConfigModule for GitHub OAuth + JWT secrets
- S04: Import ConfigModule for ENGINE_BINARY_PATH
- S06: Run `prisma migrate deploy` on Railway PostgreSQL; restrict CORS origin to frontend URL

## Files Created/Modified

- `backend/package.json` — NestJS project manifest with core + Prisma + dev dependencies, Jest config
- `backend/tsconfig.json` — TypeScript strict mode, decorators, ES2021, incremental: false
- `backend/tsconfig.build.json` — Build config excluding test files
- `backend/nest-cli.json` — NestJS CLI config with tsc builder, deleteOutDir
- `backend/.env.example` — Environment variable documentation with all current/future keys
- `backend/src/main.ts` — Bootstrap with /api prefix, CORS, shutdown hooks, global filter/interceptor/pipe
- `backend/src/app.module.ts` — Root module importing ConfigModule, PrismaModule, HealthModule
- `backend/src/config/configuration.ts` — Typed config factory (port, database.url, nodeEnv)
- `backend/prisma/schema.prisma` — PostgreSQL schema with User, Deployment, Analysis, VibeScore models
- `backend/src/prisma/prisma.service.ts` — PrismaService extending PrismaClient with OnModuleInit
- `backend/src/prisma/prisma.module.ts` — @Global() PrismaModule exporting PrismaService
- `backend/src/health/health.controller.ts` — /health and /health/readiness endpoints
- `backend/src/health/health.module.ts` — HealthModule importing TerminusModule
- `backend/src/health/prisma-health.indicator.ts` — PrismaHealthIndicator with SELECT 1 DB check
- `backend/src/common/dto/api-response.dto.ts` — ApiResponse<T> with ok/fail factory methods
- `backend/src/common/filters/http-exception.filter.ts` — Global exception filter
- `backend/src/common/interceptors/transform.interceptor.ts` — Response wrapper interceptor
- `backend/test/health.controller.spec.ts` — HealthController unit tests (3 tests)
- `backend/test/prisma.service.spec.ts` — PrismaService unit tests (2 tests)
- `backend/test/jest-e2e.json` — E2E test configuration scaffold

## Forward Intelligence

### What the next slice should know
- All npm commands must run from `backend/` — the project is a subdirectory, not root
- `PrismaModule` is `@Global()` — just inject `PrismaService` in any service constructor, no module import needed
- `ConfigModule` is `isGlobal: true` — inject `ConfigService` anywhere with `configService.get<string>('database.url')` (dot-notation for nested keys)
- The `.env.example` already documents env vars needed by future slices: `MONAD_RPC_URL`, `MONAD_PRIVATE_KEY`, `GITHUB_CLIENT_ID/SECRET`, `GEMINI_API_KEY`, `JWT_SECRET`, `ENGINE_BINARY_PATH`
- `TransformInterceptor` auto-wraps all controller returns as `{ success: true, data: ... }` — controllers should return raw data, not manually wrap in ApiResponse

### What's fragile
- `deleteOutDir: true` + tsconfig — if anyone adds `incremental: true` back to tsconfig.json, builds will silently produce empty dist/. The KNOWLEDGE.md entry documents this but it's an easy trap to fall into
- Prisma schema has no migration files yet — first `prisma migrate dev` or `prisma migrate deploy` will create the initial migration from scratch. Don't be surprised by a large initial migration

### Authoritative diagnostics
- `cd backend && npm run build` exit 0 = TypeScript compiles clean — check this after any source change
- `cd backend && npm test` — 2 suites, 5 tests — check after adding new test files
- `cd backend && npx prisma generate` exit 0 = schema valid — check after any schema.prisma change
- GET /api/health returns `{ status: "ok" }` when server is running with DB — the definitive runtime health signal

### What assumptions changed
- Original plan didn't specify the NestJS build tool — assumed default would work. Actually, default is webpack which requires ts-loader. tsc builder is the right choice for this project size.
