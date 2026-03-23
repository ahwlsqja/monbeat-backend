---
estimated_steps: 5
estimated_files: 8
---

# T03: Add WalletConnect integration + unit tests

**Slice:** S05 — Frontend Integration
**Milestone:** M001

## Description

Add WalletConnect/wagmi/viem integration for wallet connection when deploys exceed 3 (R013 frontend support). Modify the deploy flow to check paymaster status before deploying — if `canUseRelay` is false, show a WalletConnect modal for the user to connect their wallet and deploy directly. Also set up Jest + React Testing Library and write unit tests for the API client to verify the slice's core contract (envelope unwrapping, auth header injection, field name mapping).

**Monad Testnet chain config:**
- Chain ID: 10143
- RPC: `https://testnet-rpc.monad.xyz` (or from `NEXT_PUBLIC_MONAD_RPC_URL` env)
- Explorer: `https://testnet.monadexplorer.com`
- Currency: MON (18 decimals)

**WalletConnect project ID:** Required from `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID` env var. If not set, WalletConnect modal won't initialize — the deploy flow should fall back to showing a message asking the user to configure it.

## Steps

1. Install WalletConnect + wagmi dependencies:
   ```bash
   cd frontend && npm install wagmi viem @tanstack/react-query @web3modal/wagmi @walletconnect/modal
   ```

2. Create `frontend/src/lib/wagmi-config.ts`:
   - Define Monad testnet chain: `{ id: 10143, name: 'Monad Testnet', nativeCurrency: { name: 'MON', symbol: 'MON', decimals: 18 }, rpcUrls: { default: { http: ['https://testnet-rpc.monad.xyz'] } }, blockExplorers: { default: { name: 'Monad Explorer', url: 'https://testnet.monadexplorer.com' } } }`
   - Export wagmi config with `defaultWagmiConfig` from `@web3modal/wagmi` using `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID`
   - Export `projectId` and `metadata` for Web3Modal

3. Create `frontend/src/components/WalletConnectModal.tsx`:
   - Props: `isOpen: boolean`, `onClose: () => void`, `onDeploySuccess: (address: string, txHash: string) => void`, `contractSource: string`
   - Uses wagmi hooks: `useAccount()`, `useConnect()`, `useSendTransaction()`, `useWaitForTransactionReceipt()`
   - UI states: (a) Not connected → show `<w3m-button>` or `useWeb3Modal().open()` to connect wallet, (b) Connected → show address + "Deploy with Wallet" button, (c) Deploying → loading state, (d) Success → show tx hash
   - On "Deploy with Wallet" click: call api-client `compileContract(contractSource)` to get bytecode, then use wagmi `sendTransaction({ data: bytecode })` to deploy from user's wallet
   - Note: The compile endpoint is `POST /api/contracts/compile` body `{ source }` — add this method to api-client if not already present
   - On success: call `onDeploySuccess(address, txHash)` and close modal

