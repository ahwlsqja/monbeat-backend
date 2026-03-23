# S01: 시제품 QA 수준 Full E2E 테스트 스위트 — UAT

**Milestone:** M003
**Written:** 2026-03-23

## UAT Type

- UAT mode: mixed (artifact-driven + live-runtime)
- Why this mode is sufficient: Test suite runs against live Vercel frontend and Railway backend — artifacts (test results, screenshots) prove live-runtime behavior. No human-experience testing required since all flows are automated.

## Preconditions

1. Vibe-Loom frontend deployed and accessible at `https://vibe-loom.xyz`
2. Vibe-Room Backend running at `https://vibe-room-backend-production.up.railway.app`
3. Playwright installed: `cd /home/ahwlsqja/Vibe-Loom && npx playwright install chromium`
4. Node.js available (v18+)
5. No VPN or firewall blocking outbound HTTPS to the above domains

## Smoke Test

```bash
cd /home/ahwlsqja/Vibe-Loom && npx playwright test e2e/full-stack.spec.ts --grep "should load the IDE page" --reporter=list
```
Expected: 1 passed in <30s. If this fails, check that `https://vibe-loom.xyz` is accessible.

## Test Cases

### 1. Full Suite Execution — 22 Tests Pass

1. `cd /home/ahwlsqja/Vibe-Loom`
2. `npx playwright test e2e/full-stack.spec.ts --reporter=list`
3. Wait for completion (~2-4 minutes depending on testnet)
4. **Expected:** Exit code 0. Output shows "22 tests" with 20+ passed, 0 failed. 1-2 may be skipped (Contract Interaction if testnet slow) or flaky (compile markers, passes on retry). Playwright overall status: "passed".

### 2. Test Count Verification

1. `grep -c 'test(' /home/ahwlsqja/Vibe-Loom/e2e/full-stack.spec.ts`
2. **Expected:** 22

### 3. Screenshot Evidence Files

1. `ls -la /home/ahwlsqja/Vibe-Loom/e2e/screenshots/*.png`
2. **Expected:** 10 PNG files present:
   - `01-page-loaded.png` — IDE initial load
   - `02-contract-loaded.png` — Contract source loaded in editor
   - `03-compiled.png` — After compilation
   - `04-vibe-score.png` — Vibe score displayed
   - `05-final-state.png` — Final IDE state
   - `deploy-result.png` — Deploy attempt result
   - `mobile-tabs.png` — Mobile viewport tab navigation
   - `mobile-layout.png` — Mobile tab switching result
   - `contract-selector.png` — 4-contract cycling
   - `ai-analysis.png` — AI error analysis flow
3. **Expected:** All files non-empty (>10KB each)

### 4. Last-Run Status Artifact

1. `cat /home/ahwlsqja/Vibe-Loom/test-results/.last-run.json`
2. **Expected:** `{"status":"passed"}`

### 5. Backend API Endpoints — Direct Verification

1. `curl -s -o /dev/null -w '%{http_code}' https://vibe-room-backend-production.up.railway.app/api/contract-source`
2. **Expected:** 200
3. `curl -s -o /dev/null -w '%{http_code}' -X POST https://vibe-room-backend-production.up.railway.app/api/compile -H 'Content-Type: application/json' -d '{"source":"pragma solidity ^0.8.0; contract T{}"}'`
4. **Expected:** 200 or 201
5. `curl -s -o /dev/null -w '%{http_code}' https://vibe-room-backend-production.up.railway.app/api/paymaster/status`
6. **Expected:** 401 or 403 (auth required)
7. `curl -s -o /dev/null -w '%{http_code}' -X POST https://vibe-room-backend-production.up.railway.app/api/analysis/error -H 'Content-Type: application/json' -d '{"source":"contract T{}","error":{"message":"revert","severity":"error"}}'`
8. **Expected:** 200 or 201 (returns fix suggestion structure)

### 6. Mobile Responsive — Manual Spot Check

1. Open `https://vibe-loom.xyz` in browser DevTools mobile emulation (375×812)
2. Verify Editor, Results, Console tab buttons are visible
3. Click each tab — content area should switch
4. **Expected:** All three tabs navigable, no layout overflow or broken elements

### 7. Contract Selector — All 4 Types Load

1. Open `https://vibe-loom.xyz`
2. Click contract selector dropdown
3. Select each: FailingContract, FixedContract, PectraTest, ParallelConflict
4. **Expected:** Each selection loads different Solidity source with `pragma solidity` visible. At least 2 contracts have unique code.

### 8. AI Error Analysis Flow

1. Open `https://vibe-loom.xyz`
2. Select "FailingContract" from selector
3. Click Compile, then Deploy
4. Wait up to 60s
5. **Expected:** Either AI analysis panel appears with fix suggestions, or error message is displayed, or deploy succeeds. Any of these three outcomes is valid.

## Edge Cases

### Testnet Timeout During Deploy

1. Run full suite when Monad testnet is slow (>60s per tx)
2. **Expected:** Contract Interaction test calls `test.skip()` gracefully. Suite still reports "passed" overall. No hard failures.

### Flaky Compile Error Markers

1. Run test "compile error shows inline markers" 5 times in a row
2. **Expected:** May fail on first attempt occasionally (Monaco marker timing), but should pass on retry (retries=1 configured). Never fails twice in a row.

### Backend Temporarily Down

1. Stop Railway backend, run API tests
2. **Expected:** Backend API tests fail with connection errors. Frontend-only tests still pass. Failures are clearly attributed to backend unavailability in the report.

## Failure Signals

- `test-results/.last-run.json` shows `{"status":"failed"}` — suite regression
- Any screenshot missing from `e2e/screenshots/` — test didn't reach that stage
- `npx playwright test` exit code non-zero — at least one test hard-failed (not skipped)
- HTML report (`npx playwright show-report`) shows red test entries with stack traces

## Not Proven By This UAT

- **GitHub OAuth login flow** — no E2E test exercises OAuth redirect + callback
- **WalletConnect integration** — 3-free-deploy → wallet connection flow not tested
- **Deploy with sufficient MON balance** — Contract Interaction test skips when testnet is slow; actual deploy success depends on wallet balance and testnet state
- **Concurrent user behavior** — tests run single-user; no load/stress testing
- **Cross-browser compatibility** — tests run Chromium only (Playwright default)

## Notes for Tester

- The suite takes 2-4 minutes depending on Monad testnet responsiveness. Deploy-dependent tests may be slower.
- 1 skipped test (Contract Interaction) is expected when testnet is slow — this is by-design, not a failure.
- 1 flaky test (compile error markers) may need a retry — configured with `retries: 1` in Playwright config.
- Screenshot files are overwritten on each run. If comparing across runs, copy them first.
- The `analysis/error` API requires `error` as `{message, severity}` object, NOT a string. If modifying API tests, use this format.
- To view the full interactive report: `cd /home/ahwlsqja/Vibe-Loom && npx playwright show-report`
