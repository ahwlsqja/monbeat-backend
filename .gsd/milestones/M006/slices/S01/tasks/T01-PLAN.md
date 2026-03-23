---
estimated_steps: 5
estimated_files: 3
---

# T01: Preserve ReadSets in scheduler after validation and extend collect_results

**Slice:** S01 — Rust CLI — R/W Set 충돌 데이터 JSON 출력
**Milestone:** M006

## Description

현재 `handle_validate()` (in `parallel_executor.rs`)에서 `scheduler.take_read_set(tx_idx)`로 ReadSet을 꺼낸 후, validation이 끝나면 ReadSet이 drop된다. 이 태스크는 validation 성공 시 ReadSet을 TxState에 돌려놓고, `collect_results()`가 `Vec<(ExecutionResult, WriteSet, ReadSet)>` 3-tuple을 반환하도록 변경한다. 이것은 conflict analysis의 전제 조건이다.

**핵심 리스크:** 이 변경은 scheduler의 hot loop에 영향을 준다. 기존 24개 테스트가 regression guard 역할을 한다.

**관련 스킬:** 없음 (순수 Rust 시스템 코드)

## Steps

1. **`coordinator.rs`에 `return_read_set()` 메서드 추가.**
   - `Scheduler` impl 블록에 새 public 메서드 추가:
   ```rust
   pub fn return_read_set(&self, tx_index: TxIndex, read_set: ReadSet) {
       let mut state = self.tx_states[tx_index as usize].lock();
       state.read_set = Some(read_set);
   }
   ```
   - 이 메서드는 `take_read_set()`의 역 — 같은 mutex 패턴이므로 thread safety 문제 없음.

2. **`parallel_executor.rs`의 `handle_validate()` 수정.**
   - 현재 코드:
   ```rust
   fn handle_validate(...) {
       let read_set = scheduler.take_read_set(tx_idx);
       let valid = validate_transaction(tx_idx, &read_set, mv_state);
       if !valid { /* mark_estimate, clear, clear_tx */ }
       scheduler.finish_validation(tx_idx, valid);
   }
   ```
   - 변경: validation 성공 시 (`valid == true`), `finish_validation()` 호출 **전에** `scheduler.return_read_set(tx_idx, read_set)` 호출. 실패 시는 호출하지 않음 (tx가 재실행되면 새 ReadSet이 생김).
   - **주의:** `finish_validation(tx_idx, false)` 내부에서 `state.read_set = None`으로 클리어하므로, 실패 경로에서 return_read_set을 호출해도 의미 없음. 그래서 성공 시에만 호출.

3. **`coordinator.rs`의 `collect_results()` 반환 타입 변경.**
   - 현재: `Vec<(ExecutionResult, WriteSet)>`
   - 변경: `Vec<(ExecutionResult, WriteSet, ReadSet)>`
   - 루프 내부에서 `state.read_set.take().unwrap_or_default()` 추가하여 ReadSet도 함께 수집.

4. **`parallel_executor.rs`의 `ParallelExecutionResult` 및 `execute_block_parallel()` 업데이트.**
   - `ParallelExecutionResult::tx_results` 타입을 `Vec<(ExecutionResult, WriteSet, ReadSet)>`로 변경.
   - `execute_block_parallel()` 내 `scheduler.collect_results()` 결과를 그대로 `tx_results`에 할당 (이미 3-tuple 반환됨).
   - `incarnations` 수집 코드는 변경 없음 (`scheduler.get_tx_state()`로 별도 수집).

5. **기존 테스트 코드의 2-tuple 패턴 매칭을 3-tuple로 업데이트 + 새 테스트 추가.**
   - `test_parallel_independent_transfers`: `for (i, (exec_result, write_set)) in ...` → `for (i, (exec_result, write_set, _read_set)) in ...`
   - `test_parallel_single_transaction`: 동일 패턴 변경
   - 새 테스트 `test_read_set_preserved_after_validation` 추가:
     - 2개의 value transfer (같은 sender → 다른 receiver) 실행
     - `par_result.tx_results`에서 3번째 원소(ReadSet)가 비어있지 않은지 확인
     - ReadSet에 `Balance`, `Nonce` 등의 LocationKey가 포함되어 있는지 확인

## Must-Haves

- [ ] `Scheduler::return_read_set()` 메서드가 존재하고 TxState.read_set에 ReadSet을 복원함
- [ ] `handle_validate()`에서 validation 성공 시 `return_read_set()` 호출
- [ ] `collect_results()` 반환 타입이 `Vec<(ExecutionResult, WriteSet, ReadSet)>`
- [ ] `ParallelExecutionResult::tx_results`가 3-tuple
- [ ] 기존 24개 테스트가 모두 통과 (regression)
- [ ] 새 `test_read_set_preserved_after_validation` 테스트 통과

## Verification

- `cargo test -p monad-scheduler` — 기존 24개 + 새 테스트 모두 통과
- `cargo build -p monad-cli` — CLI 바이너리 빌드 확인 (3-tuple 변경이 CLI 코드에 영향줌 — 컴파일만 확인, 기능은 T02에서)

## Inputs

- `crates/scheduler/src/coordinator.rs` — `Scheduler` struct, `take_read_set()`, `collect_results()`, `finish_validation()` 메서드
- `crates/scheduler/src/parallel_executor.rs` — `handle_validate()`, `ParallelExecutionResult`, 기존 테스트들
- `crates/scheduler/src/types.rs` — `TxState` (read_set: Option<ReadSet> 필드 이미 존재)
- `crates/cli/src/main.rs` — T01 변경 후 `tx_results` 3-tuple로 인해 컴파일 에러 발생할 수 있으므로, 최소한 컴파일 가능하도록 `_read_set` 무시 패턴 적용 필요

## Expected Output

- `crates/scheduler/src/coordinator.rs` — `return_read_set()` 추가, `collect_results()` 3-tuple 반환
- `crates/scheduler/src/parallel_executor.rs` — `ParallelExecutionResult` 3-tuple, `handle_validate()` ReadSet 보존, 테스트 업데이트 + 새 테스트
- `crates/cli/src/main.rs` — 3-tuple 변경에 따른 최소 컴파일 호환 (destructure에 `_read_set` 추가)
