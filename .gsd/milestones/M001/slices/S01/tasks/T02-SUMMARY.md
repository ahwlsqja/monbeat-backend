---
id: T02
parent: S01
milestone: M001
provides:
  - HealthModule with /api/health and /api/health/readiness endpoints using @nestjs/terminus and PrismaHealthIndicator
  - PrismaHealthIndicator that verifies DB connectivity via $queryRaw SELECT 1
  - ApiResponse DTO with static ok/fail factory methods for standardized API responses
  - HttpExceptionFilter catching all exceptions and returning structured error responses
  - TransformInterceptor wrapping successful responses as { success: true, data }
  - Global ValidationPipe with whitelist + transform registered in main.ts
  - Unit tests for PrismaService (onModuleInit calls $connect) and HealthController (check/checkReadiness)
  - Jest config updated to scan both src/ and test/ directories
  - E2E test config scaffold (test/jest-e2e.json) for future use
key_files:
  - backend/src/health/health.controller.ts
  - backend/src/health/health.module.ts
  - backend/src/health/prisma-health.indicator.ts
  - backend/src/common/dto/api-response.dto.ts
  - backend/src/common/filters/http-exception.filter.ts
  - backend/src/common/interceptors/transform.interceptor.ts
  - backend/src/main.ts
  - backend/src/app.module.ts
  - backend/test/health.controller.spec.ts
  - backend/test/prisma.service.spec.ts
key_decisions:
  - "Changed Jest rootDir from 'src' to '.' with roots: ['<rootDir>/src', '<rootDir>/test'] so tests in test/ directory are discovered alongside any future src/ co-located tests"
  - "Used definite assignment assertion (!) on ApiResponse.success field to satisfy strict TypeScript — field is always set via static factory methods ok/fail"
patterns_established:
  - "All API errors return { success: false, error: { statusCode, message, timestamp, path } } via HttpExceptionFilter"
  - "All successful API responses are wrapped as { success: true, data: ... } via TransformInterceptor"
  - "Health checks live at /api/health (liveness) and /api/health/readiness (readiness with DB check)"
  - "Unit tests use @nestjs/testing Test.createTestingModule with mock providers — no real DB needed"
observability_surfaces:
  - "GET /api/health returns { status: 'ok', info: { database: { status: 'up' } } } when DB is connected"
  - "GET /api/health/readiness same check — for Kubernetes/Railway readiness probes"
  - "HttpExceptionFilter logs unhandled (non-HTTP) exceptions with stack trace via NestJS Logger"
  - "Unknown errors return 500 with 'Internal server error' message — stack traces stay server-side"
duration: ~10m
verification_result: passed
completed_at: 2026-03-22
blocker_discovered: false
---

# T02: Add HealthModule, common API patterns, and unit tests

**Added HealthModule with Terminus DB health checks, global HttpExceptionFilter/TransformInterceptor/ValidationPipe, and 5 passing unit tests completing the S01 NestJS foundation**

## What Happened

Installed `@nestjs/terminus` and created three health module files: `PrismaHealthIndicator` extending `HealthIndicator` that calls `$queryRaw\`SELECT 1\`` to verify database connectivity, `HealthController` with `@Get()` check() and `@Get('readiness')` checkReadiness() endpoints both running the Prisma health check via `HealthCheckService`, and `HealthModule` importing `TerminusModule` and providing the indicator.

Created three common API pattern files: `ApiResponse<T>` DTO with `success`, `data?`, and `error?` fields plus static `ok()` and `fail()` factory methods; `HttpExceptionFilter` implementing `@Catch()` that handles `HttpException` (extracts status/message) and unknown errors (500, "Internal server error", logs stack trace), returning the standardized `ApiResponse.fail()` format; and `TransformInterceptor` that wraps all successful responses as `{ success: true, data: response }` via the `map()` RxJS operator.

Updated `AppModule` to import `HealthModule`. Updated `main.ts` to register global `HttpExceptionFilter`, `TransformInterceptor`, and `ValidationPipe` (whitelist + transform enabled).

Wrote 2 test suites: `prisma.service.spec.ts` tests PrismaService is defined and `onModuleInit` calls `$connect`; `health.controller.spec.ts` tests HealthController is defined, `check()` returns mock health status, and `checkReadiness()` returns mock health status — all using `@nestjs/testing` with mock providers. Updated Jest config to use `rootDir: "."` with `roots: ["<rootDir>/src", "<rootDir>/test"]` so both directories are scanned.

