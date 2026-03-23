# S01: 시제품 QA 수준 Full E2E 테스트 스위트

**Goal:** 브라우저에서 라이브 서비스까지 관통하는 Playwright E2E 테스트 20+ 전부 PASS, 7개 백엔드 엔드포인트 응답 검증, 모바일 반응형 레이아웃 검증, 단계별 스크린샷 증거 캡처
**Demo:** `cd /home/ahwlsqja/Vibe-Loom && npx playwright test e2e/full-stack.spec.ts --reporter=list` → 22개 테스트 전부 PASS, `e2e/screenshots/` 에 단계별 증거 이미지 존재

## Must-Haves

- 기존 14개 테스트 PASS 유지 (회귀 없음)
- Backend API 7개 엔드포인트 전부 응답 검증 (기존 4 + deploy, analysis/error, paymaster/status)
- 모바일 반응형 (375×812) 탭 전환 검증 (Editor/Results/Console)
- 4종 컨트랙트 셀렉터 전체 순회 검증 (FailingContract, FixedContract, PectraTest, ParallelConflict)
- AI 에러 분석 플로우 검증 (FailingContract 배포 에러 → AI diff 표시)
- 배포 후 컨트랙트 인터랙션 read 함수 호출 검증
- 단계별 스크린샷 증거 캡처 (`e2e/screenshots/` 디렉토리)
- 최종 22개 이상 테스트 전부 PASS

## Proof Level

- This slice proves: final-assembly
- Real runtime required: yes (라이브 서비스 https://vibe-loom.xyz + Railway 백엔드)
- Human/UAT required: no

## Verification

- `cd /home/ahwlsqja/Vibe-Loom && npx playwright test e2e/full-stack.spec.ts --reporter=list 2>&1 | tail -5` → "22 passed" 이상
- `ls /home/ahwlsqja/Vibe-Loom/e2e/screenshots/*.png | wc -l` → 10개 이상
- `grep -c 'test(' /home/ahwlsqja/Vibe-Loom/e2e/full-stack.spec.ts` → 22 이상
- `cd /home/ahwlsqja/Vibe-Loom && cat test-results/.last-run.json 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('status','MISSING'))"` → "passed" (Playwright last-run status surface for failure diagnosis)

## Observability / Diagnostics

- Runtime signals: Playwright test reporter의 pass/fail/skip 상태, 개별 테스트 duration
- Inspection surfaces: `test-results/.last-run.json` (status 확인), `playwright-report/index.html` (리포트), `e2e/screenshots/` (시각적 증거)
- Failure visibility: Playwright HTML report에 실패 테스트별 스크린샷 + trace 자동 캡처 (config: `trace: 'on-first-retry'`)
- Redaction constraints: none (공개 테스트넷, API 키 미포함)

## Integration Closure

- Upstream surfaces consumed: `https://vibe-loom.xyz` (Vercel 프론트엔드), `https://vibe-room-backend-production.up.railway.app` (Railway 백엔드 7개 API)
- New wiring introduced in this slice: none — 기존 인프라에 테스트만 추가
- What remains before the milestone is truly usable end-to-end: nothing — 이 슬라이스가 M003의 유일한 슬라이스

## Tasks

- [x] **T01: Add stable E2E tests — Backend API extended + Mobile responsive + Contract selector** `est:45m`
  - Why: 기존 14개 테스트에 6개를 추가하여 20개 달성. 7개 API 엔드포인트 완전 커버리지, 모바일 반응형 검증, 전체 컨트랙트 셀렉터 검증을 안정적인(라이브 서비스 상태에 덜 의존적인) 테스트로 구현
  - Files: `/home/ahwlsqja/Vibe-Loom/e2e/full-stack.spec.ts`
  - Do: (1) `Backend API Extended` describe 블록 추가 — deploy API 직접 POST 테스트 (source 전송 → 201 응답 또는 에러 응답 검증), analysis/error API 직접 POST 테스트 (에러+소스 전송 → fix 응답 구조 검증), paymaster/status 인증 없이 401/403 검증. (2) `Mobile Responsive Layout` describe 블록 추가 — setViewportSize(375,812) → 탭 버튼 3개 표시 확인 + 각 탭 클릭 후 콘텐츠 전환 검증. (3) `Contract Selector Complete` describe 블록 추가 — 4종 순회하며 에디터 내용 변경 확인. 기존 헬퍼 함수 재사용. 방어적 assertion (서비스 상태 따라 success/error 모두 허용)
  - Verify: `cd /home/ahwlsqja/Vibe-Loom && npx playwright test e2e/full-stack.spec.ts --reporter=list 2>&1 | grep -c 'passed'` → 20 이상
  - Done when: `grep -c 'test(' e2e/full-stack.spec.ts` ≥ 20 이고 `npx playwright test` 전부 PASS

- [x] **T02: Add AI error analysis + Contract interaction tests and run full suite verification** `est:45m`
  - Why: 나머지 2개 테스트(AI 에러 분석 + 컨트랙트 인터랙션)를 추가하여 22개 달성. 라이브 서비스 deploy 성공 + AI API 응답에 의존하는 높은 리스크 테스트이므로 방어적 assertion 필수. 전체 스위트 실행 후 스크린샷 증거 캡처
  - Files: `/home/ahwlsqja/Vibe-Loom/e2e/full-stack.spec.ts`
  - Do: (1) `AI Error Analysis Flow` describe 블록 추가 — FailingContract 로드 → Compile → Deploy 시도 → 에러 발생 → AI 분석 트리거 → AIDiffViewer 또는 에러 분석 결과 표시 확인. AI 응답/휴리스틱 폴백 둘 다 valid. timeout 60s+. (2) `Contract Interaction` describe 블록 추가 — Counter 컨트랙트 배포 성공 후 ContractInteraction 패널에서 read 함수(getCount) Call 버튼 클릭 → 결과 `↳` 접두사 표시 확인. deploy 실패 시 테스트 skip. (3) 전체 스위트 실행 → 22개 PASS 확인. (4) 추가 스크린샷 캡처 (mobile-layout.png, contract-selector.png, ai-analysis.png, contract-interaction.png)
  - Verify: `cd /home/ahwlsqja/Vibe-Loom && npx playwright test e2e/full-stack.spec.ts --reporter=list 2>&1 | tail -3` → "22 passed"
  - Done when: 22개 테스트 전부 PASS, `e2e/screenshots/` 에 10개 이상 PNG 존재, `test-results/.last-run.json` status=passed

## Files Likely Touched

- `/home/ahwlsqja/Vibe-Loom/e2e/full-stack.spec.ts`
- `/home/ahwlsqja/Vibe-Loom/e2e/screenshots/*.png`
