---
estimated_steps: 4
estimated_files: 8
---

# T03: Build PaymasterModule with deploy-count gating and signed-tx relay

**Slice:** S03 — Auth + Analysis + Paymaster Module
**Milestone:** M001

## Description

Build the PaymasterModule implementing the 3-free-deploy-then-WalletConnect pattern. PaymasterService manages deploy count tracking (Prisma User.deployCount), relay eligibility checks, and signed transaction broadcasting via ethers.js. PaymasterController exposes two JWT-protected endpoints: GET /api/paymaster/status and POST /api/paymaster/relay-signed. Both use JwtAuthGuard from T01's AuthModule.

This delivers requirements R008 (GitHub OAuth + Paymaster 3-deploy limit) and R013 (WalletConnect signed-tx relay).

**Relevant skills:** None specific — standard NestJS service patterns + ethers.js broadcast.

**Key context:**
- `JwtAuthGuard` is exported from AuthModule (T01) — import and use via `@UseGuards(JwtAuthGuard)`
- `PrismaModule` is `@Global()` — inject PrismaService directly
- User model has `deployCount` (int, default 0) and `githubId` (unique)
- `configService.get<string>('monad.rpcUrl')` for ethers.js provider (already in config from S01/S02)
- `TransformInterceptor` auto-wraps responses — controllers return raw data
- The `req.user` object from JwtAuthGuard contains `{ id, githubId, username }` (set by JwtStrategy.validate in T01)
- S02's DeployService defaults to userId='anonymous' which causes FK error — this task does NOT fix that FK issue but provides the infrastructure (userId from JWT, deploy count gating) that S05 will use to wire auth into deploy endpoints

## Steps

1. **Create PaymasterService:**
   - `backend/src/paymaster/paymaster.service.ts` — `@Injectable()`, injects PrismaService and ConfigService
   - `getDeployStatus(userId: string): Promise<DeployStatusDto>` — queries `prisma.user.findUnique({ where: { id: userId } })`, returns `{ used: user.deployCount, max: 3, remaining: Math.max(0, 3 - user.deployCount), canUseRelay: user.deployCount < 3 }`
   - `canUseRelay(userId: string): Promise<boolean>` — returns `deployCount < MAX_FREE_DEPLOYMENTS`
   - `incrementDeployCount(userId: string): Promise<number>` — `prisma.user.update({ where: { id: userId }, data: { deployCount: { increment: 1 } } })`, returns new count
   - `broadcastSignedTransaction(signedTxHex: string): Promise<{ txHash: string }>` — creates `new JsonRpcProvider(monad.rpcUrl)`, calls `provider.broadcastTransaction(signedTxHex)`, returns `{ txHash: response.hash }`. Wraps errors in BadRequestException with RPC error detail.
   - Constant: `MAX_FREE_DEPLOYMENTS = 3`
   - Logger for all operations

2. **Create DTOs:**
   - `backend/src/paymaster/dto/deploy-status.dto.ts` — `{ used: number, max: number, remaining: number, canUseRelay: boolean }`
   - `backend/src/paymaster/dto/relay-signed.dto.ts` — `{ signedTransaction: string }` with `@IsString()` and `@IsNotEmpty()` validators

3. **Create PaymasterController and Module:**
   - `backend/src/paymaster/paymaster.controller.ts`:
     - `GET /api/paymaster/status` — `@UseGuards(JwtAuthGuard)`, extracts `req.user.id`, calls `paymasterService.getDeployStatus(userId)`, returns DeployStatusDto
     - `POST /api/paymaster/relay-signed` — `@UseGuards(JwtAuthGuard)`, validates RelaySignedDto body, calls `paymasterService.broadcastSignedTransaction(dto.signedTransaction)`, returns `{ txHash }`
   - `backend/src/paymaster/paymaster.module.ts` — Imports `AuthModule` (for JwtAuthGuard/JwtStrategy). Provides PaymasterService. No need to import PrismaModule (global).
   - Add `PaymasterModule` to `backend/src/app.module.ts` imports

4. **Write unit tests:**
   - `backend/test/paymaster.service.spec.ts` (≥3 tests):
     - `getDeployStatus` returns correct remaining count
     - `canUseRelay` returns true when deployCount < 3, false when >= 3
     - `incrementDeployCount` calls Prisma update with increment
   - `backend/test/paymaster.controller.spec.ts` (≥2 tests):
     - Controller is defined
     - `getStatus` calls service with userId from req.user
   - Mock PrismaService and ConfigService. For controller tests, mock PaymasterService directly and provide a mock JwtAuthGuard (override canActivate to return true with mock user).

## Must-Haves

- [ ] PaymasterService.getDeployStatus() returns `{ used, max: 3, remaining, canUseRelay }`
- [ ] PaymasterService.canUseRelay() correctly gates at deployCount < 3
- [ ] PaymasterService.broadcastSignedTransaction() uses ethers.js JsonRpcProvider.broadcastTransaction()
- [ ] PaymasterController GET /api/paymaster/status is protected by JwtAuthGuard
- [ ] PaymasterController POST /api/paymaster/relay-signed is protected by JwtAuthGuard
- [ ] PaymasterModule registered in AppModule
- [ ] ≥5 unit tests passing

## Verification

- `cd backend && npm run build` — exit 0
- `cd backend && npm test -- --testPathPattern=paymaster` — ≥5 tests pass
- `grep -q "PaymasterModule" backend/src/app.module.ts` — module registered
- `grep -q "JwtAuthGuard" backend/src/paymaster/paymaster.controller.ts` — guard used
- `cd backend && npm test` — all suites pass (full regression, ≥49 tests total)

## Inputs

- `backend/src/auth/jwt-auth.guard.ts` — JwtAuthGuard from T01
- `backend/src/auth/auth.module.ts` — AuthModule exports for import
- `backend/src/prisma/prisma.service.ts` — PrismaService for User.deployCount
- `backend/prisma/schema.prisma` — User model with deployCount field
- `backend/src/config/configuration.ts` — monad.rpcUrl config key
- `backend/src/app.module.ts` — add PaymasterModule import

## Expected Output

- `backend/src/paymaster/paymaster.module.ts` — NestJS module importing AuthModule
- `backend/src/paymaster/paymaster.service.ts` — deploy count + relay service
- `backend/src/paymaster/paymaster.controller.ts` — 2 JWT-protected endpoints
- `backend/src/paymaster/dto/deploy-status.dto.ts` — status response type
- `backend/src/paymaster/dto/relay-signed.dto.ts` — relay request DTO
- `backend/src/app.module.ts` — updated with PaymasterModule import
- `backend/test/paymaster.service.spec.ts` — service unit tests
- `backend/test/paymaster.controller.spec.ts` — controller unit tests

## Observability Impact

- **New signals:** PaymasterService logs deploy status queries, relay eligibility checks, deploy count increments, and broadcast results/failures. All operations include userId for correlation.
- **Inspection surfaces:** `GET /api/paymaster/status` returns current deploy count, remaining free deploys, and relay eligibility for the authenticated user. User.deployCount in Prisma is the source of truth.
- **Failure visibility:** Broadcast failures surface as BadRequestException with RPC error detail in the response body and ERROR-level log. Missing users trigger NotFoundException (getDeployStatus) or silent false return (canUseRelay).
- **Redaction:** No secrets logged — monad.rpcUrl is a public testnet endpoint. Signed transaction hex is not logged (only broadcast result hash).

