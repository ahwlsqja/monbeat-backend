---
estimated_steps: 5
estimated_files: 12
---

# T02: Build AnalysisModule with Gemini RAG error analysis, optimizer, and streaming

**Slice:** S03 — Auth + Analysis + Paymaster Module
**Milestone:** M001

## Description

Port the Vibe-Loom error analysis pipeline into NestJS as the AnalysisModule. This includes: GeminiService (wraps `@google/generative-ai` with ConfigService), OptimizerService (rule-based `calculateMonadParallelismScore`), AnalysisService (orchestrates RAG context loading → prompt building → Gemini call → JSON parsing → fallback heuristics), prompt templates and error handler (pure modules ported from Vibe-Loom), and the AnalysisController with streaming support. Also copies the 5 monad-docs RAG files into the backend project.

This delivers requirement R004 (Gemini AI RAG error analysis + streaming).

**Relevant skills:** None specific — standard NestJS service patterns + Gemini AI SDK.

**Key context:**
- `@google/generative-ai` was installed in T01
- `ConfigService.get<string>('gemini.apiKey')` available from T01's config extension
- `PrismaModule` is `@Global()` — inject PrismaService for Analysis model storage
- Vibe-Loom source files are at `/tmp/vibe-loom/src/lib/` — port logic, don't copy verbatim (NestJS DI patterns differ from Next.js)
- Monad-docs RAG files at `/tmp/vibe-loom/data/monad-docs/` — 5 files, ~5KB total
- `TransformInterceptor` wraps responses — streaming endpoints must use `@Res()` to bypass it

## Steps

1. **Copy monad-docs RAG files:** Copy all 5 `.md` files from `/tmp/vibe-loom/data/monad-docs/` to `backend/data/monad-docs/`. These are the RAG context documents loaded by the analysis service.

2. **Create pure utility modules (no DI):**
   - `backend/src/analysis/prompt-templates.ts` — Port from Vibe-Loom: `DeploymentErrorContext` interface, keyword arrays (`MONAD_PARALLELISM_KEYWORDS`, `MONAD_GAS_POLICY_KEYWORDS`, `PECTRA_FORK_KEYWORDS`), `DEPLOYMENT_ERROR_ANALYSIS_PROMPT`, `RAG_FIX_REQUEST_PROMPT`, `isLikelyMonadSpecificError()`, `buildErrorAnalysisPrompt()`, `buildRagFixPrompt()`. These are pure functions, no class needed.
   - `backend/src/analysis/error-handler.ts` — Port from Vibe-Loom: `ParsedDeploymentError` interface, `parseProviderError()`, `handleDeploymentError()`. Pure functions.

3. **Create NestJS services:**
   - `backend/src/analysis/gemini.service.ts` — `@Injectable()` wrapping `@google/generative-ai`. Constructor injects ConfigService for `gemini.apiKey`. Lazy-initializes `GoogleGenerativeAI` client on first call. Methods: `generateContent(prompt: string): Promise<string>` (returns text), `generateContentStream(prompt: string): AsyncGenerator<string>` (yields chunks). Uses `gemini-2.5-flash` as default model with `gemini-pro` fallback. Returns `null` if API key not configured (graceful degradation).
   - `backend/src/analysis/optimizer.service.ts` — `@Injectable()` wrapping `calculateMonadParallelismScore()` ported from Vibe-Loom's optimizer.ts. Method: `calculateScore(solidityCode: string): MonadOptimizationResult` returning `{ score, deductions, suggestions }`.
   - `backend/src/analysis/analysis.service.ts` — `@Injectable()` orchestrating the full pipeline. Injects GeminiService, OptimizerService, PrismaService. Methods:
     - `loadRagContext(): Promise<string>` — reads all .md files from `backend/data/monad-docs/` directory
     - `analyzeError(error: object, contractSource: string, errorCode?: string): Promise<AnalysisResult>` — parses error → loads RAG context → builds prompt → calls GeminiService → parses JSON → fallback heuristics if AI fails → runs OptimizerService → optionally saves to Analysis DB table
     - `analyzeErrorStream(error: object, contractSource: string, errorCode?: string): AsyncGenerator<string>` — same pipeline but uses GeminiService streaming

