# S01: NestJS Foundation + Database — Research

**Date:** 2026-03-22
**Depth:** Targeted — known stack (NestJS + Prisma), new project scaffolding with clear downstream contracts

## Summary

S01 establishes the Vibe-Room-Backend NestJS project from scratch. It covers scaffolding, ConfigModule, PrismaModule with PostgreSQL schema, HealthModule with Terminus, and a BaseController/response pattern used by all downstream slices (S02–S06). The existing Vibe-Loom is a Next.js monolith with 5 API routes, file-based JSON storage (`data/deploy-counts.json`), and inline config — all need proper backend architecture.

The Prisma schema must support three downstream consumers: S02 needs `Deployment` records (address, txHash, status, userId), S03 needs `User` (GitHub OAuth identity, deploy count) and `Analysis` records, and S04 needs a place to store `VibeScore` results. The schema design is the highest-leverage decision in S01 because every subsequent slice depends on it.

No `Vibe-Room-Backend` repo exists yet — it must be created as a new NestJS project within the working directory structure. The repo will live at the worktree root as the NestJS project itself.

## Recommendation

**Scaffold NestJS with `@nestjs/cli`, use `@nestjs/config` for env, `@prisma/client` + `prisma` for DB, `@nestjs/terminus` for health checks.** Build order: (1) NestJS scaffold + ConfigModule, (2) Prisma schema + PrismaModule, (3) HealthModule + readiness endpoint, (4) Base response DTOs + global exception filter.

Use `@nestjs/config` with `isGlobal: true` rather than custom ConfigModule — it's the NestJS standard and handles `.env` loading, validation, and typed access. For Prisma, create a `PrismaService extends PrismaClient implements OnModuleInit` pattern (NestJS docs recipe) and wrap it in a `PrismaModule` with `@Global()` for cross-module access.

## Implementation Landscape

### Key Files

**Vibe-Loom (source material — read-only reference):**
- `/tmp/vibe-loom/src/app/api/deploy/route.ts` — deploy API, uses Hardhat spawn, file-based (S02 will migrate)
- `/tmp/vibe-loom/src/app/api/contract-source/route.ts` — contract source lookup from filesystem (S02)
- `/tmp/vibe-loom/src/app/api/deploy-status/route.ts` — paymaster status, reads JSON file store (S03)
- `/tmp/vibe-loom/src/app/api/analyze-deployment-error/route.ts` — RAG error analysis with Gemini (S03)
- `/tmp/vibe-loom/src/app/api/vibe-score/route.ts` — regex/AI-based vibe-score (S04 replaces with engine)
- `/tmp/vibe-loom/src/lib/deploy-count-store.ts` — **file-based JSON** deploy count (→ replace with Prisma User.deployCount)
- `/tmp/vibe-loom/src/lib/paymaster.ts` — paymaster logic, MAX_FREE_DEPLOYMENTS=3 (→ S03 migrates)
- `/tmp/vibe-loom/src/lib/ai.ts` — Gemini singleton (→ S03 migrates)
- `/tmp/vibe-loom/.env.example` — env vars: MONAD_RPC_URL, MONAD_PRIVATE_KEY, GEMINI_API_KEY
- `/tmp/vibe-loom/hardhat.config.ts` — Solidity 0.8.24, cancun EVM, monadTestnet chainId 10143

**New files to create (Vibe-Room-Backend):**
- `src/main.ts` — NestJS bootstrap, global pipes, CORS, enableShutdownHooks
- `src/app.module.ts` — root module importing ConfigModule, PrismaModule, HealthModule
- `src/config/configuration.ts` — typed config factory (env validation)
- `src/prisma/prisma.service.ts` — `PrismaService extends PrismaClient implements OnModuleInit`
- `src/prisma/prisma.module.ts` — `@Global()` PrismaModule exporting PrismaService
- `prisma/schema.prisma` — PostgreSQL datasource, User/Deployment/Analysis/VibeScore models
- `src/health/health.controller.ts` — `/api/health` + `/api/health/readiness` endpoints
- `src/health/health.module.ts` — TerminusModule + PrismaHealthIndicator
- `src/common/dto/api-response.dto.ts` — standard `{ success, data, error }` wrapper
- `src/common/filters/http-exception.filter.ts` — global exception filter
- `src/common/interceptors/transform.interceptor.ts` — response wrapping interceptor
- `.env.example` — all required env vars documented
- `package.json` — NestJS deps
- `tsconfig.json` — NestJS TypeScript config
- `nest-cli.json` — NestJS CLI config

