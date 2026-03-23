---
estimated_steps: 5
estimated_files: 4
---

# T02: Build CLI conflict detection module and wire conflict_details into JSON output

**Slice:** S01 — Rust CLI — R/W Set 충돌 데이터 JSON 출력
**Milestone:** M006

## Description

T01에서 보존된 ReadSet/WriteSet을 활용하여 conflict detection 로직을 구현하고, CLI JSON 출력에 `conflict_details` 필드를 추가한다. 이것이 S01의 최종 산출물이자 S02(NestJS)에서 파싱할 인터페이스 스키마다.

**중요 제약:** `LocationKey`, `WriteValue`, `ReadOrigin` 등 `monad-mv-state` 내부 타입에는 `Serialize` derive를 추가하지 않는다. CLI 전용 직렬화 타입(`LocationInfo`, `TxAccessSummary`, `ConflictPair`, `ConflictDetails`)을 정의하고 패턴 매칭으로 변환한다. 이유: mv-state 크레이트는 병렬 실행의 hot path이며 serde 의존성이 없다.

**관련 스킬:** 없음 (순수 Rust CLI 코드)

## Steps

1. **`crates/cli/Cargo.toml`에 `monad-mv-state` 의존성 추가.**
   ```toml
   monad-mv-state = { path = "../mv-state" }
   ```
   - ReadSet, WriteSet, LocationKey, WriteValue 타입에 접근하기 위해 필요.

2. **`crates/cli/src/conflict.rs` 모듈 생성 — 직렬화 타입 정의.**
   - `#[derive(Serialize)]` 타입들:
     - `ConflictDetails { per_tx: Vec<TxAccessSummary>, conflicts: Vec<ConflictPair> }`
     - `TxAccessSummary { tx_index: usize, reads: Vec<LocationInfo>, writes: Vec<LocationInfo> }`
     - `LocationInfo { location_type: String, address: String, slot: Option<String> }`
       - `location_type`: "Storage" | "Balance" | "Nonce" | "CodeHash"
       - `address`: "0x..." hex 문자열
       - `slot`: Storage 타입일 때만 Some("0x..." hex)
     - `ConflictPair { location: LocationInfo, tx_a: usize, tx_b: usize, conflict_type: String }`
       - `conflict_type`: "write-write" | "read-write"
   - `fn location_key_to_info(key: &LocationKey) -> LocationInfo` — 패턴 매칭으로 변환:
     - `LocationKey::Storage(addr, slot)` → `LocationInfo { location_type: "Storage", address: format!("0x{:x}", addr), slot: Some(format!("0x{:x}", slot)) }`
     - `LocationKey::Balance(addr)` → `LocationInfo { location_type: "Balance", address: ..., slot: None }`
     - `LocationKey::Nonce(addr)` / `LocationKey::CodeHash(addr)` — 동일 패턴

3. **`crates/cli/src/conflict.rs` — `detect_conflicts()` 함수 구현.**
   ```rust
   pub fn detect_conflicts(
       tx_results: &[(ExecutionResult, WriteSet, ReadSet)],
   ) -> ConflictDetails
   ```
   - **per_tx 구축:** 각 tx의 ReadSet.iter(), WriteSet.iter()를 순회하여 `TxAccessSummary` 생성.
   - **conflicts 검출:** 모든 tx 쌍 (tx_a, tx_b) where tx_a < tx_b에 대해:
     - **write-write:** `write_set_a.keys() ∩ write_set_b.keys()` — 교차하는 LocationKey마다 ConflictPair 추가
     - **read-write (양방향):** `read_set_a.keys() ∩ write_set_b.keys()` + `write_set_a.keys() ∩ read_set_b.keys()`
   - **BTreeMap keys 교차점 검출:** ReadSet.iter()와 WriteSet.iter()는 모두 BTreeMap 기반이므로 정렬 순서가 보장됨. 간단한 이중 루프 또는 `HashSet` 변환 후 교차점 — tx 수가 적으므로 성능 무관.
   - **중복 제거:** 같은 (location, tx_a, tx_b) 조합에서 write-write가 이미 검출된 경우 read-write는 추가하지 않음 (또는 모두 포함 — S02에서 사용할 때 더 많은 정보가 좋으므로 모두 포함).

