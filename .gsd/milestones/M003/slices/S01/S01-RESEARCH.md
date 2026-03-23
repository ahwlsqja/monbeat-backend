# S01 — Research: 시제품 QA 수준 Full E2E 테스트 스위트

**Date:** 2026-03-23

## Summary

현재 `Vibe-Loom/e2e/full-stack.spec.ts`에 14개 Playwright E2E 테스트가 존재하며, 마지막 실행 결과 전부 PASS 상태다. M003 목표는 20+ 테스트이므로 **최소 6개 이상 추가**해야 한다. 기존 테스트는 Backend API 4개, Frontend IDE 4개, Compile 3개, Vibe-Score 1개, Deploy 1개, Full E2E Flow 1개로 구성되어 있다.

M003-CONTEXT에서 요구하는 미검증 플로우는 다음과 같다:
1. **모바일 반응형 레이아웃** — 768px 이하에서 탭 전환 동작 (Editor/Results/Console 탭)
2. **AI 에러 분석 플로우** — FailingContract 배포 실패 → AI 수정 제안 diff 표시
3. **배포된 컨트랙트 ABI 인터랙션** — read 함수 호출 → 결과값 표시
4. **백엔드 API 누락 엔드포인트** — `/api/contracts/deploy` 직접 API 테스트, `/api/analysis/error` 직접 API 테스트
5. **4종 컨트랙트 전환 후 에디터 검증** — FailingContract, FixedContract, PectraTest, ParallelConflict 각각 로드 확인

인프라는 완비되어 있다: Playwright 1.58.2, Chromium headless, 라이브 서비스 URL 모두 사용 가능. 기존 헬퍼 함수(`waitForMonaco`, `setEditorContent`, `getEditorContent`, `getEditorMarkers`)를 재사용하면 새 테스트 추가가 직관적이다.

## Recommendation

**단일 파일 확장 방식**: 기존 `e2e/full-stack.spec.ts`에 새로운 `test.describe` 블록을 추가하여 20+ 테스트를 달성한다. 파일 분리보다 단일 파일이 스크린샷 증거 관리와 실행 순서 제어에 유리하다.

추가할 테스트 그룹:
1. **Mobile Responsive** (2개) — viewport 768px 설정 후 탭 전환, 에디터/사이드바/콘솔 전환 확인
2. **AI Error Analysis** (2개) — FailingContract 배포 에러 → AI diff 표시, diff Apply Fix 동작
3. **Contract Interaction** (1개) — 배포 성공 후 read 함수 호출 결과 확인
4. **Backend API Extended** (2개) — deploy 엔드포인트 API 직접 테스트, analysis/error 엔드포인트 직접 테스트
5. **Contract Selector Complete** (1개) — 4종 컨트랙트 모두 순회 전환 후 에디터 내용 변경 확인

이렇게 하면 14 + 8 = **22개 테스트**로 20+ 목표를 달성한다.

## Implementation Landscape

### Key Files

- `/home/ahwlsqja/Vibe-Loom/e2e/full-stack.spec.ts` — **유일한 E2E 테스트 파일**. 14개 테스트 + 헬퍼 함수 4개 포함. 새 테스트를 이 파일 하단에 `test.describe` 블록으로 추가
- `/home/ahwlsqja/Vibe-Loom/playwright.config.ts` — Playwright 설정. `baseURL: 'https://vibe-loom.xyz'`, timeout 120s, screenshot 'on', viewport 1440×900. 변경 불필요
- `/home/ahwlsqja/Vibe-Loom/src/components/ide/IDELayout.tsx` — 모바일 탭 전환 로직. 768px 이하에서 Editor/Results/Console 탭 렌더. 테스트에서 `page.setViewportSize({width: 375, height: 812})` 후 탭 버튼 클릭으로 검증
- `/home/ahwlsqja/Vibe-Loom/src/app/page.tsx` — 메인 페이지. `CONTRACT_OPTIONS` 4종, 에러 시 `analyzeError()` 호출 → `errorDiff` state → `AIDiffViewer` 렌더 확인 포인트
- `/home/ahwlsqja/Vibe-Loom/src/components/ide/ContractInteraction.tsx` — ABI 기반 read/write 함수 카드. 배포 후 `ReadFunctionCard`의 Call 버튼 → 결과값 `↳` 접두사로 표시
- `/home/ahwlsqja/Vibe-Loom/src/components/ide/VibeScoreDashboard.tsx` — SVG circle gauge + Engine-Based 뱃지 + suggestions 카드 렌더. 기존 테스트에서 부분 검증됨

### Backend API Endpoints (7개 — 전부 검증 대상)

