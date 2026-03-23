# S03: 컨트랙트 인터랙션 + Vibe-Score 대시보드

**Goal:** 배포된 컨트랙트의 ABI 함수를 사이드바에서 호출 가능하고, Vibe-Score가 시각적 대시보드(게이지 + 통계 + 제안 카드)로 표시된다.
**Demo:** 컴파일 → 배포 후 ContractInteraction UI에서 read/write 함수 폼이 렌더링되고, Vibe Score 클릭 시 VibeScoreDashboard에 원형 게이지 + 충돌/재실행 통계 + 제안 카드가 표시된다.

## Must-Haves

- ABI 파싱: view/pure 함수와 nonpayable/payable 함수를 분리하여 read/write 섹션으로 렌더링
- ContractInteraction: read 호출은 viem publicClient (지갑 불필요), write 호출은 wagmi useWriteContract (지갑 필수)
- 함수 호출 결과가 TransactionConsole에 `type: "call"` 엔트리로 기록
- VibeScoreResult 타입에 conflicts, reExecutions, gasEfficiency 필드 추가
- VibeScoreDashboard: 원형 게이지 + 통계 그리드 + 제안 카드 3단 구성
- page.tsx에서 VibeScoreGauge를 VibeScoreDashboard로 교체
- ContractInteraction이 compileResult + deployResult 조건부로 사이드바에 렌더링
- `npm run build` 성공, 기존 30개 테스트 + 새 테스트 모두 통과

## Proof Level

- This slice proves: integration
- Real runtime required: no (wallet interaction tested via mocked hooks, UI tested via unit tests)
- Human/UAT required: yes (full flow visual verification in S04)

## Verification

- `cd frontend && npm run build` — exits 0, no type errors
- `cd frontend && npm test` — all tests pass (existing 30 + new ABI parser tests + VibeScoreDashboard tests)
- `grep -q "ContractInteraction" frontend/src/app/page.tsx` — component wired
- `grep -q "VibeScoreDashboard" frontend/src/app/page.tsx` — component wired
- `! grep -q "VibeScoreGauge" frontend/src/app/page.tsx` — old gauge removed from page
- `grep -q "conflicts" frontend/src/lib/api-client.ts` — type expanded
- `test -f frontend/src/components/ide/ContractInteraction.tsx`
- `test -f frontend/src/components/ide/VibeScoreDashboard.tsx`
- `test -f frontend/src/lib/abi-utils.ts`
- `test -f frontend/src/__tests__/abi-utils.test.ts`
- `test -f frontend/src/__tests__/VibeScoreDashboard.test.tsx`

## Observability / Diagnostics

- Runtime signals: Contract call results logged to TransactionConsole via `addEntry({ type: "call", ... })` — success shows return value, error shows revert reason
- Inspection surfaces: React DevTools → `entries` state shows call history; browser console → `monacoInstance.editor.getModelMarkers({owner:'solc'})` for compile errors
- Failure visibility: Write calls show wagmi error messages (user rejection, gas estimation failure, revert) in both ContractInteraction result area and TransactionConsole
- Redaction constraints: none (testnet only, no secrets)

## Integration Closure

- Upstream surfaces consumed: `compileResult` (ABI, bytecode, contractName) from S02 compile flow, `deployResult` (address) from S02 deploy flow, `useTransactionLog.addEntry()` from S02, `monadTestnet` chain from `wagmi-config.ts`, wagmi hooks from providers.tsx WagmiProvider
- New wiring introduced in this slice: ContractInteraction + VibeScoreDashboard composed into page.tsx sidebarContent, `handleAnalyzeVibeScore` expanded to store full VibeScoreResult
- What remains before the milestone is truly usable end-to-end: S04 — responsive layout, dark mode polish, Vercel deployment

## Tasks