4. **Create controller and DTOs:**
   - `backend/src/analysis/dto/analysis-request.dto.ts` — `{ error: object, contractSource: string, errorCode?: string }` with class-validator decorators
   - `backend/src/analysis/dto/analysis-response.dto.ts` — Type for `{ analysis: { summary, fixedCode, explanation, isMonadSpecific }, optimization: { score, deductions, suggestions } }`
   - `backend/src/analysis/analysis.controller.ts` — `POST /api/analysis/error`:
     - Normal mode: calls `analysisService.analyzeError()`, returns JSON response (auto-wrapped by TransformInterceptor)
     - Stream mode (`?stream=true`): uses `@Res() res: Response`, sets `Content-Type: text/plain; charset=utf-8`, pipes `analyzeErrorStream()` chunks, calls `res.end()`. The `@Res()` decorator bypasses TransformInterceptor.
   - `backend/src/analysis/analysis.module.ts` — Provides GeminiService, OptimizerService, AnalysisService. No imports needed (PrismaModule is global, ConfigModule is global).

5. **Register and test:**
   - Add `AnalysisModule` to `backend/src/app.module.ts` imports
   - Create `backend/test/optimizer.service.spec.ts` — Test `calculateScore()` with known patterns: ParallelConflict contract (global counter → low score ≤35), FixedContract (simple storage → high score ≥80), empty contract (score 100)
   - Create `backend/test/analysis.service.spec.ts` — Test `analyzeError()` with mocked GeminiService: AI success path returns parsed analysis, AI failure falls back to heuristics, gas error gets gas-specific suggestion, optimizer result included

## Must-Haves

- [ ] 5 monad-docs .md files copied to `backend/data/monad-docs/`
- [ ] GeminiService wraps `@google/generative-ai` with ConfigService injection and model fallback
- [ ] OptimizerService ports `calculateMonadParallelismScore` with all 6 detection patterns
- [ ] AnalysisService orchestrates RAG context → prompt → AI call → parse → fallback → optimizer
- [ ] AnalysisController supports both normal JSON and streaming (`?stream=true`) responses
- [ ] AnalysisModule registered in AppModule
- [ ] ≥6 unit tests passing across analysis + optimizer suites

## Verification

- `cd backend && npm run build` — exit 0
- `cd backend && npm test -- --testPathPattern="analysis|optimizer"` — ≥6 tests pass
- `grep -q "AnalysisModule" backend/src/app.module.ts` — module registered
- `test -d backend/data/monad-docs && ls backend/data/monad-docs/*.md | wc -l` — outputs 5
- `test -f backend/src/analysis/gemini.service.ts` — Gemini service exists

## Inputs

- `backend/package.json` — `@google/generative-ai` already installed by T01
- `backend/src/config/configuration.ts` — `gemini.apiKey` config key from T01
- `backend/src/app.module.ts` — add AnalysisModule import
- `backend/prisma/schema.prisma` — Analysis model for DB storage
- `/tmp/vibe-loom/src/lib/ai.ts` — Gemini client reference (port to GeminiService)
- `/tmp/vibe-loom/src/lib/prompt-templates.ts` — prompts + builder functions (port as pure module)
- `/tmp/vibe-loom/src/lib/deployment-error-handler.ts` — error parsing (port as pure module)
- `/tmp/vibe-loom/src/lib/optimizer.ts` — rule-based scoring (port to OptimizerService)
- `/tmp/vibe-loom/src/app/api/analyze-deployment-error/route.ts` — full pipeline reference
- `/tmp/vibe-loom/data/monad-docs/` — 5 RAG context files to copy

## Expected Output

- `backend/data/monad-docs/00-index.md` — RAG context file
- `backend/data/monad-docs/01-parallel-execution.md` — RAG context file
- `backend/data/monad-docs/02-rpc-and-gas.md` — RAG context file
- `backend/data/monad-docs/03-evm-and-pectra.md` — RAG context file
- `backend/data/monad-docs/04-consensus-and-mempool.md` — RAG context file
- `backend/src/analysis/analysis.module.ts` — NestJS module
- `backend/src/analysis/analysis.service.ts` — orchestration service
- `backend/src/analysis/analysis.controller.ts` — HTTP controller with streaming
- `backend/src/analysis/gemini.service.ts` — Gemini AI wrapper service
- `backend/src/analysis/optimizer.service.ts` — rule-based parallelism scorer
- `backend/src/analysis/prompt-templates.ts` — prompt constants + builders
- `backend/src/analysis/error-handler.ts` — error parsing utilities
- `backend/src/analysis/dto/analysis-request.dto.ts` — request DTO
- `backend/src/analysis/dto/analysis-response.dto.ts` — response type
- `backend/src/app.module.ts` — updated with AnalysisModule import
- `backend/test/analysis.service.spec.ts` — analysis service tests
- `backend/test/optimizer.service.spec.ts` — optimizer service tests
