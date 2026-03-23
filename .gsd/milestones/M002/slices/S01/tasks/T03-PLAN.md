---
estimated_steps: 4
estimated_files: 2
---

# T03: Rewrite page.tsx to compose IDE layout with existing features and update globals.css

**Slice:** S01 — Monaco Editor + IDE 레이아웃
**Milestone:** M002

## Description

This is the integration closure task. It rewrites `page.tsx` to compose the IDE layout with the Monaco editor and all existing features (contract selector, deploy, analyze, vibe-score, auth, wallet connect). Without this task, the Monaco wrapper and IDE layout components from T01/T02 are unused scaffolding.

The existing `page.tsx` is a ~200-line monolithic component with a `<textarea>` for Solidity input. The rewrite:
1. Replaces `<textarea>` with the `MonacoEditor` wrapper from T01
2. Wraps everything in `IDELayout` from T02
3. Distributes existing UI elements into panel slots (editor toolbar, sidebar content, console placeholder)
4. Updates `globals.css` for full-viewport IDE layout
5. Preserves ALL existing state management and API call logic — zero behavioral changes

## Steps

1. **Update `frontend/src/app/globals.css`:**
   Add full-viewport constraints so the IDE fills the screen without scrolling:
   ```css
   html, body {
     height: 100%;
     overflow: hidden;
   }
   ```
   Keep existing `@import "tailwindcss"` and body margin/padding reset.

