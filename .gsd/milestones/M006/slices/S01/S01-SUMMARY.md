---
id: S01
parent: M006
milestone: M006
provides:
  - Scheduler::return_read_set() method to preserve ReadSets after successful validation
  - collect_results() returns Vec<(ExecutionResult, WriteSet, ReadSet)> 3-tuple
  - ParallelExecutionResult::tx_results updated to 3-tuple throughout codebase
  - conflict.rs module with detect_conflicts() for write-write and read-write conflict analysis
  - CliOutput extended with conflict_details field containing per_tx access summaries and conflict pairs
  - CLI-specific serializable types (LocationInfo, ConflictPair, TxAccessSummary, ConflictDetails) avoiding serde on mv-state internals
requires:
  - slice: none
    provides: first slice, no dependencies
affects:
  - S02
key_files:
  - crates/scheduler/src/coordinator.rs
  - crates/scheduler/src/parallel_executor.rs
  - crates/cli/src/conflict.rs
  - crates/cli/src/main.rs
  - crates/cli/Cargo.toml
key_decisions:
  - ReadSet preserved only on validation success; on failure the tx re-executes with fresh ReadSet
  - CLI-specific serializable types used instead of adding serde derives to mv-state hot-path types
  - Both write-write and read-write conflicts emitted even for same location+pair for maximum downstream information
  - HashSet intersection for conflict detection — O(n²) pairwise is acceptable for typical block tx counts
  - serde skip_serializing_if for optional slot field — cleaner JSON for non-Storage locations
patterns_established:
  - return_read_set() mirrors take_read_set() using the same mutex pattern for thread-safe TxState access
  - Pattern-match LocationKey variants to CLI-specific LocationInfo at serialization boundary
  - detect_conflicts() takes the 3-tuple slice directly from ParallelExecutionResult::tx_results
  - unwrap_or_default() for missing ReadSets makes data loss explicitly observable as empty arrays
observability_surfaces:
  - conflict_details.conflicts array length 0 = no conflicts, >0 = conflicts detected
  - conflict_details.per_tx[i].reads/writes length shows per-tx state access scope
  - Empty reads array with non-empty writes indicates ReadSet was not preserved (data loss visible)
  - Inspect via jq .conflict_details on CLI stdout
drill_down_paths:
  - .gsd/milestones/M006/slices/S01/tasks/T01-SUMMARY.md
  - .gsd/milestones/M006/slices/S01/tasks/T02-SUMMARY.md
duration: 25m
verification_result: passed
completed_at: 2026-03-24
---

# S01: Rust CLI — R/W Set 충돌 데이터 JSON 출력

**Extended monad-core CLI with conflict_details field: per-tx ReadSet/WriteSet access summaries and write-write/read-write conflict pair detection, fully backward-compatible with existing output schema**

## What Happened

Two tasks delivered the slice goal in sequence:

**T01 — ReadSet Preservation (15m):** The core risk was that `handle_validate()` called `take_read_set()` and dropped the ReadSet after validation. Added `Scheduler::return_read_set(tx_index, read_set)` which mirrors `take_read_set()` using the same mutex pattern. Modified `handle_validate()` to call `return_read_set()` on validation success (not failure — failed txs re-execute and get fresh ReadSets). Extended `collect_results()` from 2-tuple to 3-tuple `Vec<(ExecutionResult, WriteSet, ReadSet)>`, using `unwrap_or_default()` for missing ReadSets so data loss is explicitly observable as empty arrays rather than silent. Updated all downstream destructuring (parallel_executor tests, CLI main.rs).

**T02 — Conflict Detection Module (10m):** Created `crates/cli/src/conflict.rs` with CLI-specific serializable types (`ConflictDetails`, `TxAccessSummary`, `LocationInfo`, `ConflictPair`) — deliberately avoiding `#[derive(Serialize)]` on mv-state hot-path types. The `detect_conflicts()` function builds per-tx access summaries from ReadSet/WriteSet iterators, then checks all tx pairs for write-write conflicts (write key intersection) and read-write conflicts (bidirectional read↔write intersection). Extended `CliOutput` with `conflict_details` field and wired everything through `main.rs`.

## Verification

All 5 slice-plan verification checks passed:

| # | Check | Result |
|---|-------|--------|
| 1 | `cargo test -p monad-scheduler` — 25 tests (24 existing + 1 new) | ✅ 25 passed |
| 2 | `cargo test -p monad-cli` — 7 conflict detection tests | ✅ 7 passed |
| 3 | `cargo build -p monad-cli` — binary build | ✅ clean build |
| 4 | Integration check — 2-tx block → assert conflict_details present | ✅ "OK" |
| 5 | Empty block diagnostic — 0-tx → assert results=[] and stats | ✅ "EMPTY_OK" |

