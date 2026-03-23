# S03: 컨트랙트 인터랙션 + Vibe-Score 대시보드 — UAT

**Milestone:** M002
**Written:** 2026-03-23

## UAT Type

- UAT mode: mixed (artifact-driven verification + live-runtime visual checks)
- Why this mode is sufficient: Components are unit-tested for rendering correctness. Wallet interaction and blockchain calls are mocked in tests. Full live flow verification deferred to S04 (Vercel deployment + responsive polish).

## Preconditions

- `cd frontend && npm install --legacy-peer-deps` completed
- `cd frontend && npm run build` exits 0
- `cd frontend && npm test` — 57 tests pass
- For live visual checks: `cd frontend && npm run dev` running on localhost:3000
- MetaMask or WalletConnect-compatible wallet available (for write function testing in live mode)

## Smoke Test

Run `cd frontend && npm run build && npm test` — build succeeds with 0 errors and all 57 tests pass across 5 suites.

## Test Cases

### 1. ABI Parsing — Read/Write Function Separation

1. Open `frontend/src/__tests__/abi-utils.test.ts`
2. Run `npm test -- --testPathPatterns abi-utils`
3. **Expected:** 17/17 tests pass. `parseAbiFunctions` correctly separates view/pure functions into `readFunctions` and nonpayable/payable into `writeFunctions`. Payable functions have `payable: true` flag.

### 2. ABI Parsing — Edge Cases

1. Verify test cases: empty ABI returns empty arrays, events-only ABI returns empty function arrays, functions without `type` field are treated as functions (Solidity ABI convention).
2. **Expected:** All edge case tests pass — defensive parsing never throws.

### 3. ContractInteraction Component — File Structure

1. Verify `frontend/src/components/ide/ContractInteraction.tsx` exists
2. Grep for `ReadFunctionCard` and `WriteFunctionCard` sub-components
3. Grep for `publicClient.readContract` (viem read) and `useWriteContract` (wagmi write)
4. Grep for `onCallResult` callback
5. **Expected:** All patterns present — read functions use viem (no wallet), write functions use wagmi (wallet required), results flow to parent via callback.

### 4. VibeScoreDashboard — Score Rendering

1. Run `npm test -- --testPathPatterns VibeScoreDashboard`
2. **Expected:** 10/10 tests pass. Score value renders in the gauge. SVG circle element present. Score color classes: `text-emerald-400` for ≥80, `text-amber-400` for ≥60, `text-red-400` for <60.

### 5. VibeScoreDashboard — Stats Grid

1. In test suite, verify stats grid test: conflicts=5, reExecutions=3, gasEfficiency=92 all render as text
2. Verify undefined stats test: undefined values render as "—" (em dash), not "0" or empty
3. **Expected:** Both behaviors confirmed by passing tests.

### 6. VibeScoreDashboard — Suggestion Cards

1. Verify suggestions test: suggestion strings render as numbered cards
2. Check for `💡` emoji icon and amber left border styling in test expectations
3. **Expected:** Cards render with numbered prefixes and suggestion text.

### 7. VibeScoreDashboard — Loading State

1. Verify loading skeleton test: `loading={true}` renders "분석 중..." text and pulsing placeholders
2. **Expected:** Loading skeleton renders, score gauge does not render.

### 8. VibeScoreDashboard — Engine Badge

1. Verify `engineBased={true}` renders "Engine-Based" badge text
2. Verify `engineBased={false}` does not render badge
3. **Expected:** Both tests pass — badge is conditional.

### 9. Page Integration — ContractInteraction Wired

1. Run `grep -c "ContractInteraction" frontend/src/app/page.tsx`
2. Verify import statement and JSX usage both present
3. Verify conditional rendering: `compileResult?.abi && deployResult?.address`
4. **Expected:** ContractInteraction imported and conditionally rendered in sidebar.

### 10. Page Integration — VibeScoreDashboard Replaces VibeScoreGauge

1. Run `grep "VibeScoreDashboard" frontend/src/app/page.tsx`
2. Run `grep "VibeScoreGauge" frontend/src/app/page.tsx`
3. **Expected:** VibeScoreDashboard found (import + JSX). VibeScoreGauge not found anywhere.

### 11. Page Integration — Full Props Wiring

1. Inspect page.tsx for VibeScoreDashboard props: `score`, `suggestions`, `conflicts`, `reExecutions`, `gasEfficiency`, `engineBased`, `loading`
2. Verify `vibeScore` state type is `VibeScoreResult | null`
3. **Expected:** All 7 props passed from vibeScore state. Full VibeScoreResult stored, not individual fields.

### 12. API Type — VibeScoreResult Extension

1. Run `grep -A 5 "conflicts" frontend/src/lib/api-client.ts`
2. **Expected:** `conflicts?: number`, `reExecutions?: number`, `gasEfficiency?: number` present as optional fields in VibeScoreResult interface.

### 13. Build Integrity

1. Run `cd frontend && npm run build`
2. **Expected:** Exits 0, no TypeScript errors. Static pages generated (4/4). First load JS ~229 kB.

## Edge Cases

### Empty ABI

1. ContractInteraction receives `abi=[]` and a valid address
2. **Expected:** "No functions found" or empty read/write sections. No crash. No errors in console.

### Payable Function Without Value

1. WriteFunctionCard for a payable function renders optional MON value input
2. User calls without providing a value
3. **Expected:** Call proceeds with 0 value — the MON input is optional.

### All Stats Undefined

1. VibeScoreDashboard receives `score=75`, `suggestions=["test"]`, but no conflicts/reExecutions/gasEfficiency
2. **Expected:** Stats grid shows "—" for all three metrics. No crash. Dashboard still renders gauge and suggestions.

### Very High Score (100)

1. VibeScoreDashboard receives `score=100`
2. **Expected:** Full circle (strokeDashoffset ≈ 0), emerald color, "100" text displayed.

### Very Low Score (0)

1. VibeScoreDashboard receives `score=0`
2. **Expected:** Empty circle (strokeDashoffset = full circumference), red color, "0" text displayed.

## Failure Signals

- `npm run build` fails with TypeScript errors — broken type wiring
- `npm test` reports failures — component rendering or ABI parsing broken
- `grep -q "VibeScoreGauge" frontend/src/app/page.tsx` returns 0 — old component not fully removed
- `ContractInteraction` missing from page.tsx — sidebar wiring incomplete
- VibeScoreDashboard missing `conflicts`/`reExecutions`/`gasEfficiency` props — state not expanded
- React DevTools shows `vibeScore` state as `{ score, suggestions }` instead of full `VibeScoreResult` — handler not updated

## Not Proven By This UAT

- Live blockchain interaction (actual contract calls on Monad testnet) — requires deployed contract and connected wallet
- Responsive layout of ContractInteraction and VibeScoreDashboard — deferred to S04
- Dark mode styling polish — deferred to S04
- Production deployment — deferred to S04 (Vercel)
- TransactionConsole correctly displaying call entries at runtime — tested via hook unit tests, not full integration

## Notes for Tester

- The VibeScoreGauge component file still exists at `frontend/src/components/VibeScoreGauge.tsx` — this is expected dead code, it's no longer imported or used anywhere
- Pre-existing wagmi connector warnings during build (porto, coinbaseWallet, metaMask) are unrelated to S03 and can be ignored
- For live visual testing on localhost:3000, you need to: (1) connect wallet, (2) write and compile Solidity code, (3) deploy the contract — only then will ContractInteraction appear in the sidebar
- The `api-client.test.ts` console.error output about HTTP 500 during test runs is expected — it's testing the error handling path