### Prisma Schema Design

The schema must serve S02, S03, and S04. Based on Vibe-Loom's current data flows:

```prisma
datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

generator client {
  provider = "prisma-client-js"
}

model User {
  id            String       @id @default(cuid())
  githubId      String       @unique
  username      String
  email         String?
  avatarUrl     String?
  deployCount   Int          @default(0)
  createdAt     DateTime     @default(now())
  updatedAt     DateTime     @updatedAt
  deployments   Deployment[]
  analyses      Analysis[]
  vibeScores    VibeScore[]
}

model Deployment {
  id              String   @id @default(cuid())
  userId          String
  user            User     @relation(fields: [userId], references: [id])
  contractName    String
  contractSource  String   @db.Text
  address         String?
  txHash          String?
  network         String   @default("monadTestnet")
  status          String   @default("pending") // pending | success | failed
  errorMessage    String?  @db.Text
  gasUsed         String?
  createdAt       DateTime @default(now())

  @@index([userId])
  @@index([status])
}

model Analysis {
  id              String   @id @default(cuid())
  userId          String
  user            User     @relation(fields: [userId], references: [id])
  contractSource  String   @db.Text
  errorMessage    String   @db.Text
  fixedCode       String?  @db.Text
  explanation     String?  @db.Text
  category        String?
  isMonadSpecific Boolean  @default(false)
  createdAt       DateTime @default(now())

  @@index([userId])
}

model VibeScore {
  id              String   @id @default(cuid())
  userId          String?
  user            User?    @relation(fields: [userId], references: [id])
  contractSource  String   @db.Text
  score           Int
  suggestions     Json     @default("[]")
  engineBased     Boolean  @default(false)
  conflicts       Int?
  reExecutions    Int?
  gasEfficiency   Float?
  createdAt       DateTime @default(now())

  @@index([userId])
}
```

### Environment Variables

All env vars needed across S01–S06 (S01 sets up ConfigModule to validate these):

| Variable | Used By | Required In S01 |
|---|---|---|
| `DATABASE_URL` | PrismaModule | Yes |
| `PORT` | main.ts | Yes (default 3000) |
| `NODE_ENV` | ConfigModule | Yes |
| `MONAD_RPC_URL` | S02 ContractsModule | No — validated later |
| `MONAD_PRIVATE_KEY` | S02/S03 Deploy/Paymaster | No |
| `GEMINI_API_KEY` | S03 AnalysisModule | No |
| `GITHUB_CLIENT_ID` | S03 AuthModule | No |
| `GITHUB_CLIENT_SECRET` | S03 AuthModule | No |
| `JWT_SECRET` | S03 AuthModule | No |
| `ENGINE_BINARY_PATH` | S04 EngineModule | No |

### Build Order

1. **NestJS scaffold + ConfigModule** — `npm i @nestjs/core @nestjs/common @nestjs/platform-express @nestjs/config` + app.module.ts + main.ts + configuration.ts. This is the foundation everything depends on.
2. **Prisma schema + PrismaModule** — `npm i prisma @prisma/client` + `prisma init` + schema.prisma + PrismaService + PrismaModule. Run `npx prisma generate` (no actual DB migration needed yet — migration will happen against Railway PostgreSQL or local Docker). This unblocks S02, S03, S04.
3. **HealthModule** — `npm i @nestjs/terminus` + health.controller.ts + health.module.ts. Provides `/api/health` and `/api/health/readiness` (DB ping). This proves the server runs and can connect to a database.
4. **Common patterns** — API response DTO, global exception filter, transform interceptor. These establish the response contract for all downstream endpoints.

### Verification Approach

