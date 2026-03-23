---
id: S06
parent: M001
milestone: M001
provides:
  - Multi-stage Dockerfile composing Rust 1.88 CLI builder + Node 20 NestJS builder into slim runtime image
  - Railway deployment config (railway.json) with health check and restart policy
  - Production startup script (scripts/start.sh) running Prisma migrate deploy before server boot
  - Environment-aware CORS (FRONTEND_URL in production, open in development)
  - 12-case E2E test suite validating full NestJS HTTP layer with mocked PrismaService
requires:
  - slice: S05
    provides: Completed NestJS backend with all modules (Health, Contracts, Auth, Analysis, Paymaster, Engine, VibeScore) and Next.js frontend
  - slice: S01
    provides: PrismaService, AppModule, ConfigModule
  - slice: S02
    provides: ContractsModule with compile and deploy endpoints
  - slice: S03
    provides: AuthModule (JWT guards), AnalysisModule, PaymasterModule
  - slice: S04
    provides: EngineModule, VibeScoreModule
affects: []
key_files:
  - backend/Dockerfile
  - backend/.dockerignore
  - railway.json
  - backend/scripts/start.sh
  - backend/src/main.ts
  - backend/test/app.e2e-spec.ts
  - backend/package.json
  - backend/.env.example
key_decisions:
  - "Rust base image bumped to rust:1.88-slim (plan said 1.82) because alloy/revm crates require edition2024 + MSRV 1.88"
  - "NestJS @Post() returns 201 not 200 — E2E tests expect 201 for all POST endpoints"
  - "supertest v7 requires default import (import request from 'supertest') not namespace import"
patterns_established:
  - "Multi-stage Docker: builder-rust (1.88) → builder-node (20-slim) → runtime (node:20-slim + openssl)"
  - "CORS env-branching: production restricts to FRONTEND_URL, development allows all origins"
  - "E2E test setup: Test.createTestingModule(AppModule) + overrideProvider(PrismaService) → replicate main.ts bootstrap (prefix, filters, interceptors, pipes)"
  - "Health endpoints wrapped by TransformInterceptor: assert on res.body.data.status not res.body.status"
observability_surfaces:
  - "Bootstrap log: CORS origin value logged at startup"
  - "scripts/start.sh: Prisma migration status echoed before server start"
  - "Docker image: docker run --rm monad-backend sh -c 'ls /app/' validates layout"
  - "npm run test:e2e: 12 pass/fail results with HTTP status + response body in terminal/CI output"
drill_down_paths:
  - .gsd/milestones/M001/slices/S06/tasks/T01-SUMMARY.md
  - .gsd/milestones/M001/slices/S06/tasks/T02-SUMMARY.md
duration: 23m
verification_result: passed
completed_at: 2026-03-22T21:05:00+09:00
---

# S06: Railway Deploy + E2E Validation

**Multi-stage Dockerfile (Rust 1.88 + Node 20) builds deployable image with monad-cli + NestJS; 12 E2E tests validate full API surface; 97 unit tests pass with zero regressions**

## What Happened

This slice delivered the deployment infrastructure and integration test layer that closes M001.

**T01 — Deployment Infrastructure:** Built a three-stage Dockerfile: (1) `rust:1.88-slim` compiles the monad-cli binary from workspace crates, (2) `node:20-slim` runs npm ci + prisma generate + nest build, (3) a slim runtime image copies both artifacts plus `contracts/` and `data/` directories needed at runtime. The plan specified Rust 1.82 but alloy/revm crates demand MSRV 1.88 (edition2024) — confirmed via iterative build failures. Created `railway.json` at repo root with Dockerfile builder config, `/api/health` healthcheck, and ON_FAILURE restart policy. Created `scripts/start.sh` that runs `prisma migrate deploy` before `node dist/main`. Updated `main.ts` CORS to branch by NODE_ENV: production restricts origin to `FRONTEND_URL`, development allows all. Docker build succeeds (283s), image verified with correct layout (`/app/monad-cli` executable, `/app/contracts/`, `/app/data/`).

**T02 — E2E Test Suite:** Installed supertest v7 + @types/supertest. Wrote 12 E2E test cases in `app.e2e-spec.ts` that boot the full AppModule via `Test.createTestingModule`, overriding only PrismaService to avoid DB connections. Tests replicate the exact `main.ts` bootstrap config (global prefix, HttpExceptionFilter, TransformInterceptor, ValidationPipe). Coverage spans: health (2), contracts source/compile (5), vibe-score (2), analysis (2), paymaster auth (1). Two key discoveries: supertest v7 requires default import syntax, and NestJS `@Post()` defaults to 201 not 200.

## Verification

