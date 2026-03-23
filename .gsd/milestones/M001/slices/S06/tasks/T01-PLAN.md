---
estimated_steps: 6
estimated_files: 7
---

# T01: Create multi-stage Dockerfile, Railway config, start script, and production CORS

**Slice:** S06 — Railway Deploy + E2E Validation
**Milestone:** M001

## Description

Create all deployment infrastructure: a multi-stage Dockerfile that compiles the monad-cli Rust binary and builds the NestJS app into a single runtime image, Railway deployment configuration, a startup script that handles Prisma migrations, and production-ready CORS configuration. This is the highest-risk task because it must correctly compose the Rust toolchain with the Node.js application, handle `process.cwd()` dependencies for `contracts/` and `data/` directories, and produce a working Docker image.

**Key architecture constraints:**
- Docker build context is the **repo root** (not `backend/`). Railway's `dockerfilePath` points to `backend/Dockerfile`.
- The Rust workspace is at repo root: `Cargo.toml`, `Cargo.lock`, `crates/` (cli depends on types, state, scheduler, evm, mv-state, precompiles, nine-fork).
- `process.cwd()` in ContractsController reads from `contracts/` and `data/monad-docs/` relative to CWD. In Docker, WORKDIR must contain these directories.
- `contracts/` and `data/` are inside `backend/` (not repo root) — `backend/contracts/FixedContract.sol`, `backend/data/monad-docs/*.md`.
- Prisma client generation happens at build time (`npx prisma generate`), migrations at runtime (`prisma migrate deploy`).
- The `backend/tsconfig.json` has `rootDir: ./src` so `nest build` outputs to `backend/dist/`.
- `ENGINE_BINARY_PATH` env var must point to where monad-cli lands in the container (e.g., `/app/monad-cli`).

**Relevant skills:** None specific needed — standard Docker/NestJS patterns.

## Steps

1. **Create `backend/Dockerfile`** — Multi-stage build:
   - Stage 1 (`builder-rust`): `FROM rust:1.82-slim AS builder-rust`. WORKDIR `/build`. COPY `Cargo.toml`, `Cargo.lock`, `crates/` from repo root. Run `cargo build --release -p monad-cli`. Output: `/build/target/release/monad-cli`.
   - Stage 2 (`builder-node`): `FROM node:20-slim AS builder-node`. WORKDIR `/app`. COPY `backend/package.json`, `backend/package-lock.json`. Run `npm ci --ignore-scripts`. COPY `backend/` (source, prisma schema, contracts, data). Run `npx prisma generate` then `npm run build`.
   - Stage 3 (runtime): `FROM node:20-slim`. WORKDIR `/app`. Copy `node_modules/`, `dist/`, `prisma/`, `contracts/`, `data/`, `scripts/`, `package.json` from builder-node. COPY `--from=builder-rust /build/target/release/monad-cli /app/monad-cli`. Set `ENV ENGINE_BINARY_PATH=/app/monad-cli`. EXPOSE 3000. CMD `["sh", "scripts/start.sh"]`.
   - Install `openssl` in runtime stage if needed by Prisma.

2. **Create `backend/.dockerignore`** — Exclude: `node_modules/`, `dist/`, `.git/`, `target/`, `*.md`, `.env`, `.env.*`, `test/`, `coverage/`, `.eslintrc.js`, `.prettierrc`.

3. **Create `railway.json`** at repo root:
   ```json
   {
     "$schema": "https://railway.com/railway.schema.json",
     "build": { "builder": "DOCKERFILE", "dockerfilePath": "backend/Dockerfile" },
     "deploy": { "healthcheckPath": "/api/health", "restartPolicyType": "ON_FAILURE" }
   }
   ```

4. **Create `backend/scripts/start.sh`** — Executable script:
   ```bash
   #!/bin/sh
   set -e
   echo "Running Prisma migrations..."
   npx prisma migrate deploy
   echo "Starting NestJS server..."
   node dist/main
   ```

