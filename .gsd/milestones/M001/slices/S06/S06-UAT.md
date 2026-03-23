# S06: Railway Deploy + E2E Validation — UAT

**Milestone:** M001
**Written:** 2026-03-22

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: This slice produces deployment infrastructure files and an E2E test suite. Verification is: files exist with correct content, Docker builds, and all 109 tests pass. No live Railway deployment is required for slice completion — Railway env var configuration is a separate operational step.

## Preconditions

- Working directory is the M001 worktree with all S01–S06 changes applied
- Node.js 20+ installed, `npm ci` completed in `backend/`
- Docker available (for Docker build verification; skip gracefully if not)
- No running PostgreSQL required (E2E tests mock PrismaService)

## Smoke Test

```bash
cd backend && npm run test:e2e && npm test
```
Expected: 12 E2E tests pass + 97 unit tests pass = 109 total, zero failures.

## Test Cases

### 1. Deployment files exist

1. Run `test -f backend/Dockerfile && test -f backend/.dockerignore && test -f railway.json && test -f backend/scripts/start.sh`
2. **Expected:** Exit code 0 — all four deployment files present

### 2. Dockerfile is multi-stage with Rust + Node

1. Run `grep -c 'FROM' backend/Dockerfile`
2. **Expected:** Output is `3` (three FROM stages: rust builder, node builder, runtime)
3. Run `grep 'rust:1.88' backend/Dockerfile`
4. **Expected:** Match found — Rust builder uses 1.88-slim

### 3. Railway config references Dockerfile

1. Run `cat railway.json`
2. **Expected:** JSON with `dockerfilePath: "backend/Dockerfile"`, healthcheckPath `/api/health`, restartPolicyType `ON_FAILURE`

### 4. Start script runs Prisma migration before server

1. Run `cat backend/scripts/start.sh`
2. **Expected:** Contains `prisma migrate deploy` before `node dist/main`
3. Run `test -x backend/scripts/start.sh || stat --format='%a' backend/scripts/start.sh`
4. **Expected:** Script has execute permission (755 or similar)

### 5. CORS uses FRONTEND_URL in production

1. Run `grep -A5 'CORS' backend/src/main.ts`
2. **Expected:** Production path reads `FRONTEND_URL` from config/env. Development path allows all origins. CORS origin value is logged at bootstrap.

### 6. E2E tests pass (12 cases)

1. Run `cd backend && npm run test:e2e`
2. **Expected:** 12/12 tests pass:
   - GET /api/health → 200
   - GET /api/health/readiness → 200
   - GET /api/contracts/source?type=FixedContract → 200 with Solidity source
   - GET /api/contracts/source?type=Invalid → 400
   - POST /api/contracts/compile (valid source) → 201 with bytecode + abi
   - POST /api/contracts/compile (invalid source) → 400
   - POST /api/contracts/compile (missing body) → 400
   - POST /api/vibe-score (valid source) → 201 with numeric score
   - POST /api/vibe-score (missing body) → 400
   - POST /api/analysis/error (revert error) → 201 with analysis
   - POST /api/analysis/error (missing body) → 400
   - GET /api/paymaster/status (no JWT) → 401

### 7. Unit tests pass without regressions (97 cases)

1. Run `cd backend && npm test`
2. **Expected:** 97/97 tests pass across 14 suites, zero failures

### 8. Docker build succeeds (skip if Docker unavailable)

1. Run `docker build -f backend/Dockerfile -t monad-backend .` from repo root
2. **Expected:** Exit code 0, final image tagged `monad-backend:latest`
3. Run `docker run --rm monad-backend:latest sh -c 'test -x /app/monad-cli && test -d /app/contracts && test -d /app/data && echo OK'`
4. **Expected:** Output `OK` — runtime image has monad-cli binary (executable), contracts/ and data/ directories

### 9. .env.example includes FRONTEND_URL

1. Run `grep 'FRONTEND_URL' backend/.env.example`
2. **Expected:** Match found with default value (e.g., `http://localhost:3001`)

### 10. supertest installed as devDependency

1. Run `grep 'supertest' backend/package.json`
2. **Expected:** Both `supertest` and `@types/supertest` present in devDependencies

## Edge Cases

### Docker build with stale Rust cache
1. Run `docker build --no-cache -f backend/Dockerfile -t monad-backend .` from repo root
2. **Expected:** Clean build succeeds (takes ~4-5 minutes). If alloy/revm crates bump MSRV, failure message names exact required Rust version.

### E2E test isolation
1. Run `cd backend && npm run test:e2e` twice in a row
2. **Expected:** Both runs pass — tests don't leak state between runs (mocked PrismaService has no persistent state)

### POST endpoints return 201 not 200
1. Check E2E test expectations for POST routes
2. **Expected:** All POST endpoints expect 201 (NestJS default), not 200

## Failure Signals

- `npm run test:e2e` fails: check E2E test output for expected vs actual HTTP status and response body diff. Common cause: new Prisma model methods need mock entries in `app.e2e-spec.ts`.
- `npm test` regression: compare test count — must be exactly 97. If lower, a source change broke an existing test.
- Docker build fails at Stage 1 (Rust): read error for MSRV requirement, update `FROM rust:` version in Dockerfile.
- Docker build fails at Stage 2 (Node): likely `npm ci` issue — check `package-lock.json` is committed.
- Docker runtime test fails: check that Dockerfile COPY commands include `contracts/` and `data/` from repo root.

## Not Proven By This UAT

- **Live Railway deployment**: The Docker image and config files are validated but no actual Railway deploy occurs in this UAT. Railway env vars (DATABASE_URL, secrets) must be configured in Railway dashboard separately.
- **Production Prisma migration**: `prisma migrate deploy` in start.sh is validated by file content inspection, not by running against a real PostgreSQL.
- **Frontend-to-backend connectivity**: CORS config is validated by code inspection and E2E tests, but real cross-origin requests from the deployed Next.js frontend are not tested here.
- **Live monad-core engine execution in Docker**: E2E tests use heuristic fallback (EngineService returns null in test env). The Docker image has the binary but actual parallel EVM execution is not E2E-tested.

## Notes for Tester

- Docker build takes ~4-5 minutes on first run (Rust compilation). Subsequent cached builds are much faster.
- E2E tests don't need a database — PrismaService is fully mocked. No `docker-compose` or PostgreSQL setup required.
- The `scripts/start.sh` is designed for Railway runtime, not local dev. Local dev still uses `npm run start:dev`.
- All POST E2E tests expect 201 status (NestJS default), not 200. This is correct behavior.
