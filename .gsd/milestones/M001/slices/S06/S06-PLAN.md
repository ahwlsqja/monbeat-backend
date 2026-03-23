# S06: Railway Deploy + E2E Validation

**Goal:** NestJS backend is deployable to Railway with a multi-stage Dockerfile (Rust CLI + Node.js), production CORS is configured, and E2E tests validate the full API surface with mocked externals.
**Demo:** `docker build -f backend/Dockerfile -t monad-backend .` succeeds from repo root; `cd backend && npm run test:e2e` passes all E2E tests; `cd backend && npm test` still passes 97 unit tests.

## Must-Haves

- Multi-stage Dockerfile: Stage 1 compiles monad-cli Rust binary, Stage 2 builds NestJS app, final image has both
- `.dockerignore` excludes node_modules, dist, target, .git
- `railway.json` at repo root with Dockerfile builder, health check path, restart policy
- `backend/scripts/start.sh` runs `prisma migrate deploy` then `node dist/main`
- `backend/package.json` updated with `supertest` + `@types/supertest` devDependencies
- Production CORS uses `FRONTEND_URL` env var instead of `origin: true`
- E2E tests cover: health, readiness, contracts/source (valid + invalid), contracts/compile, vibe-score, analysis/error, paymaster/status (401 without JWT)
- All 97 existing unit tests still pass (no regressions)

## Proof Level

- This slice proves: operational
- Real runtime required: yes (Docker build, E2E test runner)
- Human/UAT required: no

## Verification

- `cd backend && npm run test:e2e` — all E2E tests pass (8+ test cases)
- `cd backend && npm test` — 97 unit tests still pass (14 suites, no regressions)
- `docker build -f backend/Dockerfile -t monad-backend .` — exits 0 from repo root (if Docker available; skip gracefully if not)
- `test -f backend/Dockerfile && test -f backend/.dockerignore && test -f railway.json && test -f backend/scripts/start.sh` — all deployment files exist
- `grep -q 'FRONTEND_URL\|frontend' backend/src/main.ts` — CORS uses FRONTEND_URL
- `docker run --rm monad-backend:latest sh -c 'test -x /app/monad-cli && test -d /app/contracts && test -d /app/data && echo OK'` — Docker image has correct runtime layout (if Docker available; skip gracefully if not)
- `grep -q 'CORS origin' backend/src/main.ts` — CORS origin is logged at bootstrap for runtime diagnostics

## Observability / Diagnostics

- Runtime signals: NestJS bootstrap log with port, Prisma migration log on startup, CORS origin logged
- Inspection surfaces: `GET /api/health` health check endpoint, `GET /api/health/readiness` readiness probe
- Failure visibility: Docker build errors surface in build log, E2E test failures show HTTP status + response body via supertest assertions
- Redaction constraints: DATABASE_URL, MONAD_PRIVATE_KEY, JWT_SECRET, GEMINI_API_KEY must never appear in logs or Dockerfile layers

## Integration Closure

- Upstream surfaces consumed: All NestJS modules from S01-S05 (AppModule, PrismaService, HealthModule, ContractsModule, AuthModule, AnalysisModule, PaymasterModule, EngineModule, VibeScoreModule), Rust CLI workspace (Cargo.toml, crates/)
- New wiring introduced in this slice: Dockerfile multi-stage build composing Rust + Node.js, `scripts/start.sh` startup sequence, production CORS configuration, railway.json deployment config
- What remains before the milestone is truly usable end-to-end: Railway environment variables must be configured in Railway dashboard (DATABASE_URL, MONAD_PRIVATE_KEY, etc.), initial Prisma migration must be created (`npx prisma migrate dev --name init`), frontend deployed separately with NEXT_PUBLIC_API_URL pointing to Railway backend

## Tasks

- [x] **T01: Create multi-stage Dockerfile, Railway config, start script, and production CORS** `est:1h`
  - Why: The Dockerfile is the hardest piece — it proves Rust + Node.js binary bundling works. The start script, railway.json, .dockerignore, and CORS refinement are tightly coupled to deployment and should be done together.
  - Files: `backend/Dockerfile`, `backend/.dockerignore`, `railway.json`, `backend/scripts/start.sh`, `backend/src/main.ts`, `backend/package.json`, `backend/.env.example`
  - Do: Build multi-stage Dockerfile (rust:1.82-slim builder → node:20-slim runtime). Context is repo root. Stage 1 copies Cargo.toml, Cargo.lock, crates/ and runs `cargo build --release -p monad-cli`. Stage 2 copies backend/, runs npm ci, prisma generate, nest build, then copies the monad-cli binary and sets ENGINE_BINARY_PATH=/app/monad-cli. Create .dockerignore. Create railway.json at repo root with dockerfilePath: backend/Dockerfile. Create scripts/start.sh that runs prisma migrate deploy then node dist/main. Update main.ts CORS to use FRONTEND_URL in production. Update start:prod script. Add FRONTEND_URL to .env.example if missing.
  - Verify: `test -f backend/Dockerfile && test -f backend/.dockerignore && test -f railway.json && test -f backend/scripts/start.sh` && `cd backend && npm test` (97 tests pass) && `docker build -f backend/Dockerfile -t monad-backend . 2>&1 | tail -5` (if Docker available)
  - Done when: All deployment files exist, CORS uses FRONTEND_URL conditionally, unit tests pass without regressions

- [x] **T02: Write E2E test suite with supertest and mocked externals** `est:45m`
  - Why: E2E tests validate the real NestJS HTTP layer (routing, guards, pipes, interceptors) with the full AppModule — something unit tests don't cover. This directly satisfies R010.
  - Files: `backend/test/app.e2e-spec.ts`, `backend/package.json`
  - Do: Install supertest + @types/supertest. Write E2E test file that boots the full AppModule via Test.createTestingModule, overriding PrismaService (mock $queryRaw, $connect, model methods) and EngineService (return null → heuristic fallback). Use supertest to test: GET /api/health (200), GET /api/health/readiness (200), GET /api/contracts/source?type=FixedContract (200 with source), GET /api/contracts/source?type=Invalid (400), POST /api/contracts/compile (200 with bytecode/abi using real solc), POST /api/vibe-score (200 with score via heuristic fallback), POST /api/analysis/error (200 with analysis result), GET /api/paymaster/status (401 without JWT).
  - Verify: `cd backend && npm run test:e2e` passes all tests && `cd backend && npm test` still passes 97 unit tests
  - Done when: `npm run test:e2e` exits 0 with 8+ test cases passing, `npm test` still shows 97 passing tests

## Files Likely Touched

- `backend/Dockerfile`
- `backend/.dockerignore`
- `railway.json`
- `backend/scripts/start.sh`
- `backend/src/main.ts`
- `backend/package.json`
- `backend/.env.example`
- `backend/test/app.e2e-spec.ts`
