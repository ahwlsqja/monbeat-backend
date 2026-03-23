# S06: Railway Deploy + E2E Validation — Research

**Date:** 2026-03-22

## Summary

S06 is the final slice: create a Dockerfile for the NestJS backend (with embedded Rust CLI binary), configure Railway deployment, add E2E integration tests using supertest, and ensure CORS is correctly configured. The codebase is well-structured — `backend/` is an independent NestJS project, `frontend/` is an independent Next.js project, and `crates/cli/` produces the `monad-cli` Rust binary. All modules (health, contracts, auth, analysis, paymaster, engine, vibe-score) are in place with 97 unit tests across 14 suites.

This is **targeted research** — the technologies (Docker multi-stage builds, NestJS E2E testing with supertest, Railway deployment) are well-known, and the codebase patterns are established. The main work is: (1) Dockerfile with Rust + Node.js multi-stage build, (2) Railway configuration, (3) supertest E2E tests validating the API surface, and (4) CORS refinement for production.

## Recommendation

Build in three tasks:

1. **Dockerfile + Railway config** — Multi-stage Dockerfile (Rust builder → Node.js runtime), railway.json, .dockerignore. This is the highest-risk item because it must correctly embed the monad-cli binary and handle Prisma client generation.
2. **E2E test suite** — Install supertest, write E2E tests for the core API endpoints (health, contracts/source, contracts/compile, vibe-score). These tests use `@nestjs/testing` to create a real NestJS app instance with mocked external services (Prisma, ethers, engine binary).
3. **CORS + env var finalization** — Refine CORS to accept the specific frontend origin (from `FRONTEND_URL` env var), add `FRONTEND_URL` to Railway env configuration, ensure `start:prod` script works with Prisma migrations.

## Implementation Landscape

### Key Files

**Existing — no changes needed:**
- `backend/src/main.ts` — Already has `app.enableCors({ origin: true })`, `app.setGlobalPrefix('api')`, health check. Port from `PORT` env var.
- `backend/src/config/configuration.ts` — All env vars mapped: DATABASE_URL, PORT, MONAD_RPC_URL, MONAD_PRIVATE_KEY, GITHUB_CLIENT_ID/SECRET, JWT_SECRET, GEMINI_API_KEY, ENGINE_BINARY_PATH, FRONTEND_URL.
- `backend/src/health/health.controller.ts` — Health + readiness endpoints using Terminus + Prisma health indicator.
- `backend/prisma/schema.prisma` — User, Deployment, Analysis, VibeScore models. No migrations directory yet.
- `backend/nest-cli.json` — Builder set to `tsc` (not webpack). `deleteOutDir: true`.
- `backend/test/jest-e2e.json` — E2E test config exists, pattern `.e2e-spec.ts$`.

**To create:**
- `backend/Dockerfile` — Multi-stage: (1) Rust builder stage compiles monad-cli binary, (2) Node.js stage installs deps, runs Prisma generate, builds NestJS, copies binary. Final runtime image has Node.js + monad-cli.
- `backend/.dockerignore` — Exclude node_modules, dist, .git, target, etc.
- `backend/railway.json` — Build command, start command, health check path, env var references.
- `backend/test/app.e2e-spec.ts` — E2E tests using supertest.
- `backend/scripts/start.sh` — Start script that runs `prisma migrate deploy` then `node dist/main`.

**To modify:**
- `backend/package.json` — Add `supertest` + `@types/supertest` as devDependencies. Update `start:prod` script if needed.
- `backend/src/main.ts` — Refine CORS to use FRONTEND_URL instead of `origin: true` in production.

### Build Order

**Task 1: Dockerfile + Railway config (risk: medium)**
Build the multi-stage Dockerfile first — this is the hardest piece and proves the Rust + Node.js binary bundling works. Verify with `docker build`. Then add railway.json and .dockerignore.

Key Dockerfile design:
```
Stage 1: rust:1.XX-slim → cargo build --release -p monad-cli → produces /target/release/monad-cli
Stage 2: node:20-slim → COPY backend/ → npm ci → npx prisma generate → npm run build → COPY --from=stage1 monad-cli → COPY contracts/ data/
```

Critical details:
- The Rust build needs the full `crates/` workspace (cli depends on types, state, scheduler, evm, mv-state, precompiles, nine-fork). The Dockerfile context must include the Cargo workspace root.
- `process.cwd()` in ContractsController resolves to the Docker WORKDIR. The `contracts/` and `data/` directories must be COPY'd into the image at the correct relative path.
- `ENGINE_BINARY_PATH` env var must point to where monad-cli lands in the container (e.g., `/app/monad-cli`).
- Prisma client is generated at build time (`npx prisma generate`), but migrations run at startup (`prisma migrate deploy`).
- The `backend/tsconfig.json` has `rootDir: ./src` so `nest build` outputs to `backend/dist/`.

Railway config (railway.json):
```json
{
  "$schema": "https://railway.com/railway.schema.json",
  "build": { "builder": "DOCKERFILE", "dockerfilePath": "backend/Dockerfile" },
  "deploy": { "healthcheckPath": "/api/health", "restartPolicyType": "ON_FAILURE" }
}
```

