---
estimated_steps: 5
estimated_files: 2
---

# T03: VibeScoreService 파이프라인 wiring + 통합 테스트

**Slice:** S02 — NestJS — Storage Layout 디코딩 + Actionable Suggestion 생성
**Milestone:** M006

## Description

T01의 타입 정의와 T02의 decoder 모듈을 실제 VibeScoreService 파이프라인에 연결한다. compile에서 storageLayout을 전달받고, engine 결과에서 conflict_details를 파싱하고, tx index → function name 매핑을 빌드하고, decoder를 호출하여 conflictAnalysis를 API 응답에 포함한다. 기존 11개 테스트의 backward compat를 보장하면서 새 통합 테스트를 추가한다.

**Key constraints:**
- `conflict_details`가 없는 경우 (older CLI, engine null) → `conflictAnalysis` omit (기존 동작 유지)
- `storageLayout`이 undefined인 경우 → `conflictAnalysis` omit
- coinbase address는 `blockEnv.coinbase` 값 사용 (현재 `0x00...C0`)
- deploy tx (index 0) → function name "constructor"
- ethers mock이 이미 테스트에 존재 — 새 mock 추가 시 기존 mock과 충돌하지 않도록 주의

**Relevant skills:** `test` (Jest test patterns)

## Steps

1. **VibeScoreService.analyzeContract() Phase 1 수정** — `Vibe-Room-Backend/src/vibe-score/vibe-score.service.ts`:
   - `this.compileService.compile(source)` 결과에서 `storageLayout` 캡처
   - 기존 코드: `const compiled = this.compileService.compile(source);` — 이미 CompileResultDto가 storageLayout 포함 (T01)

2. **Phase 3: tx index → function name Map 빌드:**
   - `constructTransactionBlock()` 내부 또는 `analyzeContract()`에서, 각 tx의 index와 function name을 매핑하는 `Map<number, string>` 생성
   - tx 0 = "constructor" (deploy tx)
   - tx 1..N = stateChangingFns[i-1].name (0-based offset because tx 0 is deploy)
   - 이 map을 `constructTransactionBlock()` 리턴에 포함하거나, `analyzeContract()`에서 별도 빌드
   - **구현 선택:** `constructTransactionBlock()`이 `{ transactions, blockEnv, txFunctionMap }` 리턴하도록 확장 — 가장 깔끔. `txFunctionMap = new Map([[0, 'constructor'], [1, fn1.name], [2, fn2.name], ...])`

3. **Phase 5b: Conflict analysis 추가:**
   ```typescript
   // Phase 5b: Conflict analysis (if engine returned conflict_details)
   let conflictAnalysis: ConflictAnalysis | undefined;
   if (engineResult.conflict_details && compiled.storageLayout) {
     this.logger.log('Phase 5b: Decoding conflict analysis');
     const analysisStart = Date.now();
     conflictAnalysis = buildConflictAnalysis(
       engineResult.conflict_details,
       compiled.storageLayout,
       compiled.abi,
       txFunctionMap,
       blockEnv.coinbase,
     );
     // Omit if no decoded conflicts
     if (conflictAnalysis.conflicts.length === 0) {
       conflictAnalysis = undefined;
     }
     this.logger.log(`Phase 5b complete in ${Date.now() - analysisStart}ms: ${conflictAnalysis?.conflicts.length ?? 0} decoded conflicts`);
   }
   ```
   - `conflictAnalysis`를 `calculateScore()` 결과에 병합: `return { ...result, conflictAnalysis }`

4. **calculateScore() 또는 analyzeContract()에서 conflictAnalysis 포함:**
   - 가장 깔끔한 방법: `analyzeContract()` 마지막에 `result.conflictAnalysis = conflictAnalysis` 설정
   - `VibeScoreResultDto`에 이미 `conflictAnalysis?: ConflictAnalysis` 필드가 있으므로 (T01), 할당만 하면 됨

