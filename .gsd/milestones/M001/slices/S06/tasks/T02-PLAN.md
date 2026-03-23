---
estimated_steps: 4
estimated_files: 2
---

# T02: Write E2E test suite with supertest and mocked externals

**Slice:** S06 — Railway Deploy + E2E Validation
**Milestone:** M001

## Description

Install supertest and write comprehensive E2E tests that boot a real NestJS app instance via `Test.createTestingModule` with the full `AppModule`, overriding PrismaService and external services with mocks. This validates the actual HTTP layer — routing, guards, validation pipes, exception filters, and interceptors — which unit tests don't exercise. Directly satisfies requirement R010 (E2E integration tests).

**Key constraints:**
- PrismaService must be overridden: the HealthController's readiness check calls `$queryRaw`, and PrismaService.onModuleInit calls `$connect`. Without a DB, both will hang/fail.
- EngineService returns null when the binary isn't available — vibe-score falls back to heuristic scoring automatically. No override needed for EngineService.
- AnalysisController uses `@Res()` and manually wraps responses in `{ success, data }` — the test must check the response shape.
- PaymasterController uses `@UseGuards(JwtAuthGuard)` — without a JWT token, requests should get 401.
- ContractsController reads `contracts/FixedContract.sol` from `process.cwd()` — the file exists at `backend/contracts/FixedContract.sol` and `process.cwd()` during tests is `backend/`.
- CompileService uses real `solc` npm — POST /api/contracts/compile should work with actual Solidity source.
- The `backend/test/jest-e2e.json` config already exists with pattern `.e2e-spec.ts$` and `rootDir: "."`.
- The GlobalPrefix `api` is set in `main.ts` bootstrap, NOT via module config. The E2E test must call `app.setGlobalPrefix('api')` on the test app before initializing.
- TransformInterceptor wraps responses in `{ success, data }` — the E2E test must also set this up if testing response envelopes. The health module uses Terminus which bypasses the interceptor.
- `supertest` is not yet installed — add both `supertest` and `@types/supertest` as devDependencies.

**Relevant skills:** `test` skill may be useful for test patterns.

## Steps

1. **Install supertest** — Run `cd backend && npm install --save-dev supertest @types/supertest`. Verify they appear in `package.json` devDependencies.

2. **Create `backend/test/app.e2e-spec.ts`** — E2E test file with:

   ```typescript
   import { Test, TestingModule } from '@nestjs/testing';
   import { INestApplication, ValidationPipe } from '@nestjs/common';
   import * as request from 'supertest';
   import { AppModule } from '../src/app.module';
   import { PrismaService } from '../src/prisma/prisma.service';
   import { TransformInterceptor } from '../src/common/interceptors/transform.interceptor';
   import { HttpExceptionFilter } from '../src/common/filters/http-exception.filter';
   ```

   **beforeAll:** Create testing module from AppModule, override PrismaService with mock that stubs:
   - `$connect` → resolves void
   - `$queryRaw` → resolves `[{ result: 1 }]` (for health check)
   - `onModuleInit` → resolves void (prevent real DB connection)
   - `deployment.create` / `vibeScore.create` → resolves mock objects (for services that persist)

   Create the NestJS app, set global prefix 'api', apply ValidationPipe, TransformInterceptor, and HttpExceptionFilter (matching main.ts setup). Initialize the app.

   **afterAll:** Close the app.

   **Test cases:**
   - `GET /api/health` → 200 with `status: 'ok'` in body
   - `GET /api/health/readiness` → 200 with `status: 'ok'` in body
   - `GET /api/contracts/source?type=FixedContract` → 200 with `data.source` containing `pragma solidity`
   - `GET /api/contracts/source?type=Invalid` → 400 with error message
   - `POST /api/contracts/compile` with `{ "source": "<valid solidity>" }` → 200 with `data.bytecode` and `data.abi` (uses real solc compiler)
   - `POST /api/vibe-score` with `{ "source": "<valid solidity>" }` → 200 with `data.vibeScore` as number (heuristic fallback since no engine binary)
   - `POST /api/analysis/error` with `{ "error": "revert", "contractSource": "...", "errorCode": "CALL_EXCEPTION" }` → 200 with response body (analysis controller uses @Res() so check manually)
   - `GET /api/paymaster/status` without Authorization header → 401

3. **Use the FixedContract.sol source** for compile and vibe-score tests — read from `contracts/FixedContract.sol` via `fs.readFileSync` in the test file, or inline a minimal valid Solidity contract:
   ```solidity
   // SPDX-License-Identifier: MIT
   pragma solidity ^0.8.20;
   contract Simple { uint256 public value; function setValue(uint256 v) public { value = v; } }
   ```

4. **Run tests and verify** — `cd backend && npm run test:e2e` should pass all cases. Then run `cd backend && npm test` to confirm 97 unit tests still pass.

## Must-Haves

- [ ] `supertest` and `@types/supertest` in devDependencies
- [ ] `backend/test/app.e2e-spec.ts` exists with 8+ test cases
- [ ] PrismaService is mocked (no real DB connection during tests)
- [ ] Health endpoint tests pass (200)
- [ ] Contract source valid/invalid tests pass (200/400)
- [ ] Compile test passes with real solc (200 with bytecode)
- [ ] Vibe-score test passes with heuristic fallback (200 with score)
- [ ] Analysis error test passes (200)
- [ ] Paymaster status returns 401 without JWT
- [ ] All 97 existing unit tests still pass

## Verification

- `cd backend && npm run test:e2e` — all E2E tests pass
- `cd backend && npm test` — 97 unit tests still pass (no regressions)
- `grep -q 'supertest' backend/package.json` — supertest is installed

## Inputs

- `backend/src/app.module.ts` — Full AppModule with all module imports
- `backend/src/main.ts` — Bootstrap config (global prefix, pipes, filters, interceptors) to replicate in test
- `backend/src/prisma/prisma.service.ts` — PrismaService to mock
- `backend/src/common/interceptors/transform.interceptor.ts` — TransformInterceptor that wraps responses
- `backend/src/common/filters/http-exception.filter.ts` — HttpExceptionFilter for error responses
- `backend/test/jest-e2e.json` — Existing E2E test config
- `backend/contracts/FixedContract.sol` — Valid Solidity source for compile/vibe-score tests
- `backend/src/health/health.controller.ts` — Health endpoints using Terminus + PrismaHealthIndicator
- `backend/src/contracts/contracts.controller.ts` — Contract source/compile/deploy endpoints
- `backend/src/vibe-score/vibe-score.controller.ts` — Vibe-score endpoint
- `backend/src/analysis/analysis.controller.ts` — Analysis error endpoint with @Res() pattern
- `backend/src/paymaster/paymaster.controller.ts` — Paymaster with JwtAuthGuard
- `backend/package.json` — To add supertest dependencies

## Observability Impact

- **E2E test output**: `npm run test:e2e` outputs pass/fail for all 12 test cases with HTTP status codes and response body assertions — visible in CI logs and local terminal
- **Failure visibility**: Each E2E test shows expected vs actual HTTP status and response body when failing, enabling quick diagnosis of routing, guard, or validation regressions
- **Mock transparency**: PrismaService mock stubs are defined at file top with clear `jest.fn()` return values — future agents can inspect which DB calls were stubbed

## Expected Output

- `backend/test/app.e2e-spec.ts` — E2E test file with 8+ test cases covering health, contracts, vibe-score, analysis, paymaster auth
- `backend/package.json` — Updated with supertest and @types/supertest in devDependencies
