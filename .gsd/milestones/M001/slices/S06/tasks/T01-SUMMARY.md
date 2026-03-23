---
id: T01
parent: S06
milestone: M001
provides:
  - Multi-stage Dockerfile composing Rust CLI + NestJS into single runtime image
  - Railway deployment config with health check and restart policy
  - Start script with Prisma migration before server boot
  - Production CORS using FRONTEND_URL env var
key_files:
  - backend/Dockerfile
  - railway.json
  - backend/scripts/start.sh
  - backend/src/main.ts
  - backend/.dockerignore
  - backend/.env.example
key_decisions:
  - "Rust base image bumped to rust:1.88-slim (plan said 1.82) because alloy/revm crates require edition2024 + MSRV 1.88"
patterns_established:
  - "Multi-stage Docker: builder-rust → builder-node → runtime (node:20-slim + openssl)"
  - "CORS env-branching: production restricts to FRONTEND_URL, development allows all origins"
observability_surfaces:
  - "Bootstrap log: CORS origin value logged at startup"
  - "scripts/start.sh: Prisma migration status echoed before server start"
  - "Docker image: `docker run --rm monad-backend sh -c 'ls /app/'` validates layout"
duration: 15m
verification_result: passed
completed_at: 2026-03-22T21:05:00+09:00
blocker_discovered: false
---

# T01: Create multi-stage Dockerfile, Railway config, start script, and production CORS

**Built multi-stage Dockerfile (Rust 1.88 + Node 20), railway.json, start.sh with Prisma migrations, and environment-aware CORS in main.ts**

## What Happened

Created all deployment infrastructure for Railway. The Dockerfile has three stages: (1) `rust:1.88-slim` compiles the monad-cli binary from the workspace crates, (2) `node:20-slim` runs npm ci, prisma generate, and nest build for the NestJS app, (3) a slim runtime image copies built artifacts plus contracts/ and data/ directories needed by ContractsController at `process.cwd()`.

The plan specified `rust:1.82-slim` but the alloy-eip2930, alloy-eip7702, and revm crate family require `edition2024` and MSRV 1.88 — confirmed through two failed Docker builds (1.82 → 1.85 → 1.88). The final `rust:1.88-slim` build succeeds.

Updated `main.ts` to branch CORS by environment: production restricts origin to `FRONTEND_URL` from ConfigService, development allows all. Added a CORS origin log line at bootstrap for runtime diagnostics. Updated `start:prod` script to use `scripts/start.sh` which runs `prisma migrate deploy` before `node dist/main`. Added `FRONTEND_URL` to `.env.example`.

## Verification

- All 4 deployment files exist (Dockerfile, .dockerignore, railway.json, start.sh)
- `grep -q 'frontend' backend/src/main.ts` confirms CORS references frontend config
- `grep -q 'prisma migrate deploy' backend/scripts/start.sh` confirms migration step
- `grep -q 'dockerfilePath' railway.json` confirms Railway config references Dockerfile
- All 97 unit tests pass across 14 suites — no regressions from CORS change
- `docker build -f backend/Dockerfile -t monad-backend .` exits 0 (283s build)
- Docker image verified: monad-cli at `/app/monad-cli` (executable, 2MB), `/app/contracts/`, `/app/data/`, `ENGINE_BINARY_PATH` set

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `test -f backend/Dockerfile && test -f backend/.dockerignore && test -f railway.json && test -f backend/scripts/start.sh` | 0 | ✅ pass | <1s |
| 2 | `grep -q 'FRONTEND_URL\|frontend' backend/src/main.ts` | 0 | ✅ pass | <1s |
| 3 | `grep -q 'prisma migrate deploy' backend/scripts/start.sh` | 0 | ✅ pass | <1s |
| 4 | `grep -q 'dockerfilePath' railway.json` | 0 | ✅ pass | <1s |
| 5 | `cd backend && npm test` | 0 | ✅ pass (97 tests, 14 suites) | 10s |
| 6 | `docker build -f backend/Dockerfile -t monad-backend .` | 0 | ✅ pass | 284s |
| 7 | `docker run --rm monad-backend:latest sh -c 'test -x /app/monad-cli && test -d /app/contracts && test -d /app/data && echo OK'` | 0 | ✅ pass | 1s |
| 8 | `grep -q 'CORS origin' backend/src/main.ts` | 0 | ✅ pass | <1s |

### Slice-level checks (partial — T02 not yet done)

| # | Check | Status |
|---|-------|--------|
| 1 | `cd backend && npm run test:e2e` — E2E tests pass | ⏳ pending T02 |
| 2 | `cd backend && npm test` — 97 unit tests pass | ✅ pass |
| 3 | `docker build -f backend/Dockerfile -t monad-backend .` — exits 0 | ✅ pass |
| 4 | Deployment files exist | ✅ pass |
| 5 | CORS uses FRONTEND_URL | ✅ pass |
| 6 | Docker image layout correct | ✅ pass |
| 7 | CORS origin logged at bootstrap | ✅ pass |

## Diagnostics

- **CORS origin**: Bootstrap log line `CORS origin: <url|all (development)>` — visible in container stdout or Railway deploy logs
- **Prisma migration**: `scripts/start.sh` echoes "Running Prisma migrations..." before `prisma migrate deploy` — visible in startup logs
- **Docker image layout**: `docker run --rm monad-backend:latest ls -la /app/` shows all runtime artifacts
- **Rust MSRV failures**: If crate versions are bumped and MSRV increases, Docker build fails at Stage 1 with explicit `error: rustc X.Y is not supported` — fix by updating the `FROM rust:` version
- **No secrets in layers**: All sensitive values (DATABASE_URL, JWT_SECRET, etc.) are runtime-only env vars, not baked into Docker image

## Deviations

- **Rust version 1.82 → 1.88**: Plan specified `rust:1.82-slim` but alloy/revm crates require MSRV 1.88. Confirmed through iterative build failures. This is a crate dependency requirement, not an architectural change.

## Known Issues

None.

## Files Created/Modified

- `backend/Dockerfile` — Multi-stage Dockerfile (Rust 1.88 builder → Node 20 builder → slim runtime)
- `backend/.dockerignore` — Excludes node_modules, dist, .git, target, test, .env files
- `railway.json` — Railway config with Dockerfile builder, /api/health healthcheck, ON_FAILURE restart
- `backend/scripts/start.sh` — Startup script: prisma migrate deploy → node dist/main
- `backend/src/main.ts` — CORS branches by NODE_ENV: production uses FRONTEND_URL, dev allows all; logs CORS origin
- `backend/package.json` — start:prod updated to `sh scripts/start.sh`
- `backend/.env.example` — Added FRONTEND_URL entry with default http://localhost:3001
