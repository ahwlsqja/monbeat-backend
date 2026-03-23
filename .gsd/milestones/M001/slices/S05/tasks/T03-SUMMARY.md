---
id: T03
parent: S05
milestone: M001
provides:
  - WalletConnect/wagmi integration with Monad Testnet chain (chainId 10143)
  - WalletConnectModal component for direct wallet deploys when canUseRelay=false
  - Deploy flow gating — checks paymaster status before server deploy
  - compileContract API method for wallet-based bytecode deployment
  - Jest + React Testing Library test setup with next/jest preset
  - 11 unit tests covering API client envelope unwrapping, auth headers, and field mapping
key_files:
  - frontend/src/lib/wagmi-config.ts
  - frontend/src/components/WalletConnectModal.tsx
  - frontend/src/app/page.tsx
  - frontend/src/app/providers.tsx
  - frontend/src/lib/api-client.ts
  - frontend/jest.config.js
  - frontend/jest.setup.js
  - frontend/src/__tests__/api-client.test.ts
key_decisions:
  - Used wagmi v2 with native injected + walletConnect connectors instead of deprecated @web3modal/wagmi (now @reown/appkit)
  - Cast walletConnect connector to CreateConnectorFn to resolve type incompatibility between injected and walletConnect storage types
patterns_established:
  - WalletConnect connector conditionally added only when NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID is set
  - Deploy gating pattern — check paymaster status first, show wallet modal if canUseRelay=false
  - Jest tests mock global fetch and test API client methods through their public interface
observability_surfaces:
  - console.warn for missing WalletConnect project ID
  - console.error for wallet deploy failures with error details
  - npm test runs 11 API client contract tests as slice verification
duration: 14m
verification_result: passed
completed_at: 2026-03-22
blocker_discovered: false
---

# T03: Add WalletConnect integration + unit tests

**Added wagmi/viem WalletConnect integration with Monad Testnet chain, deploy flow gating on canUseRelay, WalletConnectModal component, and 11 API client unit tests covering envelope unwrapping, auth injection, and field mapping.**

## What Happened

Installed wagmi, viem, @tanstack/react-query, and @walletconnect/ethereum-provider as runtime dependencies. Created `wagmi-config.ts` defining Monad Testnet chain (id 10143, MON currency, RPC from env) and configuring wagmi with injected + WalletConnect connectors. The WalletConnect connector is conditionally added only when `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID` is set.

Created `WalletConnectModal.tsx` component with four UI states: (1) not connected — shows available connectors, (2) connected — shows truncated address + "Deploy with Wallet" button, (3) deploying — loading state through compile→sendTransaction→waitForReceipt, (4) success — shows tx hash. The modal compiles contract source via the API client to get bytecode, then uses wagmi's `sendTransaction` to deploy directly from the user's wallet.

Added `compileContract` method to `api-client.ts` for `POST /api/contracts/compile` with source body and auth headers.

Modified `page.tsx` deploy flow: before deploying, if authenticated, calls `getDeployStatus()`. When `canUseRelay` is false, opens WalletConnectModal instead of proceeding with server-side deploy. Added `handleWalletDeploySuccess` callback that closes the modal and shows deploy result.

Updated `providers.tsx` to wrap the component tree with `WagmiProvider` + `QueryClientProvider` inside the existing `AuthProvider`. The wagmi config is created once at module scope to avoid re-renders.

Set up Jest with `next/jest` preset, jsdom test environment, and `@testing-library/jest-dom` setup. Wrote 11 unit tests in `api-client.test.ts` covering: envelope unwrapping (success, error, non-ok HTTP), correct URL/path for each endpoint, `{ source }` field mapping (not `{ code }`), `vibeScore` return field (not `score`), `{ error, contractSource }` mapping for analyzeError, auth header injection from localStorage, and no auth header when token absent.

## Verification

- `cd frontend && npm run build` — exits 0 with zero TypeScript errors
- `cd frontend && npm test` — 11 tests pass across 7 test groups
- `grep -q "10143" frontend/src/lib/wagmi-config.ts` — Monad testnet chain ID present
- `grep -q "canUseRelay" frontend/src/app/page.tsx` — deploy gating logic present
- `grep -r "NEXT_PUBLIC_API_URL" frontend/src/` — 3 matches in api-client.ts and auth-context.tsx
- `! grep -rn "fetch(\"/api/" frontend/src/` — no hardcoded fetch calls remain
- `cd backend && npm test` — 97 tests pass (14 suites), no regressions

