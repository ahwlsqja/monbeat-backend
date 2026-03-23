---
id: S02
parent: M001
milestone: M001
provides:
  - CompileService wrapping solc Standard JSON Input/Output with evmVersion=cancun, exported for S04 EngineModule consumption
  - DeployService with ethers.js v6 ContractFactory deployment and Prisma DB persistence (pending → deployed/failed lifecycle)
  - ContractsController with 3 HTTP endpoints (GET /source, POST /compile, POST /deploy)
  - ContractsModule registered in AppModule with CompileService export
  - 4 sample Solidity contracts (FixedContract, FailingContract, ParallelConflict, PectraTest)
  - Configuration extended with monad.rpcUrl and monad.privateKey
  - Request/response DTOs with class-validator decorators
  - 29 new unit tests across 3 test suites (8 compile + 9 deploy + 12 controller)
requires:
  - slice: S01
    provides: PrismaService (Deployment model writes), ConfigModule (typed config factory), AppModule (module registration), BaseController pattern
affects:
  - S04 (consumes CompileService.compile() for bytecode → engine pipeline)
  - S05 (consumes all 3 HTTP endpoints for frontend integration)
key_files:
  - backend/src/contracts/compile.service.ts
  - backend/src/contracts/deploy.service.ts
  - backend/src/contracts/contracts.controller.ts
  - backend/src/contracts/contracts.module.ts
  - backend/src/contracts/dto/compile-request.dto.ts
  - backend/src/contracts/dto/compile-result.dto.ts
  - backend/src/contracts/dto/deploy-request.dto.ts
  - backend/src/config/configuration.ts
  - backend/contracts/FixedContract.sol
  - backend/contracts/test/FailingContract.sol
  - backend/contracts/test/ParallelConflict.sol
  - backend/contracts/test/PectraTest.sol
  - backend/test/compile.service.spec.ts
  - backend/test/deploy.service.spec.ts
  - backend/test/contracts.controller.spec.ts
key_decisions:
  - solc errors extracted from output.errors array filtered by severity='error'; warnings ignored for compilation success/failure determination
  - Bytecode always prefixed with '0x' for ethers.js v6 compatibility
  - Contract name extracted from first key of output.contracts['Contract.sol'] — single-contract-per-source assumption
  - DeployService creates pending Deployment record AFTER compilation succeeds but BEFORE ethers deploy — compilation failures never create DB records
  - ServiceUnavailableException (503) for missing MONAD_PRIVATE_KEY checked before any other operation
  - userId defaults to 'anonymous' when not provided to deploy()
  - GET /source uses process.cwd() for contract file path resolution with VALID_CONTRACT_TYPES whitelist
  - Controller returns raw data objects — TransformInterceptor wraps as { success: true, data: ... }
patterns_established:
  - solc Standard JSON Input with evmVersion='cancun' and outputSelection for abi + evm.bytecode.object
  - BadRequestException with structured { message, errors[] } for compilation failures
  - jest.mock('ethers') with mockImplementation for ContractFactory chain (deploy → waitForDeployment → getAddress → deploymentTransaction)
  - Definite assignment assertion (!) on DTO properties to satisfy strict TypeScript
  - Deployment lifecycle: pending → deployed (address + txHash) or failed (errorMessage)
  - VALID_CONTRACT_TYPES const array with includes() for whitelist validation
observability_surfaces:
  - CompileService NestJS Logger: compilation success/error with contract name, ABI count, bytecode length
  - DeployService NestJS Logger: deploy attempt (contractName, userId), success (address, txHash), failure (errorMessage)
  - ContractsController NestJS Logger: source retrieval (type), read errors
  - Deployment DB records with status, errorMessage, address, txHash for post-mortem inspection
  - ServiceUnavailableException (503) when MONAD_PRIVATE_KEY not configured
  - BadRequestException (400) with solc error messages for compilation failures
drill_down_paths:
  - .gsd/milestones/M001/slices/S02/tasks/T01-SUMMARY.md
  - .gsd/milestones/M001/slices/S02/tasks/T02-SUMMARY.md
  - .gsd/milestones/M001/slices/S02/tasks/T03-SUMMARY.md
duration: 25m
verification_result: passed
completed_at: 2026-03-22T12:06:00Z
---

# S02: Contract & Deploy Module

**Solidity compilation via solc (cancun EVM) + Monad testnet deployment via ethers.js v6 with Prisma DB persistence, exposed as 3 REST endpoints with CompileService exported for S04 engine integration — 34/34 tests passing**

## What Happened

Built the complete contract compilation and deployment pipeline in three tasks.

**T01 (CompileService + sample contracts):** Installed `solc@^0.8.34` and `ethers@^6.16.0`. Created 4 sample Solidity contracts covering different patterns: FixedContract (simple storage baseline for vibe-score), FailingContract (always-reverts for error paths), ParallelConflict (global counter bottleneck — key for S04 vibe-score differentiation), and PectraTest (TSTORE/TLOAD transient storage requiring cancun EVM). Implemented CompileService wrapping solc's Standard JSON Input/Output with `evmVersion: 'cancun'`, error extraction filtering by `severity === 'error'`, and `0x`-prefixed bytecode output. 8 unit tests covering valid compilation, error handling, and cancun EVM compatibility.

