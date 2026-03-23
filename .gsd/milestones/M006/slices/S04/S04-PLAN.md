# S04: E2E 검증 — 전체 파이프라인 통합 테스트

**Goal:** ParallelConflict 컨트랙트로 Rust CLI→NestJS→Vibe-Loom 전체 파이프라인이 E2E 검증되고, 충돌 없는 컨트랙트의 하위 호환도 확인된다.
**Demo:** `npx jest test/app.e2e-spec.ts --forceExit` (Backend)와 `npx playwright test e2e/full-stack.spec.ts` (Frontend) 모두 conflict analysis 관련 새 테스트가 통과한다.

## Must-Haves

- NestJS E2E 테스트가 ParallelConflict 소스에 대해 `conflictAnalysis` 필드(decoded conflicts + matrix + suggestions)를 검증한다
- NestJS E2E 테스트가 FixedContract (충돌 없음) 소스에서 `conflictAnalysis` 부재 + 기존 필드 온전함을 검증한다
- Playwright E2E 테스트가 ParallelConflict 선택 → Vibe Score → 히트맵(`[data-testid="conflict-matrix"]`) + suggestion 카드(`[data-testid="conflict-card"]`) 렌더링을 검증한다
- Playwright E2E 테스트가 FixedContract 선택 → Vibe Score → 히트맵 부재 + 기존 UI(SVG gauge) 정상을 검증한다
- 기존 테스트 전부 통과 유지 (Backend 43+ tests, Frontend 27+ tests)

## Verification

- `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/app.e2e-spec.ts --forceExit` — 새 conflict analysis 테스트 포함 전체 pass
- `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest --forceExit` — 기존 전체 테스트 스위트 pass 유지
- `cd /home/ahwlsqja/Vibe-Loom && npx jest` — 기존 프론트엔드 단위 테스트 pass 유지
- `cd /home/ahwlsqja/Vibe-Loom && npx playwright test e2e/full-stack.spec.ts` — 새 conflict analysis 테스트 pass 또는 graceful skip (라이브 서비스 상태 의존)
- `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest --config ./test/jest-e2e.json --forceExit -t "isolation check"` — EngineService mock이 Conflict Analysis E2E 블록에만 격리되었는지 확인

## Tasks

- [x] **T01: NestJS E2E — conflict analysis API 응답 형태 검증** `est:30m`
  - Why: NestJS 파이프라인(compile → storageLayout 추출 → conflict decoding → suggestion 생성 → API 응답)을 통합 레벨에서 검증. S02에서 추가된 Phase 5b 로직이 실제 HTTP 레이어에서 올바른 응답을 반환하는지 확인.
  - Files: `Vibe-Room-Backend/test/app.e2e-spec.ts`
  - Do: (1) 기존 E2E describe 블록 아래에 새 `describe('Conflict Analysis E2E')` 추가. (2) EngineService를 override하여 `conflict_details` 포함 mock 반환하는 별도 TestingModule 부트. (3) ParallelConflict 소스로 `POST /api/vibe-score` → 201, `conflictAnalysis.conflicts[0].variableName === 'counter'`, matrix rows/cols 존재, suggestions 비어있지 않음. (4) FixedContract(충돌 없음) 소스로 `POST /api/vibe-score` → 201, `conflictAnalysis` undefined, 기존 vibeScore/suggestions 정상.
  - Verify: `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/app.e2e-spec.ts --forceExit`
  - Done when: 새 conflict analysis 테스트 2개 이상 pass + 기존 테스트 모두 pass

- [ ] **T02: Playwright E2E — 히트맵 렌더링 + 하위 호환 검증** `est:30m`
  - Why: 라이브 서비스에서 전체 파이프라인(ParallelConflict → Rust CLI → NestJS → Vibe-Loom)이 실제로 동작하는지 브라우저 레벨에서 검증. S03에서 추가된 UI 컴포넌트가 실제 API 데이터로 렌더링되는지 확인.
  - Files: `Vibe-Loom/e2e/full-stack.spec.ts`
  - Do: (1) 기존 파일에 `test.describe('Conflict Analysis E2E')` 블록 추가. (2) ParallelConflict 테스트: contract selector에서 ParallelConflict 선택 → Vibe Score 클릭 → `[data-testid="conflict-matrix"]` visible + `[data-testid="conflict-card"]` present + 'counter' 텍스트 확인. Promise.race 패턴으로 타임아웃/서비스 미배포 시 graceful skip. (3) FixedContract 하위 호환 테스트: FixedContract 선택 → Vibe Score → SVG gauge visible + `[data-testid="conflict-matrix"]` NOT present. (4) 각 단계에서 스크린샷 증거 캡처.
  - Verify: `cd /home/ahwlsqja/Vibe-Loom && npx playwright test e2e/full-stack.spec.ts`
  - Done when: 새 conflict analysis 테스트 2개가 pass 또는 서비스 상태에 따라 graceful skip

## Observability / Diagnostics

- **Test output signals:** Each E2E test suite produces pass/fail per test case with timing — visible in CI logs and local `npx jest` output.
- **EngineService mock isolation:** The `Conflict Analysis E2E` describe block uses a separate TestingModule with EngineService overridden. An isolation check test (`should only mock EngineService in this describe block`) verifies the mock is active only within this scope.
- **Pipeline phase logging:** Real VibeScoreService logs each phase (compile, block construction, engine, scoring, Phase 5b conflict analysis) with duration. In E2E tests with mocked EngineService, Phase 4/5/5b logs are still emitted to stderr, providing pipeline trace visibility.
- **Failure path diagnostics:** If conflictAnalysis assertions fail, the test output shows the full response body diff (Jest's `expect` detailed matcher output). The `conflict_details` mock data is explicit in the test for easy debugging.
- **Backward compat verification:** The FixedContract/Simple contract test asserts that `conflictAnalysis` is `undefined`, confirming no regression in existing API shape.
- **Pre-existing test failure:** `deploy.service.spec.ts` has 1 pre-existing failure (userId 'anonymous' vs null) on `main` — unrelated to conflict analysis changes.

## Files Likely Touched

- `Vibe-Room-Backend/test/app.e2e-spec.ts`
- `Vibe-Loom/e2e/full-stack.spec.ts`
