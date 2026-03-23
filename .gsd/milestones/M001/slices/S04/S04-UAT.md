# S04: Engine Bridge + Vibe-Score — UAT

**Milestone:** M001
**Written:** 2026-03-22

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All components are verified through compilation (Rust + TypeScript), unit tests (28 NestJS + 8 Rust), and structured test output. No live server or human interaction required — the subprocess bridge and scoring pipeline are fully testable via mocked dependencies and CLI invocation.

## Preconditions

- Rust toolchain installed (cargo available in PATH)
- Node.js + npm installed
- Working directory: `/home/ahwlsqja/monad-core/.gsd/worktrees/M001`
- `cd backend && npm install` completed (dependencies present)

## Smoke Test

```bash
cd /home/ahwlsqja/monad-core/.gsd/worktrees/M001 && \
  cargo build -p monad-cli && \
  cd backend && npm run build && npm test
```
Expected: Rust binary compiles, NestJS compiles with zero errors, all 96 tests pass.

## Test Cases

### 1. Rust CLI binary compiles and produces correct output

1. Run `cargo build -p monad-cli`
2. Run `echo '{"transactions":[],"block_env":{"number":1,"coinbase":"0x00000000000000000000000000000000000000C0","timestamp":1700000000,"gas_limit":30000000,"base_fee":"0x0","difficulty":"0x0"}}' | cargo run --quiet -p monad-cli`
3. **Expected:** JSON output with `{"results":[],"incarnations":[],"stats":{"total_gas":0,"num_transactions":0,"num_conflicts":0,"num_re_executions":0}}`

### 2. Rust CLI binary handles invalid JSON gracefully

1. Run `echo 'not json' | cargo run --quiet -p monad-cli 2>/tmp/cli_err; echo "exit=$?"`
2. Run `cat /tmp/cli_err`
3. **Expected:** Exit code 1. stderr contains JSON `{"error":"..."}` with parse error message. No panic or crash.

### 3. Rust CLI incarnation tracking works for transfers

1. Run `echo '{"transactions":[{"from":"0x00000000000000000000000000000000000000E1","to":"0x00000000000000000000000000000000000000E2","value":"0x1000","data":"0x","gas_limit":21000,"gas_price":"0x1","nonce":0}],"block_env":{"number":1,"coinbase":"0x00000000000000000000000000000000000000C0","timestamp":1700000000,"gas_limit":30000000,"base_fee":"0x0","difficulty":"0x0"}}' | cargo run --quiet -p monad-cli`
2. **Expected:** JSON output with `incarnations: [0]` (no re-execution needed for single tx), `results` array with one entry showing `success: true` and `gas_used: 21000`.

### 4. Rust CLI integration tests pass

1. Run `cargo test -p monad-cli`
2. **Expected:** 8 tests pass — `test_cli_input_json_parsing`, `test_empty_block`, `test_prefunded_accounts_exist`, `test_single_transfer`, `test_json_roundtrip`, `test_independent_transfers_no_conflicts`, `test_conflicting_transactions_same_sender`, `test_multiple_independent_transfers`.

### 5. NestJS EngineService tests verify all failure modes

1. Run `cd backend && npm test -- --testPathPattern=engine.service --verbose`
2. **Expected:** 10 tests pass including: returns null for empty binary path, undefined binary path, binary file not found, spawnSync timeout (ETIMEDOUT), spawnSync exception, non-zero exit code, and invalid JSON stdout. Returns parsed JSON on success.

### 6. NestJS VibeScoreService tests verify scoring pipeline

1. Run `cd backend && npm test -- --testPathPattern=vibe-score.service --verbose`
2. **Expected:** 13 tests pass including: engine-based scoring with engineBased=true, high score (≥80) with no conflicts, lower score with conflicts, gasEfficiency calculation, heuristic fallback when engine returns null, heuristic fallback for view-only ABI, DB persistence, DB failure resilience, compilation error propagation.

### 7. NestJS VibeScoreController tests verify endpoint

1. Run `cd backend && npm test -- --testPathPattern=vibe-score.controller --verbose`
2. **Expected:** 5 tests pass including: controller is defined, calls analyzeContract with source, returns VibeScoreResultDto shape, propagates service errors.

### 8. Full NestJS test suite passes with no regressions

1. Run `cd backend && npm test --verbose`
2. **Expected:** 96 tests pass across 14 suites. Zero failures. No regressions from S01/S02/S03 tests.

### 9. NestJS build succeeds with zero TypeScript errors

1. Run `cd backend && npm run build`
2. **Expected:** Build completes with zero errors. EngineModule and VibeScoreModule are wired into AppModule.

### 10. Module wiring verification

1. Run `grep -c 'EngineModule\|VibeScoreModule' backend/src/app.module.ts`
2. **Expected:** Output ≥ 2 (both modules imported in AppModule).

## Edge Cases

### Missing ENGINE_BINARY_PATH env var

1. Unset ENGINE_BINARY_PATH (or leave empty)
2. EngineService.executeBlock() should return null
3. VibeScoreService should fall back to OptimizerService heuristic scoring
4. **Expected:** Response has `engineBased: false`, valid vibeScore, no crash

### Rust struct serde roundtrip

1. Run `cargo test -p monad-cli test_json_roundtrip`
2. **Expected:** CliOutput serializes to JSON and deserializes back to identical struct

### Score formula boundaries

1. VibeScoreService unit test `returns vibeScore=100 with no conflicts, no re-execs, all success`
2. VibeScoreService unit test `applies capped conflict penalty`
3. **Expected:** Perfect conditions → score 100. Maximum penalties → score ≥ 10 (penalties capped at 90 total)

### Compilation failure propagation

1. VibeScoreService unit test `propagates compilation error`
2. **Expected:** CompileService error propagates to controller as HTTP error, no engine invocation attempted

## Failure Signals

- `cargo build -p monad-cli` fails → serde derives on ExecutionResult/BlockResult broken or Cargo.toml workspace config wrong
- `cargo test -p monad-cli` fails → CLI pipeline, JSON parsing, or incarnation tracking broken
- `npm run build` fails → TypeScript types in engine/vibe-score modules don't align with imports
- `npm test` shows < 96 tests → regressions in existing S01/S02/S03 tests
- `npm test -- --testPathPattern=engine.service` fails → EngineService subprocess handling broken
- `npm test -- --testPathPattern=vibe-score` fails → scoring pipeline, fallback, or DB persistence broken
- EngineService returns a value instead of null on error → graceful degradation contract violated

## Not Proven By This UAT

- **Live HTTP endpoint behavior** — no server is started; controller tests use NestJS testing module, not real HTTP
- **Real Solidity compilation through the pipeline** — CompileService is mocked in vibe-score tests; real compilation is tested in S02 test suite
- **Actual vibe-score differentiation between ParallelConflict vs FixedContract** — requires live server with ENGINE_BINARY_PATH set (integration test scope, not unit)
- **Frontend display of vibe-score results** — deferred to S05
- **Rust binary in Docker container** — deferred to S06

## Notes for Tester

- The ERROR log lines in test output (e.g., `[EngineService] Engine CLI timed out`) are expected — these are the error handling paths being exercised by tests, not real failures.
- Test count should be exactly 96 (68 from S01-S03 + 28 from S04). If more or fewer, something has changed.
- The Rust binary takes ~10s to compile from scratch but <1s for incremental builds. First run may be slow.
- `cargo test -p monad-types` (31 tests) and `cargo test -p monad-scheduler` (24 tests) verify the serde derives and incarnation changes didn't break existing Rust code.
