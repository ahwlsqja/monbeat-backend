---
id: T01
parent: S01
milestone: M001
provides:
  - NestJS backend project scaffold with package.json, tsconfig, nest-cli config
  - ConfigModule with typed configuration factory (port, database.url, nodeEnv)
  - Prisma schema with User, Deployment, Analysis, VibeScore models targeting PostgreSQL
  - PrismaService extending PrismaClient with OnModuleInit ($connect)
  - Global PrismaModule exporting PrismaService for all downstream modules
  - Bootstrap entry point (main.ts) with /api prefix, CORS, shutdown hooks
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
  - backend/.env.example
key_decisions:
  - "Used tsc builder instead of webpack in nest-cli.json to avoid ts-loader dependency and deleteOutDir/incremental conflict"
  - "Set incremental: false in tsconfig.json because deleteOutDir: true in nest-cli.json conflicts with tsc incremental builds (cache thinks nothing changed after dist is deleted)"
patterns_established:
  - "NestJS project lives in backend/ to avoid conflicting with root Rust workspace"
  - "ConfigModule is isGlobal: true with typed config factory in src/config/configuration.ts"
  - "PrismaModule is @Global() so any module can inject PrismaService without importing"
  - "All Prisma text-heavy fields use @db.Text; all models use cuid() IDs with createdAt/updatedAt"
observability_surfaces:
  - "NestJS Logger on bootstrap: 'Listening on port ${port}'"
  - "PrismaService logs: 'Connecting to database...' and 'Database connection established'"
  - "Build health: npm run build exits 0; npx prisma generate exits 0"
  - "Config test: npx ts-node -e \"import('./src/config/configuration').then(m => console.log('config OK'))\""
duration: ~15m
verification_result: passed
completed_at: 2026-03-22
blocker_discovered: false
---

# T01: Scaffold NestJS project with ConfigModule and Prisma data layer

**Created backend/ NestJS project with ConfigModule, Prisma ORM (4 PostgreSQL models), and bootstrap entry point — all builds and generates clean**

## What Happened

Created the `backend/` directory with a complete NestJS project scaffold. Wrote `package.json` with all required NestJS core dependencies (@nestjs/core, @nestjs/common, @nestjs/platform-express, @nestjs/config, @prisma/client, reflect-metadata, rxjs, class-validator, class-transformer) and dev dependencies (@nestjs/cli, @nestjs/testing, typescript, ts-node, ts-jest, jest, prisma, types). Configured TypeScript with strict mode, decorator support (experimentalDecorators, emitDecoratorMetadata), ES2021 target, and commonjs modules.

Wrote `main.ts` bootstrapping the NestJS app with `setGlobalPrefix('api')`, `enableCors({ origin: true, credentials: true })`, `enableShutdownHooks()`, and listening on the configured PORT (default 3000). The `AppModule` imports `ConfigModule.forRoot({ isGlobal: true })` with a typed configuration factory and `PrismaModule`.

The Prisma schema defines 4 models: User (githubId @unique, username, email, avatarUrl, deployCount), Deployment (userId FK, contractName, contractSource @db.Text, address, txHash, network, status, errorMessage, gasUsed), Analysis (userId FK, contractSource, errorMessage, fixedCode, explanation, category, isMonadSpecific), and VibeScore (optional userId FK, contractSource, score, suggestions Json, engineBased, conflicts, reExecutions, gasEfficiency). All models use cuid IDs with appropriate indexes on userId FKs.

PrismaService extends PrismaClient with OnModuleInit that calls `$connect()` with diagnostic logging. PrismaModule is `@Global()` so downstream modules can inject PrismaService without explicit imports.

Hit one notable issue: `nest build` with `deleteOutDir: true` and `incremental: true` in tsconfig caused silent build failures — tsc's incremental cache thought nothing changed after dist was deleted, so it emitted no files. Fixed by setting `incremental: false` and configuring `builder: "tsc"` in nest-cli.json (the default webpack builder required ts-loader which wasn't installed).

## Verification

