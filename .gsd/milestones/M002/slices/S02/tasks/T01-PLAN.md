---
estimated_steps: 5
estimated_files: 5
---

# T01: Create useTransactionLog hook, TransactionConsole component, and solc-error-parser utility

**Slice:** S02 — 컴파일·배포 UX + 트랜잭션 콘솔
**Milestone:** M002

## Description

Create three foundational building blocks for the compile/deploy UX:

1. **`useTransactionLog` hook** — manages an array of `TransactionLogEntry` objects with `addEntry()` and `clearLog()` methods. Each entry has: type (compile|deploy|call), timestamp (ISO string), status (success|error|pending), message (string), and optional details (address, txHash, errors array).

2. **`TransactionConsole` component** — renders the transaction log entries inside the ConsolePanel slot. Color-coded: green for success, red for error, amber for pending. Shows timestamp, type badge, message, and expandable details section.

3. **`solc-error-parser.ts` utility** — parses solc `formattedMessage` strings (format: `ContractName.sol:LINE:COL: Warning|Error: message`) into structured objects `{ line, column, severity, message }` suitable for `monaco.editor.setModelMarkers()`. Returns an array of parsed markers. Handles both structured arrays and plain string messages.

All three are pure units with no API wiring — tests validate them in isolation.

**Relevant skills:** The project uses Jest 30 with jsdom environment, next/jest config, and `@/` path alias. Existing test pattern is in `frontend/src/__tests__/api-client.test.ts`.

## Steps

1. Create `frontend/src/hooks/useTransactionLog.ts`:
   - Define `TransactionLogEntry` interface: `{ id: string, type: 'compile' | 'deploy' | 'call', timestamp: string, status: 'success' | 'error' | 'pending', message: string, details?: { address?: string, txHash?: string, errors?: string[], contractName?: string, abi?: unknown[] } }`
   - Export the hook returning `{ entries: TransactionLogEntry[], addEntry: (entry: Omit<TransactionLogEntry, 'id' | 'timestamp'>) => void, clearLog: () => void }`
   - Use `useState` for entries array, auto-generate `id` (crypto.randomUUID or counter) and `timestamp` (new Date().toISOString()) in `addEntry`
   - New entries prepend to the array (newest first)

2. Create `frontend/src/components/ide/TransactionConsole.tsx`:
   - Props: `{ entries: TransactionLogEntry[] }`
   - Render a scrollable list of entries
   - Each entry shows: timestamp (formatted HH:MM:SS), type badge (colored pill), status icon (✓/✗/⏳), and message
   - Status colors: success → `text-emerald-400 border-emerald-700`, error → `text-red-400 border-red-700`, pending → `text-amber-400 border-amber-700`
   - Details section: if entry has details, show them in a collapsible `<details>` element with monospace font
   - Empty state: show "No transactions yet" in gray
   - Style with Tailwind, dark theme consistent with existing IDE (bg-gray-900, text-gray-300)

3. Create `frontend/src/lib/solc-error-parser.ts`:
   - Export `parseSolcErrors(errorMessages: string | string[]): MonacoMarker[]`
   - `MonacoMarker` type: `{ startLineNumber: number, startColumn: number, endLineNumber: number, endColumn: number, severity: number, message: string }`
   - Monaco severity constants: Error=8, Warning=4, Info=2
   - Parse each message with regex: `/^[^:]*:(\d+):(\d+):\s*(Warning|Error|TypeError|ParserError|DeclarationError):\s*(.+)$/m`
   - For unmatched strings (e.g., plain "Compilation failed"), return a single marker at line 1, column 1 with the full message
   - Handle both single string input and array of strings
   - `endLineNumber` = `startLineNumber`, `endColumn` = `startColumn + 1` (Monaco highlights the position)

4. Create `frontend/src/__tests__/useTransactionLog.test.ts`:
   - Test `addEntry` adds an entry with auto-generated id and timestamp
   - Test entries prepend (newest first)
   - Test `clearLog` empties the array
   - Test multiple entry types (compile, deploy, call)
   - Use `@testing-library/react` `renderHook` for hook testing

5. Create `frontend/src/__tests__/solc-error-parser.test.ts`:
   - Test parsing standard solc error: `"Contract.sol:10:5: Error: Undeclared identifier"` → `{ startLineNumber: 10, startColumn: 5, severity: 8, message: "Undeclared identifier" }`
   - Test parsing warning: `"Contract.sol:3:1: Warning: Unused variable"` → `severity: 4`
   - Test parsing array of errors → multiple markers
   - Test fallback for unparseable string → marker at line 1, col 1
   - Test empty input → empty array

## Must-Haves

- [ ] `useTransactionLog` hook manages entries with addEntry/clearLog
- [ ] `TransactionConsole` renders entries with color-coded severity and timestamps
- [ ] `solc-error-parser` extracts line/column/severity from solc error strings
- [ ] All tests pass: `npm test -- --testPathPattern="useTransactionLog|solc-error-parser"`
- [ ] No TypeScript errors in new files

## Verification

- `cd frontend && npm test -- --testPathPattern="useTransactionLog|solc-error-parser"` — new tests pass
- `cd frontend && npm test` — all tests pass (existing 11 + new ones)
- `test -f frontend/src/hooks/useTransactionLog.ts`
- `test -f frontend/src/components/ide/TransactionConsole.tsx`
- `test -f frontend/src/lib/solc-error-parser.ts`

## Observability Impact

- **useTransactionLog hook state**: Inspectable via React DevTools — entries array shows all compile/deploy/call events with timestamps, status, and details. A future agent can verify log state by checking entries count and last entry status.
- **TransactionConsole rendering**: Color-coded entries (emerald/red/amber) provide visual severity indicators. Empty state ("No transactions yet") signals no operations have run. Entry details are expandable for debugging.
- **solc-error-parser output**: Returns structured `MonacoMarker[]` — marker count, line numbers, and severity values are verifiable in tests and at runtime via `setModelMarkers` diagnostics. Unparseable errors fall through to line 1 col 1 with the original message preserved.
- **Failure visibility**: Parse failures produce fallback markers rather than silent drops — every error string produces at least one visible marker.

## Inputs

- `frontend/src/components/ide/ConsolePanel.tsx` — the slot where TransactionConsole will be placed (by T02)
- `frontend/jest.config.js` — existing Jest configuration
- `frontend/src/__tests__/api-client.test.ts` — existing test pattern reference

## Expected Output

- `frontend/src/hooks/useTransactionLog.ts` — new hook file
- `frontend/src/components/ide/TransactionConsole.tsx` — new console component
- `frontend/src/lib/solc-error-parser.ts` — new parser utility
- `frontend/src/__tests__/useTransactionLog.test.ts` — new hook tests
- `frontend/src/__tests__/solc-error-parser.test.ts` — new parser tests
