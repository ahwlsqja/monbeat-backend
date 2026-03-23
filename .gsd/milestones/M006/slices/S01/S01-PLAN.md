# S01: Rust CLI — R/W Set 충돌 데이터 JSON 출력

**Goal:** CLI가 ParallelConflict 컨트랙트 실행 후 `conflict_details` 필드를 포함한 확장 JSON을 반환하며, LocationKey별 충돌 tx 쌍 + 각 tx의 ReadSet/WriteSet 접근 위치가 포함된다.
**Demo:** `echo '<block JSON>' | cargo run -p monad-cli` 실행 시 기존 `results`, `incarnations`, `stats` 외에 `conflict_details.per_tx`(tx별 read/write 위치)와 `conflict_details.conflicts`(충돌 쌍)가 JSON으로 출력된다. 충돌 없는 독립 tx들은 `conflicts: []`.

## Must-Haves

- `Scheduler::collect_results()`가 `Vec<(ExecutionResult, WriteSet, ReadSet)>`를 반환하여 ReadSet이 validation 이후에도 보존됨
- `ParallelExecutionResult`에 `read_sets: Vec<ReadSet>` 필드 추가 (또는 tx_results 3-tuple로 통합)
- CLI `CliOutput`에 `conflict_details: ConflictDetails` 필드가 포함된 확장 JSON 출력
- `ConflictDetails.per_tx`에 tx별 read/write location 목록이 직렬화됨
- `ConflictDetails.conflicts`에 write-write, read-write 충돌 쌍이 정확히 검출됨
- 기존 CLI 출력 필드(`results`, `incarnations`, `stats`)가 하위 호환 유지
- 기존 24개 scheduler 테스트 전부 통과 (regression guard)
- 충돌 검출 로직에 대한 단위 테스트 존재

## Proof Level

- This slice proves: contract (scheduler ReadSet preservation) + integration (CLI JSON output)
- Real runtime required: yes (cargo test + cargo run)
- Human/UAT required: no

## Verification

- `cargo test -p monad-scheduler` — 기존 24개 + 새 ReadSet 보존 테스트 모두 통과
- `cargo test -p monad-cli` — conflict detection 단위 테스트 통과
- `cargo build -p monad-cli` — 바이너리 빌드 성공
- Integration check: `echo '{"transactions":[{"sender":"0x00000000000000000000000000000000000000E1","to":"0x00000000000000000000000000000000000000F1","data":"0x","value":"1000","gas_limit":100000,"nonce":0,"gas_price":"1000000000"},{"sender":"0x00000000000000000000000000000000000000E1","to":"0x00000000000000000000000000000000000000F2","data":"0x","value":"2000","gas_limit":100000,"nonce":1,"gas_price":"1000000000"}],"block_env":{"number":1,"coinbase":"0x00000000000000000000000000000000000000C0","timestamp":1700000000,"gas_limit":30000000,"base_fee":"0","difficulty":"0"}}' | cargo run -p monad-cli 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'conflict_details' in d; assert 'per_tx' in d['conflict_details']; assert 'conflicts' in d['conflict_details']; print('OK')"` — "OK" 출력
- Diagnostic/failure-path check: `echo '{"transactions":[],"block_env":{"number":1,"coinbase":"0x00000000000000000000000000000000000000C0","timestamp":1700000000,"gas_limit":30000000,"base_fee":"0","difficulty":"0"}}' | cargo run -p monad-cli 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['results']==[]; assert d['stats']['num_transactions']==0; print('EMPTY_OK')"` — "EMPTY_OK" 출력 (빈 블록에 대한 정상 처리 확인)

## Observability / Diagnostics

- Runtime signals: `conflict_details.conflicts` 배열 길이 0 = 충돌 없음, >0 = 충돌 존재. `per_tx[i].reads/writes` 길이로 각 tx의 state 접근 범위 확인 가능.
- Inspection surfaces: CLI stdout JSON 파싱 (`jq .conflict_details`)
- Failure visibility: ReadSet이 None인 경우 `per_tx[i].reads` 빈 배열, `conflict_details.conflicts` 빈 배열 — 데이터 손실이 명시적으로 관찰 가능
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: none (S01은 첫 슬라이스, Rust-only)
- New wiring introduced in this slice: `Scheduler::return_read_set()` 메서드, `collect_results()` 3-tuple 반환, CLI `conflict_details` JSON 출력 경로
- What remains before the milestone is truly usable end-to-end: S02 (NestJS storage layout 디코딩), S03 (히트맵 UI), S04 (E2E 검증)

