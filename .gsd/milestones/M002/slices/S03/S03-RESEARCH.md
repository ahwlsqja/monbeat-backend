# S03: 컨트랙트 인터랙션 + Vibe-Score 대시보드 — Research

**Date:** 2026-03-23
**Depth:** Targeted — known tech (wagmi/viem for contract interaction, React components for dashboard), moderate integration with existing page.tsx state.

## Summary

S03 delivers two features: (1) **ContractInteraction** — a sidebar UI that reads a compiled contract's ABI and renders callable read/write function forms, executing them via viem `publicClient.readContract` for view functions and wagmi `useWriteContract` for state-changing functions; (2) **VibeScoreDashboard** — a richer replacement for the current `VibeScoreGauge` that adds a conflict heatmap and suggestion cards alongside the existing gauge.

Both features are well-scoped. The frontend already has `compileResult` state (with `abi`, `bytecode`, `contractName`) and `deployResult` state (with `address`) in `page.tsx` — S03 consumes these directly. For contract interaction, viem and wagmi are already installed (wagmi v3 provides `useReadContract`/`useWriteContract` hooks, viem provides `createPublicClient`). For the dashboard, the backend `VibeScoreResultDto` already returns `conflicts`, `reExecutions`, `gasEfficiency`, and `traceResults` — but the frontend `VibeScoreResult` type only captures `vibeScore`, `suggestions`, and `engineBased`. The type must be expanded.

The natural split is three tasks: (1) ContractInteraction component + ABI form generation, (2) VibeScoreDashboard with expanded data, (3) page.tsx wiring + integration.

## Recommendation

**Build ContractInteraction first** — it's the riskier piece (ABI parsing, dynamic form generation, wallet integration for writes). Then VibeScoreDashboard (pure UI, no wallet dependency). Finally wire both into page.tsx.

For contract reads, use viem's `publicClient.readContract()` directly (no wallet needed). For writes, use wagmi's `useWriteContract` hook (requires connected wallet). This follows the same wallet-gating pattern already established by `WalletConnectModal`.

For the dashboard, expand `VibeScoreResult` in `api-client.ts` to include `conflicts`, `reExecutions`, `gasEfficiency`, `engineBased`, and `traceResults` from the backend DTO. Build three visual sub-components: gauge (refactor existing), conflict heatmap (new), suggestion cards (new).

## Implementation Landscape

### Key Files

- `frontend/src/app/page.tsx` — Main orchestrator. Already holds `compileResult` (ABI, bytecode, contractName), `deployResult` (address), `vibeScore`, `useTransactionLog`. S03 wires ContractInteraction and VibeScoreDashboard into the sidebar.
- `frontend/src/lib/api-client.ts` — `VibeScoreResult` interface needs expansion: add `conflicts: number`, `reExecutions: number`, `gasEfficiency: number`, `traceResults?: TxTraceResult[]`. `getVibeScore()` method is unchanged (backend already returns these fields, just not typed).
- `frontend/src/components/VibeScoreGauge.tsx` — Current gauge. Will be **replaced** by the new `VibeScoreDashboard` which includes the gauge as a sub-component. The existing gauge logic (score → color → bar) can be reused or inlined.
- `frontend/src/lib/wagmi-config.ts` — Already has `monadTestnet` chain and wagmi config. Used by `useWriteContract` for chain context.
- `frontend/src/components/WalletConnectModal.tsx` — Existing wallet connection pattern using wagmi hooks (`useAccount`, `useConnect`, `useWriteContract`). S03's write function calls follow this same pattern.
- `frontend/src/hooks/useTransactionLog.ts` — Already supports `type: "call"`. S03 uses `addEntry({ type: "call", ... })` for contract interaction results.
- `frontend/src/components/ide/SidebarPanel.tsx` — Container for sidebar content. ContractInteraction and VibeScoreDashboard render inside this.
- `frontend/src/components/ide/TransactionConsole.tsx` — Already has `call` type badge styling (cyan). Contract interaction results will appear here automatically.

### New Files to Create

- `frontend/src/components/ide/ContractInteraction.tsx` — ABI-based function call UI. Renders deployed contract functions as expandable cards with input fields. Uses viem for reads, wagmi for writes.
- `frontend/src/components/ide/VibeScoreDashboard.tsx` — Rich dashboard replacing VibeScoreGauge. Contains: score gauge (circular), conflict/reExecution stats, suggestion cards with icons.

### Build Order

**Task 1: ContractInteraction component** (riskiest — ABI parsing + dynamic forms + wallet integration)
- Create `ContractInteraction.tsx` with:
  - Props: `abi: unknown[]`, `contractAddress: string`, `onCallResult: (entry) => void`
  - Parse ABI to separate `view`/`pure` (read) vs `nonpayable`/`payable` (write) functions
  - For each function: render name, input fields (typed by ABI param types), call button
  - Read calls: use viem `createPublicClient` + `readContract()` — no wallet needed
  - Write calls: use wagmi `useWriteContract` hook — requires wallet connection
  - Result display: show return values for reads, tx hash for writes
  - Call `onCallResult` to log to TransactionConsole
