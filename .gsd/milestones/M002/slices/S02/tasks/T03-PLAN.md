---
estimated_steps: 4
estimated_files: 3
---

# T03: Create AIDiffViewer with Monaco DiffEditor and replace CodeDiffView in page.tsx

**Slice:** S02 — 컴파일·배포 UX + 트랜잭션 콘솔
**Milestone:** M002

## Description

Replace the text-based `CodeDiffView` component with a proper Monaco `DiffEditor` for AI fix suggestions. The `DiffEditor` component from `@monaco-editor/react` provides syntax-highlighted, character-level diff visualization — a significant UX improvement over the current line-by-line text comparison.

Uses the same **two-file SSR-safe pattern** established in S01 for MonacoEditor:
- `AIDiffViewerInner.tsx` — the actual DiffEditor implementation (imports `DiffEditor` from `@monaco-editor/react`)
- `AIDiffViewer.tsx` — thin `next/dynamic({ ssr: false })` wrapper

The `DiffEditor` component shares Monaco's core (already loaded by MonacoEditor), so no additional bundle size impact.

**Key constraints:**
- `DiffEditor` needs explicit height (not `height="100%"` unless parent has explicit height). Use `height="300px"` for the sidebar context.
- `DiffEditor` also depends on `window`/`document` — same SSR crash risk as `Editor`. Must use `next/dynamic({ ssr: false })`.
- `@monaco-editor/react` already exports `DiffEditor` — no new npm packages needed.

## Steps

1. Create `frontend/src/components/ide/AIDiffViewerInner.tsx`:
   - Import `DiffEditor` from `@monaco-editor/react`
   - Props interface `AIDiffViewerProps`: `{ original: string, modified: string, summary?: string, onApplyFix?: (fixedCode: string) => void }`
   - Render:
     - Optional summary banner at top (amber bg, same style as CodeDiffView's summary)
     - `<DiffEditor>` with: `height="300px"`, `language="sol"`, `theme="vs-dark"`, `original={original}`, `modified={modified}`, `options={{ readOnly: true, renderSideBySide: true, minimap: { enabled: false }, scrollBeyondLastLine: false }}`
     - "Apply Fix" button below the diff (emerald-600 bg, same style as CodeDiffView's button) — calls `onApplyFix(modified)` when clicked. Only shown when `original !== modified && onApplyFix` is provided
   - Loading fallback inside DiffEditor: `<div className="flex items-center justify-center h-[300px] text-gray-500">Loading diff...</div>`

2. Create `frontend/src/components/ide/AIDiffViewer.tsx`:
   - Same pattern as `frontend/src/components/ide/MonacoEditor.tsx`
   - `import dynamic from 'next/dynamic'`
   - `export type { AIDiffViewerProps } from './AIDiffViewerInner'`
   - Dynamic import: `const AIDiffViewer = dynamic(() => import('./AIDiffViewerInner').then(mod => ({ default: mod.AIDiffViewerInner })), { ssr: false, loading: () => <div>Loading diff...</div> })`
   - `export default AIDiffViewer`

3. Update `frontend/src/app/page.tsx` to use AIDiffViewer:
   - Remove `import { CodeDiffView } from "../components/CodeDiffView"` 
   - Add `import AIDiffViewer from "@/components/ide/AIDiffViewer"`
   - In the sidebar content where `<CodeDiffView>` is used, replace with:
     ```jsx
     <AIDiffViewer
       original={errorDiff.original}
       modified={errorDiff.fixed}
       summary={errorDiff.summary}
       onApplyFix={handleApplyFix}
     />
     ```
   - The `handleApplyFix` callback already exists and sets `contractSource` to the fixed code — no changes needed there

4. Verify the build:
   - `cd frontend && npm run build` must exit 0
   - Confirm no SSR errors related to DiffEditor
   - Confirm `CodeDiffView` is no longer imported in page.tsx
   - Note: `CodeDiffView.tsx` file is kept (not deleted) in case other files reference it — but page.tsx no longer uses it

## Must-Haves

- [ ] `AIDiffViewerInner.tsx` renders Monaco `DiffEditor` with sol language and vs-dark theme
- [ ] `AIDiffViewer.tsx` uses `next/dynamic({ ssr: false })` for SSR safety
- [ ] `page.tsx` imports `AIDiffViewer` instead of `CodeDiffView`
- [ ] "Apply Fix" button works — calls `onApplyFix(modified)` to update editor source
- [ ] Summary banner displays when provided
- [ ] `npm run build` exits 0 with no SSR errors

## Verification

- `cd frontend && npm run build` exits 0
- `grep -q "AIDiffViewer" frontend/src/app/page.tsx` — new component used
- `! grep -q "CodeDiffView" frontend/src/app/page.tsx` — old component removed from page
- `grep -q "ssr: false" frontend/src/components/ide/AIDiffViewer.tsx` — SSR-safe pattern
- `grep -q "DiffEditor" frontend/src/components/ide/AIDiffViewerInner.tsx` — uses Monaco DiffEditor
- `test -f frontend/src/components/ide/AIDiffViewer.tsx` — wrapper exists
- `test -f frontend/src/components/ide/AIDiffViewerInner.tsx` — implementation exists
- `cd frontend && npm test` — all tests still pass

## Inputs

- `frontend/src/components/ide/MonacoEditor.tsx` — SSR-safe wrapper pattern to replicate
- `frontend/src/components/CodeDiffView.tsx` — existing text diff component being replaced (reference for props/behavior)
- `frontend/src/app/page.tsx` — page with CodeDiffView usage to replace (modified by T02)

## Expected Output

- `frontend/src/components/ide/AIDiffViewer.tsx` — new SSR-safe DiffEditor wrapper
- `frontend/src/components/ide/AIDiffViewerInner.tsx` — new DiffEditor implementation
- `frontend/src/app/page.tsx` — modified: CodeDiffView replaced with AIDiffViewer

## Observability Impact

- **New signal**: Monaco DiffEditor renders syntax-highlighted side-by-side diff in sidebar when AI fix is suggested — replaces plain-text CodeDiffView
- **Inspection**: React DevTools shows `errorDiff` state (original/fixed/summary) in VibeLoomPage; DiffEditor content visible in browser sidebar panel
- **Failure state**: If DiffEditor fails to load (SSR or runtime), the `loading` fallback ("Loading diff...") remains visible; Apply Fix button only appears when original ≠ modified
- **No new logs/metrics**: This is a pure UI swap — no new console logs, API calls, or telemetry added
