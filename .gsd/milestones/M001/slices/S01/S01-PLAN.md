# S01: NestJS Foundation + Database

**Goal:** Scaffold a complete NestJS backend project with ConfigModule, Prisma ORM (PostgreSQL schema for User/Deployment/Analysis/VibeScore), HealthModule with Terminus, and standard API response patterns — providing the foundation that S02–S06 all consume.
**Demo:** `cd backend && npm run build` compiles cleanly, `npm test` passes, `npx prisma generate` succeeds, and `curl http://localhost:3000/api/health` returns `{ "status": "ok" }`.

## Must-Haves

- NestJS project in `backend/` with working `npm run build`, `npm run start:dev`, `npm test`
- `@nestjs/config` ConfigModule with `isGlobal: true`, typed configuration, env validation for `DATABASE_URL`, `PORT`, `NODE_ENV`
- Prisma schema (`backend/prisma/schema.prisma`) targeting PostgreSQL with `User`, `Deployment`, `Analysis`, `VibeScore` models — all fields matching research spec
- `PrismaService` extending `PrismaClient` with `OnModuleInit` (`$connect`), wrapped in `@Global()` `PrismaModule`
- `HealthModule` using `@nestjs/terminus` with `/api/health` and `/api/health/readiness` endpoints (DB ping via PrismaHealthIndicator)
- Standard API response DTO (`{ success, data, error }`), global `HttpExceptionFilter`, `TransformInterceptor`
- `main.ts` with `app.setGlobalPrefix('api')`, `app.enableCors()`, `app.enableShutdownHooks()`, listening on `process.env.PORT || 3000`
- Unit tests for PrismaService and HealthController

## Proof Level

- This slice proves: contract (downstream slices can import PrismaModule, ConfigModule, HealthModule and get working services)
- Real runtime required: yes (npm run build + npm test must pass)
- Human/UAT required: no

## Verification

- `cd backend && npm run build` — TypeScript compiles with zero errors
- `cd backend && npm test` — unit tests pass (PrismaService mock, HealthController)
- `cd backend && npx prisma generate` — Prisma client generates without errors
- `cd backend && npx ts-node -e "import('./src/config/configuration').then(m => console.log('config OK'))"` — configuration module loads
- `grep -q "setGlobalPrefix" backend/src/main.ts` — global /api prefix is set
- `grep -q "enableCors" backend/src/main.ts` — CORS is enabled
- `grep -q "enableShutdownHooks" backend/src/main.ts` — shutdown hooks are enabled

## Observability / Diagnostics

- Runtime signals: NestJS Logger on bootstrap (`Listening on port ${port}`), Prisma connection events in PrismaService.onModuleInit
- Inspection surfaces: `GET /api/health` returns `{ status: "ok", info: { database: { status: "up" } } }`, `GET /api/health/readiness` for DB connectivity
- Failure visibility: Global HttpExceptionFilter catches all unhandled errors and returns `{ success: false, error: { statusCode, message, timestamp, path } }`
- Redaction constraints: `DATABASE_URL`, `MONAD_PRIVATE_KEY` must never appear in logs or responses

## Integration Closure

- Upstream surfaces consumed: none (first slice)
- New wiring introduced in this slice: `AppModule` imports `ConfigModule.forRoot()`, `PrismaModule`, `HealthModule`; `main.ts` bootstraps the app with global prefix, CORS, pipes, filters, interceptors
- What remains before the milestone is truly usable end-to-end: S02 (ContractsModule), S03 (AuthModule, AnalysisModule, PaymasterModule), S04 (EngineModule, VibeScoreModule), S05 (Frontend), S06 (Railway deploy)

## Tasks