1. `npm run build` — TypeScript compiles without errors
2. `npm run start:dev` — NestJS server starts on PORT (default 3000)
3. `curl http://localhost:3000/api/health` — returns `{ status: "ok", info: { ... } }`
4. `npx prisma generate` — Prisma client generates without errors
5. `npx prisma migrate dev --name init` — migration runs against local PostgreSQL (or SQLite for test)
6. `npm run test` — unit tests pass for PrismaService, HealthController

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Config/env loading | `@nestjs/config` (ConfigModule) | Built-in NestJS, supports `.env`, typed, validation with `joi` or `class-validator` |
| Health checks | `@nestjs/terminus` (TerminusModule) | Standard health check framework, Kubernetes/Railway probe compatible |
| Database ORM | Prisma (`@prisma/client`) | Decision D003, type-safe, migration system, Railway PostgreSQL plugin support |
| Validation pipes | `class-validator` + `class-transformer` | NestJS standard, auto-validate DTOs |
| API docs (optional) | `@nestjs/swagger` | Low-effort OpenAPI docs for frontend integration |

## Constraints

- **Separate repo structure (D001):** The NestJS project is `Vibe-Room-Backend`, a standalone repo. In this worktree context, scaffold at the worktree root or a subdirectory — the planner must decide where the NestJS project root lives relative to `.gsd/`.
- **PostgreSQL required (D003):** Prisma schema must target `postgresql` provider. For local dev without Docker, SQLite can be used via Prisma's provider swap, but the schema must be PostgreSQL-compatible.
- **Railway deploy target (D006):** `main.ts` must listen on `process.env.PORT` (Railway injects this). Health check at `/api/health` is needed for Railway's health probe.
- **Global `/api` prefix:** All Vibe-Loom routes use `/api/...` prefix. NestJS should set `app.setGlobalPrefix('api')` in `main.ts` so downstream controllers use bare paths.
- **CORS required (R009):** `main.ts` must enable CORS for the Next.js frontend origin.

## Common Pitfalls

- **Prisma `onModuleInit` connection** — PrismaService must call `this.$connect()` in `onModuleInit()`. Without this, the first query triggers a lazy connection which can cause health check timeouts on cold start.
- **`enableShutdownHooks` conflict** — In Prisma v5+, the old `this.$on('beforeExit')` pattern is removed. Use NestJS's built-in `app.enableShutdownHooks()` in `main.ts` instead.
- **ConfigModule `isGlobal: true`** — Without this, every module that injects `ConfigService` must import `ConfigModule`. Set it global once in AppModule.
- **Prisma schema `@db.Text` for source code** — Solidity source code can be large. Use `@db.Text` (PostgreSQL `TEXT` type) instead of default `String` (`VARCHAR(191)`).

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| NestJS | `kadajett/agent-nestjs-skills@nestjs-best-practices` | available (8.5K installs) — covers module patterns, testing, project structure |
| NestJS | `sickn33/antigravity-awesome-skills@nestjs-expert` | available (1.2K installs) |
| Prisma | `sickn33/antigravity-awesome-skills@prisma-expert` | available (2.8K installs) — covers schema design, migrations, NestJS integration |
| Prisma | `prisma/skills@prisma-driver-adapter-implementation` | available (829 installs) — official Prisma skill |

**Recommendation:** Consider installing `kadajett/agent-nestjs-skills@nestjs-best-practices` and `sickn33/antigravity-awesome-skills@prisma-expert` before execution — they have the highest install counts and directly apply to S01's work.

## Sources

- NestJS Prisma recipe: PrismaService pattern with `OnModuleInit`, `$connect()` (source: [NestJS docs - Prisma](https://docs.nestjs.com/recipes/prisma))
- NestJS Terminus health checks: HealthModule + HealthController + indicators (source: [NestJS docs - Terminus](https://docs.nestjs.com/recipes/terminus))
- NestJS ConfigModule: `forRoot({ isGlobal: true })` pattern (source: [NestJS docs - Configuration](https://docs.nestjs.com/techniques/configuration))
- Prisma PostgreSQL schema + migration: `prisma migrate dev --name init` (source: [Prisma docs](https://www.prisma.io/docs/orm/prisma-migrate))
