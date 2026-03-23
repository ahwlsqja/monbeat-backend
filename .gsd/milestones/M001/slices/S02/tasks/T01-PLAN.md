---
estimated_steps: 5
estimated_files: 9
---

# T01: Install solc + ethers, create sample contracts, build CompileService with DTOs and tests

**Slice:** S02 — Contract & Deploy Module
**Milestone:** M001

## Description

Install the `solc` and `ethers` npm packages, create 4 sample Solidity contracts in `backend/contracts/`, implement `CompileService` wrapping the solc Standard JSON Input/Output API, create request/response DTOs with class-validator, and write unit tests proving compilation works for valid contracts and errors correctly for invalid source.

This is the riskiest piece of S02 — the solc npm package has a specific JSON input/output format, EVM version must be set to `cancun` (for TSTORE/TLOAD opcode support in PectraTest.sol), and compilation errors are embedded in the output JSON rather than thrown as exceptions. Getting this right unblocks both T02 (DeployService) and S04 (EngineModule).

**Relevant skills:** None required — standard NestJS service + npm package integration.

## Steps

1. **Install dependencies:** Run `cd backend && npm install solc ethers` to add both packages. Verify they appear in `package.json` dependencies.

2. **Create sample Solidity contracts:** Write 4 `.sol` files with `pragma solidity ^0.8.24`:
   - `backend/contracts/FixedContract.sol` — Simple storage contract (store/retrieve a uint256). This is the "normal" baseline for vibe-score comparison.
   - `backend/contracts/test/FailingContract.sol` — Contract that uses `revert()` or always-failing logic. Tests error path handling.
   - `backend/contracts/test/ParallelConflict.sol` — Contract with a global counter that creates state access conflicts (reads+writes a single storage slot from multiple transactions). This is the "bottleneck" pattern for vibe-score comparison.
   - `backend/contracts/test/PectraTest.sol` — Contract using TSTORE/TLOAD (EIP-1153 transient storage, requires cancun EVM). Tests EVM version compatibility.

3. **Create DTOs:**
   - `backend/src/contracts/dto/compile-request.dto.ts` — `CompileRequestDto` with `source: string` (required, `@IsString()`, `@IsNotEmpty()`)
   - `backend/src/contracts/dto/compile-result.dto.ts` — `CompileResultDto` type with `contractName: string`, `abi: any[]`, `bytecode: string`

4. **Implement CompileService:** Create `backend/src/contracts/compile.service.ts`:
   - Injectable NestJS service with Logger
   - `compile(source: string): CompileResultDto` method
   - Construct solc Standard JSON Input: `{ language: 'Solidity', sources: { 'Contract.sol': { content: source } }, settings: { evmVersion: 'cancun', outputSelection: { '*': { '*': ['abi', 'evm.bytecode.object'] } } } }`
   - Call `solc.compile(JSON.stringify(input))` and parse the JSON output
   - Check `output.errors` for severity='error' entries — if found, throw `BadRequestException` with the error messages
   - Extract the first contract name from `output.contracts['Contract.sol']` keys
   - Return `{ contractName, abi, bytecode }` (bytecode prefixed with '0x' if not already)
   - Log compilation success/failure with contract name

5. **Write unit tests:** Create `backend/test/compile.service.spec.ts`:
   - Test: compile FixedContract.sol → returns non-empty ABI array and hex bytecode
   - Test: compile invalid Solidity source → throws BadRequestException with error details
   - Test: extracted contractName matches the contract defined in the source
   - Test: compile PectraTest.sol with TSTORE/TLOAD → succeeds (verifies cancun EVM version)
   - Use `fs.readFileSync` to load sample contracts from `backend/contracts/`
   - Note: solc compilation is CPU-intensive — set jest timeout to 30s for compile tests

## Must-Haves

- [ ] `solc` and `ethers` installed in backend/package.json dependencies
- [ ] 4 sample .sol files with `pragma solidity ^0.8.24` that compile successfully
- [ ] CompileService uses solc Standard JSON Input with `evmVersion: 'cancun'`
- [ ] CompileService extracts compilation errors from output JSON and throws BadRequestException
- [ ] CompileService returns `{ contractName, abi, bytecode }` with bytecode as hex string
- [ ] CompileRequestDto validates `source` field with class-validator
- [ ] Unit tests pass for valid compilation, invalid source, and contract name extraction

## Verification

- `cd backend && npm run build` — zero TypeScript errors after adding solc + ethers + new files
- `cd backend && npm test -- --testPathPattern=compile.service` — all CompileService tests pass
- `test -f backend/contracts/FixedContract.sol` — sample contract exists
- `test -f backend/contracts/test/ParallelConflict.sol` — test contract exists

## Observability Impact

- **New signal:** CompileService logs compilation success with contract name, ABI entry count, and bytecode length via NestJS Logger (`CompileService.log`).
- **New signal:** CompileService logs compilation errors/warnings via `CompileService.warn` and `CompileService.error`.
- **Failure visibility:** `BadRequestException` thrown on compilation failure includes structured `errors` array from solc output — downstream callers (controller, engine) can surface these to API consumers.
- **Inspection:** A future agent can verify compilation works by calling `CompileService.compile()` with any `.sol` file from `backend/contracts/` and checking the result shape.
- **Redaction:** No secrets handled in this task (MONAD_PRIVATE_KEY is only in T02/DeployService).

## Inputs

- `backend/package.json` — existing package manifest to add solc + ethers dependencies
- `backend/tsconfig.json` — TypeScript configuration (no changes needed, but referenced for compilation)

## Expected Output

- `backend/package.json` — updated with solc + ethers in dependencies
- `backend/contracts/FixedContract.sol` — simple storage contract
- `backend/contracts/test/FailingContract.sol` — always-failing contract
- `backend/contracts/test/ParallelConflict.sol` — global counter bottleneck pattern
- `backend/contracts/test/PectraTest.sol` — TSTORE/TLOAD transient storage contract
- `backend/src/contracts/compile.service.ts` — solc wrapper service
- `backend/src/contracts/dto/compile-request.dto.ts` — request DTO
- `backend/src/contracts/dto/compile-result.dto.ts` — response type
- `backend/test/compile.service.spec.ts` — CompileService unit tests
