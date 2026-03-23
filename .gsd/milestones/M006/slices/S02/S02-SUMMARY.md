---
id: S02
parent: M006
milestone: M006
provides:
  - CompileService storageLayout extraction via solc outputSelection
  - StorageLayout/StorageEntry/StorageTypeInfo TypeScript interfaces
  - LocationInfo/ConflictPair/TxAccessSummary/ConflictDetails interfaces (S01 schema mirror)
  - storage-layout-decoder pure-function module (decodeSlotToVariable, buildConflictAnalysis, generateSuggestion, buildMatrix)
  - VibeScoreService Phase 5b conflict analysis pipeline with txFunctionMap construction
  - DecodedConflict/ConflictMatrix/ConflictAnalysis interfaces on VibeScoreResultDto
  - Coinbase address conflict filtering
  - Backward-compatible API response (conflictAnalysis omitted when absent)
requires:
  - slice: S01
    provides: CLI conflict_details JSON schema (LocationInfo, ConflictPair, ConflictDetails)
affects:
  - S03
key_files:
  - Vibe-Room-Backend/src/contracts/compile.service.ts
  - Vibe-Room-Backend/src/contracts/dto/compile-result.dto.ts
  - Vibe-Room-Backend/src/engine/engine.service.ts
  - Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts
  - Vibe-Room-Backend/src/vibe-score/storage-layout-decoder.ts
  - Vibe-Room-Backend/src/vibe-score/vibe-score.service.ts
  - Vibe-Room-Backend/test/compile.service.spec.ts
  - Vibe-Room-Backend/test/storage-layout-decoder.spec.ts
  - Vibe-Room-Backend/test/vibe-score.service.spec.ts
key_decisions:
  - D028: Pure function module for decoder (no NestJS DI) — enables mock-free testing and flexible wiring
  - D029: BigInt normalization for hex/decimal slot comparison — safe for keccak256-derived large slots
  - D030: Mapping heuristic — runtime slot > max declared slot → mapping base attribution with fallback
  - storageLayout optional on CompileResultDto — absence signals solc failure without throwing
  - conflictAnalysis set to undefined (not empty object) when absent — cleaner API responses
  - txFunctionMap built inside constructTransactionBlock() — co-located with tx construction for accuracy
patterns_established:
  - Hex slot → BigInt → exact match against decimal storageLayout entries
  - Large runtime slot heuristic for mapping/dynamic_array attribution
  - Phase 5b conditional pipeline step in VibeScoreService (only when conflict_details + storageLayout both present)
  - constructTransactionBlock() returns { transactions, blockEnv, txFunctionMap } triple
  - Variable grouping by name for conflict deduplication across multiple ConflictPair entries
observability_surfaces:
  - VibeScoreService logs "Phase 5b: Decoding conflict analysis" with decoded conflict count and elapsed ms
  - API response conflictAnalysis field presence/absence signals conflict decoding success
  - decodeSlotToVariable returns "unknown_slot_0xNNN" on decode failure — grep API responses for "unknown_slot" to detect misses
  - CompileResultDto.storageLayout presence/absence indicates solc layout extraction success
drill_down_paths:
  - .gsd/milestones/M006/slices/S02/tasks/T01-SUMMARY.md
  - .gsd/milestones/M006/slices/S02/tasks/T02-SUMMARY.md
  - .gsd/milestones/M006/slices/S02/tasks/T03-SUMMARY.md
duration: 37m
verification_result: passed
completed_at: 2026-03-24T05:56:00+09:00
---

# S02: NestJS — Storage Layout 디코딩 + Actionable Suggestion 생성

**Built the complete NestJS pipeline from solc storageLayout extraction through hex→variable name decoding, coinbase filtering, actionable suggestion generation, and function×variable matrix construction — 43 tests pass across 3 test suites with full backward compatibility**

## What Happened

Three tasks assembled a complete conflict analysis pipeline in the NestJS backend:

**T01 — Type Foundation (15m):** Extended CompileService to extract `storageLayout` from solc by adding it to `outputSelection`. Defined all TypeScript interfaces across 4 files: `StorageLayout`/`StorageEntry`/`StorageTypeInfo` for solc output, `LocationInfo`/`ConflictPair`/`TxAccessSummary`/`ConflictDetails` mirroring the S01 Rust CLI schema exactly, and `DecodedConflict`/`ConflictMatrix`/`ConflictAnalysis` for the decoded API output. All new fields are optional to preserve backward compatibility. Added 2 tests verifying storageLayout extraction for ParallelConflict.sol (counter at slot 0) and FixedContract.sol.

**T02 — Core Domain Logic (12m):** Implemented `storage-layout-decoder.ts` as a pure-function module (no NestJS DI). Four exported functions: `decodeSlotToVariable()` converts hex runtime slots to variable names via BigInt normalization, with a mapping heuristic for keccak256-derived slots. `buildConflictAnalysis()` orchestrates the full pipeline — filters coinbase conflicts (case-insensitive), keeps only Storage-type conflicts, decodes slots, maps tx indices to function names, groups by variable, generates suggestions, and builds the matrix. `generateSuggestion()` produces English actionable text per variable type (mapping → key range separation, simple var → per-function splitting). `buildMatrix()` builds a function×variable 2D conflict count matrix for S03's heatmap. 17 unit tests cover all paths including edge cases.

**T03 — Pipeline Wiring (10m):** Connected T01's types and T02's decoder into VibeScoreService. Extended `constructTransactionBlock()` to return `txFunctionMap` alongside transactions and blockEnv. Added Phase 5b as a conditional step after score calculation — runs only when both `conflict_details` and `storageLayout` are present. If all conflicts are filtered (coinbase/non-Storage), `conflictAnalysis` is set to undefined rather than an empty object. 3 integration tests verify: decoded analysis present with mock conflict data, backward compat without conflicts, and graceful degradation without storageLayout.

