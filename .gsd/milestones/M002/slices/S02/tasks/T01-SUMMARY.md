---
id: T01
parent: S02
milestone: M002
provides:
  - useTransactionLog hook for managing compile/deploy/call event log
  - TransactionConsole component for rendering color-coded log entries
  - solc-error-parser utility for parsing solc errors into Monaco markers
  - 19 tests covering hook state transitions and parser edge cases
key_files:
  - frontend/src/hooks/useTransactionLog.ts
  - frontend/src/components/ide/TransactionConsole.tsx
  - frontend/src/lib/solc-error-parser.ts
  - frontend/src/__tests__/useTransactionLog.test.ts
  - frontend/src/__tests__/solc-error-parser.test.ts
key_decisions:
  - Used counter-based fallback ID generation (txlog-N-timestamp) when crypto.randomUUID unavailable in jsdom test environment
  - Entries prepend (newest first) for natural console ordering
  - Unparseable solc errors produce fallback markers at line 1 col 1 rather than being silently dropped
patterns_established:
  - Hook testing pattern using @testing-library/react renderHook + act for state transitions
  - Pure utility testing pattern for solc-error-parser (no DOM or React needed)
observability_surfaces:
  - React DevTools: useTransactionLog entries array shows all events with timestamps and status
  - TransactionConsole: color-coded visual indicators (emerald/red/amber) for success/error/pending
  - solc-error-parser: fallback markers ensure every error string produces at least one visible marker
duration: 12m
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T01: Create useTransactionLog hook, TransactionConsole component, and solc-error-parser utility

**Created useTransactionLog hook, TransactionConsole component, and solc-error-parser utility with 19 passing tests — foundational units for compile/deploy UX**

## What Happened

Built three foundational units for the S02 compile/deploy UX pipeline:

1. **`useTransactionLog` hook** — manages an array of `TransactionLogEntry` objects with `addEntry()` (auto-generates id + timestamp, prepends newest first) and `clearLog()`. Supports compile/deploy/call types with optional details (address, txHash, errors, contractName, abi).

2. **`TransactionConsole` component** — renders entries as a scrollable list with color-coded status (emerald=success, red=error, amber=pending), type badges (blue=compile, purple=deploy, cyan=call), HH:MM:SS timestamps, and expandable `<details>` for entry metadata. Shows "No transactions yet" empty state.

3. **`solc-error-parser` utility** — regex-based parser that extracts line/column/severity from solc `formattedMessage` strings (Error, Warning, TypeError, ParserError, DeclarationError) into `MonacoMarker[]` objects for `setModelMarkers()`. Unmatched strings fall through to a marker at line 1, col 1.

Had to install `@testing-library/dom` as a missing peer dependency for `@testing-library/react` in the Jest 30 / React 19 setup.

## Verification

- `npx jest --testPathPatterns="useTransactionLog|solc-error-parser"` — 19 new tests pass (7 hook + 12 parser)
- `npx jest` — all 30 tests pass (11 existing api-client + 19 new), zero failures
- File existence verified for all 3 source files

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `npx jest --testPathPatterns="useTransactionLog\|solc-error-parser" --verbose` | 0 | ✅ pass | 1.4s |
| 2 | `npx jest --verbose` (full suite) | 0 | ✅ pass | 2.1s |
| 3 | `test -f frontend/src/hooks/useTransactionLog.ts` | 0 | ✅ pass | <1s |
| 4 | `test -f frontend/src/components/ide/TransactionConsole.tsx` | 0 | ✅ pass | <1s |
| 5 | `test -f frontend/src/lib/solc-error-parser.ts` | 0 | ✅ pass | <1s |

## Diagnostics

- **Hook state**: Inspect `useTransactionLog` entries via React DevTools in any component that uses the hook. Entries array contains id, type, timestamp, status, message, and details.
- **Console rendering**: TransactionConsole renders inside ConsolePanel slot (wiring in T02). Visual verification: green/red/amber borders and backgrounds indicate entry status.
- **Parser output**: `parseSolcErrors()` returns `MonacoMarker[]` — test with any solc error string to verify line/column extraction. Fallback markers at line 1 for unmatched strings ensure no silent drops.

## Deviations

- Installed `@testing-library/dom` as missing peer dependency — required by `@testing-library/react` in this Jest 30 setup. Not in the plan but necessary for hook tests to run.
- Jest 30 renamed `--testPathPattern` to `--testPathPatterns` — verification commands in the slice plan use the old flag. This doesn't affect `npm test` invocations (which work via config), only direct CLI usage.

## Known Issues

- Slice verification commands using `--testPathPattern` will fail with Jest 30's renamed flag `--testPathPatterns`. The `npm test` script works fine; only direct `jest` CLI invocations need updating.

## Files Created/Modified

- `frontend/src/hooks/useTransactionLog.ts` — new: transaction log state hook with addEntry/clearLog
- `frontend/src/components/ide/TransactionConsole.tsx` — new: color-coded console log renderer component
- `frontend/src/lib/solc-error-parser.ts` — new: solc error message → Monaco marker parser
- `frontend/src/__tests__/useTransactionLog.test.ts` — new: 7 tests for hook state transitions
- `frontend/src/__tests__/solc-error-parser.test.ts` — new: 12 tests for parser regex + edge cases
- `frontend/package.json` — modified: added @testing-library/dom dev dependency
- `.gsd/milestones/M002/slices/S02/tasks/T01-PLAN.md` — modified: added Observability Impact section