Note: Railway uses the repo root as Docker context. The Dockerfile must handle paths relative to repo root.

**Task 2: E2E test suite (risk: low)**
Install supertest. Write E2E tests that boot a real NestJS app instance via `Test.createTestingModule` with the full AppModule but override PrismaService and external services. Test:
- `GET /api/health` → 200 with health check response
- `GET /api/health/readiness` → 200
- `GET /api/contracts/source?type=FixedContract` → 200 with source
- `GET /api/contracts/source?type=Invalid` → 400
- `POST /api/contracts/compile` → 200 with bytecode/abi (uses real solc)
- `POST /api/vibe-score` → 200 with score (engine unavailable → heuristic fallback)
- `POST /api/analysis/error` → 200 with analysis result (Gemini unavailable → heuristic fallback)
- `GET /api/paymaster/status` → 401 without JWT

The E2E test config at `backend/test/jest-e2e.json` already exists with pattern `.e2e-spec.ts$`.

PrismaService must be overridden in E2E tests since there's no DB. Override with a mock that stubs `$queryRaw` (health check) and model operations.

**Task 3: CORS + production polish (risk: low)**
- Update `main.ts` CORS to conditionally use `FRONTEND_URL` in production, keep `origin: true` in dev.
- Add a startup script (`scripts/start.sh`) that runs `npx prisma migrate deploy && node dist/main`.
- Update package.json `start:prod` to use the startup script.
- Verify `FRONTEND_URL` is in the env var template.

### Verification Approach

1. **Dockerfile builds**: `docker build -f backend/Dockerfile -t monad-backend .` from repo root exits 0.
2. **E2E tests pass**: `cd backend && npm run test:e2e` exits 0. Tests use supertest against the real NestJS app with mocked externals.
3. **Backend unit tests stable**: `cd backend && npm test` still passes all 97 tests (no regressions).
4. **Frontend build stable**: `cd frontend && npm run build` still passes.
5. **Health check reachable**: `docker run -e DATABASE_URL=... monad-backend` → `curl localhost:3000/api/health` returns 200.
6. **monad-cli binary works in container**: `docker run monad-backend /app/monad-cli` with JSON stdin produces valid output.

## Constraints

- **Docker build context must be repo root** — The Rust workspace is at the repo root (`Cargo.toml`, `crates/`), while the NestJS app is in `backend/`. The Dockerfile must be in `backend/` but the build context is `../` (or use `-f backend/Dockerfile .` from root). Railway's `dockerfilePath` handles this.
- **No Prisma migrations exist yet** — `backend/prisma/migrations/` directory is empty. The first `prisma migrate deploy` on Railway will need an initial migration created via `prisma migrate dev`. The Dockerfile should include `prisma generate` only; migration creation is a manual step before first deploy.
- **`process.cwd()` dependency** — ContractsController and AnalysisService use `process.cwd()` to find `contracts/` and `data/monad-docs/`. In Docker, WORKDIR must be set to the directory containing these folders. Set WORKDIR to `/app` and copy `contracts/` and `data/` alongside `dist/`.
- **supertest not installed** — Must add `supertest` and `@types/supertest` to devDependencies.
- **Railway uses PORT env var** — Already handled in `configuration.ts`: `parseInt(process.env.PORT ?? '3000', 10)`.

## Common Pitfalls

- **Prisma generate vs migrate in Docker** — `prisma generate` creates the client library (needed at build time). `prisma migrate deploy` runs DDL against the real database (needed at runtime). Confusing these causes either build failures or runtime "table not found" errors. In the Dockerfile: `generate` at build. In the start script: `migrate deploy` at startup.
- **Rust builder stage size** — A full `cargo build --release` with all workspace members produces a large builder layer. Use `rust:1.82-slim` (not `rust:1.82`) and install only `build-essential` if needed. The final stage doesn't include the Rust toolchain — just the compiled binary (~15MB stripped).
- **Missing contracts/data in Docker** — If `COPY` doesn't bring `contracts/` and `data/monad-docs/` into the runtime image, `GET /api/contracts/source` returns 400 and analysis uses empty RAG context. The Dockerfile must explicitly copy these.
- **E2E test Prisma mock** — The HealthController's readiness endpoint calls `PrismaService.$queryRaw`. The E2E test must override PrismaService with a mock that resolves `$queryRaw`, otherwise the test hangs waiting for a DB connection.

## Open Risks

- **Railway Rust build time** — Compiling the full Rust workspace from scratch on Railway's builders could take 5-10 minutes. Railway has no Rust build cache by default. Subsequent builds should be faster with Docker layer caching if only backend/ files change.
- **Initial Prisma migration** — No migration files exist. Before the first Railway deploy, someone must run `npx prisma migrate dev --name init` locally to generate the migration SQL. Without this, `prisma migrate deploy` in the start script has nothing to apply and tables won't exist.
- **FRONTEND_URL chicken-and-egg** — The frontend needs `NEXT_PUBLIC_API_URL` (backend URL) at build time, and the backend needs `FRONTEND_URL` (frontend URL) for OAuth redirect. On Railway, both are known after initial deploy. First deploy may need manual env var updates after URLs are assigned.
