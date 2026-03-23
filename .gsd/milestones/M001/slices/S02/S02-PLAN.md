# S02: Contract & Deploy Module

**Goal:** ContractsModule delivers Solidity compilation (solc) and Monad testnet deployment (ethers.js v6) with DB persistence, and exposes CompileService for S04 engine integration.
**Demo:** POST /api/contracts/deploy with FixedContract source → compiles → deploys to Monad testnet → returns `{ address, txHash }` with DB record. GET /api/contracts/source?type=FixedContract → returns Solidity source. POST /api/contracts/compile → returns ABI + bytecode.

## Must-Haves

- CompileService using solc npm package with Standard JSON Input/Output and `evmVersion: 'cancun'`
- DeployService using ethers.js v6 ContractFactory with DB persistence (pending → deployed/failed)
- ContractsController with 3 endpoints: GET /source, POST /compile, POST /deploy
- ContractsModule exporting CompileService (for S04 EngineModule consumption)
- 4 sample Solidity contracts in `backend/contracts/` (FixedContract, FailingContract, ParallelConflict, PectraTest)
- Request/response DTOs with class-validator decorators
- Config extension for `monad.rpcUrl` and `monad.privateKey`
- Unit tests for CompileService, DeployService, and ContractsController

## Proof Level

- This slice proves: contract
- Real runtime required: no (all network calls mocked in tests)
- Human/UAT required: no

## Verification

- `cd backend && npm run build` — zero TypeScript errors
- `cd backend && npm test` — all existing tests (5) + new tests pass (target: ≥12 new tests across 3 suites)
- `cd backend && npm test -- --testPathPattern=compile.service` — CompileService tests pass (valid compilation, invalid source error, contract name extraction)
- `cd backend && npm test -- --testPathPattern=deploy.service` — DeployService tests pass (success path with DB write, failure path with error recording, missing private key 503)
- `cd backend && npm test -- --testPathPattern=contracts.controller` — Controller tests pass (GET source 200/400, POST compile 200/400, POST deploy 200)
- `grep -q "ContractsModule" backend/src/app.module.ts` — module registered in AppModule
- `grep -q "exports:" backend/src/contracts/contracts.module.ts` — CompileService exported for S04

## Observability / Diagnostics

- Runtime signals: NestJS Logger in CompileService (compilation success/error with contract name), DeployService (deploy attempt/success/failure with address or error)
- Inspection surfaces: Deployment DB table (status, errorMessage, address, txHash), GET /api/contracts/source for contract availability
- Failure visibility: Deployment record with status='failed' + errorMessage; 503 when MONAD_PRIVATE_KEY missing; compilation errors array from solc output
- Redaction constraints: MONAD_PRIVATE_KEY must never appear in logs or API responses

## Integration Closure

- Upstream surfaces consumed: `backend/src/prisma/prisma.service.ts` (Deployment model writes), `backend/src/config/configuration.ts` (extended with monad config), `backend/src/app.module.ts` (ContractsModule import added)
- New wiring introduced in this slice: ContractsModule → AppModule import; CompileService export for S04
- What remains before the milestone is truly usable end-to-end: S03 (auth guards on endpoints), S04 (engine bridge consuming CompileService), S05 (frontend integration), S06 (Railway deploy)

## Tasks

- [x] **T01: Install solc + ethers, create sample contracts, build CompileService with DTOs and tests** `est:45m`
  - Why: CompileService is the foundation — both DeployService and S04's EngineModule depend on it. solc integration is the riskiest piece (EVM version config, error extraction from JSON output). Sample contracts are needed for all tests.
  - Files: `backend/package.json`, `backend/contracts/FixedContract.sol`, `backend/contracts/test/FailingContract.sol`, `backend/contracts/test/ParallelConflict.sol`, `backend/contracts/test/PectraTest.sol`, `backend/src/contracts/compile.service.ts`, `backend/src/contracts/dto/compile-request.dto.ts`, `backend/src/contracts/dto/compile-result.dto.ts`, `backend/test/compile.service.spec.ts`
  - Do: `npm install solc ethers` in backend/. Create 4 sample Solidity contracts (pragma ^0.8.24). Build CompileService wrapping solc Standard JSON Input with evmVersion='cancun'. Create DTOs with class-validator. Write unit tests: valid compilation returns ABI+bytecode, invalid source returns error, contract name extraction works.
  - Verify: `cd backend && npm run build && npm test -- --testPathPattern=compile.service`
  - Done when: CompileService compiles FixedContract.sol successfully in tests, returns ABI array + hex bytecode string, and handles compilation errors gracefully

- [x] **T02: Build DeployService with ethers.js v6 deployment, DB persistence, config extension, and tests** `est:35m`
  - Why: DeployService implements the core deploy flow — compile → deploy → persist to DB. Config extension adds monad RPC/key to the typed config factory. Tests verify the pending→deployed/failed lifecycle with mocked ethers.
  - Files: `backend/src/contracts/deploy.service.ts`, `backend/src/contracts/dto/deploy-request.dto.ts`, `backend/src/config/configuration.ts`, `backend/test/deploy.service.spec.ts`
  - Do: Extend configuration.ts with `monad: { rpcUrl, privateKey }`. Build DeployService injecting PrismaService, ConfigService, CompileService. Implement deploy flow: create pending Deployment → ContractFactory.deploy() → update record. Handle missing private key (503), deploy failure (update status='failed'). Write unit tests with mocked ethers provider/wallet/factory and mocked PrismaService.
  - Verify: `cd backend && npm run build && npm test -- --testPathPattern=deploy.service`
  - Done when: DeployService tests pass — success path creates Deployment with status='deployed', failure path sets status='failed' + errorMessage, missing key returns 503

- [x] **T03: Build ContractsController, ContractsModule, wire into AppModule, and add controller tests** `est:30m`
  - Why: Wires CompileService + DeployService into HTTP endpoints and registers the module in AppModule. This is the integration task that makes the slice demo true.
  - Files: `backend/src/contracts/contracts.controller.ts`, `backend/src/contracts/contracts.module.ts`, `backend/src/app.module.ts`, `backend/test/contracts.controller.spec.ts`
  - Do: Create ContractsController with GET /contracts/source (reads .sol files by type), POST /contracts/compile (delegates to CompileService), POST /contracts/deploy (delegates to DeployService). Create ContractsModule registering controller + services, exporting CompileService. Add ContractsModule to AppModule imports. Write controller tests verifying response shapes, 400 for invalid input, source lookup by type.
  - Verify: `cd backend && npm run build && npm test`
  - Done when: All tests pass (existing 5 + new ≥12), `npm run build` succeeds, ContractsModule registered in AppModule with CompileService exported

## Files Likely Touched

- `backend/package.json`
- `backend/src/config/configuration.ts`
- `backend/src/app.module.ts`
- `backend/contracts/FixedContract.sol`
- `backend/contracts/test/FailingContract.sol`
- `backend/contracts/test/ParallelConflict.sol`
- `backend/contracts/test/PectraTest.sol`
- `backend/src/contracts/contracts.module.ts`
- `backend/src/contracts/contracts.controller.ts`
- `backend/src/contracts/compile.service.ts`
- `backend/src/contracts/deploy.service.ts`
- `backend/src/contracts/dto/compile-request.dto.ts`
- `backend/src/contracts/dto/compile-result.dto.ts`
- `backend/src/contracts/dto/deploy-request.dto.ts`
- `backend/test/compile.service.spec.ts`
- `backend/test/deploy.service.spec.ts`
- `backend/test/contracts.controller.spec.ts`