Observability confirmed: `jq .conflict_details` on CLI stdout shows per-tx reads/writes and conflict pairs with correct location types, addresses, and conflict_type values.

## Requirements Advanced

- R005 (monad-core CLI JSON interface) — CLI output schema extended with conflict_details while maintaining backward compatibility
- R006 (Vibe Score 강화) — Foundation laid: conflict data now available in CLI output for S02 to consume and decode

## New Requirements Surfaced

- none

## Deviations

- T02 added 2 extra tests beyond the 5 specified in the plan (`test_empty_tx_results`, `test_storage_location_includes_slot`) for better edge-case coverage — 7 total.
- Used `#[serde(skip_serializing_if = "Option::is_none")]` on `LocationInfo.slot` to produce cleaner JSON — omits `"slot": null` for non-Storage locations. This slightly differs from the plan's schema which showed `slot?` as always present.

## Known Limitations

- Conflict detection is O(n²) pairwise — fine for typical block sizes (< 1000 txs) but would need optimization for very large blocks.
- The same location+pair can appear in both write-write and read-write conflicts — this is intentional (more info for S02) but downstream consumers should deduplicate if needed.
- ReadSets only capture locations read from the multi-version data structure, not from base state — some reads may not appear in per_tx summaries if the value came from base state directly via StorageNotFound fallback.

## Follow-ups

- S02 must parse the `conflict_details` JSON schema in NestJS `CliOutput` interface — the schema is: `{ per_tx: [{ tx_index, reads: [LocationInfo], writes: [LocationInfo] }], conflicts: [{ location: LocationInfo, tx_a, tx_b, conflict_type }] }` where `LocationInfo = { location_type, address, slot? }`.
- S02 should handle the case where `conflict_details.per_tx[i].reads` is empty (ReadSet was not preserved) gracefully — this indicates a data collection gap, not "no reads".

## Files Created/Modified

- `crates/scheduler/src/coordinator.rs` — Added `return_read_set()` method, changed `collect_results()` to return 3-tuple with ReadSet
- `crates/scheduler/src/parallel_executor.rs` — Updated `ParallelExecutionResult::tx_results` to 3-tuple, modified `handle_validate()` to preserve ReadSet on success, updated test patterns, added `test_read_set_preserved_after_validation`
- `crates/cli/src/main.rs` — Extended `CliOutput` with `conflict_details` field, added `mod conflict` declaration, wired `detect_conflicts()` call into execution pipeline
- `crates/cli/src/conflict.rs` — New: serializable types (`ConflictDetails`, `TxAccessSummary`, `LocationInfo`, `ConflictPair`), `detect_conflicts()` function, `location_key_to_info()` converter, 7 unit tests
- `crates/cli/Cargo.toml` — Added `monad-mv-state` dependency

## Forward Intelligence

### What the next slice should know
- The `conflict_details` JSON schema uses `location_type` (not `type`) and `slot` is only present for Storage locations (omitted via `skip_serializing_if`). S02's NestJS `CliOutput` TypeScript interface must match: `slot?: string` not `slot: string | null`.
- Addresses are lowercase hex with `0x` prefix (e.g., `0x00000000000000000000000000000000000000e1`). The NestJS parser should be case-insensitive for address matching.
- `conflict_type` is exactly `"write-write"` or `"read-write"` — no other values.

### What's fragile
- The coinbase address appears in both reads and writes for all txs (gas fee processing). This means nearly every tx pair will show conflicts involving the coinbase. S02 should filter out coinbase-related conflicts or at least deprioritize them in suggestions — they're inherent to EVM execution, not user-fixable.
- ReadSet preservation depends on `handle_validate()` calling `return_read_set()` on the success path. If the scheduler internals change (e.g., validation logic refactored), ReadSets could silently go missing — the symptom would be empty `reads` arrays in `per_tx`.

### Authoritative diagnostics
- `cargo test -p monad-scheduler test_read_set_preserved_after_validation` — confirms ReadSet contains Balance and Nonce LocationKeys after transfer execution. If this test fails, the entire conflict analysis pipeline has no data.
- `echo '<block>' | cargo run -p monad-cli 2>/dev/null | jq '.conflict_details.per_tx[0].reads | length'` — should be >0 for any block with transactions. If 0, ReadSet preservation is broken.

### What assumptions changed
- Original assumption: ReadSet would need significant scheduler restructuring to preserve. Actual: Only a single `return_read_set()` call on the validation success path was needed — the `TxState` already had `read_set: Option<ReadSet>` field.
- Original plan estimated 2.5h for both tasks. Actual: ~25m total. The scheduler's existing `take_read_set()`/mutex pattern made the inverse trivial.
