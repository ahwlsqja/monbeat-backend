---
estimated_steps: 5
estimated_files: 3
---

# T01: ABI 유틸리티 + ContractInteraction 컴포넌트 구현

**Slice:** S03 — 컨트랙트 인터랙션 + Vibe-Score 대시보드
**Milestone:** M002

## Description

Create the ABI parsing utility and ContractInteraction component that allows users to call deployed contract functions from the sidebar. This is the riskiest piece of S03 — it involves parsing arbitrary ABI entries, generating dynamic forms, and executing contract calls via viem (reads) and wagmi (writes).

The project uses wagmi v3 (`useWriteContract` hook) and viem (`createPublicClient`). The Monad testnet chain is already defined in `wagmi-config.ts`. The `useTransactionLog` hook already supports `type: "call"` entries.

**Relevant skills:** react-best-practices (React component patterns)

## Steps

1. **Create `frontend/src/lib/abi-utils.ts`** — ABI parsing utility:
   - `parseAbiFunctions(abi: unknown[]): { readFunctions: AbiFunction[], writeFunctions: AbiFunction[] }` — filters ABI entries where `type === "function"`, separates `stateMutability === "view" | "pure"` (read) from `"nonpayable" | "payable"` (write)
   - `AbiFunction` type: `{ name: string, inputs: AbiParam[], outputs: AbiParam[], stateMutability: string, payable: boolean }`
   - `AbiParam` type: `{ name: string, type: string, internalType?: string }`
   - `solidityTypeToInputType(solType: string): "text" | "number" | "checkbox"` — maps `uint*`/`int*` → `"text"` (use text to support large numbers), `bool` → `"checkbox"`, `address`/`string`/`bytes*` → `"text"`
   - `parseInputValue(value: string, solType: string): unknown` — converts form string to typed value: `uint*`/`int*` → `BigInt(value)`, `bool` → `value === "true"`, `address`/`string`/`bytes*` → string as-is. Handle empty strings gracefully (return `BigInt(0)` for numbers, `false` for bool, `""` for strings)
   - `formatOutputValue(value: unknown): string` — serializes return values to display strings (`bigint` → `.toString()`, arrays → `JSON.stringify`, objects → `JSON.stringify`, primitives → `String()`)
   - Export all functions and types

2. **Create `frontend/src/__tests__/abi-utils.test.ts`** — 12+ unit tests:
   - `parseAbiFunctions` with mixed ABI (events, functions, constructor) — correctly filters only functions
   - `parseAbiFunctions` separates view/pure from nonpayable/payable
   - `parseAbiFunctions` with empty ABI returns empty arrays
   - `parseAbiFunctions` with ABI containing only events returns empty arrays
   - `solidityTypeToInputType` maps uint256 → text, bool → checkbox, address → text, string → text, bytes32 → text
   - `parseInputValue` converts "42" with uint256 to BigInt(42)
   - `parseInputValue` converts "true" with bool to true
   - `parseInputValue` handles empty string for uint256 → BigInt(0)
   - `parseInputValue` handles address string passthrough
   - `formatOutputValue` formats BigInt, arrays, plain strings
   - `parseAbiFunctions` with functions that have no inputs — inputs array is empty, not missing
   - `parseAbiFunctions` ignores ABI entries without `type` field

