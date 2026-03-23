# M006: Parallel Execution Optimization Suggestions

**Vision:** Vibe Score를 "점수"에서 "처방전"으로 전환한다. monad-core CLI가 R/W set 충돌 데이터를 추가 반환하고, NestJS가 solc storage layout으로 slot→변수명 디코딩 + 구체적 코드 수정 제안을 생성하고, Vibe-Loom이 함수×변수명 매트릭스 히트맵과 구조화된 suggestion 카드로 시각화한다.

## Success Criteria

- Rust CLI가 기존 출력(`results`, `incarnations`, `stats`)에 더해 `conflict_details` 필드를 반환하고, LocationKey별 충돌 tx 쌍이 포함되어 있다
- NestJS `/api/vibe-score` 응답에 `conflictAnalysis` 필드가 포함되어, slot이 Solidity 변수명/mapping명으로 디코딩되어 있다
- NestJS 응답의 `suggestions`가 generic 문장이 아니라 구체적 변수명 + 함수명 + 수정 방법을 포함한다
- Vibe-Loom VibeScoreDashboard에 함수×변수명 매트릭스 히트맵이 렌더링된다
- 충돌 없는 컨트랙트에서도 기존 기능(점수 게이지, stat grid)이 정상 동작한다
- ParallelConflict 컨트랙트로 전체 파이프라인이 E2E 검증된다

## Key Risks / Unknowns

- **ReadSet 보존 경로** — coordinator/scheduler 수정 필요. validation 후 ReadSet이 폐기되는 현재 구조를 변경해야 함 → S01에서 가장 먼저 검증
- **Storage layout 디코딩 정확도** — mapping/dynamic array의 runtime slot → base slot 매칭이 heuristic 기반. 100% 정확하지 않을 수 있으나, 주요 패턴(단순 변수, mapping, array)은 커버 → S02에서 검증
- **3개 레포 동시 수정** — CLI 출력 스키마 변경이 NestJS→Frontend까지 연쇄. 인터페이스 먼저 확정 후 구현

## Proof Strategy

- ReadSet 보존 → S01에서 Rust 단위 테스트로 `conflict_details` 출력 검증
- Storage layout 디코딩 → S02에서 ParallelConflict 컨트랙트의 slot→변수명 매핑 정확도 검증
- 전체 파이프라인 → S04에서 E2E 테스트로 컴파일→엔진→디코딩→히트맵 렌더링 검증

## Verification Classes

- Contract verification: Rust 단위 테스트 (`cargo test` — conflict_details 직렬화), NestJS 단위 테스트 (storage layout 디코딩)
- Integration verification: NestJS → CLI subprocess 호출 → 충돌 데이터 파싱 → suggestion 생성 파이프라인
- Operational verification: Vibe-Loom 브라우저에서 히트맵 렌더링 + suggestion 카드 표시
- UAT / human verification: 히트맵의 시각적 명확성, suggestion의 실용성은 사람이 판단

## Milestone Definition of Done

This milestone is complete only when all are true:

- Rust CLI가 `conflict_details` 필드를 포함한 확장 JSON을 반환하고 단위 테스트 통과
- NestJS가 solc storage layout으로 slot→변수명 디코딩하고 구체적 수정 제안을 API 응답에 포함
- Vibe-Loom VibeScoreDashboard에 함수×변수명 매트릭스 히트맵이 렌더링됨
- 충돌 없는 컨트랙트에서 기존 기능이 하위 호환으로 동작
- E2E 테스트가 전체 파이프라인(ParallelConflict → 충돌 분석 → 히트맵)을 검증
- 각 레포(Core, Backend, Frontend)에 변경사항이 올바르게 커밋됨

## Requirement Coverage

- Covers: R017 (병렬 실행 최적화 제안), R018 (R/W Set 충돌 시각화), R006 (Vibe Score 강화)
- Partially covers: none
- Leaves for later: R019-R025 (M007/M008 스코프), R014 (CachedStateProvider — deferred), R015 (블록 replay — deferred)
- Orphan risks: none

## Slices