- Type helper: map Solidity types (`uint256`, `address`, `bool`, `string`, `bytes`) to HTML input types and parse back

**Task 2: VibeScoreDashboard component** (pure UI, no external deps)
- Expand `VibeScoreResult` in `api-client.ts` with backend fields
- Create `VibeScoreDashboard.tsx` with:
  - Props: `score`, `conflicts`, `reExecutions`, `gasEfficiency`, `suggestions`, `engineBased`, `loading`
  - Sub-sections: Circular gauge (from VibeScoreGauge logic), Stats grid (conflicts, reExecutions, gasEfficiency as metric cards), Suggestion cards (each suggestion as a styled card with index/icon)
  - The "heatmap" from the roadmap can be simplified to a conflict stats visualization (no real 2D heatmap data from the backend — `traceResults` is a flat array of tx results, not a slot-level conflict map)

**Task 3: page.tsx wiring**
- Replace `VibeScoreGauge` with `VibeScoreDashboard` in sidebarContent
- Add `ContractInteraction` to sidebarContent (shown when `compileResult && deployResult`)
- Update `handleAnalyzeVibeScore` to store full `VibeScoreResult` (not just score+suggestions)
- Wire `addEntry` to ContractInteraction's `onCallResult` callback
- No new API calls needed — `compileResult` and `getVibeScore` already exist

### Verification Approach

1. **Build**: `cd frontend && npm run build` — must exit 0, no SSR errors
2. **Tests**: Create tests for:
   - ABI parsing utility (separate view/write functions, Solidity type → input mapping)
   - VibeScoreDashboard rendering with various scores/data
3. **Existing tests**: `npm test` — all 30 existing tests must still pass
4. **File existence**: Verify all new files created
5. **Integration checks via grep**:
   - `ContractInteraction` imported in `page.tsx` ✅
   - `VibeScoreDashboard` imported in `page.tsx` ✅
   - `VibeScoreGauge` removed from `page.tsx` imports ✅
   - `conflicts` in `VibeScoreResult` type ✅
   - `type: "call"` usage in page.tsx ✅

## Constraints

- **Wallet required for writes** — `useWriteContract` only works when wallet is connected. Read functions must work without wallet (use viem publicClient directly).
- **No SSR issues** — ContractInteraction and VibeScoreDashboard are pure React components (no Monaco/window dependency), so no `next/dynamic` needed. They render inside the already client-side `page.tsx`.
- **wagmi v3 hooks must be called inside WagmiProvider** — already the case since `providers.tsx` wraps the entire app.
- **`compileResult.abi` is `unknown[]`** — needs runtime casting/validation when parsing function entries. Use defensive checks.
- **Backend `traceResults` contains `TxResult[]`** (success, gas_used, output, error, logs_count) — this is per-transaction, not per-storage-slot. A true "conflict heatmap" would need slot-level data which the backend doesn't expose. Simplify to a conflict stats visualization.

## Common Pitfalls

- **ABI `inputs` array can be empty** — functions with no parameters still need a call button but no input fields. Don't assume `inputs.length > 0`.
- **Solidity `bytes` and `tuple` types** — generating input forms for complex types (structs, nested arrays) is hard. Use a simple text input with JSON parsing as fallback for non-primitive types. The contract templates in this project (FailingContract, FixedContract, PectraTest, ParallelConflict) use simple types (uint256, address, bool), so comprehensive tuple support isn't critical.
- **`useWriteContract` returns tx hash, not receipt** — need `useWaitForTransactionReceipt` to get confirmation. This pattern is already in `WalletConnectModal.tsx`.
- **viem publicClient needs chain config** — use the `monadTestnet` chain from `wagmi-config.ts` with `createPublicClient({ chain: monadTestnet, transport: http() })`.
- **`deployResult.address` might be undefined** — guard ContractInteraction rendering with `deployResult?.address` check.
- **VibeScoreResult backward compatibility** — the expanded type adds optional fields. Existing code that only reads `vibeScore` and `suggestions` continues to work.

## Open Risks

- **The "conflict heatmap" mentioned in the roadmap** has no direct data source. `traceResults` is a flat array of per-tx results (`success`, `gas_used`), and `incarnations[]` shows re-execution counts per tx index. This gives per-transaction conflict data but not per-storage-slot conflicts. A simplified "per-tx incarnation bar chart" or "conflict summary" is more honest than a fake heatmap. The planner should decide the appropriate visualization given the data.