- [x] **T01: Scaffold NestJS project with ConfigModule and Prisma data layer** `est:1h`
  - Why: Everything in S01–S06 depends on a working NestJS project with ConfigModule for env vars and PrismaModule for database access. This is the foundation that must exist before any feature module.
  - Files: `backend/package.json`, `backend/tsconfig.json`, `backend/tsconfig.build.json`, `backend/nest-cli.json`, `backend/src/main.ts`, `backend/src/app.module.ts`, `backend/src/config/configuration.ts`, `backend/prisma/schema.prisma`, `backend/src/prisma/prisma.service.ts`, `backend/src/prisma/prisma.module.ts`, `backend/.env.example`
  - Do: Create `backend/` directory. Initialize package.json with NestJS core deps (`@nestjs/core`, `@nestjs/common`, `@nestjs/platform-express`, `@nestjs/config`, `prisma`, `@prisma/client`, `reflect-metadata`, `rxjs`, `class-validator`, `class-transformer`). Dev deps: `@nestjs/cli`, `@nestjs/testing`, `typescript`, `ts-node`, `ts-jest`, `jest`, `@types/jest`, `@types/node`, `@types/express`, `source-map-support`. Write tsconfig.json (strict, ES2021 target, experimentalDecorators, emitDecoratorMetadata). Write nest-cli.json. Write main.ts with bootstrap (setGlobalPrefix('api'), enableCors, enableShutdownHooks, listen on PORT). Write AppModule importing ConfigModule.forRoot({ isGlobal: true, load: [configuration] }) and PrismaModule. Write configuration.ts with typed config factory returning { port, databaseUrl, nodeEnv } with defaults. Write prisma/schema.prisma with postgresql datasource, prisma-client-js generator, and 4 models: User (githubId unique, username, email?, avatarUrl?, deployCount default 0), Deployment (userId FK, contractName, contractSource @db.Text, address?, txHash?, network default "monadTestnet", status default "pending", errorMessage? @db.Text, gasUsed?), Analysis (userId FK, contractSource @db.Text, errorMessage @db.Text, fixedCode? @db.Text, explanation? @db.Text, category?, isMonadSpecific default false), VibeScore (userId? FK, contractSource @db.Text, score Int, suggestions Json default "[]", engineBased default false, conflicts?, reExecutions?, gasEfficiency?). All models have id cuid, createdAt, and appropriate @@index. Write PrismaService extending PrismaClient implementing OnModuleInit with $connect in onModuleInit. Write PrismaModule with @Global() exporting PrismaService. Write .env.example documenting DATABASE_URL, PORT, NODE_ENV and placeholders for future env vars. Run npm install and npx prisma generate.
  - Verify: `cd backend && npm run build && npx prisma generate`
  - Done when: `backend/dist/main.js` exists, `backend/node_modules/.prisma/client` exists, TypeScript compiles with zero errors

- [x] **T02: Add HealthModule, common API patterns, and unit tests** `est:45m`
  - Why: The HealthModule proves the server runs and can connect to a database — Railway needs `/api/health` for probes. The common patterns (response DTO, exception filter, transform interceptor) establish the API contract for all downstream endpoints (S02–S05). Unit tests verify the foundation works and provide the test infrastructure for downstream slices.
  - Files: `backend/src/health/health.controller.ts`, `backend/src/health/health.module.ts`, `backend/src/health/prisma-health.indicator.ts`, `backend/src/common/dto/api-response.dto.ts`, `backend/src/common/filters/http-exception.filter.ts`, `backend/src/common/interceptors/transform.interceptor.ts`, `backend/src/app.module.ts`, `backend/src/main.ts`, `backend/test/health.controller.spec.ts`, `backend/test/prisma.service.spec.ts`, `backend/test/jest-e2e.json`
  - Do: Install `@nestjs/terminus`. Write PrismaHealthIndicator extending HealthIndicator that calls prisma.$queryRaw`SELECT 1`. Write HealthController with two endpoints: `@Get('health')` using HealthCheckService + PrismaHealthIndicator, `@Get('health/readiness')` same check. Write HealthModule importing TerminusModule and providing PrismaHealthIndicator. Write ApiResponse DTO class with success:boolean, data?:T, error? fields. Write HttpExceptionFilter implementing ExceptionFilter: catches HttpException and unknown errors, returns `{ success: false, error: { statusCode, message, timestamp, path } }`, logs unknown errors. Write TransformInterceptor implementing NestInterceptor: wraps successful responses as `{ success: true, data: response }`. Update AppModule to import HealthModule. Update main.ts to register global filter (new HttpExceptionFilter) and interceptor (new TransformInterceptor), also add ValidationPipe globally. Write unit test for HealthController: mock PrismaService, assert /api/health returns ok. Write unit test for PrismaService: test onModuleInit calls $connect. Write jest-e2e.json config. Verify npm test passes and npm run build succeeds.
  - Verify: `cd backend && npm test && npm run build`
  - Done when: `npm test` reports all tests passing, `npm run build` succeeds, `backend/src/health/health.controller.ts` exists with /health endpoint

## Files Likely Touched

- `backend/package.json`
- `backend/tsconfig.json`
- `backend/tsconfig.build.json`
- `backend/nest-cli.json`
- `backend/.env.example`
- `backend/src/main.ts`
- `backend/src/app.module.ts`
- `backend/src/config/configuration.ts`
- `backend/prisma/schema.prisma`
- `backend/src/prisma/prisma.service.ts`
- `backend/src/prisma/prisma.module.ts`
- `backend/src/health/health.controller.ts`
- `backend/src/health/health.module.ts`
- `backend/src/health/prisma-health.indicator.ts`
- `backend/src/common/dto/api-response.dto.ts`
- `backend/src/common/filters/http-exception.filter.ts`
- `backend/src/common/interceptors/transform.interceptor.ts`
- `backend/test/health.controller.spec.ts`
- `backend/test/prisma.service.spec.ts`
- `backend/test/jest-e2e.json`
