---
id: T03
parent: S03
milestone: M001
provides:
  - PaymasterModule with deploy-count gating and signed-tx relay
  - PaymasterService with getDeployStatus, canUseRelay, incrementDeployCount, broadcastSignedTransaction
  - PaymasterController with 2 JWT-protected endpoints (GET /api/paymaster/status, POST /api/paymaster/relay-signed)
  - 12 paymaster unit tests (10 service + 2 controller)
key_files:
  - backend/src/paymaster/paymaster.module.ts
  - backend/src/paymaster/paymaster.service.ts
  - backend/src/paymaster/paymaster.controller.ts
  - backend/src/paymaster/dto/deploy-status.dto.ts
  - backend/src/paymaster/dto/relay-signed.dto.ts
  - backend/src/app.module.ts
  - backend/test/paymaster.service.spec.ts
  - backend/test/paymaster.controller.spec.ts
key_decisions:
  - DeployStatusDto uses definite assignment assertions (!) matching codebase DTO pattern for strictPropertyInitialization compliance
  - PaymasterService exports MAX_FREE_DEPLOYMENTS constant for test assertions and future reference
  - broadcastSignedTransaction creates a fresh JsonRpcProvider per call (stateless, no connection pooling) — appropriate for infrequent relay use
patterns_established:
  - Controller tests override JwtAuthGuard via .overrideGuard() with mock canActivate that injects test user into request
  - PaymasterService throws NotFoundException for missing users in getDeployStatus but returns false in canUseRelay (fail-safe default)
observability_surfaces:
  - GET /api/paymaster/status — returns { used, max: 3, remaining, canUseRelay } for authenticated user
  - PaymasterService logs deploy count checks, relay eligibility, increment operations, and broadcast results
  - Broadcast failures logged at ERROR level with RPC error detail, re-thrown as BadRequestException to client
  - NotFoundException thrown (and logged) when getDeployStatus called with non-existent userId
duration: 6m
verification_result: passed
completed_at: 2026-03-22
blocker_discovered: false
---

# T03: Build PaymasterModule with deploy-count gating and signed-tx relay

**Built PaymasterModule with deploy-count gating (3 free deploys), signed-tx relay via ethers.js JsonRpcProvider.broadcastTransaction, 2 JWT-protected endpoints, and 12 unit tests; registered in AppModule completing S03 slice.**

## What Happened

Created PaymasterService with four methods: `getDeployStatus(userId)` queries `User.deployCount` from Prisma and returns `{ used, max: 3, remaining, canUseRelay }`, `canUseRelay(userId)` checks `deployCount < 3`, `incrementDeployCount(userId)` atomically increments via Prisma `{ increment: 1 }`, and `broadcastSignedTransaction(signedTxHex)` creates an ethers.js `JsonRpcProvider` and calls `broadcastTransaction()`, wrapping failures in `BadRequestException` with the RPC error message.

Created DTOs: `DeployStatusDto` (response type) and `RelaySignedDto` (request with `@IsString()` + `@IsNotEmpty()` validators). Both use definite assignment assertions (`!`) matching the existing codebase pattern.

Created PaymasterController with two JWT-protected endpoints: `GET /api/paymaster/status` extracts `req.user.id` from the JWT and returns deploy status, `POST /api/paymaster/relay-signed` validates the `RelaySignedDto` body and broadcasts via the service.

Created PaymasterModule importing AuthModule (for JwtAuthGuard/JwtStrategy availability) and exporting PaymasterService. Registered PaymasterModule in AppModule.

Wrote 12 unit tests: 10 service tests covering `getDeployStatus` (correct remaining, max reached, user not found), `canUseRelay` (under limit, at limit, missing user), `incrementDeployCount` (Prisma update with increment), `broadcastSignedTransaction` (BadRequestException on failure), and `MAX_FREE_DEPLOYMENTS` constant. 2 controller tests covering definition and `getStatus` delegation to service with correct userId. Controller tests use `.overrideGuard(JwtAuthGuard)` to inject mock user.

## Verification

- `npm run build` — exit 0, zero TypeScript errors
- `npm test -- --testPathPattern=paymaster` — 12 tests pass across 2 suites
- `npm test` — 68 total tests pass (34 existing + 9 auth + 13 analysis + 12 paymaster), no regressions
- `grep -q "PaymasterModule" backend/src/app.module.ts` — confirmed registered
- `grep -q "JwtAuthGuard" backend/src/paymaster/paymaster.controller.ts` — confirmed used
- All 9 slice-level verification checks pass (final task of slice)

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd backend && npm run build` | 0 | ✅ pass | 4s |
| 2 | `cd backend && npm test -- --testPathPattern=paymaster` | 0 | ✅ pass (12 tests) | 5s |
| 3 | `cd backend && npm test` | 0 | ✅ pass (68 tests) | 9s |
| 4 | `grep -q "PaymasterModule" backend/src/app.module.ts` | 0 | ✅ pass | <1s |
| 5 | `grep -q "JwtAuthGuard" backend/src/paymaster/paymaster.controller.ts` | 0 | ✅ pass | <1s |
| 6 | `grep -q "AuthModule" backend/src/app.module.ts` | 0 | ✅ pass | <1s |
| 7 | `grep -q "AnalysisModule" backend/src/app.module.ts` | 0 | ✅ pass | <1s |
| 8 | `cd backend && npm test -- --testPathPattern=auth` | 0 | ✅ pass | — |
| 9 | `cd backend && npm test -- --testPathPattern=analysis` | 0 | ✅ pass | — |
| 10 | `test -d backend/data/monad-docs && ls backend/data/monad-docs/*.md \| wc -l` | 0 | ✅ pass (5) | <1s |

## Diagnostics

- **Deploy status inspection:** `GET /api/paymaster/status` with valid JWT returns `{ used, max: 3, remaining, canUseRelay }` for the authenticated user. Returns 401 without JWT.
- **PaymasterService logging:** Logs deploy status queries (`Deploy status for user X: used=Y, remaining=Z, canUseRelay=W`), relay eligibility checks, deploy count increments, and broadcast results at LOG level. Broadcast failures logged at ERROR level with full RPC error detail.
- **Error shapes:** `NotFoundException` for unknown userId in getDeployStatus. `BadRequestException` with RPC error message on broadcast failure. `401 Unauthorized` on missing/invalid JWT.
- **Integration with S05:** PaymasterService.incrementDeployCount() and canUseRelay() are exported and available for S05's deploy flow wiring.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `backend/src/paymaster/paymaster.module.ts` — NestJS module importing AuthModule, providing/exporting PaymasterService
- `backend/src/paymaster/paymaster.service.ts` — Deploy count gating + signed-tx relay service with ethers.js
- `backend/src/paymaster/paymaster.controller.ts` — 2 JWT-protected endpoints (status + relay-signed)
- `backend/src/paymaster/dto/deploy-status.dto.ts` — Response DTO for deploy status
- `backend/src/paymaster/dto/relay-signed.dto.ts` — Request DTO with class-validator decorators
- `backend/src/app.module.ts` — Added PaymasterModule import
- `backend/test/paymaster.service.spec.ts` — 10 PaymasterService unit tests
- `backend/test/paymaster.controller.spec.ts` — 2 PaymasterController unit tests (+ 1 definition test)
