# S01: Rust CLI — R/W Set 충돌 데이터 JSON 출력 — UAT

**Milestone:** M006
**Written:** 2026-03-24

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S01 is Rust-only. All outputs are JSON via CLI stdout and Rust test results. No live services, no UI, no human-experience judgment needed. Cargo test + CLI piping fully verifies the deliverables.

## Preconditions

- Rust toolchain installed (`cargo` available)
- Working directory is monad-core repo root (or worktree)
- No external services needed (Rust-only, in-memory state)

## Smoke Test

```bash
echo '{"transactions":[{"sender":"0xE1","to":"0xF1","data":"0x","value":"1000","gas_limit":100000,"nonce":0,"gas_price":"1000000000"}],"block_env":{"number":1,"coinbase":"0xC0","timestamp":1700000000,"gas_limit":30000000,"base_fee":"0","difficulty":"0"}}' | cargo run -p monad-cli 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'conflict_details' in d; print('SMOKE OK')"
```
**Expected:** `SMOKE OK` — confirms CLI produces JSON with conflict_details field.

## Test Cases

### 1. Scheduler ReadSet Preservation — Unit Test

1. Run `cargo test -p monad-scheduler test_read_set_preserved_after_validation`
2. **Expected:** Test passes. Confirms ReadSet contains Balance and Nonce LocationKeys after value transfer execution through the Block-STM validation pipeline.

### 2. Full Scheduler Regression — 25 Tests

1. Run `cargo test -p monad-scheduler`
2. **Expected:** 25 tests pass (24 existing + 1 new ReadSet preservation test). Zero failures. No regressions in Block-STM coordinator, worker, or parallel executor logic.

### 3. Conflict Detection Unit Tests — 7 Tests

1. Run `cargo test -p monad-cli`
2. **Expected:** 7 tests pass:
   - `test_location_key_to_info` — Storage/Balance/Nonce/CodeHash all convert correctly
   - `test_detect_write_write_conflict` — Two txs writing same Balance detected as write-write
   - `test_detect_read_write_conflict` — Read-write crossing detected
   - `test_no_conflict_independent_txs` — Independent txs produce empty conflicts
   - `test_per_tx_summary` — Per-tx reads/writes counted correctly
   - `test_empty_tx_results` — Zero txs produce empty results
   - `test_storage_location_includes_slot` — Storage locations include hex slot

### 4. CLI Build Verification

1. Run `cargo build -p monad-cli`
2. **Expected:** Clean build, no warnings related to conflict module. Binary produced at `target/debug/monad-cli`.

### 5. Integration — Two-Tx Block with Shared Sender

1. Run:
```bash
echo '{"transactions":[{"sender":"0x00000000000000000000000000000000000000E1","to":"0x00000000000000000000000000000000000000F1","data":"0x","value":"1000","gas_limit":100000,"nonce":0,"gas_price":"1000000000"},{"sender":"0x00000000000000000000000000000000000000E1","to":"0x00000000000000000000000000000000000000F2","data":"0x","value":"2000","gas_limit":100000,"nonce":1,"gas_price":"1000000000"}],"block_env":{"number":1,"coinbase":"0x00000000000000000000000000000000000000C0","timestamp":1700000000,"gas_limit":30000000,"base_fee":"0","difficulty":"0"}}' | cargo run -p monad-cli 2>/dev/null | python3 -c "
import sys, json
d = json.load(sys.stdin)
assert 'conflict_details' in d
assert 'per_tx' in d['conflict_details']
assert 'conflicts' in d['conflict_details']
assert len(d['conflict_details']['per_tx']) == 2
assert d['conflict_details']['per_tx'][0]['tx_index'] == 0
assert d['conflict_details']['per_tx'][1]['tx_index'] == 1
# Both txs share sender 0xE1, so reads/writes should be non-empty
assert len(d['conflict_details']['per_tx'][0]['reads']) > 0
assert len(d['conflict_details']['per_tx'][0]['writes']) > 0
# Shared sender means conflicts should exist (Balance/Nonce of 0xE1)
assert len(d['conflict_details']['conflicts']) > 0
# Verify conflict structure
c = d['conflict_details']['conflicts'][0]
assert 'location' in c
assert 'tx_a' in c
assert 'tx_b' in c
assert c['conflict_type'] in ['write-write', 'read-write']
assert c['location']['location_type'] in ['Storage', 'Balance', 'Nonce', 'CodeHash']
assert c['location']['address'].startswith('0x')
print('INTEGRATION OK')
"
```
2. **Expected:** `INTEGRATION OK` — confirms full conflict detection pipeline works end-to-end with real parallel execution.

