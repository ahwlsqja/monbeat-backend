---
id: S03
parent: M002
milestone: M002
provides:
  - ABI parsing utility (parseAbiFunctions, solidityTypeToInputType, parseInputValue, formatOutputValue)
  - ContractInteraction component — ABI-driven read/write function call UI (viem for reads, wagmi for writes)
  - VibeScoreDashboard component — SVG circular gauge + 3-stat grid (conflicts, reExecutions, gasEfficiency) + suggestion cards
  - Full VibeScoreResult state wiring in page.tsx with conflicts/reExecutions/gasEfficiency fields
  - TransactionConsole integration for contract call results (type: "call" entries)
requires:
  - slice: S02
    provides: compileResult (ABI, bytecode, contractName), deployResult (address), useTransactionLog.addEntry(), TransactionConsole component, MonacoEditor wrapper
affects:
  - S04
key_files:
  - frontend/src/lib/abi-utils.ts
  - frontend/src/components/ide/ContractInteraction.tsx
  - frontend/src/components/ide/VibeScoreDashboard.tsx
  - frontend/src/lib/api-client.ts
  - frontend/src/app/page.tsx
  - frontend/src/__tests__/abi-utils.test.ts
  - frontend/src/__tests__/VibeScoreDashboard.test.tsx
key_decisions:
  - Text input (not number) for uint/int Solidity types to support BigInt-scale values beyond Number.MAX_SAFE_INTEGER
  - ReadFunctionCard and WriteFunctionCard as separate sub-components for clean viem vs wagmi logic separation
  - SVG circle gauge with strokeDasharray/offset for score visualization — no chart library dependency
  - Emoji icons (💥🔄⛽💡) for stat labels instead of icon library to avoid bundle bloat
  - Store full VibeScoreResult in state rather than extracting fields — avoids double-mapping and keeps props in sync with backend schema
  - ContractInteraction placed between deploy-success block and error block in sidebar for natural top-down flow
patterns_established:
  - Defensive ABI parsing with try/catch fallback to empty arrays
  - onCallResult callback pattern emitting TransactionLogEntry-compatible objects for every call (success or error)
  - Optional stat props with undefined→"—" fallback for graceful partial data rendering
  - Score→color mapping (≥80 emerald, ≥60 amber, <60 red) reused across gauge components
observability_surfaces:
  - onCallResult fires TransactionLogEntry-shaped objects for every contract call — inspect TransactionConsole entries via React DevTools
  - console.error for ABI parse failures in ContractInteraction
  - UI-level error display (red box) for both read and write call failures
  - VibeScoreDashboard is pure presentational — inspect props via React DevTools
  - SVG gauge strokeDashoffset directly reflects score value (circumference × (1 - score/100))
drill_down_paths:
  - .gsd/milestones/M002/slices/S03/tasks/T01-SUMMARY.md
  - .gsd/milestones/M002/slices/S03/tasks/T02-SUMMARY.md
  - .gsd/milestones/M002/slices/S03/tasks/T03-SUMMARY.md
duration: 35m
verification_result: passed
completed_at: 2026-03-23
---

# S03: 컨트랙트 인터랙션 + Vibe-Score 대시보드

**ABI-driven contract interaction UI (read via viem, write via wagmi) and rich Vibe-Score dashboard (SVG gauge + stats grid + suggestion cards) integrated into IDE sidebar — completing the compile→deploy→interact→analyze flow**

## What Happened

Three tasks built the two core components and wired them into the IDE:

**T01 — ABI Utilities + ContractInteraction** created the contract interaction subsystem. `abi-utils.ts` provides four functions: `parseAbiFunctions` separates view/pure (read) from nonpayable/payable (write) functions; `solidityTypeToInputType` maps Solidity types to HTML input types; `parseInputValue` converts form strings to typed values (BigInt for ints, boolean for bool); `formatOutputValue` serializes return values including BigInt via custom replacer. `ContractInteraction.tsx` uses `ReadFunctionCard` (viem `publicClient.readContract`, no wallet needed) and `WriteFunctionCard` (wagmi `useWriteContract`, wallet guard) sub-components. Handles no-input functions, payable functions with optional MON value input, and defensive ABI parse errors. All results flow through `onCallResult` callback for TransactionConsole integration. 17 unit tests verify the ABI utility layer.

**T02 — VibeScoreResult Type Extension + VibeScoreDashboard** expanded the `VibeScoreResult` interface with three optional fields (`conflicts`, `reExecutions`, `gasEfficiency`) — backward compatible since existing code only reads `vibeScore` and `suggestions`. The dashboard component renders three sections: an SVG circular gauge (120×120 viewBox, color-coded by score tier), a 3-column stats grid with emoji icons and undefined→"—" fallback, and numbered suggestion cards with amber accent borders. Loading state shows a pulsing skeleton. An "Engine-Based" badge appears when `engineBased=true`. 10 render tests cover all display states.