| # | Check | Result |
|---|-------|--------|
| 1 | `cd backend && npm run test:e2e` — 12 E2E tests pass | ✅ pass (12/12, 10.6s) |
| 2 | `cd backend && npm test` — 97 unit tests pass (14 suites, no regressions) | ✅ pass (97/97, 10.5s) |
| 3 | `docker build -f backend/Dockerfile -t monad-backend .` — exits 0 | ✅ pass (283s) |
| 4 | All deployment files exist (Dockerfile, .dockerignore, railway.json, start.sh) | ✅ pass |
| 5 | CORS uses FRONTEND_URL (`grep -q 'FRONTEND_URL\|frontend' backend/src/main.ts`) | ✅ pass |
| 6 | Docker image layout correct (monad-cli executable, contracts/, data/) | ✅ pass |
| 7 | CORS origin logged at bootstrap (`grep -q 'CORS origin' backend/src/main.ts`) | ✅ pass |

**Total test count: 109 (97 unit + 12 E2E) across 15 suites, zero failures.**

## New Requirements Surfaced

- none

## Deviations

- **Rust version 1.82 → 1.88**: Plan specified `rust:1.82-slim` but alloy/revm crates require MSRV 1.88 (edition2024). Confirmed through iterative Docker build failures. Crate dependency, not architectural change.
- **POST status 200 → 201**: Plan implied 200 for POST endpoints but NestJS `@Post()` defaults to 201 Created. E2E tests corrected to match actual framework behavior.
- **supertest import syntax**: Plan showed `import * as request` but supertest v7 requires `import request from 'supertest'` with esModuleInterop.

## Known Limitations

- **Railway environment variables not yet configured**: DATABASE_URL, MONAD_PRIVATE_KEY, JWT_SECRET, GEMINI_API_KEY, GITHUB_CLIENT_ID/SECRET must be set in Railway dashboard before first deploy.
- **Prisma initial migration not created**: `npx prisma migrate dev --name init` must be run to create the first migration file before `prisma migrate deploy` can execute in production.
- **Frontend deployed separately**: Next.js frontend needs its own deployment with NEXT_PUBLIC_API_URL pointing to the Railway backend URL.
- **Docker build time**: Full build takes ~283s due to Rust compilation. Cached builds are faster but first deploy will be slow.

## Follow-ups

- Configure Railway environment variables via Railway dashboard or CLI before first production deploy
- Create initial Prisma migration (`npx prisma migrate dev --name init`) and commit it
- Deploy frontend (Next.js) to Vercel/Railway with `NEXT_PUBLIC_API_URL` pointing to backend
- Consider Docker layer caching strategy (cargo-chef) to speed up Rust compilation in CI/CD

## Files Created/Modified

- `backend/Dockerfile` — Multi-stage Dockerfile (Rust 1.88 builder → Node 20 builder → slim runtime)
- `backend/.dockerignore` — Excludes node_modules, dist, .git, target, test, .env files
- `railway.json` — Railway config with Dockerfile builder, /api/health healthcheck, ON_FAILURE restart
- `backend/scripts/start.sh` — Startup script: prisma migrate deploy → node dist/main
- `backend/src/main.ts` — CORS branches by NODE_ENV: production uses FRONTEND_URL, dev allows all; logs CORS origin
- `backend/package.json` — start:prod updated to scripts/start.sh; added supertest + @types/supertest devDeps
- `backend/.env.example` — Added FRONTEND_URL entry
- `backend/test/app.e2e-spec.ts` — 12 E2E test cases covering full API surface with mocked PrismaService

## Forward Intelligence

### What the next slice should know
- The Docker image is self-contained: monad-cli binary at `/app/monad-cli`, NestJS dist at `/app/dist/`, Prisma client generated, contracts/ and data/ directories copied. ENGINE_BINARY_PATH is set to `/app/monad-cli` in the Dockerfile ENV.
- Railway deployment requires railway.json at repo root with `dockerfilePath: backend/Dockerfile` and build context as repo root (not backend/).
- E2E tests override only PrismaService — all other modules (including EngineService, CompileService, AuthService) use real implementations. EngineService returns null (binary not present in test env), triggering heuristic fallback in VibeScoreService.

### What's fragile
- **Rust MSRV coupling**: The Dockerfile `FROM rust:1.88-slim` is pinned to the current alloy/revm MSRV. Any crate version bump may require updating this tag — Docker build will fail with a clear MSRV error message.
- **E2E PrismaService mock shape**: The mock must match every Prisma model method that any service calls. Adding new DB operations requires updating the mock at the top of `app.e2e-spec.ts`.

### Authoritative diagnostics
- `cd backend && npm run test:e2e` — 12 tests covering the full HTTP layer. If any fail, the response body diff shows exactly what changed.
- `docker build -f backend/Dockerfile -t monad-backend .` — build log shows which stage failed. Rust MSRV errors name the exact crate and required version.
- `docker run --rm monad-backend:latest sh -c 'test -x /app/monad-cli && test -d /app/contracts && test -d /app/data && echo OK'` — validates runtime layout.

### What assumptions changed
- **Rust 1.82 is sufficient** → alloy/revm ecosystem bumps MSRV aggressively; 1.88 required for edition2024. Always read crate error output for exact MSRV, don't guess intermediate versions.
- **POST endpoints return 200** → NestJS @Post() defaults to 201 Created. E2E tests must expect 201 unless controller explicitly uses @HttpCode(200).