### Slice-level verification (T03 of 3 — final task):
- ✅ `cd frontend && npm run build` — zero TypeScript/build errors
- ✅ `cd frontend && npm test` — 11 API client unit tests pass (envelope unwrapping, auth header injection, field mapping)
- ✅ `cd backend && npm test` — 97 tests pass (14 suites, no regressions)
- ✅ `grep -r "NEXT_PUBLIC_API_URL" frontend/src/` — all API calls use environment variable-based base URL
- ✅ `! grep -rn "fetch(\"/api/" frontend/src/` — no hardcoded `/api/` fetch calls remain

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd frontend && npm run build` | 0 | ✅ pass | 35.8s |
| 2 | `cd frontend && npm test` | 0 | ✅ pass | 0.7s |
| 3 | `grep -q "10143" frontend/src/lib/wagmi-config.ts` | 0 | ✅ pass | <1s |
| 4 | `grep -q "canUseRelay" frontend/src/app/page.tsx` | 0 | ✅ pass | <1s |
| 5 | `grep -r "NEXT_PUBLIC_API_URL" frontend/src/` | 0 | ✅ pass | <1s |
| 6 | `grep -rn 'fetch("/api/' frontend/src/` (expect no matches) | 1 | ✅ pass | <1s |
| 7 | `cd backend && npm test` | 0 | ✅ pass | 9.4s |

## Diagnostics

- **Wagmi config health:** `grep "10143" frontend/src/lib/wagmi-config.ts` confirms Monad Testnet chain ID
- **WalletConnect status:** Missing project ID logged to `console.warn` with `[wagmi-config]` prefix in browser DevTools
- **Deploy gating inspection:** Network tab shows `GET /api/paymaster/status` call before deploy attempts — `canUseRelay` in response body determines wallet vs server deploy
- **Wallet deploy failures:** `console.error('[WalletConnectModal] compile/deploy failed: ...')` with error details
- **Test contract verification:** `npm test` in frontend validates all 5 API client methods match NestJS backend contract
- **API client observability:** Existing `console.error('[api-client] ...')` logging preserved for all endpoint failures

## Deviations

- Used wagmi's built-in `walletConnect` connector instead of the `@web3modal/wagmi` package specified in the task plan. The `@web3modal/wagmi` package is deprecated (now `@reown/appkit`), and wagmi's native connector provides the same WalletConnect functionality without the deprecated dependency.
- Updated `providers.tsx` instead of `layout.tsx` for WagmiProvider wrapping, since `providers.tsx` was already the client-side wrapper component created in T02 to keep `layout.tsx` as a server component.
- Cast `walletConnect()` connector to `CreateConnectorFn` type to resolve a generic type parameter mismatch between `injected()` and `walletConnect()` storage types in wagmi's TypeScript definitions.

## Known Issues

- WalletConnect QR modal requires a valid `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID` from WalletConnect Cloud dashboard — without it, only injected wallets (MetaMask, etc.) are available.
- Jest config warning about workspace root due to multiple lockfiles (frontend/ and root) — harmless, Next.js auto-detects correctly.

## Files Created/Modified

- `frontend/src/lib/wagmi-config.ts` — Wagmi configuration with Monad Testnet chain (10143), injected + WalletConnect connectors
- `frontend/src/components/WalletConnectModal.tsx` — Modal for wallet connection and direct wallet deploys
- `frontend/src/app/page.tsx` — Added deploy gating on canUseRelay, WalletConnectModal rendering, handleWalletDeploySuccess
- `frontend/src/app/providers.tsx` — Added WagmiProvider + QueryClientProvider wrapping
- `frontend/src/lib/api-client.ts` — Added CompileResult interface and compileContract method
- `frontend/jest.config.js` — Jest configuration with next/jest preset and jsdom environment
- `frontend/jest.setup.js` — Jest setup importing @testing-library/jest-dom
- `frontend/src/__tests__/api-client.test.ts` — 11 unit tests for API client (envelope, auth, field mapping)
- `frontend/package.json` — Updated test script, added wagmi/viem/testing dependencies
