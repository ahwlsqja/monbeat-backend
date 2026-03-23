# S02: NestJS — Storage Layout 디코딩 + Actionable Suggestion 생성 — UAT

**Milestone:** M006
**Written:** 2026-03-24

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All behavior is verified through unit/integration tests with mock data that simulates real CLI output. No live runtime (Rust CLI, Monad testnet) required — the decoder is a pure function module and the pipeline wiring uses mocked engine results.

## Preconditions

- `cd /home/ahwlsqja/Vibe-Room-Backend && npm install` completed (dependencies available)
- Node.js 18+ and solc available (already part of project setup)
- No running server required — all tests use NestJS testing module with mocks

## Smoke Test

```bash
cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/storage-layout-decoder.spec.ts test/vibe-score.service.spec.ts test/compile.service.spec.ts --silent
```
Expected: 43 tests pass, 0 failures.

## Test Cases

### 1. CompileService extracts storageLayout from solc

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/compile.service.spec.ts --verbose --testNamePattern="storageLayout"`
2. **Expected:** 2 tests pass:
   - ParallelConflict.sol produces storageLayout with `storage` array containing an entry where `label === "counter"` and `slot === "0"`
   - FixedContract.sol also produces a non-empty storageLayout

### 2. Hex slot → variable name exact match decoding

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/storage-layout-decoder.spec.ts --verbose --testNamePattern="exact slot"`
2. **Expected:** 2 tests pass:
   - `decodeSlotToVariable("0x0", layout)` returns `variableName: "counter"` for a layout with counter at slot "0"
   - `decodeSlotToVariable("0x5", layout)` returns correct variable for a layout with entry at slot "5"

### 3. Mapping heuristic for large runtime slots

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/storage-layout-decoder.spec.ts --verbose --testNamePattern="mapping|unknown"`
2. **Expected:** 3 tests pass:
   - Single mapping → large slot attributed to that mapping variable
   - Multiple mappings → "unknown (possibly X or Y)" reported
   - No mapping at all → "unknown_slot_0xNNN" fallback

### 4. Coinbase address conflict filtering

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/storage-layout-decoder.spec.ts --verbose --testNamePattern="coinbase"`
2. **Expected:** 1 test passes. Conflicts where `location.address` matches coinbase (case-insensitive) are excluded from decoded results.

### 5. Non-Storage conflict skipping

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/storage-layout-decoder.spec.ts --verbose --testNamePattern="non-Storage"`
2. **Expected:** 1 test passes. Balance/Nonce/CodeHash location type conflicts are skipped (only Storage type decoded).

### 6. Actionable suggestion generation by variable type

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/storage-layout-decoder.spec.ts --verbose --testNamePattern="suggestion"`
2. **Expected:** 4 tests pass with type-specific suggestion content:
   - Mapping variable → suggestion mentions key range separation
   - Simple uint256 → suggestion mentions per-function variable splitting
   - Dynamic array → suggestion mentions mapping-based structure
   - Unknown slot → suggestion mentions verifying storage layout

### 7. Function×variable conflict matrix

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/storage-layout-decoder.spec.ts --verbose --testNamePattern="matrix"`
2. **Expected:** 3 tests pass:
   - Matrix dimensions match unique function/variable counts
   - Intersection counts are correct
   - Empty conflicts → empty matrix (rows=[], cols=[], cells=[])

### 8. Full ParallelConflict-like integration

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/storage-layout-decoder.spec.ts --verbose --testNamePattern="ParallelConflict"`
2. **Expected:** 1 test passes. Given a fixture mimicking ParallelConflict:
   - slot "0x0" decoded to "counter"
   - functions include "increment" and "incrementBy"
   - suggestion contains both variable name and function names
   - matrix has correct row/column labels

### 9. VibeScoreService — conflictAnalysis present with conflict data

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/vibe-score.service.spec.ts --verbose --testNamePattern="includes conflictAnalysis"`
2. **Expected:** 1 test passes. When mocked engine returns conflict_details and compiled result has storageLayout:
   - `result.conflictAnalysis` is defined
   - `result.conflictAnalysis.conflicts[0].variableName === "counter"`
   - `result.conflictAnalysis.conflicts[0].functions` contains "increment" and "incrementBy"
   - `result.conflictAnalysis.matrix` has non-empty rows and cols

### 10. VibeScoreService — backward compat without conflict_details

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/vibe-score.service.spec.ts --verbose --testNamePattern="omits conflictAnalysis when no conflict_details"`
2. **Expected:** 1 test passes. When engine returns no conflict_details (default mock — simulates older CLI or zero conflicts):
   - `result.conflictAnalysis` is `undefined`
   - All existing fields (vibeScore, conflicts, reExecutions, gasEfficiency, suggestions) are intact

### 11. VibeScoreService — graceful degradation without storageLayout

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/vibe-score.service.spec.ts --verbose --testNamePattern="storageLayout is undefined"`
2. **Expected:** 1 test passes. When conflict_details exist but storageLayout is undefined:
   - `result.conflictAnalysis` is `undefined`
   - No error thrown

## Edge Cases

### storageLayout undefined (solc extraction failure)

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/storage-layout-decoder.spec.ts --verbose --testNamePattern="storageLayout is undefined"`
2. **Expected:** buildConflictAnalysis returns `{ conflicts: [], matrix: { rows: [], cols: [], cells: [] } }` — no error thrown

### All conflicts filtered (all coinbase)

1. If every conflict in conflict_details has a coinbase address, buildConflictAnalysis returns empty conflicts array
2. VibeScoreService sets conflictAnalysis to `undefined` (not empty object) — omitted from API response

### Unknown slot fallback

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/storage-layout-decoder.spec.ts --verbose --testNamePattern="unknown_slot"`
2. **Expected:** When a slot cannot be decoded (no match, no mapping heuristic), variable name is `"unknown_slot_0xNNN"` with appropriate suggestion text

### Existing test suite regression

1. Run `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/compile.service.spec.ts test/vibe-score.service.spec.ts --silent`
2. **Expected:** All pre-existing tests (8 compile + 13 vibe-score = 21) still pass — no regressions from new code

## Failure Signals

- `npx jest` exits non-zero → test regression or module import error
- `conflictAnalysis` present in API response when conflict_details is absent → backward compat broken
- `"unknown_slot"` appearing in decoded results for known simple variables (counter, owner) → slot comparison bug
- Phase 5b log not appearing when conflict_details + storageLayout both present → wiring broken
- Matrix dimensions (rows.length × cols.length) don't match cells dimensions → buildMatrix bug

## Not Proven By This UAT

- Live CLI → NestJS pipeline integration (proven in S04 E2E tests)
- Frontend heatmap rendering from matrix data (proven in S03)
- Real Monad testnet transaction execution generating actual conflict_details (proven in S04 E2E tests)
- Mapping heuristic accuracy on real-world complex contracts (probabilistic — needs production monitoring)

## Notes for Tester

- All tests use mock data, not live services — they should pass consistently regardless of network or testnet state
- The `ERROR [VibeScoreService] Failed to persist vibe-score: DB connection lost` log in vibe-score tests is expected — it tests the "database write failure doesn't crash the service" path
- To run the complete slice verification in one command: `cd /home/ahwlsqja/Vibe-Room-Backend && npx jest test/compile.service.spec.ts test/storage-layout-decoder.spec.ts test/vibe-score.service.spec.ts`