4. **`crates/cli/src/main.rs` 수정 — conflict_details 통합.**
   - `mod conflict;` 선언 추가.
   - `use conflict::{detect_conflicts, ConflictDetails};` 추가.
   - `CliOutput` struct에 `conflict_details: ConflictDetails` 필드 추가.
   - 기존 result-mapping 루프 수정: `par_result.tx_results.iter().map(|(exec_result, _write_set, _read_set)| ...)` — T01에서 이미 `_read_set` 무시 패턴을 적용했으므로, 이제 `_`를 제거하고 실제 참조로 변경.
   - 실행 완료 후 `detect_conflicts(&par_result.tx_results)` 호출하여 `ConflictDetails` 생성.
   - `CliOutput { results, incarnations, stats, conflict_details }` 구성.

5. **`crates/cli/src/conflict.rs` — 단위 테스트 작성.**
   - `#[cfg(test)] mod tests` 블록:
   - **`test_detect_write_write_conflict`:** 2개 tx가 같은 `Balance(addr)` 위치에 write → `conflicts`에 write-write 1건 검출.
   - **`test_detect_read_write_conflict`:** tx_a가 `Balance(addr)` read, tx_b가 같은 위치에 write → read-write 1건 검출.
   - **`test_no_conflict_independent_txs`:** 2개 tx가 다른 주소에 read/write → `conflicts` 빈 배열.
   - **`test_per_tx_summary`:** 각 tx의 reads/writes 목록이 정확히 생성되는지 검증.
   - **`test_location_key_to_info`:** Storage, Balance, Nonce, CodeHash 각각의 변환 정확성 검증.
   - 테스트에서 사용할 헬퍼: 빈 `ExecutionResult::Success`와 수동 구성한 `ReadSet`/`WriteSet` 사용. `ReadSet::record()`, `WriteSet::record()` 직접 호출.

## Must-Haves

- [ ] `conflict.rs` 모듈에 `ConflictDetails`, `TxAccessSummary`, `LocationInfo`, `ConflictPair` 타입 정의
- [ ] `detect_conflicts()` 함수가 write-write, read-write 충돌을 정확히 검출
- [ ] `CliOutput`에 `conflict_details` 필드 포함
- [ ] 충돌 없는 독립 tx에서 `conflicts: []`
- [ ] `monad-mv-state` 내부 타입에 Serialize 추가하지 않음
- [ ] 단위 테스트 5개 이상 통과

## Verification

- `cargo test -p monad-cli` — 모든 conflict detection 테스트 통과
- `cargo build -p monad-cli` — 빌드 성공
- Integration: `echo '{"transactions":[{"sender":"0x00000000000000000000000000000000000000E1","to":"0x00000000000000000000000000000000000000F1","data":"0x","value":"1000","gas_limit":100000,"nonce":0,"gas_price":"1000000000"},{"sender":"0x00000000000000000000000000000000000000E1","to":"0x00000000000000000000000000000000000000F2","data":"0x","value":"2000","gas_limit":100000,"nonce":1,"gas_price":"1000000000"}],"block_env":{"number":1,"coinbase":"0x00000000000000000000000000000000000000C0","timestamp":1700000000,"gas_limit":30000000,"base_fee":"0","difficulty":"0"}}' | cargo run -p monad-cli 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'conflict_details' in d; assert 'per_tx' in d['conflict_details']; assert 'conflicts' in d['conflict_details']; print('OK')"` — "OK" 출력

## Inputs

- `crates/scheduler/src/parallel_executor.rs` — T01에서 변경된 `ParallelExecutionResult` (3-tuple tx_results)
- `crates/scheduler/src/coordinator.rs` — T01에서 변경된 `collect_results()` (3-tuple 반환)
- `crates/cli/src/main.rs` — T01에서 최소 수정된 파일 (3-tuple destructure)
- `crates/cli/Cargo.toml` — 현재 의존성 목록
- `crates/mv-state/src/types.rs` — `LocationKey`, `WriteValue` enum 정의
- `crates/mv-state/src/read_write_sets.rs` — `ReadSet`, `WriteSet` API (iter, record 등)

## Expected Output

- `crates/cli/src/conflict.rs` — 신규: 직렬화 타입 + detect_conflicts() + 단위 테스트
- `crates/cli/src/main.rs` — CliOutput 확장, conflict_details 통합
- `crates/cli/Cargo.toml` — monad-mv-state 의존성 추가
