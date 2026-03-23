# S01: NestJS Foundation + Database — UAT

**Milestone:** M001
**Written:** 2026-03-22

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S01 is a project scaffold slice — it produces build artifacts, generated Prisma client, and passing unit tests. No live runtime (DB, server) or human interaction is required to verify the foundation is correctly built. The slice plan explicitly states "Human/UAT required: no" and "Proof level: contract."

## Preconditions

- Working directory is the worktree root containing `backend/`
- `backend/node_modules/` exists (npm install completed)
- No DATABASE_URL needed — unit tests use mocks, prisma generate doesn't need a live DB

## Smoke Test

Run `cd backend && npm run build && npm test && npx prisma generate` — all three must exit 0. If any fails, the foundation is broken.

## Test Cases

### 1. TypeScript compilation produces clean build

1. Run `cd backend && npm run build`
2. **Expected:** Exit code 0, `backend/dist/main.js` exists, no TypeScript errors in output

### 2. Unit tests pass

1. Run `cd backend && npm test`
2. **Expected:** 2 test suites, 5 tests, all pass:
   - `prisma.service.spec.ts`: PrismaService defined, onModuleInit calls $connect
   - `health.controller.spec.ts`: HealthController defined, check() returns ok, checkReadiness() returns ok

### 3. Prisma client generation succeeds

1. Run `cd backend && npx prisma generate`
2. **Expected:** Exit code 0, "Generated Prisma Client" message, `backend/node_modules/.prisma/client` directory exists

### 4. Configuration module loads

1. Run `cd backend && npx ts-node -e "import('./src/config/configuration').then(m => console.log('config OK'))"`
2. **Expected:** Prints "config OK", exit code 0

### 5. main.ts has required bootstrap features

1. Run `grep -c "setGlobalPrefix\|enableCors\|enableShutdownHooks" backend/src/main.ts`
2. **Expected:** Output is `3` (all three features present)

### 6. Prisma schema has all 4 models

1. Run `grep -c "^model " backend/prisma/schema.prisma`
2. **Expected:** Output is `4` (User, Deployment, Analysis, VibeScore)

### 7. Global modules are properly configured

1. Run `grep -q "isGlobal: true" backend/src/app.module.ts && echo "ConfigModule global OK"`
2. Run `grep -q "@Global()" backend/src/prisma/prisma.module.ts && echo "PrismaModule global OK"`
3. **Expected:** Both echo lines print

### 8. Common API patterns are registered in main.ts

1. Run `grep -c "HttpExceptionFilter\|TransformInterceptor\|ValidationPipe" backend/src/main.ts`
2. **Expected:** Output is `3` (all three patterns registered globally)

### 9. AppModule imports all foundation modules

1. Run `grep -c "ConfigModule\|PrismaModule\|HealthModule" backend/src/app.module.ts`
2. **Expected:** Output is at least `3` (all three imported)

## Edge Cases

### Schema field types for large text

1. Run `grep -c "@db.Text" backend/prisma/schema.prisma`
2. **Expected:** Output is ≥ 5 — fields like contractSource, errorMessage, fixedCode, explanation use @db.Text for PostgreSQL TEXT type instead of default VARCHAR(191)

### Environment variable documentation

1. Run `grep -c "DATABASE_URL\|PORT\|NODE_ENV\|MONAD_RPC_URL\|MONAD_PRIVATE_KEY\|GITHUB_CLIENT_ID\|GEMINI_API_KEY\|JWT_SECRET\|ENGINE_BINARY_PATH" backend/.env.example`
2. **Expected:** Output is ≥ 9 — all env vars documented for downstream slices

### Build output is deterministic

1. Run `rm -rf backend/dist && cd backend && npm run build && test -f dist/main.js && echo "rebuild OK"`
2. **Expected:** Clean rebuild succeeds even after deleting dist/ (deleteOutDir + incremental:false are correctly configured)

## Failure Signals

- `npm run build` exits non-zero → TypeScript configuration or source error
- `npm test` fails → Unit test or Jest configuration broken
- `npx prisma generate` fails → schema.prisma has syntax errors or missing generator config
- `dist/main.js` doesn't exist after build → deleteOutDir/incremental conflict (see KNOWLEDGE.md)
- `node_modules/.prisma/client` missing → Prisma generate wasn't run or failed silently

## Not Proven By This UAT

- **Runtime server startup** — requires DATABASE_URL pointing to a live PostgreSQL instance
- **Health endpoint responses** — require running server with DB; unit tests verify controller logic with mocks
- **Database migration** — no migration files created; first migration happens when a DB is available (S06)
- **CORS behavior** — configured but not tested until frontend integration (S05)
- **Error response format at runtime** — HttpExceptionFilter is registered but only tested through code inspection, not live requests

## Notes for Tester

- This is a build-time verification slice. No running server or database is needed.
- All tests use mocks — they verify wiring and contracts, not runtime behavior.
- The `deleteOutDir + incremental` gotcha is documented in KNOWLEDGE.md — if a future edit adds `incremental: true` to tsconfig.json, builds will silently break.
- The `.env.example` is documentation only — no actual `.env` file should exist in the repo.