3. **Create `frontend/src/components/ide/ContractInteraction.tsx`**:
   - Props interface: `{ abi: unknown[], contractAddress: string, onCallResult: (entry: Omit<TransactionLogEntry, "id" | "timestamp">) => void }`
   - Import `parseAbiFunctions`, `solidityTypeToInputType`, `parseInputValue`, `formatOutputValue` from `@/lib/abi-utils`
   - Import `createPublicClient`, `http` from `viem`
   - Import `monadTestnet` from `@/lib/wagmi-config`
   - Import `useWriteContract`, `useAccount` from `wagmi`
   - Use `useMemo` to create viem `publicClient` with `createPublicClient({ chain: monadTestnet, transport: http() })`
   - Use `useMemo` to parse ABI into read/write function lists via `parseAbiFunctions(abi)`
   - For each function, maintain local state for input values (`Record<string, Record<string, string>>` keyed by function name then param name) and result display
   - **Read section**: Each function renders as a card with input fields (one per ABI param), a "Call" button, and a result area. On click, call `publicClient.readContract({ address, abi, functionName, args })`. Display result via `formatOutputValue`. Call `onCallResult({ type: "call", status: "success", message: "..." })`. Catch errors and display + log them
   - **Write section**: Similar cards but with a "Send" button. Use wagmi `useWriteContract` hook. Guard with `useAccount().isConnected` — show "Connect wallet" message if disconnected. On `writeContract({ address, abi, functionName, args })`, call `onCallResult` with tx hash. Handle errors
   - **Empty state**: If `readFunctions.length === 0 && writeFunctions.length === 0`, show "No callable functions" message
   - Style with Tailwind dark theme consistent with sidebar (gray-800/700/600 palette, emerald for success, red for errors, cyan for call type accent)

4. **Handle edge cases in ContractInteraction**:
   - Functions with no inputs: render just a call/send button, no input fields
   - `payable` functions: add an optional "Value (MON)" input for msg.value
   - Defensive ABI casting: wrap the entire `parseAbiFunctions` call in try/catch, fallback to empty arrays
   - Loading states: disable button and show spinner during call execution

5. **Verify**:
   - Run `cd frontend && npm test -- --testPathPatterns abi-utils` — all 12+ tests pass
   - Check file exists: `test -f frontend/src/components/ide/ContractInteraction.tsx`
   - Check file exists: `test -f frontend/src/lib/abi-utils.ts`

## Must-Haves

- [ ] `abi-utils.ts` exports `parseAbiFunctions`, `solidityTypeToInputType`, `parseInputValue`, `formatOutputValue`
- [ ] `parseAbiFunctions` correctly separates view/pure (read) from nonpayable/payable (write)
- [ ] ContractInteraction renders read functions callable without wallet (viem publicClient)
- [ ] ContractInteraction renders write functions with wagmi useWriteContract (wallet required)
- [ ] `onCallResult` callback fires with TransactionLogEntry-compatible objects
- [ ] 12+ unit tests for abi-utils pass
- [ ] Functions with empty inputs array render with just a call button (no crash)

## Verification

- `cd frontend && npm test -- --testPathPatterns abi-utils` — all tests pass
- `test -f frontend/src/components/ide/ContractInteraction.tsx` — file exists
- `test -f frontend/src/lib/abi-utils.ts` — file exists
- `grep -q "parseAbiFunctions" frontend/src/lib/abi-utils.ts` — main function exported
- `grep -q "onCallResult" frontend/src/components/ide/ContractInteraction.tsx` — callback wired

## Inputs

- `frontend/src/lib/wagmi-config.ts` — monadTestnet chain definition for viem publicClient
- `frontend/src/hooks/useTransactionLog.ts` — TransactionLogEntry type for onCallResult callback shape
- `frontend/src/components/WalletConnectModal.tsx` — reference pattern for wagmi hook usage (useAccount, useWriteContract)
- `frontend/package.json` — wagmi v3 + viem v2 already installed

## Observability Impact

- **New signal: Contract call results** — Every read/write call fires `onCallResult` with a `TransactionLogEntry`-shaped object (`type: "call"`, `status: "success"|"error"`, `message` with function name + return value or error). Downstream `useTransactionLog.addEntry()` persists these in the console.
- **Inspection**: React DevTools → ContractInteraction component state shows `result`/`error` per function card. TransactionConsole `entries` array shows call history.
- **Failure visibility**: ABI parse errors caught defensively and logged to `console.error('[ContractInteraction] Failed to parse ABI:', err)`. Read call errors and write call errors both surface in the UI (red error box) and in TransactionConsole.
- **No new env vars or secrets required** (testnet RPC is public).

## Expected Output

- `frontend/src/lib/abi-utils.ts` — new: ABI parsing utility with type mappings
- `frontend/src/__tests__/abi-utils.test.ts` — new: 12+ unit tests for ABI utils
- `frontend/src/components/ide/ContractInteraction.tsx` — new: ABI-based contract interaction UI