**T03 — Page Integration** made four surgical edits to `page.tsx`: replaced `VibeScoreGauge` import with `VibeScoreDashboard`, changed `vibeScore` state to store the full `VibeScoreResult` object, simplified `handleAnalyzeVibeScore` to `setVibeScore(data)`, and added `ContractInteraction` conditionally rendered when `compileResult?.abi && deployResult?.address` exist with `onCallResult={addEntry}` wiring.

## Verification

| # | Check | Result |
|---|-------|--------|
| 1 | `npm run build` exits 0 | ✅ pass (229 kB first load) |
| 2 | `npm test` — 57 tests, 5 suites | ✅ all pass |
| 3 | `ContractInteraction` in page.tsx | ✅ present |
| 4 | `VibeScoreDashboard` in page.tsx | ✅ present |
| 5 | `VibeScoreGauge` removed from page.tsx | ✅ absent |
| 6 | `conflicts` in api-client.ts | ✅ present |
| 7 | `ContractInteraction.tsx` exists | ✅ |
| 8 | `VibeScoreDashboard.tsx` exists | ✅ |
| 9 | `abi-utils.ts` exists | ✅ |
| 10 | `abi-utils.test.ts` exists | ✅ |
| 11 | `VibeScoreDashboard.test.tsx` exists | ✅ |

## New Requirements Surfaced

- none

## Deviations

- T01 produced 17 tests instead of planned 12+ (extra tests for payable flag and null formatting)
- T02 produced 10 tests instead of planned 5+ (added badge absence test and 3 color-tier tests)
- Added `bigIntReplacer` helper in abi-utils.ts for safe JSON.stringify of BigInt values — unplanned but necessary

## Known Limitations

- `VibeScoreGauge` component file (`frontend/src/components/VibeScoreGauge.tsx`) still exists on disk but is now dead code — should be cleaned up
- Write function calls require a connected wallet — no mock wallet for development testing
- Pre-existing wagmi connector warnings during build (porto, coinbaseWallet, metaMask modules) — unrelated to S03

## Follow-ups

- Delete `frontend/src/components/VibeScoreGauge.tsx` (dead code after VibeScoreDashboard replacement)
- S04 should verify ContractInteraction and VibeScoreDashboard render correctly in responsive layout (mobile tab switching)

## Files Created/Modified

- `frontend/src/lib/abi-utils.ts` — new: ABI parsing utility with type mappings and value conversion (4 exported functions)
- `frontend/src/__tests__/abi-utils.test.ts` — new: 17 unit tests for ABI utilities
- `frontend/src/components/ide/ContractInteraction.tsx` — new: ABI-based contract interaction UI with read (viem) / write (wagmi) sections
- `frontend/src/components/ide/VibeScoreDashboard.tsx` — new: rich Vibe-Score dashboard with SVG gauge + stats grid + suggestion cards
- `frontend/src/__tests__/VibeScoreDashboard.test.tsx` — new: 10 render tests for VibeScoreDashboard
- `frontend/src/lib/api-client.ts` — modified: added `conflicts?`, `reExecutions?`, `gasEfficiency?` to VibeScoreResult interface
- `frontend/src/app/page.tsx` — modified: replaced VibeScoreGauge with VibeScoreDashboard, added ContractInteraction, expanded vibeScore state

## Forward Intelligence

### What the next slice should know
- The IDE's full flow (editor→compile→deploy→interact→analyze) is now feature-complete in terms of components. S04 needs to focus on responsive layout, dark mode polish, and Vercel deployment — no new functional components needed.
- `VibeScoreGauge.tsx` is dead code — safe to delete in S04 cleanup.
- The page.tsx sidebar renders in this order: deploy button → deploy result → ContractInteraction → error analysis → VibeScoreDashboard. Responsive layout should preserve this logical flow.

### What's fragile
- `ContractInteraction` conditionally renders based on `compileResult?.abi && deployResult?.address` — if the compile/deploy state shape changes, the interaction UI silently disappears with no error
- The SVG gauge uses hardcoded `r=52` and `120×120` viewBox — responsive scaling relies on CSS container width, not SVG internal resizing

### Authoritative diagnostics
- React DevTools → `entries` state on TransactionConsole shows all contract call results (type: "call") — this is the single source of truth for call history
- Browser console → `console.error('[ContractInteraction] Failed to parse ABI:')` appears on ABI parse failures
- SVG gauge `strokeDashoffset` value directly reveals the rendered score: lower offset = higher score

### What assumptions changed
- Plan assumed VibeScoreGauge would be deleted — it was only removed from page.tsx, the file remains (dead code cleanup deferred)
- Plan estimated 20m for T03 — actual was 8m because the surgical edits were straightforward
