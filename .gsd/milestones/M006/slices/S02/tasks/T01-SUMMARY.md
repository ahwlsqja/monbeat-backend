---
id: T01
parent: S02
milestone: M006
provides:
  - StorageLayout/StorageEntry/StorageTypeInfo interfaces in compile-result.dto.ts
  - CompileService storageLayout extraction via solc outputSelection
  - LocationInfo/ConflictPair/TxAccessSummary/ConflictDetails interfaces in engine.service.ts
  - DecodedConflict/ConflictMatrix/ConflictAnalysis interfaces in vibe-score-result.dto.ts
key_files:
  - Vibe-Room-Backend/src/contracts/compile.service.ts
  - Vibe-Room-Backend/src/contracts/dto/compile-result.dto.ts
  - Vibe-Room-Backend/src/engine/engine.service.ts
  - Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts
  - Vibe-Room-Backend/test/compile.service.spec.ts
key_decisions:
  - storageLayout field is optional on CompileResultDto — absence signals solc extraction failure without throwing
  - All new CliOutput/VibeScoreResultDto fields are optional to preserve backward compatibility
patterns_established:
  - Typed solc storageLayout extraction via outputSelection array extension
  - S01 Rust CLI conflict_details schema mirrored 1:1 in TypeScript interfaces
observability_surfaces:
  - CompileResultDto.storageLayout presence/absence indicates solc layout extraction success
  - No new runtime logs in T01 — downstream T03 will add Phase 5b logging
duration: 15m
verification_result: passed
completed_at: 2026-03-24T04:57:00+09:00
blocker_discovered: false
---

# T01: CompileService storageLayout 추출 + 전체 TypeScript 인터페이스 정의

**Added storageLayout extraction to CompileService and defined all TypeScript interfaces for conflict_details, ConflictAnalysis, and storage layout types**

## What Happened

Implemented all 5 steps from the task plan:

1. **CompileService** (`compile.service.ts`): Added `storageLayout` to `SolcOutput` interface, appended `'storageLayout'` to solc `outputSelection` array, and included `storageLayout: contract.storageLayout` in the return value.

2. **CompileResultDto** (`compile-result.dto.ts`): Defined `StorageEntry`, `StorageTypeInfo`, and `StorageLayout` interfaces with full typing for solc's storage layout output. Extended `CompileResultDto` with optional `storageLayout` field.

3. **CliOutput** (`engine.service.ts`): Defined `LocationInfo`, `ConflictPair`, `TxAccessSummary`, and `ConflictDetails` interfaces matching the S01 Rust CLI `conflict_details` JSON schema exactly. Extended `CliOutput` with optional `conflict_details` field.

4. **VibeScoreResultDto** (`vibe-score-result.dto.ts`): Defined `DecodedConflict`, `ConflictMatrix`, and `ConflictAnalysis` interfaces for the decoded conflict output. Extended `VibeScoreResultDto` with optional `conflictAnalysis` field.

5. **Tests** (`compile.service.spec.ts`): Added two new tests — one verifying ParallelConflict.sol compilation produces `storageLayout` with a `counter` variable at slot 0, and one verifying FixedContract.sol also produces a storageLayout.

Also addressed pre-flight observability gaps: added failure-path verification step to S02-PLAN.md and Observability Impact section to T01-PLAN.md.

## Verification

- `npx jest test/compile.service.spec.ts` — 10 tests pass (8 existing + 2 new storageLayout tests)
- `npx jest test/engine.service.spec.ts` — 10 tests pass (all existing, no breakage from optional field addition)
- ParallelConflict.sol storageLayout contains `counter` at slot `"0"` with populated types map

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd Vibe-Room-Backend && npx jest test/compile.service.spec.ts` | 0 | ✅ pass | 4.6s |
| 2 | `cd Vibe-Room-Backend && npx jest test/engine.service.spec.ts` | 0 | ✅ pass | 3.7s |

### Slice-level verification (partial — T01 is intermediate)

| # | Slice Check | Status |
|---|------------|--------|
| 1 | `npx jest test/compile.service.spec.ts` — storageLayout 추출 테스트 통과 | ✅ pass |
| 2 | `npx jest test/storage-layout-decoder.spec.ts` — decoder 모듈 테스트 | ⏳ not yet created (T02) |
| 3 | `npx jest test/vibe-score.service.spec.ts` — conflictAnalysis 테스트 | ⏳ not yet wired (T03) |
| 4 | failure-path verification | ⏳ depends on T02 decoder tests |

## Diagnostics

- Inspect `CompileResultDto.storageLayout` field to verify solc extraction: if `undefined`, solc did not produce layout (check evmVersion/source).
- Run `npx jest test/compile.service.spec.ts --verbose` to see all test names and results.
- All new interfaces are exports — downstream consumers can import `StorageLayout`, `ConflictDetails`, `ConflictAnalysis` etc. directly.

## Deviations

- Added a second storageLayout test for FixedContract.sol (not in plan but zero-cost verification).
- Vibe-Room-Backend project is at `/home/ahwlsqja/Vibe-Room-Backend` rather than inside the worktree — files modified in-place there since it's a separate NestJS project outside the monad-core Rust monorepo.

## Known Issues

None.

## Files Created/Modified

- `Vibe-Room-Backend/src/contracts/compile.service.ts` — Added storageLayout to SolcOutput, outputSelection, and return value
- `Vibe-Room-Backend/src/contracts/dto/compile-result.dto.ts` — StorageEntry/StorageTypeInfo/StorageLayout interfaces + CompileResultDto extension
- `Vibe-Room-Backend/src/engine/engine.service.ts` — LocationInfo/ConflictPair/TxAccessSummary/ConflictDetails interfaces + CliOutput extension
- `Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts` — DecodedConflict/ConflictMatrix/ConflictAnalysis interfaces + VibeScoreResultDto extension
- `Vibe-Room-Backend/test/compile.service.spec.ts` — Two new storageLayout verification tests