4. Modify `frontend/src/app/page.tsx` deploy flow:
   - Add state: `showWalletModal: boolean`
   - In `handleDeploy`: first call `getDeployStatus()` (if authenticated). If `canUseRelay` is false, set `showWalletModal = true` and return (don't proceed with server deploy). If `canUseRelay` is true or user is not authenticated, proceed with current server deploy flow.
   - Render `<WalletConnectModal>` when `showWalletModal` is true

5. Modify `frontend/src/app/layout.tsx`:
   - Import wagmi config and providers
   - Wrap app with `<WagmiProvider config={wagmiConfig}><QueryClientProvider><Web3Modal>` (inside existing AuthProvider)
   - Handle case where `NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID` is not set — skip WagmiProvider wrapping (SSR-safe check)

6. Set up Jest for frontend testing:
   - Create `frontend/jest.config.js` with Next.js preset (`next/jest`)
   - Install dev dependencies: `@testing-library/react`, `@testing-library/jest-dom`, `jest`, `jest-environment-jsdom`
   - Create `frontend/jest.setup.js` importing `@testing-library/jest-dom`

7. Write `frontend/src/__tests__/api-client.test.ts`:
   - Mock global `fetch` with jest.fn()
   - Test `unwrapResponse` with valid `{ success: true, data: { foo: 'bar' } }` → returns `{ foo: 'bar' }`
   - Test `unwrapResponse` with `{ success: false, error: 'msg' }` → throws
   - Test `unwrapResponse` with non-ok response → throws with status code
   - Test `getContractSource('FixedContract')` → calls correct URL, returns unwrapped data
   - Test `deployContract(source)` → calls `/api/contracts/deploy` with `{ source }` body
   - Test `getVibeScore(source)` → sends `{ source }` not `{ code }`, returns `vibeScore` not `score`
   - Test `analyzeError(error, source)` → calls `/api/analysis/error` with `{ error, contractSource: source }`
   - Test `getDeployStatus()` → calls `/api/paymaster/status` with auth header
   - Test auth header injection: set `localStorage.setItem('vibe-loom-token', 'test-jwt')`, verify `Authorization: Bearer test-jwt` header sent

## Must-Haves

- [ ] wagmi + viem + @web3modal/wagmi installed
- [ ] Monad testnet chain defined with correct chainId 10143
- [ ] WalletConnectModal component shows wallet connection UI
- [ ] Deploy flow checks paymaster status and gates on `canUseRelay`
- [ ] Jest configured with next/jest preset
- [ ] API client unit tests cover envelope unwrapping, auth headers, and field name mapping
- [ ] `npm run build` exits 0
- [ ] `npm test` passes

## Verification

- `cd frontend && npm run build` exits 0
- `cd frontend && npm test` — all API client tests pass
- `grep -q "10143" frontend/src/lib/wagmi-config.ts` — Monad testnet chain ID present
- `grep -q "canUseRelay" frontend/src/app/page.tsx` — deploy gating logic present

## Inputs

- `frontend/src/lib/api-client.ts` — API client to potentially extend with compile method (from T02)
- `frontend/src/app/page.tsx` — page component to modify deploy flow (from T02)
- `frontend/src/app/layout.tsx` — layout to wrap with WagmiProvider (from T02)
- `frontend/src/lib/auth-context.tsx` — auth context for checking authentication state (from T02)

## Expected Output

- `frontend/src/lib/wagmi-config.ts` — wagmi configuration with Monad testnet chain
- `frontend/src/components/WalletConnectModal.tsx` — wallet connection modal component
- `frontend/src/app/page.tsx` — updated with wallet deploy flow gating
- `frontend/src/app/layout.tsx` — updated with WagmiProvider wrapping
- `frontend/jest.config.js` — Jest configuration
- `frontend/jest.setup.js` — Jest setup with testing-library
- `frontend/src/__tests__/api-client.test.ts` — API client unit tests
- `frontend/package.json` — updated with wagmi, viem, testing dependencies

## Observability Impact

- **New signal:** `console.warn('[wagmi-config] NEXT_PUBLIC_WALLETCONNECT_PROJECT_ID is not set')` when WalletConnect env var is missing — visible in browser DevTools console
- **New signal:** `console.error('[WalletConnectModal] compile/deploy failed: ...')` when wallet deploy fails — includes error details for debugging
- **Deploy flow inspection:** `canUseRelay` check in `handleDeploy` can be observed in Network tab — look for `GET /api/paymaster/status` call before deploy attempts
- **Wallet connection state:** wagmi state accessible via React DevTools (WagmiProvider context) — shows connected address, chain, connector
- **Test health:** `npm test` in frontend runs 11 API client tests covering envelope unwrapping, auth injection, and field mapping — acts as contract verification against backend API
- **Failure state:** When `canUseRelay=false`, WalletConnectModal becomes visible in the DOM — no wallet connection needed for the modal to appear, just the gating check