## Verification

All 8 task-level checks pass. All 7 slice-level checks pass — S01 is fully complete.

- `npm test` — 2 suites, 5 tests, all pass
- `npm run build` — TypeScript compiles with zero errors
- `TerminusModule` is imported in `health.module.ts`
- `HttpExceptionFilter`, `TransformInterceptor`, `ValidationPipe` all registered in `main.ts`
- Both test files exist in `test/`

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| T1 | `cd backend && npm test` | 0 | ✅ pass | 3.4s |
| T2 | `cd backend && npm run build` | 0 | ✅ pass | ~2s |
| T3 | `grep -q "TerminusModule" backend/src/health/health.module.ts` | 0 | ✅ pass | <1s |
| T4 | `grep -q "HttpExceptionFilter" backend/src/main.ts` | 0 | ✅ pass | <1s |
| T5 | `grep -q "TransformInterceptor" backend/src/main.ts` | 0 | ✅ pass | <1s |
| T6 | `grep -q "ValidationPipe" backend/src/main.ts` | 0 | ✅ pass | <1s |
| T7 | `test -f backend/test/health.controller.spec.ts` | 0 | ✅ pass | <1s |
| T8 | `test -f backend/test/prisma.service.spec.ts` | 0 | ✅ pass | <1s |
| S1 | `cd backend && npm run build` (slice) | 0 | ✅ pass | ~2s |
| S2 | `cd backend && npm test` (slice) | 0 | ✅ pass | 3.4s |
| S3 | `cd backend && npx prisma generate` (slice) | 0 | ✅ pass | ~1s |
| S4 | `npx ts-node -e "import('./src/config/configuration')..."` | 0 | ✅ pass | <2s |
| S5 | `grep -q "setGlobalPrefix" backend/src/main.ts` (slice) | 0 | ✅ pass | <1s |
| S6 | `grep -q "enableCors" backend/src/main.ts` (slice) | 0 | ✅ pass | <1s |
| S7 | `grep -q "enableShutdownHooks" backend/src/main.ts` (slice) | 0 | ✅ pass | <1s |

## Diagnostics

- **Health check:** `curl localhost:3000/api/health` returns `{ status: "ok", info: { database: { status: "up" } } }` when DB is connected, or `{ status: "error" }` when DB is down
- **Readiness probe:** `curl localhost:3000/api/health/readiness` — same check, use for Railway/K8s readiness probes
- **Error responses:** Any unhandled error returns `{ success: false, error: { statusCode, message, timestamp, path } }`
- **Test health:** `cd backend && npm test` — 2 suites, 5 tests
- **Build health:** `cd backend && npm run build` — exit 0 means clean compile

## Deviations

- Changed Jest `rootDir` from `"src"` to `"."` with explicit `roots: ["<rootDir>/src", "<rootDir>/test"]` — the T01 config only scanned `src/` which would miss tests in the `test/` directory
- Added definite assignment assertion (`!`) to `ApiResponse.success` property — required by strict TypeScript since the property is set via static factory methods rather than the constructor

## Known Issues

None.

## Files Created/Modified

- `backend/src/health/prisma-health.indicator.ts` — PrismaHealthIndicator extending HealthIndicator with $queryRaw DB check
- `backend/src/health/health.controller.ts` — HealthController with /health and /health/readiness endpoints
- `backend/src/health/health.module.ts` — HealthModule importing TerminusModule
- `backend/src/common/dto/api-response.dto.ts` — ApiResponse<T> DTO with ok/fail factory methods
- `backend/src/common/filters/http-exception.filter.ts` — Global exception filter returning standardized error responses
- `backend/src/common/interceptors/transform.interceptor.ts` — Response wrapper interceptor for { success: true, data }
- `backend/src/app.module.ts` — Updated to import HealthModule
- `backend/src/main.ts` — Updated with global filter, interceptor, and validation pipe
- `backend/test/health.controller.spec.ts` — HealthController unit tests (3 tests)
- `backend/test/prisma.service.spec.ts` — PrismaService unit tests (2 tests)
- `backend/test/jest-e2e.json` — E2E test configuration for future use
- `backend/package.json` — Added @nestjs/terminus dependency, updated Jest config roots