All 8 task-level verification checks pass. All 6 applicable slice-level checks pass (the `npm test` check requires T02's unit tests).

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd backend && npm run build` | 0 | ✅ pass | 2.9s |
| 2 | `cd backend && npx prisma generate` | 0 | ✅ pass | 1.4s |
| 3 | `test -f backend/dist/main.js` | 0 | ✅ pass | <1s |
| 4 | `test -d backend/node_modules/.prisma/client` | 0 | ✅ pass | <1s |
| 5 | `grep -q "setGlobalPrefix" backend/src/main.ts` | 0 | ✅ pass | <1s |
| 6 | `grep -q "enableCors" backend/src/main.ts` | 0 | ✅ pass | <1s |
| 7 | `grep -q "enableShutdownHooks" backend/src/main.ts` | 0 | ✅ pass | <1s |
| 8 | `grep -q "postgresql" backend/prisma/schema.prisma` | 0 | ✅ pass | <1s |
| S1 | `cd backend && npm run build` (slice) | 0 | ✅ pass | 2.9s |
| S3 | `cd backend && npx prisma generate` (slice) | 0 | ✅ pass | 1.4s |
| S4 | `npx ts-node -e "import('./src/config/configuration')..."` | 0 | ✅ pass | <2s |
| S5 | `grep -q "setGlobalPrefix" backend/src/main.ts` (slice) | 0 | ✅ pass | <1s |
| S6 | `grep -q "enableCors" backend/src/main.ts` (slice) | 0 | ✅ pass | <1s |
| S7 | `grep -q "enableShutdownHooks" backend/src/main.ts` (slice) | 0 | ✅ pass | <1s |
| S2 | `cd backend && npm test` (slice — requires T02) | — | ⏳ pending | — |

## Diagnostics

- **Build health:** `cd backend && npm run build` — exit 0 means TypeScript compiles clean
- **Schema health:** `cd backend && npx prisma generate` — exit 0 means Prisma schema is valid and client generated
- **Config health:** `cd backend && npx ts-node -e "import('./src/config/configuration').then(m => console.log('config OK'))"` — prints "config OK"
- **Runtime bootstrap:** Requires DATABASE_URL to be set; PrismaService will log connection status on startup
- **Failure signals:** Missing DATABASE_URL → PrismaService.$connect() throws connection error; TypeScript errors → npm run build exits non-zero

## Deviations

- Set `incremental: false` in tsconfig.json (plan didn't specify) — required because `deleteOutDir: true` in nest-cli.json conflicts with tsc incremental builds, causing silent no-emit
- Added `"builder": "tsc"` to nest-cli.json compilerOptions — the default webpack builder requires ts-loader which was not in the dependency list and is unnecessary for a pure tsc build
- Added Observability Impact section to T01-PLAN.md as required by pre-flight check

## Known Issues

None.

## Files Created/Modified

- `backend/package.json` — NestJS project manifest with all core + Prisma + dev dependencies
- `backend/tsconfig.json` — TypeScript config with strict mode, decorators, ES2021 target
- `backend/tsconfig.build.json` — Build-specific config excluding test files
- `backend/nest-cli.json` — NestJS CLI config with tsc builder and deleteOutDir
- `backend/src/main.ts` — Bootstrap entry point with /api prefix, CORS, shutdown hooks
- `backend/src/app.module.ts` — Root module importing ConfigModule (global) and PrismaModule
- `backend/src/config/configuration.ts` — Typed config factory (port, database.url, nodeEnv)
- `backend/prisma/schema.prisma` — PostgreSQL schema with User, Deployment, Analysis, VibeScore models
- `backend/src/prisma/prisma.service.ts` — PrismaService extending PrismaClient with OnModuleInit
- `backend/src/prisma/prisma.module.ts` — Global PrismaModule exporting PrismaService
- `backend/.env.example` — Environment variable documentation with all current and future keys
- `.gsd/milestones/M001/slices/S01/tasks/T01-PLAN.md` — Added Observability Impact section