- [x] **S01: Rust CLI — R/W Set 충돌 데이터 JSON 출력** `risk:high` `depends:[]`
  > After this: CLI에 ParallelConflict 컨트랙트를 넣으면 `conflict_details`에 LocationKey별 충돌 tx 쌍 + 각 tx의 ReadSet/WriteSet 접근 위치가 JSON으로 반환된다. `cargo test`로 검증 가능.

- [x] **S02: NestJS — Storage Layout 디코딩 + Actionable Suggestion 생성** `risk:high` `depends:[S01]`
  > After this: `/api/vibe-score` 호출 시 충돌된 slot이 Solidity 변수명/mapping명으로 디코딩되고, "mapping `balances`에서 transfer()와 approve()가 충돌 — 별도 mapping 분리 권장" 같은 구체적 수정 제안이 응답에 포함된다.

- [x] **S03: Vibe-Loom — 매트릭스 히트맵 + Suggestion 카드 UI** `risk:medium` `depends:[S02]`
  > After this: VibeScoreDashboard에 함수×변수명 충돌 매트릭스 히트맵이 색상으로 충돌 강도를 표현하고, 각 충돌에 대한 구조화된 suggestion 카드(변수명, 관련 함수, 수정 방법)가 표시된다.

- [ ] **S04: E2E 검증 — 전체 파이프라인 통합 테스트** `risk:low` `depends:[S03]`
  > After this: ParallelConflict 컨트랙트로 Rust CLI→NestJS→Vibe-Loom 전체 파이프라인이 E2E 검증되고, 충돌 없는 컨트랙트의 하위 호환도 확인된다.

## Boundary Map

### S01 (standalone — Rust CLI extension)

Produces:
- `crates/cli/src/main.rs` — 확장된 `CliOutput` JSON: 기존 `results`, `incarnations`, `stats` + 새로운 `conflict_details` 필드
- `conflict_details` 스키마: `{ per_tx: [{ tx_index, reads: [{location_type, address, slot?, variant}], writes: [{location_type, address, slot?, variant, value_type}] }], conflicts: [{ location: {type, address, slot?}, tx_a, tx_b, conflict_type: "write-write"|"read-write" }] }`
- `crates/scheduler/src/parallel_executor.rs` — `ParallelExecutionResult`에 `read_sets: Vec<ReadSet>` 추가
- `crates/scheduler/src/coordinator.rs` — validation 후 ReadSet 보존 경로

Consumes:
- nothing (first slice, Rust-only)

### S01 → S02

Produces:
- CLI `conflict_details` JSON 스키마 (S02의 NestJS `CliOutput` 인터페이스가 이 스키마에 맞춰야 함)

### S02 (NestJS Backend extension)

Produces:
- `Vibe-Room-Backend/src/engine/engine.service.ts` — `CliOutput` 인터페이스에 `conflict_details` 추가
- `Vibe-Room-Backend/src/vibe-score/vibe-score.service.ts` — storage layout 디코딩 + actionable suggestion 생성 로직
- `Vibe-Room-Backend/src/contracts/compile.service.ts` — solc `storageLayout` 추출 추가
- `Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts` — `conflictAnalysis` 필드 추가
- API 응답 확장: `{ vibeScore, conflicts, reExecutions, gasEfficiency, engineBased, suggestions, conflictAnalysis: { conflicts: [{ variableName, variableType, slot, functions: [name], suggestion }], matrix: { rows: [funcName], cols: [varName], cells: [[intensity]] } } }`

Consumes from S01:
- CLI `conflict_details` JSON 스키마

### S02 → S03

Produces:
- API `/api/vibe-score` 응답의 `conflictAnalysis` 필드 스키마 (S03의 프론트엔드 타입이 이에 맞춰야 함)

### S03 (Vibe-Loom Frontend extension)

Produces:
- `Vibe-Loom/src/components/ide/VibeScoreDashboard.tsx` — 매트릭스 히트맵 + 구조화된 suggestion 카드 추가
- `Vibe-Loom/src/lib/api-client.ts` — `VibeScoreResult` 타입에 `conflictAnalysis` 추가

Consumes from S02:
- API `conflictAnalysis` 응답 스키마

### S04 (E2E verification)

Produces:
- E2E 테스트: ParallelConflict 전체 파이프라인 + FixedContract 하위 호환

Consumes from S01, S02, S03:
- 전체 파이프라인이 동작하는 상태
