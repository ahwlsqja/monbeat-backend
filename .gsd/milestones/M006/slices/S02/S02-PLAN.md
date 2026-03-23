# S02: NestJS — Storage Layout 디코딩 + Actionable Suggestion 생성

**Goal:** `/api/vibe-score` API 응답에 `conflictAnalysis` 필드가 포함되어, 충돌된 slot이 Solidity 변수명/mapping명으로 디코딩되고 구체적 수정 제안이 생성된다.
**Demo:** ParallelConflict 컨트랙트로 `/api/vibe-score` 호출 시, `conflictAnalysis.conflicts[0].variableName === "counter"` 이고 `suggestion`에 "counter" 변수명과 "increment"/"incrementBy" 함수명이 포함된 구체적 수정 제안이 반환된다. 충돌 없는 컨트랙트에서는 `conflictAnalysis`가 omit되고 기존 응답이 그대로 동작한다.

## Must-Haves

- CompileService가 solc `storageLayout`을 추출하여 CompileResultDto에 포함
- CLI `conflict_details` JSON을 파싱하는 `CliOutput` 인터페이스 확장 (S01 스키마와 정확히 일치)
- storage-layout-decoder 모듈이 hex runtime slot을 decimal storageLayout slot으로 변환하여 변수명 매핑
- 코인베이스 주소 충돌 필터링 (EVM 내재적 동작 — actionable하지 않음)
- mapping/dynamic array 타입의 runtime slot을 heuristic으로 base variable에 귀속
- 함수×변수명 매트릭스 생성 (S03 히트맵 소비용)
- 변수명/함수명/수정방법이 포함된 actionable suggestion 텍스트 생성
- `conflict_details` 없을 때 (older CLI, 0 conflicts) 기존 응답과 동일하게 동작 (backward compat)

## Proof Level

- This slice proves: contract + integration
- Real runtime required: no (unit tests with mock data, solc compilation in test)
- Human/UAT required: no

## Verification

- `cd Vibe-Room-Backend && npx jest test/compile.service.spec.ts` — 기존 테스트 모두 통과 + storageLayout 추출 테스트 통과
- `cd Vibe-Room-Backend && npx jest test/storage-layout-decoder.spec.ts` — decoder 모듈 8+ 테스트 통과 (slot 매칭, mapping heuristic, coinbase 필터, suggestion 생성, matrix 빌드)
- `cd Vibe-Room-Backend && npx jest test/vibe-score.service.spec.ts` — 기존 테스트 모두 통과 + conflict_details 있는 경우 conflictAnalysis 포함 테스트 + conflict_details 없는 경우 backward compat 테스트
- `cd Vibe-Room-Backend && npx jest test/storage-layout-decoder.spec.ts -- --testNamePattern="undefined|unknown"` — storageLayout undefined 시 graceful omit + 디코딩 실패 시 unknown_slot fallback 테스트 통과

## Observability / Diagnostics

- Runtime signals: VibeScoreService logs "Phase 5b: Conflict analysis" with decoded conflict count and timing
- Inspection surfaces: API response `conflictAnalysis` field presence/absence, `conflictAnalysis.conflicts` array length
- Failure visibility: If storageLayout is undefined (solc failure), conflictAnalysis is silently omitted — logged as warning. If slot decode fails, variable is reported as "unknown_slot_0xNNN"
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `crates/cli/src/conflict.rs` `ConflictDetails`/`ConflictPair`/`LocationInfo` JSON schema (from S01), `Vibe-Room-Backend/src/engine/engine.service.ts` `CliOutput` interface, `Vibe-Room-Backend/src/contracts/compile.service.ts` solc compilation
- New wiring introduced in this slice: CompileService → storageLayout extraction, EngineService.CliOutput → conflict_details parsing, VibeScoreService → decoder pipeline call, VibeScoreResultDto → conflictAnalysis field
- What remains before the milestone is truly usable end-to-end: S03 (frontend heatmap + suggestion cards), S04 (E2E test)

## Tasks

- [x] **T01: CompileService storageLayout 추출 + 전체 TypeScript 인터페이스 정의** `est:30m`
  - Why: 모든 downstream 로직의 기반. solc에서 storageLayout을 추출하지 않으면 slot→변수명 디코딩이 불가능. CliOutput 인터페이스도 S01 스키마와 일치시켜야 decoder가 파싱 가능.
  - Files: `Vibe-Room-Backend/src/contracts/compile.service.ts`, `Vibe-Room-Backend/src/contracts/dto/compile-result.dto.ts`, `Vibe-Room-Backend/src/engine/engine.service.ts`, `Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts`, `Vibe-Room-Backend/test/compile.service.spec.ts`
  - Do: (1) compile.service.ts의 solc outputSelection에 `'storageLayout'` 추가, SolcOutput 타입에 storageLayout 추가. (2) compile-result.dto.ts에 `storageLayout?: StorageLayout` 추가 + StorageLayout/StorageEntry/StorageTypeInfo 인터페이스 정의. (3) engine.service.ts의 CliOutput에 `conflict_details?: ConflictDetails` + S01 스키마 매칭 타입들 추가. (4) vibe-score-result.dto.ts에 `conflictAnalysis?: ConflictAnalysis` + DecodedConflict/ConflictMatrix 인터페이스 정의. (5) compile.service.spec.ts에 storageLayout 추출 확인 테스트 추가.
  - Verify: `cd Vibe-Room-Backend && npx jest test/compile.service.spec.ts`
  - Done when: 기존 compile 테스트 전부 통과 + ParallelConflict.sol 컴파일 시 storageLayout.storage에 "counter" 변수 포함 확인