5. **Update `backend/src/main.ts`** — Change CORS from `origin: true` to:
   ```typescript
   const configService = app.get(ConfigService);
   const frontendUrl = configService.get<string>('frontend.url');
   const nodeEnv = configService.get<string>('nodeEnv');
   
   app.enableCors({
     origin: nodeEnv === 'production' ? frontendUrl : true,
     credentials: true,
   });
   ```
   This keeps `origin: true` (allow all) in development, but restricts to `FRONTEND_URL` in production.

6. **Update `backend/package.json`** — Change `start:prod` to `"sh scripts/start.sh"`. Add `FRONTEND_URL` to `backend/.env.example` if not already present (check first — it may already be there from S05 configuration.ts).

## Must-Haves

- [ ] Dockerfile builds successfully with `docker build -f backend/Dockerfile -t monad-backend .` from repo root (or structure is correct if Docker unavailable)
- [ ] Rust CLI binary is at `/app/monad-cli` in the final image
- [ ] `contracts/` and `data/` directories are in the runtime image at the correct path relative to WORKDIR
- [ ] `railway.json` references `backend/Dockerfile` and has health check at `/api/health`
- [ ] `scripts/start.sh` runs prisma migrate deploy before node dist/main
- [ ] CORS in main.ts uses FRONTEND_URL in production mode, allows all origins in development
- [ ] All 97 existing unit tests pass after main.ts change

## Verification

- `test -f backend/Dockerfile && test -f backend/.dockerignore && test -f railway.json && test -f backend/scripts/start.sh` — all files exist
- `grep -q 'frontend' backend/src/main.ts` — CORS references frontend config
- `grep -q 'prisma migrate deploy' backend/scripts/start.sh` — start script runs migrations
- `grep -q 'dockerfilePath' railway.json` — Railway config references Dockerfile
- `cd backend && npm test` — 97 tests pass (no regressions from main.ts CORS change)
- If Docker is available: `docker build -f backend/Dockerfile -t monad-backend .` exits 0

## Inputs

- `backend/src/main.ts` — Current main.ts with `app.enableCors({ origin: true })` to be refined
- `backend/package.json` — Current package.json with `start:prod` script to update
- `backend/src/config/configuration.ts` — Already has `frontend.url` and `nodeEnv` config mapped
- `Cargo.toml` — Rust workspace definition at repo root
- `Cargo.lock` — Lockfile for reproducible Rust builds
- `crates/cli/Cargo.toml` — CLI crate depending on types, state, scheduler
- `backend/prisma/schema.prisma` — Prisma schema for client generation
- `backend/.env.example` — Env var documentation to update with FRONTEND_URL

## Expected Output

- `backend/Dockerfile` — Multi-stage Dockerfile (Rust builder + Node.js builder + runtime)
- `backend/.dockerignore` — Docker ignore file excluding build artifacts
- `railway.json` — Railway deployment configuration at repo root
- `backend/scripts/start.sh` — Startup script with prisma migrate + node dist/main
- `backend/src/main.ts` — Updated with production CORS using FRONTEND_URL
- `backend/package.json` — Updated start:prod script
- `backend/.env.example` — Updated with FRONTEND_URL entry if missing

## Observability Impact

- **New signal — CORS origin log**: Bootstrap now logs `CORS origin: <value>` showing whether production FRONTEND_URL or development wildcard is active. Inspect via container stdout.
- **New signal — Prisma migration log**: `scripts/start.sh` emits "Running Prisma migrations..." before `prisma migrate deploy`, making migration step visible in deploy logs.
- **New inspection surface**: `docker run --rm monad-backend:latest sh -c 'ls /app/'` validates runtime image layout (monad-cli, contracts/, data/, dist/).
- **Failure visibility**: Docker build failures surface at the exact stage (Rust compile vs Node build vs runtime copy) via Docker build log. Missing Rust MSRV shows as `error: rustc X.Y is not supported`.
- **Redaction**: No secrets baked into Dockerfile layers — all sensitive values are runtime env vars only.
