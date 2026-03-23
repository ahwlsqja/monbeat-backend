---
id: T03
parent: S02
milestone: M001
provides:
  - ContractsController with 3 HTTP endpoints (GET /source, POST /compile, POST /deploy)
  - ContractsModule registering controller + services and exporting CompileService for S04
  - AppModule wired with ContractsModule import
  - 12 controller unit tests covering source retrieval, compile delegation, deploy delegation, and error paths
key_files:
  - backend/src/contracts/contracts.controller.ts
  - backend/src/contracts/contracts.module.ts
  - backend/src/app.module.ts
  - backend/test/contracts.controller.spec.ts
key_decisions:
  - GET /source uses process.cwd() for contract file path resolution — contracts/ lives alongside src/ at backend root
  - Source endpoint wraps fs.readFileSync in try/catch to convert file-not-found to BadRequestException
  - Controller tests read actual .sol files from disk (created in T01) rather than mocking fs — validates real integration
patterns_established:
  - Controller methods return raw data objects — TransformInterceptor wraps them as { success: true, data: ... }
  - VALID_CONTRACT_TYPES const array with includes() check for whitelist validation before fs.readFileSync
  - async deploy() endpoint delegates to DeployService and returns the DeployResult directly
observability_surfaces:
  - ContractsController NestJS Logger: source retrieval (type), read errors (type + error message)
  - GET /api/contracts/source?type=FixedContract — inspect contract availability and content
  - POST /api/contracts/compile — compile endpoint returns ABI + bytecode or 400 with error details
  - POST /api/contracts/deploy — deploy endpoint returns address + txHash or propagates error
duration: 5m
verification_result: passed
completed_at: 2026-03-22T12:05:00Z
blocker_discovered: false
---

# T03: Build ContractsController, ContractsModule, wire into AppModule, and add controller tests

**Created ContractsController with GET /source, POST /compile, POST /deploy endpoints, ContractsModule with CompileService export, wired into AppModule, and added 12 controller unit tests — all 34 tests pass**

## What Happened

Created `ContractsController` with three endpoints: GET /source validates contract type against a whitelist (FixedContract, FailingContract, ParallelConflict, PectraTest), reads the .sol file from disk using `path.join(process.cwd(), 'contracts', ...)`, and returns `{ contractType, source }`. POST /compile delegates to CompileService. POST /deploy delegates to DeployService. Invalid or missing contract types throw BadRequestException.

Created `ContractsModule` with controllers, providers (CompileService, DeployService), and exports (CompileService for S04's EngineModule). Added ContractsModule to AppModule imports.

Wrote 12 controller unit tests: source retrieval for all 4 contract types using real .sol files from T01, BadRequestException for invalid type/missing type/empty string, compile delegation with result verification, compile error propagation, deploy delegation with result verification, and deploy error propagation.

## Verification

- `cd backend && npm run build` — zero TypeScript errors (exit 0)
- `cd backend && npm test` — 34/34 tests pass across 5 suites
- `cd backend && npm test -- --testPathPattern=contracts.controller` — 12/12 controller tests pass
- `grep -q "ContractsModule" backend/src/app.module.ts` — ContractsModule registered
- `grep -q "exports:" backend/src/contracts/contracts.module.ts` — CompileService exported

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd backend && npm run build` | 0 | ✅ pass | 3.9s |
| 2 | `cd backend && npm test` | 0 | ✅ pass | 8.0s |
| 3 | `cd backend && npm test -- --testPathPattern=compile.service` | 0 | ✅ pass | 5.0s |
| 4 | `cd backend && npm test -- --testPathPattern=deploy.service` | 0 | ✅ pass | 4.8s |
| 5 | `cd backend && npm test -- --testPathPattern=contracts.controller` | 0 | ✅ pass | 5.1s |
| 6 | `grep -q "ContractsModule" backend/src/app.module.ts` | 0 | ✅ pass | <1s |
| 7 | `grep -q "exports:" backend/src/contracts/contracts.module.ts` | 0 | ✅ pass | <1s |

### Slice-level checks (final — T03 is last of 3 tasks)

| # | Check | Status | Notes |
|---|-------|--------|-------|
| 1 | `npm run build` — zero TS errors | ✅ pass | |
| 2 | `npm test` — all tests pass (target ≥12 new) | ✅ pass | 29 new tests (8 compile + 9 deploy + 12 controller), exceeds ≥12 target |
| 3 | CompileService tests pass | ✅ pass | 8/8 |
| 4 | DeployService tests pass | ✅ pass | 9/9 |
| 5 | Controller tests pass | ✅ pass | 12/12 |
| 6 | ContractsModule in AppModule | ✅ pass | |
| 7 | CompileService exported | ✅ pass | |

## Diagnostics

- **Source endpoint:** `GET /api/contracts/source?type=FixedContract` returns `{ contractType, source }` with raw Solidity source. Invalid types return 400 with `'Invalid contract type'`.
- **Compile endpoint:** `POST /api/contracts/compile` with `{ source }` body returns `{ contractName, abi, bytecode }` or 400 with compilation errors.
- **Deploy endpoint:** `POST /api/contracts/deploy` with `{ source }` body returns `{ contractName, address, txHash, deploymentId }` or propagates 503/400 errors.
- **Logs:** ContractsController logs via NestJS Logger — `ContractsController` context with source retrieval (type) and file read errors.

## Deviations

None — all steps executed as planned.

## Known Issues

None.

## Files Created/Modified

- `backend/src/contracts/contracts.controller.ts` — 3-endpoint HTTP controller (GET /source, POST /compile, POST /deploy)
- `backend/src/contracts/contracts.module.ts` — NestJS module with CompileService export for S04
- `backend/src/app.module.ts` — added ContractsModule to imports
- `backend/test/contracts.controller.spec.ts` — 12 controller unit tests
