# S02: Contract & Deploy Module — Research

**Date:** 2026-03-22
**Depth:** Targeted

## Summary

S02 delivers the ContractsModule — three API endpoints for contract source retrieval, Solidity compilation, and Monad testnet deployment, plus DB persistence of deployment history. The Vibe-Loom prior art spawns Hardhat CLI as a subprocess for both compile and deploy, which is heavyweight and fragile for a long-running NestJS service. The better approach is to use `solc` (JavaScript Solidity compiler bindings) for programmatic compilation and `ethers.js` v6 `ContractFactory` for deployment — both are library calls, no subprocess needed.

The Prisma schema already has the `Deployment` model with all required fields (userId, contractName, contractSource, address, txHash, network, status, errorMessage, gasUsed). S01's `PrismaModule` is `@Global()` and `ConfigModule` is `isGlobal: true`, so S02 just injects `PrismaService` and `ConfigService` without module imports.

The S04 engine bridge will need compiled bytecode + ABI from S02's compile service. This means the compile service must be independently injectable (not just an internal helper for the deploy endpoint).

## Recommendation

Use `solc` npm package for compilation and `ethers.js` v6 for deployment. This avoids Hardhat subprocess spawning entirely. Structure the module as:

- **ContractsModule** — NestJS module registering controller + two services
- **ContractsController** — 3 endpoints: GET /contracts/source, POST /contracts/compile, POST /contracts/deploy
- **CompileService** — wraps solc: takes Solidity source → returns `{ abi, bytecode, contractName }`. Exported for S04 consumption.
- **DeployService** — wraps ethers.js v6: takes bytecode+ABI → deploys to Monad testnet via `ContractFactory` → returns `{ address, txHash }`. Saves deployment record to DB via PrismaService.

Bundle the 4 sample `.sol` files from Vibe-Loom into `backend/contracts/` so the source endpoint can serve them. User-submitted source code is compiled in-memory (no temp file writes needed — solc accepts source strings directly).

## Implementation Landscape

### Key Files

**Existing (from S01, do not modify except app.module.ts):**
- `backend/src/app.module.ts` — add `ContractsModule` to imports array
- `backend/src/prisma/prisma.service.ts` — inject in DeployService for DB writes
- `backend/src/config/configuration.ts` — already has patterns for env vars; needs `monad.rpcUrl` and `monad.privateKey` added
- `backend/prisma/schema.prisma` — Deployment model already defined, no changes needed
- `backend/.env.example` — already documents `MONAD_RPC_URL` and `MONAD_PRIVATE_KEY`

**New files to create:**
- `backend/src/contracts/contracts.module.ts` — NestJS module exporting CompileService (for S04)
- `backend/src/contracts/contracts.controller.ts` — 3 endpoints
- `backend/src/contracts/compile.service.ts` — solc wrapper
- `backend/src/contracts/deploy.service.ts` — ethers.js v6 deployment + DB persistence
- `backend/src/contracts/dto/compile-request.dto.ts` — `{ source: string }` with class-validator
- `backend/src/contracts/dto/deploy-request.dto.ts` — `{ contractType?: string, contractSource?: string }` 
- `backend/src/contracts/dto/compile-result.dto.ts` — response type `{ contractName, abi, bytecode }`
- `backend/contracts/FixedContract.sol` — copy from Vibe-Loom
- `backend/contracts/test/FailingContract.sol` — copy from Vibe-Loom
- `backend/contracts/test/ParallelConflict.sol` — copy from Vibe-Loom
- `backend/contracts/test/PectraTest.sol` — copy from Vibe-Loom
- `backend/test/compile.service.spec.ts` — unit tests for solc compilation
- `backend/test/deploy.service.spec.ts` — unit tests for deployment (mocked ethers)
- `backend/test/contracts.controller.spec.ts` — controller endpoint tests

**Config extension (add to configuration.ts):**
```typescript
monad: {
  rpcUrl: process.env.MONAD_RPC_URL ?? 'https://testnet-rpc.monad.xyz',
  privateKey: process.env.MONAD_PRIVATE_KEY,
},
```

### Endpoint Design

| Method | Path | Description | Auth? |
|--------|------|-------------|-------|
| GET | /api/contracts/source?type=FixedContract | Return sample contract source by type | No |
| POST | /api/contracts/compile | Compile Solidity source → ABI + bytecode | No |
| POST | /api/contracts/deploy | Compile + deploy to Monad testnet | No* |

*Auth is added in S03 (JwtAuthGuard). S02 endpoints are unguarded — S03 will add guards later.

### CompileService Design

```typescript
// Input: Solidity source string
// Output: { contractName, abi, bytecode }
// Uses solc npm package — Standard JSON Input/Output

const input = {
  language: 'Solidity',
  sources: { 'Contract.sol': { content: sourceCode } },
  settings: {
    evmVersion: 'cancun',  // matches Vibe-Loom's hardhat.config (EIP-1153 TSTORE/TLOAD)
    outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object'] } },
  },
};

const output = JSON.parse(solc.compile(JSON.stringify(input)));
// Extract first contract's ABI + bytecode from output.contracts['Contract.sol'][contractName]
```

Key: Set `evmVersion: 'cancun'` to match Vibe-Loom's Hardhat config (`settings.evmVersion: "cancun"`), enabling TSTORE/TLOAD opcodes used in FailingContract and PectraTest.

