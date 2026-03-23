---
id: T01
parent: S03
milestone: M002
provides:
  - ABI parsing utility (parseAbiFunctions, solidityTypeToInputType, parseInputValue, formatOutputValue)
  - ContractInteraction component for read/write contract calls via viem/wagmi
key_files:
  - frontend/src/lib/abi-utils.ts
  - frontend/src/__tests__/abi-utils.test.ts
  - frontend/src/components/ide/ContractInteraction.tsx
key_decisions:
  - Use text input (not number) for uint/int types to support BigInt-scale values
  - ReadFunctionCard and WriteFunctionCard as separate sub-components for clean separation of viem vs wagmi logic
patterns_established:
  - Defensive ABI parsing with try/catch fallback to empty arrays
  - onCallResult callback pattern for TransactionLogEntry-compatible objects
observability_surfaces:
  - onCallResult callback fires TransactionLogEntry-shaped objects for every call (success or error)
  - console.error for ABI parse failures in ContractInteraction
  - UI-level error display (red box) for both read and write call failures
duration: 15m
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T01: ABI 유틸리티 + ContractInteraction 컴포넌트 구현

**Created abi-utils.ts with ABI parsing/type-mapping utilities and ContractInteraction.tsx with read (viem) / write (wagmi) function call UI, verified by 17 passing tests**

## What Happened

Created three files for S03's core contract interaction feature:

1. **`frontend/src/lib/abi-utils.ts`** — ABI parsing utility with four exported functions: `parseAbiFunctions` separates view/pure functions (read) from nonpayable/payable (write); `solidityTypeToInputType` maps Solidity types to HTML input types (text for numerics to support BigInt); `parseInputValue` converts form strings to typed values (BigInt for ints, boolean for bool, string passthrough for address/bytes); `formatOutputValue` serializes return values including BigInt support via custom replacer.

2. **`frontend/src/__tests__/abi-utils.test.ts`** — 17 unit tests covering all four functions: mixed ABI filtering, read/write separation, empty ABI handling, events-only ABI, no-input functions, entries without `type` field, payable flag detection, type mappings, input parsing with edge cases (empty strings), and output formatting (BigInt, arrays, null).

3. **`frontend/src/components/ide/ContractInteraction.tsx`** — React component with `ReadFunctionCard` and `WriteFunctionCard` sub-components. Read functions use viem `publicClient.readContract` (no wallet needed); write functions use wagmi `useWriteContract` with wallet connection guard. Handles edge cases: functions with no inputs (just a call/send button), payable functions (optional MON value input), defensive ABI parse errors, and loading spinners during execution. Results and errors feed into `onCallResult` for TransactionConsole integration.

## Verification

- `npm test -- --testPathPatterns abi-utils` — 17/17 tests pass
- File existence checks: all 3 output files exist
- Grep checks: `parseAbiFunctions` exported, `onCallResult` wired in component

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd frontend && npm test -- --testPathPatterns abi-utils` | 0 | ✅ pass | 48.8s |
| 2 | `test -f frontend/src/components/ide/ContractInteraction.tsx` | 0 | ✅ pass | <1s |
| 3 | `test -f frontend/src/lib/abi-utils.ts` | 0 | ✅ pass | <1s |
| 4 | `grep -q "parseAbiFunctions" frontend/src/lib/abi-utils.ts` | 0 | ✅ pass | <1s |
| 5 | `grep -q "onCallResult" frontend/src/components/ide/ContractInteraction.tsx` | 0 | ✅ pass | <1s |

### Slice-level verification (partial — T01 is intermediate):

| # | Check | Verdict | Notes |
|---|-------|---------|-------|
| 1 | `test -f frontend/src/components/ide/ContractInteraction.tsx` | ✅ pass | T01 |
| 2 | `test -f frontend/src/lib/abi-utils.ts` | ✅ pass | T01 |
| 3 | `test -f frontend/src/__tests__/abi-utils.test.ts` | ✅ pass | T01 |
| 4 | `test -f frontend/src/components/ide/VibeScoreDashboard.tsx` | ⏳ T02 | Expected |
| 5 | `test -f frontend/src/__tests__/VibeScoreDashboard.test.tsx` | ⏳ T02 | Expected |
| 6 | `grep -q "ContractInteraction" frontend/src/app/page.tsx` | ⏳ T03 | Expected |
| 7 | `grep -q "VibeScoreDashboard" frontend/src/app/page.tsx` | ⏳ T03 | Expected |
| 8 | `grep -q "conflicts" frontend/src/lib/api-client.ts` | ⏳ T02 | Expected |

## Diagnostics

- **ABI parse errors**: `console.error('[ContractInteraction] Failed to parse ABI:', err)` in browser console
- **Call results**: Inspect TransactionConsole entries via React DevTools → `entries` state array (type: "call")
- **Component state**: Each ReadFunctionCard/WriteFunctionCard maintains local `result`/`error`/`loading` state visible in React DevTools
- **Write call errors**: wagmi error messages (user rejection, gas estimation, revert) shown in both ContractInteraction UI and TransactionConsole

## Deviations

- Added 17 tests instead of the planned 12+ (extra tests for payable flag and null formatting)
- Added `bigIntReplacer` helper for safe JSON.stringify of objects/arrays containing BigInt values

## Known Issues

None

## Files Created/Modified

- `frontend/src/lib/abi-utils.ts` — new: ABI parsing utility with type mappings and value conversion
- `frontend/src/__tests__/abi-utils.test.ts` — new: 17 unit tests for all ABI utility functions
- `frontend/src/components/ide/ContractInteraction.tsx` — new: ABI-based contract interaction UI with read/write sections
- `.gsd/milestones/M002/slices/S03/tasks/T01-PLAN.md` — modified: added Observability Impact section
