---
id: T01
parent: S02
milestone: M001
provides:
  - CompileService wrapping solc Standard JSON Input/Output with evmVersion=cancun
  - 4 sample Solidity contracts (FixedContract, FailingContract, ParallelConflict, PectraTest)
  - CompileRequestDto with class-validator, CompileResultDto interface
  - 8 unit tests for compilation success, error handling, and EVM compatibility
key_files:
  - backend/src/contracts/compile.service.ts
  - backend/src/contracts/dto/compile-request.dto.ts
  - backend/src/contracts/dto/compile-result.dto.ts
  - backend/contracts/FixedContract.sol
  - backend/contracts/test/FailingContract.sol
  - backend/contracts/test/ParallelConflict.sol
  - backend/contracts/test/PectraTest.sol
  - backend/test/compile.service.spec.ts
key_decisions:
  - solc errors extracted from output.errors array filtered by severity='error'; warnings are ignored for compilation success/failure determination
  - Bytecode always prefixed with '0x' for consistency with ethers.js expectations in T02
  - Contract name extracted from first key of output.contracts['Contract.sol'] — single-contract-per-source assumption
patterns_established:
  - solc Standard JSON Input with evmVersion='cancun' and outputSelection for abi + evm.bytecode.object
  - BadRequestException with structured { message, errors[] } response for compilation failures
  - Sample contracts loaded via fs.readFileSync in tests with 30s jest timeout for CPU-intensive solc
observability_surfaces:
  - CompileService NestJS Logger: compilation success (contract name, ABI count, bytecode length)
  - CompileService NestJS Logger: compilation errors/warnings
  - BadRequestException includes solc error messages array for API consumers
duration: 12m
verification_result: passed
completed_at: 2026-03-22T12:05:00Z
blocker_discovered: false
---

# T01: Install solc + ethers, create sample contracts, build CompileService with DTOs and tests

**Installed solc + ethers, created 4 Solidity contracts, implemented CompileService with cancun EVM and error extraction, added 8 passing unit tests**

## What Happened

Installed `solc@^0.8.34` and `ethers@^6.16.0` in backend. Created 4 sample Solidity contracts: FixedContract (simple storage baseline), FailingContract (always-reverts error path), ParallelConflict (global counter bottleneck pattern), and PectraTest (TSTORE/TLOAD transient storage requiring cancun EVM).

Implemented `CompileService` as an injectable NestJS service that wraps solc's Standard JSON Input/Output API. The service constructs the solc input with `evmVersion: 'cancun'`, parses the JSON output, filters errors by `severity === 'error'` (ignoring warnings), extracts the first contract name from the output, and returns `{ contractName, abi, bytecode }` with bytecode prefixed with `0x`. Compilation errors throw `BadRequestException` with a structured `{ message, errors[] }` response.

Created `CompileRequestDto` with `@IsString()` and `@IsNotEmpty()` class-validator decorators, and `CompileResultDto` as a TypeScript interface.

Wrote 8 unit tests covering: valid compilation with ABI/bytecode shape, contract name extraction, syntax error handling, error detail inclusion in exceptions, PectraTest TSTORE/TLOAD cancun compatibility, and compilation of all test contracts.

## Verification

- `npm run build` — zero TypeScript errors (exit 0)
- `npm test -- --testPathPattern=compile.service` — 8/8 tests pass
- `npm test` — 13/13 total tests pass (5 existing + 8 new)
- `test -f backend/contracts/FixedContract.sol` — exists
- `test -f backend/contracts/test/ParallelConflict.sol` — exists

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd backend && npm run build` | 0 | ✅ pass | 9.5s |
| 2 | `cd backend && npm test -- --testPathPattern=compile.service` | 0 | ✅ pass | 4.6s |
| 3 | `cd backend && npm test` | 0 | ✅ pass | 11.8s |
| 4 | `test -f backend/contracts/FixedContract.sol` | 0 | ✅ pass | <1s |
| 5 | `test -f backend/contracts/test/ParallelConflict.sol` | 0 | ✅ pass | <1s |

### Slice-level checks (partial — T01 is first of 3 tasks)

| # | Check | Status | Notes |
|---|-------|--------|-------|
| 1 | `npm run build` — zero TS errors | ✅ pass | |
| 2 | `npm test` — all tests pass (target ≥12 new) | ⏳ partial | 8 new tests (need ≥12 across 3 suites — remaining in T02/T03) |
| 3 | CompileService tests pass | ✅ pass | 8/8 |
| 4 | DeployService tests pass | ⏳ not yet | T02 |
| 5 | Controller tests pass | ⏳ not yet | T03 |
| 6 | ContractsModule in AppModule | ⏳ not yet | T03 |
| 7 | CompileService exported | ⏳ not yet | T03 |

## Diagnostics

- **Inspect compilation:** Call `CompileService.compile(source)` with any `.sol` file content. Returns `{ contractName, abi, bytecode }` on success.
- **Error shape:** `BadRequestException` response body: `{ message: 'Compilation failed', errors: ['...formatted solc error...'] }`
- **Logs:** CompileService logs via NestJS Logger — `CompileService` context with success/failure messages including contract name.
- **Sample contracts:** `backend/contracts/FixedContract.sol` (baseline), `backend/contracts/test/` (FailingContract, ParallelConflict, PectraTest).

## Deviations

None — all steps executed as planned.

## Known Issues

None.

## Files Created/Modified

- `backend/package.json` — added solc and ethers dependencies
- `backend/contracts/FixedContract.sol` — simple storage contract (vibe-score baseline)
- `backend/contracts/test/FailingContract.sol` — always-reverts contract for error path testing
- `backend/contracts/test/ParallelConflict.sol` — global counter bottleneck pattern contract
- `backend/contracts/test/PectraTest.sol` — TSTORE/TLOAD transient storage contract (cancun EVM)
- `backend/src/contracts/compile.service.ts` — solc wrapper service with error extraction and logging
- `backend/src/contracts/dto/compile-request.dto.ts` — request DTO with class-validator
- `backend/src/contracts/dto/compile-result.dto.ts` — response interface (contractName, abi, bytecode)
- `backend/test/compile.service.spec.ts` — 8 unit tests for CompileService
