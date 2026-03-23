---
estimated_steps: 4
estimated_files: 4
---

# T02: Build DeployService with ethers.js v6 deployment, DB persistence, config extension, and tests

**Slice:** S02 — Contract & Deploy Module
**Milestone:** M001

## Description

Extend the configuration factory with Monad network settings, implement DeployService using ethers.js v6 ContractFactory for Monad testnet deployment with Prisma-based deployment record persistence (pending → deployed/failed lifecycle), and write unit tests with mocked ethers and PrismaService.

DeployService injects CompileService (from T01) to compile source before deploying. It creates a Deployment record with status='pending' before attempting deployment, then updates it to 'deployed' (with address, txHash, gasUsed) on success or 'failed' (with errorMessage) on failure. Missing MONAD_PRIVATE_KEY results in a 503 ServiceUnavailableException rather than a crash.

**Relevant skills:** None required — standard NestJS service with ethers.js v6 patterns.

## Steps

1. **Extend configuration.ts:** Add `monad` section to the config factory in `backend/src/config/configuration.ts`:
   ```typescript
   monad: {
     rpcUrl: process.env.MONAD_RPC_URL ?? 'https://testnet-rpc.monad.xyz',
     privateKey: process.env.MONAD_PRIVATE_KEY,
   },
   ```

2. **Create deploy-request DTO:** Create `backend/src/contracts/dto/deploy-request.dto.ts`:
   - `DeployRequestDto` with `source: string` (required, `@IsString()`, `@IsNotEmpty()`) and optional `contractName?: string`

3. **Implement DeployService:** Create `backend/src/contracts/deploy.service.ts`:
   - Injectable NestJS service with Logger
   - Inject `PrismaService`, `ConfigService`, `CompileService`
   - `async deploy(source: string, userId?: string): Promise<DeployResult>` method
   - Check `configService.get<string>('monad.privateKey')` — if missing, throw `ServiceUnavailableException('Server not configured for deployment')`
   - Call `compileService.compile(source)` to get `{ contractName, abi, bytecode }`
   - Create Deployment record via `prismaService.deployment.create()` with status='pending', userId (use 'anonymous' if not provided), contractName, contractSource
   - Create `ethers.JsonRpcProvider(rpcUrl)` and `ethers.Wallet(privateKey, provider)`
   - Create `ethers.ContractFactory(abi, bytecode, wallet)` and call `factory.deploy()`
   - Wait for deployment: `await contract.waitForDeployment()`
   - Get address via `await contract.getAddress()` and txHash via `contract.deploymentTransaction()?.hash`
   - Update Deployment record: status='deployed', address, txHash
   - On any error: update Deployment record with status='failed', errorMessage, and re-throw
   - Log deployment attempts and results

4. **Write unit tests:** Create `backend/test/deploy.service.spec.ts`:
   - Mock PrismaService with `deployment: { create: jest.fn(), update: jest.fn() }`
   - Mock ConfigService with `get()` returning rpcUrl and privateKey
   - Mock CompileService with `compile()` returning valid ABI + bytecode
   - Mock ethers module: `JsonRpcProvider`, `Wallet`, `ContractFactory` (with deploy → waitForDeployment → getAddress → deploymentTransaction chain)
   - Test: successful deploy → Deployment created with status='pending', then updated to status='deployed' with address + txHash
   - Test: deploy failure → Deployment updated to status='failed' with errorMessage
   - Test: missing MONAD_PRIVATE_KEY → throws ServiceUnavailableException (503)
   - Test: compile error propagates (CompileService throws BadRequestException)

## Must-Haves

- [ ] configuration.ts extended with `monad.rpcUrl` and `monad.privateKey`
- [ ] DeployService creates pending Deployment record before attempting deploy
- [ ] DeployService updates record to 'deployed' with address/txHash on success
- [ ] DeployService updates record to 'failed' with errorMessage on failure
- [ ] Missing MONAD_PRIVATE_KEY throws ServiceUnavailableException (503)
- [ ] DeployRequestDto validates `source` field
- [ ] All unit tests pass with mocked ethers and PrismaService

## Verification

- `cd backend && npm run build` — zero TypeScript errors
- `cd backend && npm test -- --testPathPattern=deploy.service` — all DeployService tests pass
- `grep -q "monad" backend/src/config/configuration.ts` — config extension present

## Observability Impact

- Signals added/changed: Logger.log on deploy attempt (contractName, userId), Logger.log on success (address, txHash), Logger.error on failure (errorMessage)
- How a future agent inspects this: Query Deployment table for status/errorMessage; check NestJS logs for deploy lifecycle events
- Failure state exposed: Deployment record with status='failed' + errorMessage; 503 response when private key not configured

## Inputs

- `backend/src/contracts/compile.service.ts` — CompileService from T01 (injected dependency)
- `backend/src/contracts/dto/compile-result.dto.ts` — CompileResultDto type from T01
- `backend/src/prisma/prisma.service.ts` — PrismaService for DB writes (from S01)
- `backend/src/config/configuration.ts` — config factory to extend
- `backend/prisma/schema.prisma` — Deployment model reference (no changes needed)

## Expected Output

- `backend/src/config/configuration.ts` — extended with monad config section
- `backend/src/contracts/deploy.service.ts` — ethers.js v6 deployment + DB persistence service
- `backend/src/contracts/dto/deploy-request.dto.ts` — deploy request DTO
- `backend/test/deploy.service.spec.ts` — DeployService unit tests