- [x] **T01: ABI 유틸리티 + ContractInteraction 컴포넌트 구현** `est:40m`
  - Why: 배포된 컨트랙트의 함수를 호출하는 UI가 S03의 핵심 기능. ABI 파싱 → 폼 생성 → viem/wagmi 호출 파이프라인 구현
  - Files: `frontend/src/lib/abi-utils.ts`, `frontend/src/components/ide/ContractInteraction.tsx`, `frontend/src/__tests__/abi-utils.test.ts`
  - Do: (1) abi-utils.ts에 ABI 엔트리 파싱(view/pure vs nonpayable/payable 분리), Solidity 타입→HTML input 매핑, 입력값 파싱 유틸 구현. (2) ContractInteraction.tsx에 read/write 섹션, 각 함수별 입력 폼 + 호출 버튼 + 결과 표시 구현. read는 viem publicClient.readContract, write는 wagmi useWriteContract 사용. (3) abi-utils.test.ts에 파싱/타입매핑/입력변환 테스트 12+개 작성
  - Verify: `cd frontend && npm test -- --testPathPatterns abi-utils` passes, `test -f frontend/src/components/ide/ContractInteraction.tsx`
  - Done when: ContractInteraction이 ABI 배열과 주소를 props로 받아 read/write 함수 폼을 렌더링하고, onCallResult 콜백으로 결과를 전달

- [x] **T02: VibeScoreResult 타입 확장 + VibeScoreDashboard 컴포넌트 구현** `est:25m`
  - Why: 기존 VibeScoreGauge를 대체하는 풍부한 대시보드 — 충돌/재실행 통계 + 제안 카드 추가
  - Files: `frontend/src/lib/api-client.ts`, `frontend/src/components/ide/VibeScoreDashboard.tsx`, `frontend/src/__tests__/VibeScoreDashboard.test.tsx`
  - Do: (1) api-client.ts의 VibeScoreResult에 conflicts, reExecutions, gasEfficiency (all optional number) 추가. (2) VibeScoreDashboard.tsx에 원형 게이지 + 3-stat 그리드(conflicts, reExecutions, gasEfficiency) + 제안 카드 목록 구현. loading 스켈레톤 포함. (3) 렌더링 테스트 5+개 작성
  - Verify: `cd frontend && npm test -- --testPathPatterns VibeScoreDashboard` passes, `test -f frontend/src/components/ide/VibeScoreDashboard.tsx`
  - Done when: VibeScoreDashboard가 score/suggestions/conflicts/reExecutions/gasEfficiency/loading props를 받아 3단 대시보드를 렌더링

- [x] **T03: page.tsx 통합 배선 — ContractInteraction + VibeScoreDashboard 연결** `est:20m`
  - Why: T01/T02에서 만든 컴포넌트를 실제 앱에 연결하여 전체 플로우 완성
  - Files: `frontend/src/app/page.tsx`
  - Do: (1) VibeScoreGauge import를 VibeScoreDashboard로 교체. (2) vibeScore 상태를 전체 VibeScoreResult로 확장하여 conflicts/reExecutions/gasEfficiency 저장. (3) handleAnalyzeVibeScore에서 full result 저장. (4) sidebarContent에 ContractInteraction 추가 (compileResult?.abi && deployResult?.address 조건부 렌더링). (5) ContractInteraction의 onCallResult에 addEntry 연결. (6) VibeScoreDashboard에 확장된 props 전달
  - Verify: `cd frontend && npm run build` exits 0, `cd frontend && npm test` all pass, `grep -q "ContractInteraction" frontend/src/app/page.tsx && grep -q "VibeScoreDashboard" frontend/src/app/page.tsx && ! grep -q "VibeScoreGauge" frontend/src/app/page.tsx`
  - Done when: 빌드 성공, 모든 테스트 통과, page.tsx에 ContractInteraction과 VibeScoreDashboard가 연결되고 VibeScoreGauge 참조 제거

## Files Likely Touched

- `frontend/src/lib/abi-utils.ts` — new: ABI 파싱 유틸리티
- `frontend/src/components/ide/ContractInteraction.tsx` — new: ABI 기반 함수 호출 UI
- `frontend/src/components/ide/VibeScoreDashboard.tsx` — new: 풍부한 Vibe-Score 대시보드
- `frontend/src/lib/api-client.ts` — modified: VibeScoreResult 타입 확장
- `frontend/src/app/page.tsx` — modified: 두 컴포넌트 통합 + 상태 확장
- `frontend/src/__tests__/abi-utils.test.ts` — new: ABI 유틸리티 테스트
- `frontend/src/__tests__/VibeScoreDashboard.test.tsx` — new: 대시보드 렌더링 테스트