### 6. Backward Compatibility — Existing Fields Preserved

1. Run:
```bash
echo '{"transactions":[{"sender":"0xE1","to":"0xF1","data":"0x","value":"1000","gas_limit":100000,"nonce":0,"gas_price":"1000000000"}],"block_env":{"number":1,"coinbase":"0xC0","timestamp":1700000000,"gas_limit":30000000,"base_fee":"0","difficulty":"0"}}' | cargo run -p monad-cli 2>/dev/null | python3 -c "
import sys, json
d = json.load(sys.stdin)
# All pre-existing fields must still be present
assert 'results' in d
assert 'incarnations' in d
assert 'stats' in d
assert 'total_gas' in d['stats']
assert 'num_transactions' in d['stats']
assert 'num_conflicts' in d['stats']
assert 'num_re_executions' in d['stats']
# New field also present
assert 'conflict_details' in d
print('BACKWARD COMPAT OK')
"
```
2. **Expected:** `BACKWARD COMPAT OK` — existing NestJS `CliOutput` interface fields are untouched.

## Edge Cases

### Empty Block — Zero Transactions

1. Run:
```bash
echo '{"transactions":[],"block_env":{"number":1,"coinbase":"0x00000000000000000000000000000000000000C0","timestamp":1700000000,"gas_limit":30000000,"base_fee":"0","difficulty":"0"}}' | cargo run -p monad-cli 2>/dev/null | python3 -c "
import sys, json
d = json.load(sys.stdin)
assert d['results'] == []
assert d['stats']['num_transactions'] == 0
assert d['conflict_details']['per_tx'] == []
assert d['conflict_details']['conflicts'] == []
print('EMPTY_OK')
"
```
2. **Expected:** `EMPTY_OK` — empty block produces empty arrays, no crashes.

### Storage Location Slot Serialization

1. Run any block with contract interactions (Storage locations) and verify via `jq`:
```bash
cargo run -p monad-cli 2>/dev/null <<< '<block with contract>' | jq '.conflict_details.per_tx[].writes[] | select(.location_type == "Storage") | .slot'
```
2. **Expected:** Storage locations have `"slot": "0x..."` present. Non-Storage locations (Balance, Nonce, CodeHash) have no `slot` field at all (not `null`, completely absent due to `skip_serializing_if`).

## Failure Signals

- `cargo test -p monad-scheduler` showing fewer than 25 tests = regression or test removed
- `conflict_details` missing from CLI output = mod conflict not wired or CliOutput struct wrong
- `per_tx[i].reads` always empty = ReadSet preservation broken in `handle_validate()` or `return_read_set()` not called
- `conflicts` always empty even for shared-sender txs = `detect_conflicts()` logic broken or ReadSet/WriteSet iterators empty
- `slot` field appearing as `null` for non-Storage locations = `skip_serializing_if` annotation missing

## Not Proven By This UAT

- Storage layout decoding (slot → variable name) — that's S02 scope
- NestJS parsing of conflict_details JSON — S02 must define its TypeScript interface
- Frontend visualization of conflicts — S03 scope
- Smart contract-level conflicts (EVM storage slots from Solidity state variables) — current test uses simple value transfers, not contract calls with storage writes. S04 E2E with ParallelConflict contract will prove this.

## Notes for Tester

- The coinbase address appears in conflicts for nearly every tx pair (gas fee processing). This is expected EVM behavior, not a bug. S02 should filter these.
- ReadSet data quality depends on Block-STM's validation flow. If you see empty reads but non-empty writes for a tx, it likely means that tx's validation failed and was re-executed — the ReadSet from the final successful execution should still be present.
- All addresses in output are lowercase hex with `0x` prefix.