### DeployService Design

```typescript
// Uses ethers.js v6 JsonRpcProvider + Wallet + ContractFactory
const provider = new ethers.JsonRpcProvider(rpcUrl);
const wallet = new ethers.Wallet(privateKey, provider);
const factory = new ethers.ContractFactory(abi, bytecode, wallet);
const contract = await factory.deploy();
await contract.waitForDeployment();
const address = await contract.getAddress();
const txHash = contract.deploymentTransaction()?.hash;
```

DB persistence pattern:
1. Create Deployment record with status='pending' before deploy attempt
2. On success: update with address, txHash, status='deployed', gasUsed
3. On failure: update with errorMessage, status='failed'

### Contract Name Extraction

Vibe-Loom uses regex: `source.match(/contract\s+(\w+)\s*\{/)`. This is sufficient for single-contract files. The solc output also provides contract names as keys in `output.contracts[filename]`, which is more reliable. Use solc output as the primary source; regex as fallback for the source endpoint.

### Build Order

1. **T01: CompileService + contracts directory + DTOs** — This is the riskiest piece (solc integration, EVM version config). Also unblocks S04 which needs compile output. Include unit test that compiles FixedContract.sol and verifies ABI+bytecode are returned.

2. **T02: DeployService + DeployService tests** — Uses CompileService output. Tests mock ethers.js provider/wallet/factory. Verifies DB persistence pattern (create pending → update success/fail).

3. **T03: ContractsController + ContractsModule + integration** — Wires everything together. Three endpoints. Register module in AppModule. Controller tests verify request validation, response shape, error handling.

### Verification Approach

1. `cd backend && npm run build` — zero TypeScript errors after adding solc + ethers deps
2. `cd backend && npm test` — all existing tests (5) + new tests pass
3. **CompileService test**: compile FixedContract.sol → verify ABI is non-empty array, bytecode is hex string starting with '0x' or raw hex
4. **CompileService test**: compile invalid Solidity → verify proper error with compilation errors
5. **DeployService test**: mock ethers ContractFactory → verify Prisma Deployment created with status='deployed'
6. **DeployService test**: mock deploy failure → verify Deployment record updated with status='failed' and errorMessage
7. **Controller test**: GET /contracts/source?type=FixedContract → 200 with source string
8. **Controller test**: GET /contracts/source?type=InvalidType → 400
9. **Controller test**: POST /contracts/compile with valid source → 200 with abi+bytecode
10. **Controller test**: POST /contracts/deploy → 200 with address+txHash (mocked)

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Solidity compilation | `solc` npm package (JavaScript bindings) | Standard JSON input/output, no subprocess, same compiler Hardhat uses internally |
| Contract deployment | `ethers.js` v6 `ContractFactory` | Industry standard, handles ABI encoding, tx management, gas estimation |
| Request validation | `class-validator` + `class-transformer` (already in S01 deps) | NestJS ValidationPipe already configured globally |

## Constraints

- **Solidity version pinned to 0.8.24** — All 4 sample contracts use `pragma solidity ^0.8.24`. The solc npm package version must match (solc@0.8.24 or compatible). Use `solc.version()` to verify.
- **EVM version must be 'cancun'** — PectraTest.sol and FailingContract.sol use TSTORE/TLOAD (EIP-1153), which require cancun or later EVM version in the compiler settings.
- **Monad testnet chain ID is 10143** — ethers.js will auto-detect from the RPC, but good to know for error messages.
- **No auth on S02 endpoints** — S03 adds JwtAuthGuard. S02 must not import or reference any auth module. Endpoints are publicly accessible until S03 is applied.
- **CompileService must be exported from ContractsModule** — S04's EngineModule will import ContractsModule to get CompileService for the vibe-score pipeline.
- **All npm commands run from `backend/`** — project is a subdirectory.
- **TransformInterceptor auto-wraps responses** — controllers return raw data objects, not wrapped in ApiResponse.

## Common Pitfalls

- **solc version mismatch** — If the installed solc npm package is older than 0.8.24, compilation of the sample contracts will fail with version pragma errors. Pin `solc` to `^0.8.24` in package.json.
- **solc compilation errors are in the output JSON, not thrown** — `solc.compile()` always returns a JSON string. Must check `output.errors` for severity='error' entries. Don't assume empty output means success.
- **ethers.js v6 API changes from v5** — `contract.address` is now `contract.target` or use `await contract.getAddress()`. `contract.deployTransaction` is now `contract.deploymentTransaction()` (method, not property). Vibe-Loom already uses v6 so patterns exist.
- **60-second timeout on Monad testnet** — Vibe-Loom sets `timeout: 60000` on the Hardhat network config. ethers.js provider should be configured with similar timeout for slow testnet responses.
- **Missing MONAD_PRIVATE_KEY** — Deploy endpoint should return a clear 503 "Server not configured for deployment" error if the private key env var is not set, rather than crashing.

## Open Risks

- **solc npm package size** — The `solc` package includes the full Solidity compiler WASM binary (~8MB). This increases `node_modules` and container image size. Acceptable for now; could be optimized later with a compilation service.
- **Monad testnet availability** — Deploy tests that hit the real testnet are flaky. All unit tests must mock the network layer. Real deployment verification is deferred to S06 E2E tests.
