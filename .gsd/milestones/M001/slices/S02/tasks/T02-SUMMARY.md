---
id: T02
parent: S02
milestone: M001
provides:
  - DeployService with ethers.js v6 ContractFactory deployment and Prisma DB persistence (pending → deployed/failed lifecycle)
  - DeployRequestDto with class-validator (source required, contractName optional)
  - DeployResult interface (contractName, address, txHash, deploymentId)
  - Configuration extended with monad.rpcUrl and monad.privateKey
  - 9 unit tests covering success path, failure path, missing key 503, and compile error propagation
key_files:
  - backend/src/contracts/deploy.service.ts
  - backend/src/contracts/dto/deploy-request.dto.ts
  - backend/src/config/configuration.ts
  - backend/test/deploy.service.spec.ts
key_decisions:
  - DeployService creates pending Deployment record AFTER compilation succeeds but BEFORE ethers deploy — compilation failures never create DB records
  - ServiceUnavailableException (503) for missing MONAD_PRIVATE_KEY checked before any other operation
  - userId defaults to 'anonymous' when not provided to deploy()
  - ethers imports use named imports (JsonRpcProvider, Wallet, ContractFactory) for tree-shaking and clean mocking
patterns_established:
  - jest.mock('ethers') with mockImplementation for ContractFactory chain (deploy → waitForDeployment → getAddress → deploymentTransaction)
  - Definite assignment assertion (!) on DTO properties to satisfy strict TypeScript (matching T01 pattern)
  - Deployment lifecycle: pending → deployed (address + txHash) or failed (errorMessage)
observability_surfaces:
  - DeployService NestJS Logger: deploy attempt (contractName, userId), success (address, txHash), failure (errorMessage)
  - Deployment DB record with status, errorMessage, address, txHash for post-mortem inspection
  - ServiceUnavailableException (503) response when private key not configured
duration: 8m
verification_result: passed
completed_at: 2026-03-22T12:06:00Z
blocker_discovered: false
---

# T02: Build DeployService with ethers.js v6 deployment, DB persistence, config extension, and tests

**Implemented DeployService with ethers.js v6 ContractFactory deployment, Prisma pending→deployed/failed lifecycle, monad config extension, and 9 passing unit tests**

## What Happened

Extended `configuration.ts` with `monad: { rpcUrl, privateKey }` section defaulting rpcUrl to Monad testnet. Created `DeployRequestDto` with class-validator decorators (`source!: string` required, `contractName?: string` optional) matching the definite assignment pattern from T01.

Implemented `DeployService` as an injectable NestJS service injecting `PrismaService`, `ConfigService`, and `CompileService`. The `deploy(source, userId?)` method:
1. Checks for `monad.privateKey` — throws `ServiceUnavailableException` (503) if missing
2. Calls `CompileService.compile(source)` to get `{ contractName, abi, bytecode }`
3. Creates a `Deployment` record with `status='pending'` via Prisma
4. Creates `JsonRpcProvider` + `Wallet` + `ContractFactory` from ethers.js v6
5. Deploys via `factory.deploy()`, waits with `contract.waitForDeployment()`
6. Updates record to `status='deployed'` with address and txHash on success
7. Updates record to `status='failed'` with errorMessage on any error (then re-throws)

Wrote 9 unit tests with fully mocked ethers module (JsonRpcProvider, Wallet, ContractFactory with deploy chain), mocked PrismaService, and mocked CompileService. Tests cover: pending record creation, deployed update with address/txHash, anonymous userId default, CompileService called with source, failure path DB update, waitForDeployment failure path, missing private key 503, and compile error propagation.

## Verification

- `cd backend && npm run build` — zero TypeScript errors
- `cd backend && npm test -- --testPathPattern=deploy.service` — 9/9 tests pass
- `cd backend && npm test` — 22/22 total tests pass (5 existing + 8 compile + 9 deploy)
- `grep -q "monad" backend/src/config/configuration.ts` — monad config present

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd backend && npm run build` | 0 | ✅ pass | 5.2s |
| 2 | `cd backend && npm test -- --testPathPattern=deploy.service` | 0 | ✅ pass | 3.2s |
| 3 | `cd backend && npm test` | 0 | ✅ pass | 6.5s |
| 4 | `grep -q "monad" backend/src/config/configuration.ts` | 0 | ✅ pass | <1s |

### Slice-level checks (partial — T02 is second of 3 tasks)

| # | Check | Status | Notes |
|---|-------|--------|-------|
| 1 | `npm run build` — zero TS errors | ✅ pass | |
| 2 | `npm test` — all tests pass (target ≥12 new) | ✅ pass | 17 new tests so far (8 compile + 9 deploy), exceeds ≥12 target |
| 3 | CompileService tests pass | ✅ pass | 8/8 |
| 4 | DeployService tests pass | ✅ pass | 9/9 |
| 5 | Controller tests pass | ⏳ not yet | T03 |
| 6 | ContractsModule in AppModule | ⏳ not yet | T03 |
| 7 | CompileService exported | ⏳ not yet | T03 |

## Diagnostics

- **Inspect deployment:** Call `DeployService.deploy(source, userId?)`. Returns `{ contractName, address, txHash, deploymentId }` on success.
- **Missing key:** If `MONAD_PRIVATE_KEY` env var is not set, `deploy()` throws `ServiceUnavailableException` with message `'Server not configured for deployment'` before any DB or network operation.
- **Failure records:** On deploy failure, the Deployment DB record is updated with `status='failed'` and `errorMessage` containing the error message. The original error is re-thrown to the caller.
- **Logs:** DeployService logs via NestJS Logger — `DeployService` context with deploy attempt (contractName, userId), success (address, txHash), and failure (errorMessage).
- **DB lifecycle:** Deployment records follow pending → deployed/failed lifecycle. Compilation failures never create DB records (error thrown before `prisma.deployment.create`).

## Deviations

None — all steps executed as planned.

## Known Issues

- The Deployment model has a foreign key constraint to User (userId → User.id). The service accepts any userId string including 'anonymous', which will fail at DB level if no matching User record exists. This is acceptable for the test-mocked context but will need consideration in T03 controller (either create anonymous user, skip FK, or require auth).

## Files Created/Modified

- `backend/src/config/configuration.ts` — extended with `monad: { rpcUrl, privateKey }` config section
- `backend/src/contracts/deploy.service.ts` — ethers.js v6 deployment service with Prisma DB persistence
- `backend/src/contracts/dto/deploy-request.dto.ts` — deploy request DTO with class-validator
- `backend/test/deploy.service.spec.ts` — 9 unit tests for DeployService with mocked ethers and PrismaService