2. **Rewrite `frontend/src/app/page.tsx`:**

   **Keep all existing imports and add new ones:**
   - Add: `import { IDELayout } from "@/components/ide/IDELayout"`
   - Add: `import { EditorPanel } from "@/components/ide/EditorPanel"`
   - Add: `import { SidebarPanel } from "@/components/ide/SidebarPanel"`
   - Add: `import { ConsolePanel } from "@/components/ide/ConsolePanel"`
   - Add: `import MonacoEditor from "@/components/ide/MonacoEditor"` (default import — dynamic export)
   - Keep all existing imports: `CodeDiffView`, `VibeStatus`, `VibeScoreGauge`, `getContractSource`, `deployContract`, `analyzeError`, `getVibeScore`, `getDeployStatus`, `useAuth`, `WalletConnectModal`

   **Keep all existing state and handlers:**
   - All `useState` declarations stay identical
   - `loadContractSource`, `handleDeploy`, `handleApplyFix`, `handleAnalyzeVibeScore`, `handleWalletDeploySuccess` — all unchanged
   - `CONTRACT_OPTIONS` — unchanged

   **New state for editor ref:**
   - Add: `const [editorInstance, setEditorInstance] = useState<any>(null)`
   - Add callback: `const handleEditorReady = (editor: any, monaco: any) => { setEditorInstance(editor); }`

   **Render structure — replace the entire return JSX:**
   ```
   IDELayout
   ├── editor slot: EditorPanel
   │   ├── toolbar: Header row with title, contract selector buttons, deploy/analyze buttons, auth controls, VibeStatus
   │   └── children: MonacoEditor with value={contractSource}, onChange={setContractSource}, onEditorReady={handleEditorReady}
   ├── sidebar slot: SidebarPanel
   │   ├── Deploy result (success/error display)
   │   ├── AI fix suggestions (CodeDiffView)
   │   └── VibeScoreGauge
   └── console slot: ConsolePanel
       └── Placeholder: "Console output will appear here" (S02 adds TransactionConsole)
   ```

   **Editor toolbar layout (inside EditorPanel toolbar prop):**
   - Left: App title "Monad Vibe-Loom" (smaller, inline)
   - Center: Contract selector buttons (same `CONTRACT_OPTIONS` map as before)
   - Right: Deploy button, Vibe Score button, auth controls (login/logout + VibeStatus)

   **Sidebar content:**
   - Move deploy result display (green success box, red error box) here
   - Move AI fix suggestions (CodeDiffView + retry button) here
   - Move VibeScoreGauge here
   - All conditional rendering logic stays the same, just positioned inside SidebarPanel

   **Console content:**
   - Simple placeholder: `<div className="p-4 text-gray-500 text-sm font-mono">Console output will appear here...</div>`
   - S02 will replace this with TransactionConsole

   **Monaco onChange handler:**
   - When Monaco value changes, update `contractSource` state:
     ```tsx
     const handleEditorChange = (value: string | undefined) => {
       setContractSource(value ?? "");
     };
     ```
   - Pass `handleEditorChange` as the `onChange` prop to MonacoEditor

   **WalletConnectModal:**
   - Keep it rendered at the bottom of the component, outside IDELayout (it's a modal overlay)

3. **Verify existing features still work:**
   - Contract selector buttons should call `loadContractSource` which calls `getContractSource()` API
   - Deploy button should call `handleDeploy` which calls `deployContract()` API
   - Vibe Score button should call `handleAnalyzeVibeScore` which calls `getVibeScore()` API
   - Auth login/logout should work via `useAuth()` hook
   - WalletConnect modal should appear when `showWalletModal` is true

4. **Build verification:**
   ```bash
   cd frontend && npm run build
   ```
   Must exit 0 with no SSR errors, no hydration warnings in build output.

## Must-Haves

- [ ] `page.tsx` uses `IDELayout`, `EditorPanel`, `SidebarPanel`, `ConsolePanel`, `MonacoEditor`
- [ ] `<textarea>` removed — replaced by `MonacoEditor`
- [ ] Contract selector still loads source via `getContractSource()` API
- [ ] Deploy button still triggers `deployContract()` flow
- [ ] Vibe Score button still triggers `getVibeScore()` flow
- [ ] Auth (login/logout) rendered in toolbar
- [ ] WalletConnect modal renders on quota exceeded
- [ ] `globals.css` has `height: 100%` and `overflow: hidden` on html/body
- [ ] `npm run build` succeeds

## Verification

- `cd frontend && npm run build` exits 0
- `grep -q "IDELayout" frontend/src/app/page.tsx` — uses IDELayout
- `grep -q "MonacoEditor" frontend/src/app/page.tsx` — uses MonacoEditor
- `! grep -q "<textarea" frontend/src/app/page.tsx` — textarea removed
- `grep -q "deployContract" frontend/src/app/page.tsx` — deploy flow preserved
- `grep -q "getVibeScore" frontend/src/app/page.tsx` — vibe-score flow preserved
- `grep -q "useAuth" frontend/src/app/page.tsx` — auth flow preserved
- `grep -q "overflow.*hidden\|overflow: hidden" frontend/src/app/globals.css` — full-viewport CSS

## Inputs

- `frontend/src/app/page.tsx` — current monolithic page with textarea (full rewrite target)
- `frontend/src/app/globals.css` — current minimal CSS
- `frontend/src/components/ide/MonacoEditor.tsx` — Monaco wrapper from T01
- `frontend/src/components/ide/IDELayout.tsx` — IDE layout from T02
- `frontend/src/components/ide/EditorPanel.tsx` — editor panel from T02
- `frontend/src/components/ide/SidebarPanel.tsx` — sidebar panel from T02
- `frontend/src/components/ide/ConsolePanel.tsx` — console panel from T02
- `frontend/src/lib/api-client.ts` — API methods (unchanged, consumed by page.tsx)
- `frontend/src/lib/auth-context.tsx` — auth hook (unchanged, consumed by page.tsx)
- `frontend/src/components/VibeStatus.tsx` — paymaster status badge (unchanged, mounted in toolbar)
- `frontend/src/components/VibeScoreGauge.tsx` — vibe-score gauge (unchanged, mounted in sidebar)
- `frontend/src/components/CodeDiffView.tsx` — code diff view (unchanged, mounted in sidebar)
- `frontend/src/components/WalletConnectModal.tsx` — wallet modal (unchanged, mounted as overlay)

## Observability Impact

- **New signal:** IDE layout renders a 3-panel resizable surface — React DevTools shows `VibeLoomPage > IDELayout > Group > Panel > EditorPanel / SidebarPanel / ConsolePanel` hierarchy. Verify via React DevTools Components tab.
- **Changed signal:** Page no longer scrolls (full-viewport CSS). Document `overflow: hidden` on html/body removes the scrollbar — confirm by inspecting `document.documentElement.style` or by the absence of scroll affordance.
- **Preserved signals:** All existing API call patterns (console.error logging in api-client) are unchanged. Contract selector, deploy, vibe-score, and auth flows fire the same requests as before — visible in browser Network tab.
- **Monaco lazy load:** The `next/dynamic` wrapper defers Monaco's ~630KB chunk. Network tab should show the chunk loading on first page render, not in the initial bundle. If the chunk fails to load, the "Loading editor..." fallback persists indefinitely.
- **Failure state:** If IDELayout crashes (e.g., react-resizable-panels SSR issue), the entire page goes blank — React error boundary or Next.js error overlay would activate. This is visible as a white screen with console errors.

## Expected Output

- `frontend/src/app/page.tsx` — rewritten to compose IDE layout with existing features
- `frontend/src/app/globals.css` — updated with full-viewport constraints
