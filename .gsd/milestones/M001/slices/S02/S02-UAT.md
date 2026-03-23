# S02: Contract & Deploy Module — UAT

**Milestone:** M001
**Written:** 2026-03-22

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All network calls (Monad RPC, ethers.js) are mocked in tests. This slice's proof level is "contract" — real runtime is not required. The 34 passing unit tests verify compilation, deployment lifecycle, and HTTP endpoint behavior.

## Preconditions

- Working directory is the worktree root containing `backend/`
- `cd backend && npm install` has been run (node_modules present)
- Node.js ≥18 available
- No live services required (all tests use mocks)

## Smoke Test

```bash
cd backend && npm run build && npm test
```
Expected: zero TS errors, 34/34 tests pass across 5 suites (prisma, health, compile, deploy, controller).

## Test Cases

### 1. CompileService compiles valid Solidity source

1. Run `cd backend && npm test -- --testPathPattern=compile.service`
2. **Expected:** 8/8 tests pass. FixedContract compiles with ABI array (length > 0), bytecode string starting with `0x`, and contractName = `FixedContract`.

### 2. CompileService handles compilation errors

1. In the compile.service.spec.ts test suite, the test `should throw BadRequestException for invalid source` passes.
2. **Expected:** `BadRequestException` thrown with `message: 'Compilation failed'` and `errors` array containing solc error messages.

### 3. CompileService supports cancun EVM (TSTORE/TLOAD)

1. In the compile.service.spec.ts suite, the PectraTest compilation test passes.
2. **Expected:** PectraTest.sol (which uses TSTORE/TLOAD opcodes) compiles successfully with `evmVersion: 'cancun'`. Contract name extracted correctly.

### 4. DeployService success path creates deployed record

1. Run `cd backend && npm test -- --testPathPattern=deploy.service`
2. **Expected:** 9/9 tests pass. Success path: `prisma.deployment.create` called with `status: 'pending'`, then `prisma.deployment.update` called with `status: 'deployed'`, `address` (0x-prefixed), and `txHash`. Method returns `{ contractName, address, txHash, deploymentId }`.

### 5. DeployService failure path records error

1. In deploy.service.spec.ts, the failure tests pass.
2. **Expected:** When ethers deployment fails, `prisma.deployment.update` is called with `status: 'failed'` and `errorMessage` containing the error message. The original error is re-thrown.

### 6. DeployService rejects when private key is missing

1. In deploy.service.spec.ts, the missing private key test passes.
2. **Expected:** `ServiceUnavailableException` (503) thrown with message `'Server not configured for deployment'` before any DB or network operation.

### 7. ContractsController GET /source returns Solidity source

1. Run `cd backend && npm test -- --testPathPattern=contracts.controller`
2. **Expected:** 12/12 tests pass. GET /source with `type=FixedContract` returns `{ contractType: 'FixedContract', source: '...' }` containing valid Solidity source with `pragma solidity`.

### 8. ContractsController GET /source validates contract type

1. In contracts.controller.spec.ts, invalid type tests pass.
2. **Expected:** `BadRequestException` thrown for: invalid type string, missing type parameter, empty string. Error message: `'Invalid contract type'`.

### 9. ContractsController POST /compile delegates to CompileService

1. In contracts.controller.spec.ts, the compile delegation test passes.
2. **Expected:** Controller calls `CompileService.compile(source)` and returns the result directly. Response shape: `{ contractName, abi, bytecode }`.

### 10. ContractsController POST /deploy delegates to DeployService

1. In contracts.controller.spec.ts, the deploy delegation test passes.
2. **Expected:** Controller calls `DeployService.deploy(source)` and returns the result. Response shape: `{ contractName, address, txHash, deploymentId }`.

### 11. ContractsModule exports CompileService for S04

1. Run `grep -A5 "exports:" backend/src/contracts/contracts.module.ts`
2. **Expected:** `CompileService` is listed in the module's `exports` array, making it available for injection by S04's EngineModule.

### 12. ContractsModule registered in AppModule

1. Run `grep "ContractsModule" backend/src/app.module.ts`
2. **Expected:** ContractsModule appears in AppModule's imports array.

### 13. All 4 sample contracts exist and are valid Solidity

1. Run:
   ```bash
   test -f backend/contracts/FixedContract.sol && echo OK
   test -f backend/contracts/test/FailingContract.sol && echo OK
   test -f backend/contracts/test/ParallelConflict.sol && echo OK
   test -f backend/contracts/test/PectraTest.sol && echo OK
   ```
2. **Expected:** All 4 files exist. Each contains `pragma solidity ^0.8.24;`.

### 14. Configuration includes monad settings

1. Run `grep -A4 "monad:" backend/src/config/configuration.ts`
2. **Expected:** Config factory returns `monad: { rpcUrl: process.env.MONAD_RPC_URL || '...testnet...', privateKey: process.env.MONAD_PRIVATE_KEY || '' }`.

## Edge Cases

### Empty source string to POST /compile

1. POST /compile with `{ source: '' }`
2. **Expected:** CompileService throws `BadRequestException` because solc receives empty input. DTO validation (`@IsNotEmpty()`) rejects before reaching service if ValidationPipe is active.

### All 4 contract types compile successfully

1. CompileService.compile() called with each of the 4 sample contracts in the test suite.
2. **Expected:** All compile without errors. PectraTest specifically validates cancun EVM support (TSTORE/TLOAD opcodes).

### Deploy with compile error

1. DeployService.deploy() called with invalid Solidity source.
2. **Expected:** CompileService throws BadRequestException. No Deployment record is created in DB (compile happens before `prisma.deployment.create`).

### Deploy with network failure after pending record

1. DeployService.deploy() called, ethers ContractFactory.deploy() throws network error.
2. **Expected:** Deployment record updated from `pending` to `failed` with error message. Original error re-thrown to caller.

## Failure Signals

- `npm run build` fails → TypeScript compilation broken, check new files for type errors
- `npm test` shows failures → check which suite fails (compile/deploy/controller), read error output
- `grep ContractsModule backend/src/app.module.ts` returns nothing → module not wired
- `grep exports backend/src/contracts/contracts.module.ts` returns nothing → S04 cannot consume CompileService
- Missing .sol files in `backend/contracts/` → T01 contract creation failed

## Not Proven By This UAT

- **Live Monad testnet deployment** — all ethers.js calls are mocked. Real deployment requires MONAD_PRIVATE_KEY and network connectivity (proven in S06 E2E).
- **Auth guards** — all endpoints are publicly accessible. Auth is S03's responsibility.
- **userId FK constraint at runtime** — mocked PrismaService doesn't enforce FK. Real DB will reject 'anonymous' userId without a matching User record (resolved in S03).
- **TransformInterceptor wrapping** — controller tests verify raw return values. The `{ success: true, data: ... }` wrapping is tested at the HTTP integration level.
- **Concurrent compilation/deployment** — no load or concurrency tests. solc is synchronous and CPU-bound.

## Notes for Tester

- The compile.service tests take 4-10 seconds because solc is CPU-intensive. Jest timeout is set to 30s for these tests.
- DeployService error logs in test output (`ERROR [DeployService] Deploy failed:`) are expected — they come from the failure path tests exercising the logger.
- The test count target was ≥12 new tests. Actual: 29 new tests (8+9+12), significantly exceeding the target.
