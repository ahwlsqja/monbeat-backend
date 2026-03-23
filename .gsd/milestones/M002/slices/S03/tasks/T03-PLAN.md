---
estimated_steps: 5
estimated_files: 1
---

# T03: page.tsx 통합 배선 — ContractInteraction + VibeScoreDashboard 연결

**Slice:** S03 — 컨트랙트 인터랙션 + Vibe-Score 대시보드
**Milestone:** M002

## Description

Wire ContractInteraction (from T01) and VibeScoreDashboard (from T02) into `page.tsx`, replacing the old VibeScoreGauge and expanding the vibeScore state to store the full VibeScoreResult. This is the integration task that closes the S03 loop.

The current `page.tsx` already has all the state needed:
- `compileResult` (ABI, bytecode, contractName) — from compile flow
- `deployResult` (address) — from deploy flow
- `addEntry` from `useTransactionLog` — for logging call results
- `vibeScore` state — currently `{ score, suggestions }`, needs expansion to full `VibeScoreResult`

**Relevant skills:** react-best-practices

## Steps

1. **Replace VibeScoreGauge import with VibeScoreDashboard**:
   - Remove: `import { VibeScoreGauge } from "../components/VibeScoreGauge";`
   - Add: `import { VibeScoreDashboard } from "@/components/ide/VibeScoreDashboard";`
   - Add: `import { ContractInteraction } from "@/components/ide/ContractInteraction";`
   - Add: `import type { VibeScoreResult } from "@/lib/api-client";` (if not already imported)

2. **Expand vibeScore state to store full VibeScoreResult**:
   - Change: `const [vibeScore, setVibeScore] = useState<{ score: number; suggestions: string[] } | null>(null);`
   - To: `const [vibeScore, setVibeScore] = useState<VibeScoreResult | null>(null);`
   - Update `handleAnalyzeVibeScore` to store the full result:
     ```typescript
     .then((data) => {
       setVibeScore(data);  // store entire VibeScoreResult (was: { score, suggestions } extraction)
     })
     ```

3. **Add ContractInteraction to sidebarContent**:
   - After the deploy success section, add ContractInteraction conditionally:
     ```tsx
     {compileResult?.abi && deployResult?.address && (
       <div className="mb-3">
         <ContractInteraction
           abi={compileResult.abi}
           contractAddress={deployResult.address}
           onCallResult={addEntry}
         />
       </div>
     )}
     ```
   - This renders only after successful compile (ABI available) AND deploy (address available)

4. **Replace VibeScoreGauge with VibeScoreDashboard in sidebarContent**:
   - Replace the existing VibeScoreGauge block:
     ```tsx
     {(vibeScore || isVibeScoreLoading) && (
       <div className="mb-3">
         <VibeScoreDashboard
           score={vibeScore?.vibeScore ?? 0}
           suggestions={vibeScore?.suggestions ?? []}
           conflicts={vibeScore?.conflicts}
           reExecutions={vibeScore?.reExecutions}
           gasEfficiency={vibeScore?.gasEfficiency}
           engineBased={vibeScore?.engineBased}
           loading={isVibeScoreLoading}
         />
       </div>
     )}
     ```

5. **Verify full integration**:
   - `cd frontend && npm run build` — exits 0 (no type errors, no SSR issues)
   - `cd frontend && npm test` — all tests pass (existing + new from T01/T02)
   - Grep checks: ContractInteraction in page.tsx ✓, VibeScoreDashboard in page.tsx ✓, VibeScoreGauge NOT in page.tsx ✓

## Must-Haves

- [ ] VibeScoreGauge import removed from page.tsx
- [ ] VibeScoreDashboard imported and rendered with full props (score, suggestions, conflicts, reExecutions, gasEfficiency, engineBased, loading)
- [ ] ContractInteraction imported and rendered conditionally (when compileResult.abi AND deployResult.address exist)
- [ ] ContractInteraction.onCallResult wired to addEntry from useTransactionLog
- [ ] vibeScore state type expanded to VibeScoreResult
- [ ] handleAnalyzeVibeScore stores full VibeScoreResult (not just score+suggestions)
- [ ] `npm run build` succeeds
- [ ] All tests pass

## Verification

- `cd frontend && npm run build` — exits 0
- `cd frontend && npm test` — all tests pass
- `grep -q "ContractInteraction" frontend/src/app/page.tsx` — wired
- `grep -q "VibeScoreDashboard" frontend/src/app/page.tsx` — wired
- `! grep -q "VibeScoreGauge" frontend/src/app/page.tsx` — old component removed

## Inputs

- `frontend/src/app/page.tsx` — current page with compileResult, deployResult, vibeScore, addEntry state
- `frontend/src/components/ide/ContractInteraction.tsx` — T01 output: contract interaction component
- `frontend/src/components/ide/VibeScoreDashboard.tsx` — T02 output: dashboard component
- `frontend/src/lib/api-client.ts` — T02 output: expanded VibeScoreResult type

## Expected Output

- `frontend/src/app/page.tsx` — modified: ContractInteraction + VibeScoreDashboard wired in, VibeScoreGauge removed, vibeScore state expanded