| # | Endpoint | Method | 기존 테스트 | 필요 여부 |
|---|----------|--------|-------------|-----------|
| 1 | `/api/health` | GET | ✅ health check test | 완료 |
| 2 | `/api/contracts/compile` | POST | ✅ compile endpoint test | 완료 |
| 3 | `/api/contracts/source?type=X` | GET | ✅ contract source test | 완료 |
| 4 | `/api/vibe-score` | POST | ✅ vibe-score endpoint test | 완료 |
| 5 | `/api/contracts/deploy` | POST | ❌ 프론트엔드 통해서만 | **추가 필요** |
| 6 | `/api/analysis/error` | POST | ❌ 없음 | **추가 필요** |
| 7 | `/api/paymaster/status` | GET | ❌ JWT 필요 | 선택적 (인증 없이 401 확인 가능) |

### Build Order

1. **Backend API 확장 테스트** (T01) — deploy API 직접 호출 + analysis/error API 직접 호출. 네트워크 의존만 있고 브라우저 UI 불필요 → 가장 빠르고 안전하게 검증 가능
2. **모바일 반응형 테스트** (T02) — `page.setViewportSize()` → 탭 버튼 visibility + 클릭 검증. 독립적이고 실패 가능성 낮음
3. **컨트랙트 셀렉터 전체 순회** (T03) — 4종 컨트랙트 전환 → 에디터 내용 변경 확인. 기존 패턴 재활용
4. **AI 에러 분석 플로우** (T04) — FailingContract 배포 → 에러 → AI 분석 diff 표시. AI API 의존으로 가장 불안정할 수 있음
5. **컨트랙트 인터랙션** (T05) — 배포 성공 후 read 함수 호출. Deploy 성공이 전제조건 → 가장 마지막
6. **전체 실행 + 스크린샷 증거** (T06) — 모든 테스트 통과 확인 + 단계별 스크린샷 캡처

### Verification Approach

```bash
# Vibe-Loom 디렉토리에서 실행
cd /home/ahwlsqja/Vibe-Loom
npx playwright test e2e/full-stack.spec.ts --reporter=list

# 성공 조건:
# 1. 20개 이상 테스트 PASS
# 2. e2e/screenshots/ 에 단계별 스크린샷 존재
# 3. test-results/.last-run.json의 "status": "passed"
```

## Constraints

- **라이브 서비스 의존**: 테스트가 `https://vibe-loom.xyz` (Vercel)와 `https://vibe-room-backend-production.up.railway.app` (Railway)에 직접 요청. 서비스 다운 시 전부 실패
- **Deploy 테스트**: 서버 페이마스터 지갑에 MON이 필요. 잔액 부족 시 deploy 테스트 실패 가능 — 테스트에서 success/error 양쪽 모두 허용하는 방어적 assertion 사용 중
- **AI Analysis**: Gemini API 키 설정 여부에 따라 AI 응답 또는 휴리스틱 폴백. 둘 다 valid한 응답이므로 테스트는 응답 존재 여부만 확인
- **모나드 테스트넷**: RPC 불안정 가능성 → deploy/interaction 테스트에 넉넉한 timeout 필요 (60s+)
- **3개 레포 분리 규칙**: E2E 테스트 코드는 **Vibe-Loom 레포**(`/home/ahwlsqja/Vibe-Loom/e2e/`)에만 존재. monad-core에는 GSD 메타데이터만

## Common Pitfalls

- **Monaco 에디터 로딩 지연** — CDN에서 Monaco 로드되므로 `waitForMonaco()` 헬퍼 반드시 사용. 직접 `.monaco-editor` 셀렉터 기다리지 않으면 flaky
- **setEditorContent 실패** — Monaco의 `getEditors()` API가 아직 준비 안 됐을 때 빈 배열 반환. `waitForMonaco` 이후에도 500ms 대기 후 사용하는 패턴이 기존 코드에서 확인됨
- **모바일 뷰포트 설정 타이밍** — `page.setViewportSize()` 호출 후 React state 업데이트(`useIsMobile` hook의 `matchMedia` listener) 까지 시간 필요. 1초 대기 또는 탭 버튼 visible 대기 필요
- **Deploy 결과 분기** — 성공(0x 주소)/실패(에러 메시지)/타임아웃 세 경우 모두 처리해야 함. 기존 `Promise.race` 패턴 참고
- **스크린샷 경로** — `e2e/screenshots/` 디렉토리에 저장. `fullPage: true` 옵션으로 전체 페이지 캡처

## Open Risks

- **서버 페이마스터 MON 잔액 부족**: Deploy 테스트와 Contract Interaction 테스트가 실패할 수 있음. 방어적 assertion으로 대응하되, 실패 시에도 나머지 테스트는 독립적으로 통과해야 함
- **AI 에러 분석 응답 시간**: Gemini API 레이턴시가 높을 수 있음 (10-30초). timeout 충분히 설정 필요
- **FailingContract 배포 에러 유형 변경**: 백엔드가 업데이트되면 에러 메시지/형태가 바뀔 수 있음. assertion은 특정 메시지가 아닌 에러 존재 여부로
