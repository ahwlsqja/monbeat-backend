---
id: T02
parent: S01
milestone: M006
provides:
  - conflict.rs module with ConflictDetails, TxAccessSummary, LocationInfo, ConflictPair serializable types
  - detect_conflicts() function detecting write-write and read-write conflicts from ReadSet/WriteSet data
  - CliOutput extended with conflict_details field in CLI JSON output
  - location_key_to_info() converting LocationKey to CLI-specific LocationInfo without adding Serialize to mv-state
key_files:
  - crates/cli/src/conflict.rs
  - crates/cli/src/main.rs
  - crates/cli/Cargo.toml
key_decisions:
  - CLI-specific serializable types (LocationInfo, ConflictPair, etc.) used instead of adding serde derives to mv-state hot-path types
  - Both write-write and read-write conflicts included even for same location+pair — more information is better for downstream S02 analysis
  - HashSet intersection used for conflict detection — tx count is small so O(n²) pairwise is acceptable
patterns_established:
  - Pattern-match LocationKey variants to CLI-specific LocationInfo for serialization boundary
  - detect_conflicts() takes the 3-tuple slice directly from ParallelExecutionResult::tx_results
observability_surfaces:
  - conflict_details.conflicts array length 0 = no conflicts, >0 = conflicts detected
  - conflict_details.per_tx[i].reads/writes length shows per-tx state access scope
  - Empty reads/writes arrays indicate ReadSet was not preserved (data loss explicitly observable)
  - Inspect via jq .conflict_details on CLI stdout
duration: 10m
verification_result: passed
completed_at: 2026-03-24
blocker_discovered: false
---

# T02: Build CLI conflict detection module and wire conflict_details into JSON output

**Added conflict.rs module with detect_conflicts() and wired conflict_details into CLI JSON output for write-write and read-write conflict analysis**

## What Happened

Implemented all 5 planned steps to create the conflict detection module and integrate it into CLI output:

1. Added `monad-mv-state` dependency to `crates/cli/Cargo.toml` for access to LocationKey, ReadSet, WriteSet types.

2. Created `crates/cli/src/conflict.rs` with CLI-specific serializable types: `ConflictDetails`, `TxAccessSummary`, `LocationInfo`, `ConflictPair`. The `location_key_to_info()` function pattern-matches each `LocationKey` variant (Storage, Balance, Nonce, CodeHash) into `LocationInfo` with lowercase hex addresses and optional slot.

3. Implemented `detect_conflicts()` which: (a) builds per-tx access summaries from ReadSet/WriteSet iterators, (b) detects write-write conflicts via write key intersection between all tx pairs, (c) detects read-write conflicts bidirectionally (a reads + b writes, and a writes + b reads). Uses HashSet intersection for simplicity.

4. Modified `main.rs`: added `mod conflict;` declaration, extended `CliOutput` with `conflict_details: ConflictDetails` field, called `detect_conflicts(&par_result.tx_results)` after parallel execution, wired result into output struct.

5. Wrote 7 unit tests covering: location_key_to_info conversion for all 4 variants, write-write conflict detection, read-write conflict detection, independent tx producing no conflicts, per-tx summary correctness, empty tx results, and storage slot inclusion.

## Verification

- `cargo test -p monad-cli`: 7 tests passed (all conflict detection tests)
- `cargo build -p monad-cli`: clean build
- `cargo test -p monad-scheduler`: 25 tests passed (all existing + T01 ReadSet test)
- Integration check: CLI outputs JSON with `conflict_details.per_tx` and `conflict_details.conflicts` — "OK"
- Empty block diagnostic: CLI handles 0 transactions correctly — "EMPTY_OK"

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p monad-cli` | 0 | ✅ pass | 3.4s |
| 2 | `cargo build -p monad-cli` | 0 | ✅ pass | 5.0s |
| 3 | `cargo test -p monad-scheduler` | 0 | ✅ pass | 3.7s |
| 4 | Integration check (2-tx block → assert conflict_details present) | 0 | ✅ pass | 3.7s |
| 5 | Empty block diagnostic (0-tx → assert results=[] and stats) | 0 | ✅ pass | 3.7s |

## Diagnostics

- Run `echo '<block JSON>' | cargo run -p monad-cli 2>/dev/null | jq .conflict_details` to inspect conflict output.
- `conflict_details.conflicts` length 0 means no conflicts detected; >0 lists each conflict with location, tx pair, and type.
- `conflict_details.per_tx[i].reads` being empty while writes are non-empty indicates the ReadSet was not preserved (likely validation failure path).
- Each `LocationInfo.slot` field is `null` for Balance/Nonce/CodeHash and present only for Storage locations.

## Deviations

- Added 2 extra tests beyond the 5 specified in the plan (`test_empty_tx_results`, `test_storage_location_includes_slot`) for better coverage — 7 total vs 5 required.
- Used `#[serde(skip_serializing_if = "Option::is_none")]` on `LocationInfo.slot` to produce cleaner JSON (omits `"slot": null` for non-Storage locations).

## Known Issues

None.

## Files Created/Modified

- `crates/cli/src/conflict.rs` — New: serializable types (ConflictDetails, TxAccessSummary, LocationInfo, ConflictPair), detect_conflicts() function, location_key_to_info() converter, 7 unit tests
- `crates/cli/src/main.rs` — Extended CliOutput with conflict_details field, added mod conflict declaration, wired detect_conflicts() call into execution pipeline
- `crates/cli/Cargo.toml` — Added monad-mv-state dependency
