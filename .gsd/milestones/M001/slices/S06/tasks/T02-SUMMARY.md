---
id: T02
parent: S06
milestone: M001
provides:
  - Full E2E test suite validating API surface with 12 test cases via supertest
  - PrismaService mock pattern for DB-free NestJS integration testing
key_files:
  - backend/test/app.e2e-spec.ts
  - backend/package.json
key_decisions:
  - "Used async/await pattern instead of return-chain for supertest to avoid noImplicitAny issues with .expect() callbacks"
  - "Expect 201 for @Post() endpoints — NestJS default, not 200 as plan suggested"
patterns_established:
  - "E2E test setup: Test.createTestingModule(AppModule) + overrideProvider(PrismaService) + replicate main.ts bootstrap (prefix, filters, interceptors, pipes)"
  - "Health endpoints wrapped by TransformInterceptor: assert on res.body.data.status not res.body.status"
observability_surfaces:
  - "npm run test:e2e: 12 pass/fail results with HTTP status + response body in terminal/CI output"
duration: 8m
verification_result: passed
completed_at: 2026-03-22T24:05:00+09:00
blocker_discovered: false
---

# T02: Write E2E test suite with supertest and mocked externals

**Installed supertest and wrote 12 E2E tests covering health, contracts, vibe-score, analysis, and paymaster auth — all passing with mocked PrismaService and no DB required**

## What Happened

Installed `supertest` and `@types/supertest` as devDependencies. Created `backend/test/app.e2e-spec.ts` with a comprehensive E2E test suite that boots the real AppModule via `Test.createTestingModule`, overriding only PrismaService to avoid DB connections. The test replicates the exact `main.ts` bootstrap config: global prefix `api`, HttpExceptionFilter, TransformInterceptor, and ValidationPipe.

12 test cases cover the full API surface:
- **Health** (2): GET /api/health and /api/health/readiness — 200 with `{ success, data: { status: 'ok' } }`
- **Contracts** (5): source retrieval (valid FixedContract + invalid type), compile (valid Solidity with real solc, invalid source, missing body)
- **Vibe-score** (2): heuristic fallback scoring + missing body validation
- **Analysis** (2): revert error analysis + missing body validation
- **Paymaster** (1): 401 without JWT token

Two discoveries during implementation: (1) supertest v7 requires `import request from 'supertest'` not `import * as request` for ESM interop, (2) NestJS `@Post()` defaults to 201 Created not 200 — the plan specified 200 but actual behavior is 201 for all POST endpoints.

## Verification

- `cd backend && npm run test:e2e` — 12/12 E2E tests pass (4.2s)
- `cd backend && npm test` — 97/97 unit tests pass across 14 suites (12.3s, no regressions)
- `grep -q 'supertest' backend/package.json` — confirmed installed

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd backend && npm run test:e2e` | 0 | ✅ pass (12 tests) | 4s |
| 2 | `cd backend && npm test` | 0 | ✅ pass (97 tests, 14 suites) | 12s |
| 3 | `grep -q 'supertest' backend/package.json` | 0 | ✅ pass | <1s |
| 4 | `test -f backend/Dockerfile && test -f backend/.dockerignore && test -f railway.json && test -f backend/scripts/start.sh` | 0 | ✅ pass | <1s |
| 5 | `grep -q 'FRONTEND_URL\|frontend' backend/src/main.ts` | 0 | ✅ pass | <1s |
| 6 | `grep -q 'CORS origin' backend/src/main.ts` | 0 | ✅ pass | <1s |

### Slice-level checks (final task — all pass)

| # | Check | Status |
|---|-------|--------|
| 1 | `cd backend && npm run test:e2e` — all E2E tests pass (12 test cases) | ✅ pass |
| 2 | `cd backend && npm test` — 97 unit tests pass (14 suites, no regressions) | ✅ pass |
| 3 | `docker build -f backend/Dockerfile -t monad-backend .` — exits 0 | ✅ pass (verified in T01) |
| 4 | Deployment files exist | ✅ pass |
| 5 | CORS uses FRONTEND_URL | ✅ pass |
| 6 | Docker image layout correct | ✅ pass (verified in T01) |
| 7 | CORS origin logged at bootstrap | ✅ pass |

## Diagnostics

- **E2E test failures**: Each test shows expected vs actual HTTP status and response body diff — look for `expect(received).toBe(expected)` in output
- **PrismaService mock shape**: All stubs defined at top of test file — if new Prisma models are added to services, add corresponding mock entries to `mockPrismaService`
- **Response envelope**: TransformInterceptor wraps all controller returns in `{ success: true, data: ... }`. AnalysisController uses `@Res()` and manually wraps. Health uses Terminus which returns to interceptor normally.
- **POST status codes**: NestJS defaults `@Post()` to 201 Created — tests expect this, not 200

## Deviations

- **supertest import**: Plan showed `import * as request from 'supertest'` but supertest v7 with `esModuleInterop: true` requires `import request from 'supertest'`
- **POST status 200 → 201**: Plan specified 200 for POST endpoints but NestJS `@Post()` decorator defaults to 201 Created; tests corrected to match actual framework behavior
- **Health body path**: Plan said `res.body.status` but TransformInterceptor wraps Terminus output, so the path is `res.body.data.status`

## Known Issues

None.

## Files Created/Modified

- `backend/test/app.e2e-spec.ts` — E2E test suite with 12 test cases covering all API endpoints with mocked PrismaService
- `backend/package.json` — Added supertest@^7.2.2 and @types/supertest@^7.2.0 to devDependencies
- `.gsd/milestones/M001/slices/S06/tasks/T02-PLAN.md` — Added Observability Impact section per pre-flight requirement
