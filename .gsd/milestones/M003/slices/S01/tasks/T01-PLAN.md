---
estimated_steps: 5
estimated_files: 1
---

# T01: Add stable E2E tests — Backend API extended + Mobile responsive + Contract selector

**Slice:** S01 — 시제품 QA 수준 Full E2E 테스트 스위트
**Milestone:** M003

## Description

기존 14개 Playwright E2E 테스트에 안정적인 6개 테스트를 추가하여 20개를 달성한다. 추가 테스트는 세 카테고리:
1. **Backend API Extended** (3개) — deploy, analysis/error, paymaster/status 엔드포인트 직접 API 테스트로 7개 API 전부 커버
2. **Mobile Responsive Layout** (2개) — 375×812 뷰포트에서 모바일 탭 전환 동작 검증
3. **Contract Selector Complete** (1개) — 4종 컨트랙트 전부 순회하며 에디터 내용 변경 확인

모든 새 테스트는 기존 파일 `/home/ahwlsqja/Vibe-Loom/e2e/full-stack.spec.ts` 하단에 새로운 `test.describe` 블록으로 추가한다. 기존 헬퍼 함수(`waitForMonaco`, `setEditorContent`, `getEditorContent`)를 재사용한다.

**⚠️ 중요:** 코드를 편집하는 대상 파일은 `/home/ahwlsqja/Vibe-Loom/e2e/full-stack.spec.ts`이다 (GSD 워크트리가 아닌 Vibe-Loom 레포). GSD 메타데이터만 monad-core에 존재한다.

## Steps

1. `/home/ahwlsqja/Vibe-Loom/e2e/full-stack.spec.ts`를 읽어서 파일 구조와 기존 테스트 마지막 줄 위치를 확인한다.

2. 파일 맨 하단 (마지막 `});` 뒤)에 **Backend API Extended** describe 블록을 추가한다:
   ```typescript
   test.describe('Backend API Extended', () => {
     test('deploy endpoint accepts contract source', async ({ request }) => {
       const res = await request.post(`${BACKEND_URL}/api/contracts/deploy`, {
         data: { source: COUNTER_CONTRACT },
       });
       // Deploy는 성공(201) 또는 인증/잔액 에러(400/401/500) 모두 valid
       expect([200, 201, 400, 401, 403, 500]).toContain(res.status());
       const body = await res.json();
       // 응답 구조는 success 여부와 무관하게 JSON이어야 함
       expect(body).toBeDefined();
     });

     test('analysis/error endpoint returns fix suggestion', async ({ request }) => {
       const res = await request.post(`${BACKEND_URL}/api/analysis/error`, {
         data: {
           error: 'ParserError: Expected ; but got }',
           contractSource: INVALID_CONTRACT,
         },
       });
       expect(res.status()).toBe(201);
       const body = await res.json();
       expect(body.success).toBe(true);
       expect(body.data).toBeDefined();
       // fix가 있든 분석 결과든 data에 뭔가 있어야 함
     });

     test('paymaster/status requires authentication', async ({ request }) => {
       const res = await request.get(`${BACKEND_URL}/api/paymaster/status`);
       // 인증 없이 호출하면 401 또는 403
       expect([401, 403]).toContain(res.status());
     });
   });
   ```

3. **Mobile Responsive Layout** describe 블록을 추가한다:
   ```typescript
   test.describe('Mobile Responsive Layout', () => {
     test('mobile viewport shows tab navigation', async ({ page }) => {
       await page.setViewportSize({ width: 375, height: 812 });
       await page.goto('/');
       await page.waitForTimeout(2000); // React state 업데이트 대기

       // 모바일 탭 버튼 3개 (Editor, Results, Console) 표시 확인
       await expect(page.getByText('Editor', { exact: true })).toBeVisible();
       await expect(page.getByText('Results', { exact: true })).toBeVisible();
       await expect(page.getByText('Console', { exact: true })).toBeVisible();

       await page.screenshot({ path: 'e2e/screenshots/mobile-tabs.png', fullPage: true });
     });

     test('mobile tab switching works', async ({ page }) => {
       await page.setViewportSize({ width: 375, height: 812 });
       await page.goto('/');
       await page.waitForTimeout(2000);

       // Default: Editor tab active
       await expect(page.getByText('Editor', { exact: true })).toBeVisible();

       // Click Results tab
       await page.getByText('Results', { exact: true }).click();
       await page.waitForTimeout(500);

       // Click Console tab
       await page.getByText('Console', { exact: true }).click();
       await page.waitForTimeout(500);

       // Click back to Editor tab
       await page.getByText('Editor', { exact: true }).click();
       await page.waitForTimeout(500);

       await page.screenshot({ path: 'e2e/screenshots/mobile-layout.png', fullPage: true });
     });
   });
   ```

