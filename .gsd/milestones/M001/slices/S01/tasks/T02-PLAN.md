---
estimated_steps: 5
estimated_files: 11
---

# T02: Add HealthModule, common API patterns, and unit tests

**Slice:** S01 — NestJS Foundation + Database
**Milestone:** M001

## Description

Add the HealthModule (using `@nestjs/terminus`) with `/api/health` and `/api/health/readiness` endpoints that include a database connectivity check. Create the common API response patterns (ApiResponse DTO, HttpExceptionFilter, TransformInterceptor) that all downstream modules (S02–S05) will use. Write unit tests for PrismaService and HealthController that verify the foundation works. This task completes S01 — after it, the NestJS backend has a running server, health checks, standard response formatting, and passing tests.

## Steps

1. **Install `@nestjs/terminus` and create PrismaHealthIndicator** — `cd backend && npm install @nestjs/terminus`. Write `src/health/prisma-health.indicator.ts`: class extends `HealthIndicator`, inject PrismaService, implements `isHealthy(key: string)` method that calls `this.prismaService.$queryRaw\`SELECT 1\`` and returns `this.getStatus(key, true)`, catches errors and throws `HealthCheckError`.

2. **Write HealthController and HealthModule** — `src/health/health.controller.ts`: `@Controller('health')` with two methods: `@Get()` check() using HealthCheckService to run PrismaHealthIndicator.isHealthy('database'), and `@Get('readiness')` checkReadiness() same check. `src/health/health.module.ts`: imports TerminusModule, provides PrismaHealthIndicator, controllers: [HealthController].

3. **Write common API response patterns** — `src/common/dto/api-response.dto.ts`: generic `ApiResponse<T>` class with `success: boolean`, `data?: T`, `error?: { statusCode: number, message: string, timestamp: string, path: string }`. `src/common/filters/http-exception.filter.ts`: `@Catch()` filter implementing ExceptionFilter, handles HttpException (extract status/message) and unknown errors (500, "Internal server error"), returns ApiResponse format, logs unknown errors. `src/common/interceptors/transform.interceptor.ts`: NestInterceptor that wraps response data as `{ success: true, data: response }` via `map()` operator.

4. **Wire into AppModule and main.ts** — Update `app.module.ts` to import HealthModule. Update `main.ts` to register global filter (`app.useGlobalFilters(new HttpExceptionFilter())`), global interceptor (`app.useGlobalInterceptors(new TransformInterceptor())`), and global ValidationPipe (`app.useGlobalPipes(new ValidationPipe({ whitelist: true, transform: true }))`). Import `ValidationPipe` from `@nestjs/common`.

5. **Write unit tests** — `test/prisma.service.spec.ts`: test that PrismaService is defined and onModuleInit calls $connect (mock $connect). `test/health.controller.spec.ts`: test that HealthController is defined, mock HealthCheckService and PrismaHealthIndicator, assert check() returns expected health status. Configure Jest: `backend/jest.config.js` or configure in package.json (moduleFileExtensions, rootDir, testRegex, transform with ts-jest). Write `test/jest-e2e.json` for future e2e tests. Verify `npm test` passes with all tests green.

## Must-Haves

- [ ] `@nestjs/terminus` is installed in package.json
- [ ] HealthController has `/health` and `/health/readiness` endpoints with Prisma DB check
- [ ] PrismaHealthIndicator calls `$queryRaw` to verify DB connectivity
- [ ] HttpExceptionFilter catches all exceptions and returns `{ success: false, error: {...} }`
- [ ] TransformInterceptor wraps responses as `{ success: true, data: ... }`
- [ ] ValidationPipe is registered globally with whitelist + transform
- [ ] AppModule imports HealthModule
- [ ] main.ts registers global filter, interceptor, and validation pipe
- [ ] `npm test` passes with at least 2 test suites (prisma.service, health.controller)
- [ ] `npm run build` still succeeds after all changes

## Verification

- `cd backend && npm test` — all tests pass, exit code 0
- `cd backend && npm run build` — TypeScript compiles clean
- `grep -q "TerminusModule" backend/src/health/health.module.ts` — Terminus imported
- `grep -q "HttpExceptionFilter" backend/src/main.ts` — exception filter registered
- `grep -q "TransformInterceptor" backend/src/main.ts` — interceptor registered
- `grep -q "ValidationPipe" backend/src/main.ts` — validation pipe registered
- `test -f backend/test/health.controller.spec.ts` — health test exists
- `test -f backend/test/prisma.service.spec.ts` — prisma test exists

## Observability Impact

- Signals added/changed: HealthModule exposes `/api/health` (Terminus-based), HttpExceptionFilter logs unhandled errors with stack trace
- How a future agent inspects this: `curl localhost:3000/api/health` returns `{ status: "ok", info: { database: { status: "up" } } }`, or `{ status: "error" }` if DB is down
- Failure state exposed: HttpExceptionFilter returns structured `{ success: false, error: { statusCode, message, timestamp, path } }` for any unhandled error

## Inputs

- `backend/package.json` — existing NestJS project from T01
- `backend/src/app.module.ts` — root module to add HealthModule import
- `backend/src/main.ts` — bootstrap to add global filter/interceptor/pipe
- `backend/src/prisma/prisma.service.ts` — PrismaService to mock in tests and use in health indicator
- `backend/src/prisma/prisma.module.ts` — PrismaModule imported by HealthModule

## Expected Output

- `backend/src/health/health.controller.ts` — Health check endpoints
- `backend/src/health/health.module.ts` — Health module with Terminus
- `backend/src/health/prisma-health.indicator.ts` — Database health indicator
- `backend/src/common/dto/api-response.dto.ts` — Standard API response DTO
- `backend/src/common/filters/http-exception.filter.ts` — Global exception filter
- `backend/src/common/interceptors/transform.interceptor.ts` — Response transform interceptor
- `backend/src/app.module.ts` — Updated with HealthModule import
- `backend/src/main.ts` — Updated with global filter, interceptor, validation pipe
- `backend/test/health.controller.spec.ts` — HealthController unit test
- `backend/test/prisma.service.spec.ts` — PrismaService unit test
- `backend/test/jest-e2e.json` — E2E test configuration for future use
