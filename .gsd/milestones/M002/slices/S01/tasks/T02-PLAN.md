---
estimated_steps: 4
estimated_files: 4
---

# T02: Create IDE 3-panel resizable layout components

**Slice:** S01 — Monaco Editor + IDE 레이아웃
**Milestone:** M002

## Description

Build the IDE layout shell using `react-resizable-panels` v4 (Group/Panel/Separator API). This creates four components: `IDELayout` (top-level composition), `EditorPanel`, `SidebarPanel`, and `ConsolePanel`. Each panel accepts `children` props for composition — the actual content (Monaco editor, deploy results, console logs) is slotted in by the parent page in T03.

The layout structure is:
```
┌──────────────────────────┬─────────────┐
│                          │             │
│      EditorPanel         │  Sidebar    │
│      (Monaco editor +    │  Panel      │
│       toolbar)           │  (deploy    │
│                          │   results,  │
├──────────────────────────┤   vibe)     │
│      ConsolePanel        │             │
│      (logs, tx console)  │             │
└──────────────────────────┴─────────────┘
```

Horizontal Group: [Editor area (grows) | Separator | Sidebar (25%, collapsible, min 200px)]
Vertical Group in editor area: [EditorPanel (70%) | Separator | ConsolePanel (30%, collapsible, min 100px)]

## Steps

1. **Create `frontend/src/components/ide/EditorPanel.tsx`:**
   - A simple panel wrapper that accepts `children: React.ReactNode` and optional `toolbar?: React.ReactNode`
   - Renders a flex column container: toolbar area at top (if provided), children fill remaining space
   - Uses `h-full overflow-hidden` to let Monaco fill the available space
   - Dark theme styling: `bg-gray-900 text-gray-100`

2. **Create `frontend/src/components/ide/SidebarPanel.tsx`:**
   - Accepts `children: React.ReactNode` and optional `title?: string`
   - Renders a scrollable container with sidebar styling
   - Dark theme: `bg-gray-800 border-l border-gray-700`
   - `overflow-y-auto` for content that exceeds panel height

3. **Create `frontend/src/components/ide/ConsolePanel.tsx`:**
   - Accepts `children: React.ReactNode`
   - Renders a container with console styling and a header bar ("Console" label)
   - Dark theme: `bg-gray-900 border-t border-gray-700`
   - `overflow-y-auto` for scrolling log output
   - Header: `bg-gray-800 px-3 py-1 text-xs text-gray-400 font-mono` with "Console" text

4. **Create `frontend/src/components/ide/IDELayout.tsx`:**
   - Import `{ Group, Panel, Separator }` from `"react-resizable-panels"`
   - Accept props: `editor: React.ReactNode`, `sidebar: React.ReactNode`, `console: React.ReactNode`
   - Structure:
     ```tsx
     <div className="h-screen w-screen overflow-hidden bg-gray-900">
       <Group direction="horizontal">
         <Panel defaultSize={75} minSize={40}>
           <Group direction="vertical">
             <Panel defaultSize={70} minSize={30}>
               {editor}
             </Panel>
             <Separator className="h-1 bg-gray-700 hover:bg-amber-500 transition-colors" />
             <Panel defaultSize={30} minSize={10} collapsible>
               {console}
             </Panel>
           </Group>
         </Panel>
         <Separator className="w-1 bg-gray-700 hover:bg-amber-500 transition-colors" />
         <Panel defaultSize={25} minSize={15} collapsible>
           {sidebar}
         </Panel>
       </Group>
     </div>
     ```
   - Separator styling: thin line (1px via w-1/h-1) with hover highlight (amber) to indicate draggability
   - Export `IDELayout` as named export and also as default

## Must-Haves

- [ ] `IDELayout.tsx` uses `Group`, `Panel`, `Separator` from `react-resizable-panels`
- [ ] Horizontal Group: editor area + sidebar, with Separator between
- [ ] Vertical Group inside editor area: editor panel + console panel, with Separator between
- [ ] Sidebar panel is collapsible with `minSize` set
- [ ] Console panel is collapsible with `minSize` set
- [ ] All panels accept `children` via props for composition
- [ ] All components marked `"use client"`
- [ ] `npm run build` succeeds

## Verification

- `cd frontend && npm run build` exits 0
- `test -f frontend/src/components/ide/IDELayout.tsx` — file exists
- `test -f frontend/src/components/ide/EditorPanel.tsx` — file exists
- `test -f frontend/src/components/ide/SidebarPanel.tsx` — file exists
- `test -f frontend/src/components/ide/ConsolePanel.tsx` — file exists
- `grep -q "Group" frontend/src/components/ide/IDELayout.tsx` — uses Group component
- `grep -q "Separator" frontend/src/components/ide/IDELayout.tsx` — uses Separator component
- `grep -q "collapsible" frontend/src/components/ide/IDELayout.tsx` — panels are collapsible

## Inputs

- `frontend/package.json` — must have `react-resizable-panels` installed (from T01)
- `frontend/src/components/ide/MonacoEditor.tsx` — exists from T01 (not used directly, but confirms ide/ directory exists)

## Expected Output

- `frontend/src/components/ide/IDELayout.tsx` — new 3-panel resizable layout
- `frontend/src/components/ide/EditorPanel.tsx` — new editor area panel wrapper
- `frontend/src/components/ide/SidebarPanel.tsx` — new sidebar panel wrapper
- `frontend/src/components/ide/ConsolePanel.tsx` — new console panel wrapper
