---
estimated_steps: 5
estimated_files: 11
---

# T01: Scaffold frontend/ Next.js project with Vibe-Loom code

**Slice:** S05 — Frontend Integration
**Milestone:** M001

## Description

Create a standalone Next.js project in `frontend/` by initializing package.json, tsconfig.json, Tailwind CSS, and copying the 4 source files + 3 components from Vibe-Loom (`/tmp/vibe-loom/src/`). The goal is a buildable Next.js 15 project — no API logic changes yet. The existing fetch calls will reference `/api/*` paths that don't exist in the Next.js app (they've moved to NestJS), but TypeScript compilation must succeed.

## Steps

1. Create `frontend/package.json` with Next.js 15, React 19, Tailwind CSS v4, and TypeScript dependencies. Scripts: `dev`, `build`, `start`, `test`. Set `"name": "vibe-room-frontend"`.
   - Dependencies: `next@^15`, `react@^19`, `react-dom@^19`
   - DevDependencies: `typescript`, `@types/react`, `@types/node`, `tailwindcss`, `@tailwindcss/postcss`, `postcss`

2. Create `frontend/tsconfig.json` matching Next.js 15 defaults:
   ```json
   {
     "compilerOptions": {
       "target": "ES2017",
       "lib": ["dom", "dom.iterable", "esnext"],
       "allowJs": true,
       "skipLibCheck": true,
       "strict": true,
       "noEmit": true,
       "esModuleInterop": true,
       "module": "esnext",
       "moduleResolution": "bundler",
       "resolveJsonModule": true,
       "isolatedModules": true,
       "jsx": "preserve",
       "incremental": true,
       "plugins": [{ "name": "next" }],
       "paths": { "@/*": ["./src/*"] }
     },
     "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx", ".next/types/**/*.ts"],
     "exclude": ["node_modules"]
   }
   ```

3. Create `frontend/next.config.js` (minimal):
   ```js
   /** @type {import('next').NextConfig} */
   const nextConfig = {};
   module.exports = nextConfig;
   ```

4. Create `frontend/postcss.config.js`:
   ```js
   module.exports = { plugins: { "@tailwindcss/postcss": {} } };
   ```

5. Copy source files from Vibe-Loom to frontend:
   - `/tmp/vibe-loom/src/app/page.tsx` → `frontend/src/app/page.tsx`
   - `/tmp/vibe-loom/src/app/layout.tsx` → `frontend/src/app/layout.tsx`
   - `/tmp/vibe-loom/src/app/globals.css` → `frontend/src/app/globals.css` (update Tailwind directives to v4: `@import "tailwindcss"` replaces the 3 `@tailwind` directives)
   - `/tmp/vibe-loom/src/components/VibeScoreGauge.tsx` → `frontend/src/components/VibeScoreGauge.tsx`
   - `/tmp/vibe-loom/src/components/CodeDiffView.tsx` → `frontend/src/components/CodeDiffView.tsx`
   - `/tmp/vibe-loom/src/components/VibeStatus.tsx` → `frontend/src/components/VibeStatus.tsx`
   - Copy files as-is — no logic changes.

6. Create `frontend/.env.local.example` with `NEXT_PUBLIC_API_URL=http://localhost:3000` as documentation for the env var pattern.

7. Run `cd frontend && npm install && npm run build` to verify.

## Must-Haves

- [ ] `frontend/package.json` exists with next, react, react-dom, tailwindcss, typescript
- [ ] `frontend/tsconfig.json` exists with Next.js-compatible settings
- [ ] `frontend/src/app/page.tsx` exists (copied from Vibe-Loom)
- [ ] `frontend/src/app/layout.tsx` exists (copied from Vibe-Loom)
- [ ] `frontend/src/app/globals.css` exists with Tailwind v4 import
- [ ] 3 component files exist in `frontend/src/components/`
- [ ] `cd frontend && npm run build` exits 0

## Verification

- `cd frontend && npm run build` exits 0 — zero TypeScript errors
- `test -f frontend/package.json && test -f frontend/tsconfig.json` — config files exist
- `test -f frontend/src/app/page.tsx && test -f frontend/src/components/VibeScoreGauge.tsx` — source files copied

## Observability Impact

- **Build signal:** `cd frontend && npm run build` exit code reflects whether the Next.js project compiles cleanly — any TypeScript errors surface here.
- **Inspection:** Future agents can verify project structure via `ls frontend/src/app/ frontend/src/components/` and check Tailwind v4 config via `grep '@import "tailwindcss"' frontend/src/app/globals.css`.
- **Failure visibility:** Build failures appear as TypeScript compilation errors in `npm run build` output with file:line references.
- **No runtime signals yet:** This task creates a buildable scaffold only. Runtime observability (API client error logging, auth state) is introduced in T02.

## Inputs

- `/tmp/vibe-loom/src/app/page.tsx` — main page component to copy
- `/tmp/vibe-loom/src/app/layout.tsx` — root layout to copy
- `/tmp/vibe-loom/src/app/globals.css` — Tailwind styles to copy (update directives to v4)
- `/tmp/vibe-loom/src/components/VibeScoreGauge.tsx` — score gauge component to copy
- `/tmp/vibe-loom/src/components/CodeDiffView.tsx` — diff viewer component to copy
- `/tmp/vibe-loom/src/components/VibeStatus.tsx` — deploy status component to copy

## Expected Output

- `frontend/package.json` — Next.js project manifest
- `frontend/tsconfig.json` — TypeScript configuration
- `frontend/next.config.js` — Next.js configuration
- `frontend/postcss.config.js` — PostCSS/Tailwind configuration
- `frontend/src/app/page.tsx` — copied page component
- `frontend/src/app/layout.tsx` — copied layout
- `frontend/src/app/globals.css` — Tailwind v4 styles
- `frontend/src/components/VibeScoreGauge.tsx` — copied component
- `frontend/src/components/CodeDiffView.tsx` — copied component
- `frontend/src/components/VibeStatus.tsx` — copied component
- `frontend/.env.local.example` — env var documentation
