---
estimated_steps: 5
estimated_files: 11
---

# T01: Scaffold NestJS project with ConfigModule and Prisma data layer

**Slice:** S01 — NestJS Foundation + Database
**Milestone:** M001

## Description

Create the `backend/` NestJS project from scratch with all core dependencies, TypeScript configuration, ConfigModule for environment variable management, and Prisma ORM with a PostgreSQL schema defining User, Deployment, Analysis, and VibeScore models. This is the foundational task — every downstream slice (S02–S06) imports PrismaModule and ConfigModule from this work.

The NestJS project lives in `backend/` at the worktree root to avoid conflicting with the Rust workspace (Cargo.toml) at root level.

**Relevant skills:** The executor should load `react-best-practices` is NOT relevant here. No installed skills directly cover NestJS/Prisma — use the NestJS docs patterns described below.

## Steps

1. **Create `backend/` directory and `package.json`** — Initialize with NestJS core deps: `@nestjs/core@^10`, `@nestjs/common@^10`, `@nestjs/platform-express@^10`, `@nestjs/config@^3`, `prisma@^5`, `@prisma/client@^5`, `reflect-metadata`, `rxjs@^7`, `class-validator`, `class-transformer`. Dev deps: `@nestjs/cli@^10`, `@nestjs/testing@^10`, `typescript@^5.3`, `ts-node`, `ts-jest@^29`, `jest@^29`, `@types/jest`, `@types/node@^20`, `@types/express`, `source-map-support`. Scripts: `build`, `start`, `start:dev`, `start:prod`, `test`, `test:e2e`, `test:cov`, `lint`.

2. **Write TypeScript and NestJS configuration files** — `tsconfig.json` (target ES2021, module commonjs, strict, experimentalDecorators, emitDecoratorMetadata, outDir dist, rootDir src). `tsconfig.build.json` extends tsconfig.json with exclude for test files. `nest-cli.json` with sourceRoot "src" and compilerOptions.

3. **Write `src/main.ts` and `src/app.module.ts`** — main.ts: NestFactory.create(AppModule), setGlobalPrefix('api'), enableCors({ origin: true, credentials: true }), enableShutdownHooks(), listen on configService.get('PORT') || 3000, log "Listening on port ${port}". AppModule: imports ConfigModule.forRoot({ isGlobal: true, load: [configuration] }), PrismaModule. Write `src/config/configuration.ts`: export default factory returning `{ port: parseInt(process.env.PORT, 10) || 3000, database: { url: process.env.DATABASE_URL }, nodeEnv: process.env.NODE_ENV || 'development' }`.

4. **Write Prisma schema and PrismaModule** — `prisma/schema.prisma`: datasource db { provider = "postgresql", url = env("DATABASE_URL") }, generator client { provider = "prisma-client-js" }, then 4 models exactly as specified in the research doc (User with githubId @unique, Deployment with userId FK + @@index, Analysis with userId FK + @@index, VibeScore with optional userId FK + @@index). All text-heavy fields use `@db.Text`. `src/prisma/prisma.service.ts`: class PrismaService extends PrismaClient implements OnModuleInit { async onModuleInit() { await this.$connect(); } }. `src/prisma/prisma.module.ts`: @Global() @Module with PrismaService as provider and export.

5. **Write `.env.example` and run install + generate** — Document all env vars (DATABASE_URL, PORT, NODE_ENV, and placeholders for future: MONAD_RPC_URL, MONAD_PRIVATE_KEY, GEMINI_API_KEY, GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET, JWT_SECRET, ENGINE_BINARY_PATH). Run `cd backend && npm install && npx prisma generate`. Verify `npm run build` produces `dist/main.js`.

## Must-Haves

- [ ] `backend/package.json` has all NestJS core + Prisma dependencies with correct versions
- [ ] `backend/tsconfig.json` enables decorators (experimentalDecorators, emitDecoratorMetadata) and strict mode
- [ ] `backend/src/main.ts` calls setGlobalPrefix('api'), enableCors(), enableShutdownHooks(), listens on PORT
- [ ] `backend/src/app.module.ts` imports ConfigModule.forRoot({ isGlobal: true }) and PrismaModule
- [ ] `backend/prisma/schema.prisma` has User, Deployment, Analysis, VibeScore models targeting postgresql
- [ ] `backend/src/prisma/prisma.service.ts` extends PrismaClient, implements OnModuleInit with $connect()
- [ ] `backend/src/prisma/prisma.module.ts` is @Global() and exports PrismaService
- [ ] `npm run build` compiles with zero errors
- [ ] `npx prisma generate` succeeds

## Verification

- `cd backend && npm run build` exits with code 0
- `cd backend && npx prisma generate` exits with code 0
- `test -f backend/dist/main.js` — compiled output exists
- `test -d backend/node_modules/.prisma/client` — Prisma client generated
- `grep -q "setGlobalPrefix" backend/src/main.ts` — global prefix is set
- `grep -q "enableCors" backend/src/main.ts` — CORS is enabled
- `grep -q "enableShutdownHooks" backend/src/main.ts` — shutdown hooks enabled
- `grep -q "postgresql" backend/prisma/schema.prisma` — PostgreSQL provider

## Observability Impact

- **New signals:** NestJS Logger emits `Listening on port ${port}` on bootstrap; PrismaService logs `Connecting to database...` and `Database connection established` on `onModuleInit`
- **Inspection surfaces:** After this task, a future agent can verify the build with `cd backend && npm run build` (exit 0 = healthy) and `npx prisma generate` (exit 0 = schema valid). Configuration can be tested with `npx ts-node -e "import('./src/config/configuration').then(m => console.log('config OK'))"`
- **Failure visibility:** TypeScript compilation errors surface in `npm run build` stderr; Prisma schema errors surface in `npx prisma generate` stderr; missing `DATABASE_URL` at runtime causes PrismaService `$connect()` to throw with connection refused error
- **Redaction:** `DATABASE_URL` and `MONAD_PRIVATE_KEY` are read from env only — never hardcoded or logged. `.env.example` contains only placeholder values

## Inputs

- `/tmp/vibe-loom/.env.example` — reference for env var names used by Vibe-Loom
- `/tmp/vibe-loom/package.json` — reference for dependency versions (Hardhat, ethers, etc. — not needed for T01 but informs .env.example)

## Expected Output

- `backend/package.json` — NestJS project manifest with all deps
- `backend/tsconfig.json` — TypeScript configuration
- `backend/tsconfig.build.json` — Build-specific TypeScript config
- `backend/nest-cli.json` — NestJS CLI configuration
- `backend/src/main.ts` — NestJS bootstrap entry point
- `backend/src/app.module.ts` — Root application module
- `backend/src/config/configuration.ts` — Typed configuration factory
- `backend/prisma/schema.prisma` — Database schema with 4 models
- `backend/src/prisma/prisma.service.ts` — Prisma service with OnModuleInit
- `backend/src/prisma/prisma.module.ts` — Global Prisma module
- `backend/.env.example` — Environment variable documentation