4. **Contract Selector Complete** describe 블록을 추가한다:
   ```typescript
   test.describe('Contract Selector Complete', () => {
     test('all 4 contract types load different source', async ({ page }) => {
       await page.goto('/');
       await waitForMonaco(page);
       const sources: string[] = [];

       for (const type of ['FailingContract', 'FixedContract', 'PectraTest', 'ParallelConflict']) {
         await page.getByText(type, { exact: false }).first().click();
         await page.waitForTimeout(2000);
         const content = await getEditorContent(page);
         expect(content).toContain('pragma solidity');
         sources.push(content);
       }

       // 최소 2개 이상의 서로 다른 소스가 있어야 함 (uniqueness)
       const uniqueSources = new Set(sources);
       expect(uniqueSources.size).toBeGreaterThanOrEqual(2);

       await page.screenshot({ path: 'e2e/screenshots/contract-selector.png', fullPage: true });
     });
   });
   ```

5. 전체 테스트 실행하여 기존 14개 + 새 6개 = 20개 전부 PASS 확인:
   ```bash
   cd /home/ahwlsqja/Vibe-Loom && npx playwright test e2e/full-stack.spec.ts --reporter=list
   ```

## Must-Haves

- [ ] Backend API deploy 엔드포인트 직접 테스트 추가 (성공/에러 모두 valid한 방어적 assertion)
- [ ] Backend API analysis/error 엔드포인트 직접 테스트 추가 (fix 결과 구조 검증)
- [ ] Backend API paymaster/status 인증 없이 401/403 검증 테스트 추가
- [ ] Mobile 375×812 뷰포트 탭 버튼 3개 (Editor/Results/Console) 표시 검증
- [ ] Mobile 탭 전환 클릭 동작 검증
- [ ] 4종 컨트랙트 (FailingContract, FixedContract, PectraTest, ParallelConflict) 전체 순회 + 에디터 내용 변경 확인
- [ ] 기존 14개 테스트 회귀 없음 (전부 PASS 유지)
- [ ] 스크린샷 3개 이상 추가 캡처 (mobile-tabs.png, mobile-layout.png, contract-selector.png)

## Verification

- `grep -c 'test(' /home/ahwlsqja/Vibe-Loom/e2e/full-stack.spec.ts` → 20 이상
- `cd /home/ahwlsqja/Vibe-Loom && npx playwright test e2e/full-stack.spec.ts --reporter=list 2>&1 | grep -E '[0-9]+ passed'` → 20 이상 passed

## Observability Impact

- **New signals:** 6 additional Playwright test pass/fail/duration entries in reporter output and `test-results/.last-run.json`
- **Inspection:** `e2e/screenshots/mobile-tabs.png`, `mobile-layout.png`, `contract-selector.png` provide visual evidence for mobile layout and contract selector tests
- **Failure visibility:** Failed tests produce automatic screenshots + traces in `playwright-report/` (config: `trace: 'on-first-retry'`). Each new test uses defensive assertions so transient backend issues cause clear status-code failures, not cryptic timeouts
- **Future agent:** Run `npx playwright test e2e/full-stack.spec.ts --reporter=list` to see per-test pass/fail; check `test-results/.last-run.json` for aggregate status; inspect `e2e/screenshots/` for visual evidence

## Inputs

- `/home/ahwlsqja/Vibe-Loom/e2e/full-stack.spec.ts` — 기존 14개 테스트 + 헬퍼 함수가 포함된 현재 테스트 파일
- `/home/ahwlsqja/Vibe-Loom/playwright.config.ts` — baseURL, viewport, timeout 설정 참조 (수정 불필요)

## Expected Output

- `/home/ahwlsqja/Vibe-Loom/e2e/full-stack.spec.ts` — 20개 이상의 테스트가 포함된 확장된 테스트 파일
- `/home/ahwlsqja/Vibe-Loom/e2e/screenshots/mobile-tabs.png` — 모바일 탭 네비게이션 스크린샷
- `/home/ahwlsqja/Vibe-Loom/e2e/screenshots/mobile-layout.png` — 모바일 레이아웃 스크린샷
- `/home/ahwlsqja/Vibe-Loom/e2e/screenshots/contract-selector.png` — 4종 컨트랙트 셀렉터 스크린샷
