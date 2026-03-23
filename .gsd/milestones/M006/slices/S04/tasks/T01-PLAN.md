---
estimated_steps: 5
estimated_files: 1
skills_used:
  - test
---

# T01: NestJS E2E — conflict analysis API 응답 형태 검증

**Slice:** S04 — E2E 검증 — 전체 파이프라인 통합 테스트
**Milestone:** M006

## Description

NestJS E2E 테스트 파일(`app.e2e-spec.ts`)에 conflict analysis 전용 `describe` 블록을 추가한다. EngineService를 mock override하여 `conflict_details`를 포함하는 응답을 반환하게 한 뒤, 실제 HTTP POST `/api/vibe-score`를 호출하여 Phase 5b 파이프라인(compile → storageLayout 추출 → slot→변수명 디코딩 → suggestion 생성 → conflictAnalysis 응답)이 통합 레벨에서 동작하는지 검증한다.

## Steps

1. **기존 E2E 파일 끝에 새 describe 블록 추가.** 기존 `describe('App (E2E)')` 안이 아닌, 별도의 `describe('Conflict Analysis E2E')` 블록으로 분리. 이 블록은 자체 `beforeAll`에서 TestingModule을 부트하되, EngineService를 추가로 override한다.

2. **EngineService mock 구성.** `executeBlock()` 메서드를 mock하여 다음을 반환:
   - ParallelConflict 소스일 때: `conflict_details` 포함 (slot `0x0`에서 write-write 충돌, tx_a=1, tx_b=2) + `stats` (num_conflicts=1, num_re_executions=1)
   - FixedContract/Simple 소스일 때: `conflict_details` 없음 (기존 동작)
   
   Mock 데이터는 S02의 단위 테스트(`vibe-score.service.spec.ts`)에 있는 `mockConflictDetails`와 동일한 형태를 사용:
   ```typescript
   {
     per_tx: [],
     conflicts: [{
       location: { location_type: 'Storage', address: '0xDeployedAddr', slot: '0x0' },
       tx_a: 1, tx_b: 2, conflict_type: 'write-write'
     }]
   }
   ```

3. **테스트 1: ParallelConflict 소스 → conflictAnalysis 필드 검증.**
   - `POST /api/vibe-score` with `{ source: <ParallelConflict 소스> }` — ParallelConflict 소스는 `GET /api/contracts/source?type=ParallelConflict`에서 가져오거나 인라인 정의
   - Assert 201, `body.success === true`
   - Assert `body.data.conflictAnalysis` 존재
   - Assert `body.data.conflictAnalysis.conflicts` 비어있지 않음
   - Assert `body.data.conflictAnalysis.conflicts[0].variableName === 'counter'`
   - Assert `body.data.conflictAnalysis.conflicts[0].conflictType === 'write-write'`
   - Assert `body.data.conflictAnalysis.conflicts[0].functions` 배열 포함 (length > 0)
   - Assert `body.data.conflictAnalysis.conflicts[0].suggestion` truthy
   - Assert `body.data.conflictAnalysis.matrix.rows.length > 0`
   - Assert `body.data.conflictAnalysis.matrix.cols.length > 0`
   - Assert 기존 필드(`vibeScore`, `engineBased`, `suggestions`) 존재

4. **테스트 2: FixedContract 소스 → conflictAnalysis 부재 + 하위 호환.**
   - EngineService mock이 `conflict_details` 없는 결과 반환 (기존 동작)
   - `POST /api/vibe-score` with `{ source: <Simple 소스> }` (기존 SIMPLE_CONTRACT 재사용)
   - Assert 201, `body.success === true`
   - Assert `body.data.conflictAnalysis` undefined 또는 absent
   - Assert `body.data.vibeScore` 숫자, 0~100
   - Assert `body.data.suggestions` 배열

5. **기존 테스트 통과 확인.** `npx jest test/app.e2e-spec.ts --forceExit` 전체 실행하여 새 테스트와 기존 테스트 모두 pass.

## Must-Haves

- [ ] ParallelConflict E2E 테스트: `conflictAnalysis.conflicts[0].variableName === 'counter'` + matrix rows/cols 존재 + suggestion 비어있지 않음
- [ ] FixedContract 하위 호환 E2E 테스트: `conflictAnalysis` 없음 + 기존 필드 정상
- [ ] 기존 E2E 테스트 전부 pass 유지
- [ ] EngineService mock은 별도 describe 블록에서만 적용 — 기존 블록의 heuristic fallback 동작에 영향 없음

## Verification

- `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/app.e2e-spec.ts --forceExit` — 전체 pass
- `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest --forceExit` — 전체 스위트 pass 유지

## Inputs

- `/home/ahwlsqja/Vibe-Room-Backend/test/app.e2e-spec.ts` — 기존 E2E 테스트 파일 (확장 대상)
- `/home/ahwlsqja/Vibe-Room-Backend/test/vibe-score.service.spec.ts` — mock 데이터 형태 참조 (mockConflictDetails, mockStorageLayout)
- `/home/ahwlsqja/Vibe-Room-Backend/src/engine/engine.service.ts` — EngineService 인터페이스 (CliOutput, executeBlock 시그니처)
- `/home/ahwlsqja/Vibe-Room-Backend/src/vibe-score/vibe-score.service.ts` — Phase 5b 파이프라인 로직 참조
- `/home/ahwlsqja/Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts` — ConflictAnalysis 응답 형태

## Expected Output

- `/home/ahwlsqja/Vibe-Room-Backend/test/app.e2e-spec.ts` — 새 `describe('Conflict Analysis E2E')` 블록 추가 (ParallelConflict + FixedContract 테스트)

## Observability Impact

- **New test signals:** 3 new tests in `Conflict Analysis E2E` describe block (ParallelConflict analysis, backward compat, isolation check). Test pass/fail status is visible in CI output and local jest runs.
- **Pipeline phase tracing:** The real VibeScoreService logs Phase 1–5b with timing to stderr during E2E test runs, enabling post-mortem diagnosis of pipeline issues.
- **Mock isolation verification:** An explicit test (`should only mock EngineService in this describe block`) serves as a canary — if EngineService mock leaks to the existing `App (E2E)` block, the heuristic fallback test there would fail.
- **Failure state inspection:** On assertion failure, Jest's detailed diff output shows the full `res.body.data` structure, including unexpected presence/absence of `conflictAnalysis`, making debugging straightforward.