## Tasks

- [x] **T01: Preserve ReadSets in scheduler after validation and extend collect_results** `est:1h`
  - Why: 현재 `handle_validate()`에서 `take_read_set()` 호출 후 ReadSet이 drop되어 실행 결과에서 접근 불가. conflict analysis의 전제 조건인 ReadSet 보존이 이 슬라이스의 가장 큰 리스크이므로 먼저 해결.
  - Files: `crates/scheduler/src/coordinator.rs`, `crates/scheduler/src/parallel_executor.rs`, `crates/scheduler/src/types.rs`
  - Do: (1) `Scheduler`에 `return_read_set(tx_index, read_set)` 메서드 추가. (2) `handle_validate()`에서 validation 성공 시 `return_read_set()` 호출하여 ReadSet을 TxState에 복원. 실패 시는 호출하지 않음 (tx가 재실행되므로). (3) `collect_results()` 반환 타입을 `Vec<(ExecutionResult, WriteSet, ReadSet)>`로 변경. (4) `ParallelExecutionResult::tx_results`를 3-tuple로 변경. (5) `parallel_executor.rs`의 기존 테스트 코드에서 2-tuple 패턴 매칭을 3-tuple로 업데이트. (6) `test_read_set_preserved_after_validation` 테스트 추가.
  - Verify: `cargo test -p monad-scheduler` — 기존 24개 + 새 테스트 모두 통과
  - Done when: `collect_results()` 반환값에 ReadSet이 포함되고, 기존 테스트 24개 + 새 테스트 1개 이상 모두 green

- [x] **T02: Build CLI conflict detection module and wire conflict_details into JSON output** `est:1h30m`
  - Why: T01에서 보존된 ReadSet/WriteSet 데이터를 활용하여 실제 충돌을 검출하고 CLI JSON에 포함. 이것이 S01의 최종 산출물이자 S02(NestJS)의 입력 스키마.
  - Files: `crates/cli/src/conflict.rs` (신규), `crates/cli/src/main.rs`, `crates/cli/Cargo.toml`
  - Do: (1) `Cargo.toml`에 `monad-mv-state` 의존성 추가. (2) `conflict.rs` 모듈 생성: `ConflictDetails`, `TxAccessSummary`, `LocationInfo`, `ConflictPair` 직렬화 타입 정의. `detect_conflicts()` 함수 구현 (모든 tx 쌍에서 write-write, read-write 교차점 검출). `LocationKey` → `LocationInfo` 변환 함수 구현. (3) `main.rs`에서 `mod conflict;` 선언, `CliOutput`에 `conflict_details` 필드 추가. (4) `par_result.tx_results` 3-tuple 반복문 업데이트. (5) 실행 후 `detect_conflicts()` 호출하여 결과에 포함. (6) `conflict.rs`에 단위 테스트 작성: 알려진 ReadSet/WriteSet 조합으로 충돌 검출 정확성 검증 + 충돌 없는 케이스 검증. **주의:** `LocationKey`, `WriteValue`, `ReadOrigin`에 Serialize를 추가하지 않음 — CLI 전용 타입으로 변환.
  - Verify: `cargo test -p monad-cli` — conflict detection 테스트 통과. `cargo build -p monad-cli` — 빌드 성공. 위 Integration check 커맨드 통과.
  - Done when: CLI가 `conflict_details` 필드를 포함한 JSON을 stdout에 출력하고, 단위 테스트가 충돌 검출 정확성을 검증

## Files Likely Touched

- `crates/scheduler/src/coordinator.rs` — `return_read_set()` 추가, `collect_results()` 반환 타입 변경
- `crates/scheduler/src/parallel_executor.rs` — `ParallelExecutionResult` 3-tuple, `handle_validate()` ReadSet 보존, 테스트 패턴 매칭 업데이트
- `crates/scheduler/src/types.rs` — 변경 없음 (TxState.read_set: Option<ReadSet> 이미 존재)
- `crates/cli/Cargo.toml` — `monad-mv-state` 의존성 추가
- `crates/cli/src/main.rs` — `CliOutput` 확장, 3-tuple 반복, `conflict_details` 출력
- `crates/cli/src/conflict.rs` — 신규: 직렬화 타입 + 충돌 검출 로직 + 단위 테스트