**T02 (DeployService + config extension):** Extended `configuration.ts` with `monad: { rpcUrl, privateKey }` section. Implemented DeployService with the full deployment lifecycle: private key check (503 if missing) → CompileService.compile() → create pending Deployment record → ethers.js v6 ContractFactory.deploy() → update to deployed/failed. Compilation failures never create DB records. 9 unit tests with fully mocked ethers module and PrismaService.

**T03 (Controller + Module wiring):** Created ContractsController with GET /source (whitelist-validated contract type → raw Solidity), POST /compile (→ CompileService), POST /deploy (→ DeployService). Created ContractsModule exporting CompileService for S04. Wired into AppModule. 12 controller unit tests using real .sol files from T01.

## Verification

| # | Check | Result |
|---|-------|--------|
| 1 | `cd backend && npm run build` — zero TS errors | ✅ pass |
| 2 | `cd backend && npm test` — 34/34 tests pass (5 suites) | ✅ pass |
| 3 | `npm test -- --testPathPattern=compile.service` — 8/8 | ✅ pass |
| 4 | `npm test -- --testPathPattern=deploy.service` — 9/9 | ✅ pass |
| 5 | `npm test -- --testPathPattern=contracts.controller` — 12/12 | ✅ pass |
| 6 | ContractsModule registered in AppModule | ✅ pass |
| 7 | CompileService exported from ContractsModule | ✅ pass |
| 8 | All 4 sample .sol contract files exist | ✅ pass |

29 new tests added (target was ≥12), bringing total to 34 across 5 suites.

## New Requirements Surfaced

- none

## Deviations

None — all three tasks executed as planned with zero deviations.

## Known Limitations

- **Deployment FK constraint**: Deployment model has a foreign key to User (userId → User.id). DeployService defaults to 'anonymous' userId, which will fail at DB level without a matching User record. S03 (auth) must resolve this — either create an anonymous user seed, make userId nullable, or require auth on deploy endpoint.
- **Single-contract-per-source assumption**: CompileService extracts the first contract name from solc output. Multi-contract sources will silently use only the first contract.
- **No auth guards**: All three endpoints are publicly accessible. S03 will add JwtAuthGuard.
- **process.cwd() contract path**: GET /source resolves contracts via `process.cwd()`. This works in dev and Docker but is fragile if the CWD changes (e.g. PM2 with different working directories).

## Follow-ups

- S03 must decide how to handle the userId FK constraint for unauthenticated deploy calls
- S04 will consume `CompileService.compile()` to get bytecode for engine simulation — the `0x` prefix convention matches ethers.js expectations
- S05 will call all 3 HTTP endpoints — the response shape follows TransformInterceptor `{ success: true, data: ... }` pattern

## Files Created/Modified

- `backend/package.json` — added solc and ethers dependencies
- `backend/src/config/configuration.ts` — extended with monad.rpcUrl and monad.privateKey
- `backend/src/app.module.ts` — added ContractsModule to imports
- `backend/contracts/FixedContract.sol` — simple storage contract (vibe-score baseline)
- `backend/contracts/test/FailingContract.sol` — always-reverts contract (error path testing)
- `backend/contracts/test/ParallelConflict.sol` — global counter bottleneck pattern
- `backend/contracts/test/PectraTest.sol` — TSTORE/TLOAD transient storage (cancun EVM)
- `backend/src/contracts/compile.service.ts` — solc wrapper with cancun EVM and error extraction
- `backend/src/contracts/deploy.service.ts` — ethers.js v6 deployment with Prisma persistence
- `backend/src/contracts/contracts.controller.ts` — 3-endpoint HTTP controller
- `backend/src/contracts/contracts.module.ts` — NestJS module with CompileService export
- `backend/src/contracts/dto/compile-request.dto.ts` — compile request DTO
- `backend/src/contracts/dto/compile-result.dto.ts` — compile result interface
- `backend/src/contracts/dto/deploy-request.dto.ts` — deploy request DTO
- `backend/test/compile.service.spec.ts` — 8 CompileService unit tests
- `backend/test/deploy.service.spec.ts` — 9 DeployService unit tests
- `backend/test/contracts.controller.spec.ts` — 12 controller unit tests

## Forward Intelligence

### What the next slice should know
- CompileService is already exported from ContractsModule — S04 can import ContractsModule and inject CompileService directly. The `compile(source)` method returns `{ contractName, abi, bytecode }` where bytecode is `0x`-prefixed hex.
- The 4 sample contracts in `backend/contracts/` are the test fixtures S04 needs. ParallelConflict (global counter) should score lower than FixedContract (simple storage) in parallel execution analysis.
- DeployService's Deployment model expects a valid userId FK. Until S03 wires auth, any live deploy calls with 'anonymous' userId will fail on the DB constraint.

### What's fragile
- **userId 'anonymous' default in DeployService** — works in test (PrismaService is mocked) but will fail at runtime until S03 resolves the FK constraint or makes userId nullable in the Prisma schema.
- **process.cwd() for contract file paths** — GET /source resolves file paths relative to CWD. Works locally and in Docker (`WORKDIR /app`) but could break if the process is started from a different directory.

### Authoritative diagnostics
- `cd backend && npm test` — 34 tests across 5 suites is the authoritative signal that S02 code is intact. If any downstream change breaks compilation or deployment logic, this suite catches it.
- `Deployment` DB table with `status` field — the definitive record of deploy lifecycle. Query for `status='failed'` to find error records with `errorMessage`.

### What assumptions changed
- No assumptions changed — the plan's approach (solc Standard JSON, ethers.js v6, Prisma persistence) worked exactly as designed.