## Verification

All 4 slice-level verification checks pass:

| # | Check | Command | Result |
|---|-------|---------|--------|
| 1 | CompileService storageLayout extraction | `npx jest test/compile.service.spec.ts` | ✅ 10/10 |
| 2 | Decoder module 8+ tests | `npx jest test/storage-layout-decoder.spec.ts` | ✅ 17/17 |
| 3 | VibeScoreService integration + backward compat | `npx jest test/vibe-score.service.spec.ts` | ✅ 16/16 |
| 4 | Failure-path verification (undefined/unknown) | `npx jest test/storage-layout-decoder.spec.ts -- --testNamePattern="undefined\|unknown"` | ✅ 17/17 |

**Total: 43 tests across 3 suites, 0 failures.**

Observability verified:
- Phase 5b log output confirmed in test execution
- `conflictAnalysis` field presence/absence correctly signals decoding outcome
- `unknown_slot_0xNNN` fallback confirmed in unit test for unresolvable slots

## Requirements Advanced

- **R006** (Vibe Score 강화) — API now returns decoded conflict analysis with variable names and actionable suggestions instead of raw scores only
- **R017** (병렬 실행 최적화 제안) — Concrete suggestions generated with variable name, function name, and modification method per conflict

## New Requirements Surfaced

- none

## Deviations

- T02 produced 17 tests instead of planned 9 — additional coverage for edge cases at zero incremental cost
- T03 found 13 existing tests in vibe-score.service.spec.ts rather than plan's estimated 11 — no functional deviation, just a counting correction

## Known Limitations

- Mapping heuristic is probabilistic for contracts with multiple mappings — returns "unknown (possibly X or Y)" instead of exact attribution. This is an inherent limitation of runtime slot→base slot reverse mapping without keccak256 preimage data.
- Suggestion text is English-only (per D019 UX text language unification decision)
- Frontend (S03) not yet consuming `conflictAnalysis` — field is present in API but not visualized

## Follow-ups

- S03 must consume `conflictAnalysis.matrix` for heatmap rendering and `conflictAnalysis.conflicts[].suggestion` for suggestion cards
- S03 TypeScript types in `Vibe-Loom/src/lib/api-client.ts` must mirror `ConflictAnalysis`/`DecodedConflict`/`ConflictMatrix` interfaces from `vibe-score-result.dto.ts`

## Files Created/Modified

- `Vibe-Room-Backend/src/contracts/compile.service.ts` — Added storageLayout to SolcOutput, outputSelection, and return value
- `Vibe-Room-Backend/src/contracts/dto/compile-result.dto.ts` — StorageEntry/StorageTypeInfo/StorageLayout interfaces + CompileResultDto extension
- `Vibe-Room-Backend/src/engine/engine.service.ts` — LocationInfo/ConflictPair/TxAccessSummary/ConflictDetails interfaces + CliOutput extension
- `Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts` — DecodedConflict/ConflictMatrix/ConflictAnalysis interfaces + VibeScoreResultDto extension
- `Vibe-Room-Backend/src/vibe-score/storage-layout-decoder.ts` — New: pure-function decoder module (4 exports, ~200 LOC)
- `Vibe-Room-Backend/src/vibe-score/vibe-score.service.ts` — Phase 5b conflict analysis, txFunctionMap, buildConflictAnalysis wiring
- `Vibe-Room-Backend/test/compile.service.spec.ts` — 2 new storageLayout tests (10 total)
- `Vibe-Room-Backend/test/storage-layout-decoder.spec.ts` — New: 17 unit tests
- `Vibe-Room-Backend/test/vibe-score.service.spec.ts` — 3 new integration tests (16 total)

## Forward Intelligence

### What the next slice should know
- The `conflictAnalysis` API field schema is: `{ conflicts: DecodedConflict[], matrix: ConflictMatrix }` where `DecodedConflict` has `variableName`, `variableType`, `slot`, `functions: string[]`, `conflictType`, `suggestion` and `ConflictMatrix` has `rows: string[]` (function names), `cols: string[]` (variable names), `cells: number[][]` (conflict counts).
- `conflictAnalysis` is `undefined` (field absent) when there are no actionable conflicts — S03 should conditionally render the heatmap section only when this field exists.
- The matrix `cells[i][j]` represents the number of conflicts between `rows[i]` (function) and `cols[j]` (variable). Zero means no conflict at that intersection.

### What's fragile
- Mapping heuristic with multiple mappings produces "unknown (possibly X or Y)" — if users report confusing suggestions, the heuristic in `decodeSlotToVariable()` is the place to improve. Adding keccak256 preimage tracking in the Rust CLI would eliminate this limitation.
- `txFunctionMap` accuracy depends on `constructTransactionBlock()` function encode order — if a function's calldata encoding fails and is skipped, the index shifts. Current implementation handles this correctly, but adding new transaction types (e.g., proxy calls) would need matching txFunctionMap updates.

### Authoritative diagnostics
- `npx jest test/storage-layout-decoder.spec.ts --verbose` — 17 tests covering every decoder code path. First place to check if slot decoding behaves unexpectedly.
- API response: presence of `conflictAnalysis` field = decoding succeeded. Absence = no conflict_details, no storageLayout, or all conflicts filtered. Check VibeScoreService Phase 5b logs for timing and count.
- Grep API responses for `"unknown_slot"` to quantify decode miss rate in production.

### What assumptions changed
- "Storage layout 디코딩 정확도" was listed as a key risk — in practice, exact-match decoding works perfectly for simple variables (the most common case), and the mapping heuristic handles single-mapping contracts correctly. The risk is limited to rare multi-mapping contracts.
