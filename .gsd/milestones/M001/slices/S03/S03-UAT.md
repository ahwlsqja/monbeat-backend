# S03: Auth + Analysis + Paymaster Module — UAT

**Milestone:** M001
**Written:** 2026-03-22

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All external services (GitHub OAuth, Gemini API, Monad RPC) are mocked in tests. The slice proves module boundaries, service logic, guard wiring, and test coverage — not live API integration. Real runtime testing requires configured API keys (S06).

## Preconditions

- Working directory: `backend/`
- Node.js installed, `npm ci` completed
- No external services required (all mocked in tests)

## Smoke Test

```bash
cd backend && npm run build && npm test
```
Expected: exit 0, 68 tests pass across 11 suites, zero TypeScript errors.

## Test Cases

### 1. Auth module compiles and tests pass

1. `cd backend && npm run build`
2. `cd backend && npm test -- --testPathPattern=auth`
3. **Expected:** Exit 0, 9 tests pass across 2 suites (auth.service.spec + auth.controller.spec)

### 2. AuthModule registered in AppModule

1. `grep "AuthModule" backend/src/app.module.ts`
2. **Expected:** AuthModule appears in imports array

### 3. Auth configuration keys present

1. `grep "github.clientId\|github.clientSecret\|jwt.secret\|gemini.apiKey" backend/src/config/configuration.ts | wc -l`
2. **Expected:** At least 4 lines matching the config keys

### 4. JwtAuthGuard is reusable

1. `grep -r "JwtAuthGuard" backend/src/paymaster/`
2. **Expected:** JwtAuthGuard imported and used in paymaster controller, confirming cross-module guard sharing

### 5. Analysis module compiles and tests pass

1. `cd backend && npm test -- --testPathPattern="analysis|optimizer"`
2. **Expected:** Exit 0, 13 tests pass across 2 suites (analysis.service.spec + optimizer.service.spec)

### 6. AnalysisModule registered in AppModule

1. `grep "AnalysisModule" backend/src/app.module.ts`
2. **Expected:** AnalysisModule appears in imports array

### 7. RAG context files present

1. `ls backend/data/monad-docs/*.md | wc -l`
2. **Expected:** 5 markdown files

### 8. Streaming support wired in AnalysisController

1. `grep -c "stream" backend/src/analysis/analysis.controller.ts`
2. **Expected:** Multiple occurrences — `?stream=true` query parameter handling

### 9. Optimizer detects parallelism patterns

1. `cd backend && npm test -- --testPathPattern=optimizer`
2. Review test output for pattern detection names
3. **Expected:** 6 tests covering loop storage, SLOAD/SSTORE, mapping access, CEI violation, centralized bottleneck, block/tx dependency

### 10. Paymaster module compiles and tests pass

1. `cd backend && npm test -- --testPathPattern=paymaster`
2. **Expected:** Exit 0, 12 tests pass across 2 suites (paymaster.service.spec + paymaster.controller.spec)

### 11. PaymasterModule registered in AppModule

1. `grep "PaymasterModule" backend/src/app.module.ts`
2. **Expected:** PaymasterModule appears in imports array

### 12. Deploy count gating logic

1. Review `backend/test/paymaster.service.spec.ts` for:
   - `getDeployStatus` with user having 1 deploy → remaining=2, canUseRelay=true
   - `getDeployStatus` with user having 3 deploys → remaining=0, canUseRelay=false
   - `canUseRelay` returns false when deployCount >= 3
2. **Expected:** All edge cases covered in unit tests

### 13. Full test suite regression check

1. `cd backend && npm test`
2. **Expected:** 68 tests pass across 11 suites, no regressions from S01/S02 tests (34 original tests still pass)

## Edge Cases

### Auth with missing GitHub user fields

1. Review `backend/test/auth.service.spec.ts` for test handling missing `displayName` or `photos`
2. **Expected:** AuthService handles partial GitHub profile gracefully, using fallback values

### Paymaster with non-existent user

1. Review `backend/test/paymaster.service.spec.ts` for `NotFoundException` test
2. **Expected:** `getDeployStatus` throws NotFoundException for unknown userId; `canUseRelay` returns false (fail-safe)

### Broadcast failure handling

1. Review `backend/test/paymaster.service.spec.ts` for broadcast error test
2. **Expected:** `broadcastSignedTransaction` wraps RPC errors in BadRequestException with error message preserved

### GeminiService without API key

1. Review `backend/test/analysis.service.spec.ts` for fallback behavior when Gemini returns null
2. **Expected:** AnalysisService falls back to heuristic-only analysis (gas/nonce/revert/opcode categories) instead of crashing

## Failure Signals

- `npm run build` exits non-zero or shows TypeScript errors → compilation broken
- `npm test` shows fewer than 68 tests → missing test files or broken imports
- Any test suite fails → service logic regression
- `grep -q "AuthModule\|AnalysisModule\|PaymasterModule" backend/src/app.module.ts` fails → modules not registered
- `backend/data/monad-docs/` missing or has fewer than 5 files → RAG context incomplete

## Not Proven By This UAT

- Live GitHub OAuth flow (requires real GitHub OAuth App credentials)
- Live Gemini AI analysis (requires GEMINI_API_KEY)
- Live signed-tx broadcast to Monad testnet (requires MONAD_RPC_URL)
- Frontend integration (covered by S05)
- E2E testing through HTTP (covered by S06)
- JWT expiration/refresh behavior (not implemented; JWT has no explicit expiry in current config)

## Notes for Tester

- The PaymasterService ERROR log during tests (`Transaction broadcast failed: could not coalesce error`) is **expected** — it's the test verifying error handling for invalid transaction hex.
- All external APIs (GitHub, Gemini, Monad RPC) are mocked in tests. No network calls are made.
- The `MAX_FREE_DEPLOYMENTS = 3` constant is exported from PaymasterService and asserted in tests — changing this value requires test updates.