- [x] **T02: storage-layout-decoder 모듈 구현 + 단위 테스트** `est:45m`
  - Why: 핵심 도메인 로직. hex runtime slot → decimal base slot 변환, mapping heuristic, coinbase 필터링, suggestion 생성, matrix 빌드를 순수 함수로 구현해야 downstream wiring이 가능.
  - Files: `Vibe-Room-Backend/src/vibe-score/storage-layout-decoder.ts`, `Vibe-Room-Backend/test/storage-layout-decoder.spec.ts`
  - Do: (1) decodeSlotToVariable(): hex slot → BigInt 변환, storageLayout.storage 순회하여 exact match (decimal slot == hex slot 비교), mapping/array heuristic (runtime slot > max declared slot → mapping base에 귀속). (2) buildConflictAnalysis(): conflict_details + storageLayout + ABI + tx→function map → decoded conflicts. coinbase 주소 필터링 (대소문자 무시). Storage 타입 conflict만 디코딩 대상. (3) generateSuggestion(): variable type별 구체적 제안 (mapping → "별도 mapping 분리", simple var → "함수별 별도 변수"). (4) buildMatrix(): function×variable 교차점에 conflict count 집계. (5) 8+ 단위 테스트: exact slot match, mapping heuristic, coinbase filter, empty conflicts, suggestion content, matrix dimensions, non-Storage conflict handling, storageLayout undefined.
  - Verify: `cd Vibe-Room-Backend && npx jest test/storage-layout-decoder.spec.ts`
  - Done when: 모든 테스트 통과. ParallelConflict 같은 fixture로 slot "0x0" → "counter" 디코딩, suggestion에 변수명+함수명 포함, matrix rows/cols 정확

- [x] **T03: VibeScoreService 파이프라인 wiring + 통합 테스트** `est:30m`
  - Why: T01의 타입 정의와 T02의 decoder를 실제 파이프라인에 연결. compile에서 storageLayout 전달, engine 결과에서 conflict_details 파싱, decoder 호출, 응답에 conflictAnalysis 포함. Backward compat 보장 필수.
  - Files: `Vibe-Room-Backend/src/vibe-score/vibe-score.service.ts`, `Vibe-Room-Backend/test/vibe-score.service.spec.ts`
  - Do: (1) Phase 1에서 compile 결과의 storageLayout 캡처. (2) Phase 3에서 tx index → function name Map 빌드 (deploy tx=0 → "constructor", 이후는 stateChangingFns[i].name). (3) Phase 5 뒤에 "Phase 5b: Conflict analysis" 추가 — engineResult.conflict_details 존재 시 buildConflictAnalysis() 호출. coinbase = blockEnv.coinbase. (4) conflictAnalysis가 비어있지 않으면 result에 포함, 비어있으면 omit. (5) 기존 vibe-score.service.spec.ts 테스트 전부 유지 + 새 테스트 3개: conflict_details 있는 mock → conflictAnalysis 포함 확인, conflict_details 없는 mock → conflictAnalysis omit 확인, storageLayout undefined → conflictAnalysis omit 확인.
  - Verify: `cd Vibe-Room-Backend && npx jest test/vibe-score.service.spec.ts`
  - Done when: 기존 11개 테스트 전부 통과 + 새 테스트 3개 통과. conflict_details 있을 때 conflictAnalysis.conflicts에 디코딩된 변수명 포함, 없을 때 conflictAnalysis undefined

## Files Likely Touched

- `Vibe-Room-Backend/src/contracts/compile.service.ts`
- `Vibe-Room-Backend/src/contracts/dto/compile-result.dto.ts`
- `Vibe-Room-Backend/src/engine/engine.service.ts`
- `Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts`
- `Vibe-Room-Backend/src/vibe-score/storage-layout-decoder.ts` (new)
- `Vibe-Room-Backend/src/vibe-score/vibe-score.service.ts`
- `Vibe-Room-Backend/test/compile.service.spec.ts`
- `Vibe-Room-Backend/test/storage-layout-decoder.spec.ts` (new)
- `Vibe-Room-Backend/test/vibe-score.service.spec.ts`