5. **통합 테스트 추가** — `Vibe-Room-Backend/test/vibe-score.service.spec.ts`:

   **테스트 A — conflict_details 있을 때 conflictAnalysis 포함:**
   - mock compileService.compile → storageLayout 포함 (ParallelConflict 패턴: counter at slot 0)
   - mock engineService.executeBlock → conflict_details 포함 (Storage conflict at slot "0x0", non-coinbase address)
   - analyzeContract() 호출 → result.conflictAnalysis가 정의됨
   - result.conflictAnalysis.conflicts[0].variableName === "counter"
   - result.conflictAnalysis.matrix.rows.length > 0

   **테스트 B — conflict_details 없을 때 backward compat:**
   - 기존 mock 그대로 (conflict_details 없음)
   - analyzeContract() 호출 → result.conflictAnalysis === undefined
   - 기존 필드 (vibeScore, conflicts, engineBased 등) 정상

   **테스트 C — storageLayout undefined 시 conflictAnalysis omit:**
   - mock compileService.compile → storageLayout 없음
   - mock engineService.executeBlock → conflict_details 있음
   - analyzeContract() 호출 → result.conflictAnalysis === undefined

   **주의:** 기존 ethers mock (`jest.mock('ethers', ...)`)과 `makeEngineResult()` 헬퍼를 유지. `makeEngineResult()`에 `conflict_details` override가 가능하도록 기존 overrides 패턴 활용. `mockCompileResult`에 `storageLayout` 추가 (일부 테스트에서만 override).

## Must-Haves

- [ ] compile storageLayout이 analyzeContract() 파이프라인에 전달
- [ ] tx index → function name Map이 빌드되고 decoder에 전달
- [ ] conflict_details 존재 + storageLayout 존재 시 buildConflictAnalysis() 호출
- [ ] 빈 conflicts일 때 conflictAnalysis omit (undefined)
- [ ] conflict_details 미존재 시 기존 동작과 동일 (backward compat)
- [ ] 기존 11개 테스트 전부 통과
- [ ] 새 테스트 3개 (A, B, C) 통과

## Verification

- `cd Vibe-Room-Backend && npx jest test/vibe-score.service.spec.ts` — 기존 11개 + 새 3개 = 14개 테스트 통과
- conflict_details가 있는 mock에서 conflictAnalysis.conflicts[0].variableName이 비어있지 않음

## Inputs

- `Vibe-Room-Backend/src/vibe-score/vibe-score.service.ts` — T01 이후: compile 결과에 storageLayout 포함되지만 미사용 상태
- `Vibe-Room-Backend/src/vibe-score/storage-layout-decoder.ts` — T02에서 구현한 buildConflictAnalysis() 함수
- `Vibe-Room-Backend/src/engine/engine.service.ts` — T01에서 확장한 CliOutput (conflict_details 포함)
- `Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts` — T01에서 확장한 VibeScoreResultDto (conflictAnalysis 포함)
- `Vibe-Room-Backend/test/vibe-score.service.spec.ts` — 현재 11개 테스트, makeEngineResult() 헬퍼, ethers mock

## Expected Output

- `Vibe-Room-Backend/src/vibe-score/vibe-score.service.ts` — storageLayout 전달, txFunctionMap 빌드, Phase 5b conflict analysis, conflictAnalysis 응답 포함
- `Vibe-Room-Backend/test/vibe-score.service.spec.ts` — 3개 새 테스트 (conflictAnalysis 포함/미포함/storageLayout없음)

## Observability Impact

- **New runtime signal:** VibeScoreService logs "Phase 5b: Decoding conflict analysis" with decoded conflict count and elapsed time when `conflict_details` and `storageLayout` are both present.
- **Inspection surface:** API response `conflictAnalysis` field presence/absence — if present, `conflicts` array contains decoded variable names and suggestions; if absent, engine lacked conflict_details or compile lacked storageLayout.
- **Failure visibility:** If storageLayout is undefined (solc failure), Phase 5b is silently skipped — no conflictAnalysis field in response. If slot decode fails, variable is reported as `unknown_slot_0xNNN` in the conflicts array.
- **Agent diagnostic:** Run `npx jest test/vibe-score.service.spec.ts --verbose` and check "conflict analysis wiring" test group for 3 pass/fail statuses.
