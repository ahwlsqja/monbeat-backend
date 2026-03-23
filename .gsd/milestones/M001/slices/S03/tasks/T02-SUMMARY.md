---
id: T02
parent: S03
milestone: M001
provides:
  - AnalysisModule with GeminiService, OptimizerService, AnalysisService, AnalysisController
  - Prompt templates and error handler pure utility modules
  - 5 monad-docs RAG context files in backend/data/monad-docs/
  - 13 unit tests (7 analysis + 6 optimizer)
key_files:
  - backend/src/analysis/analysis.module.ts
  - backend/src/analysis/analysis.service.ts
  - backend/src/analysis/analysis.controller.ts
  - backend/src/analysis/gemini.service.ts
  - backend/src/analysis/optimizer.service.ts
  - backend/src/analysis/prompt-templates.ts
  - backend/src/analysis/error-handler.ts
  - backend/src/analysis/dto/analysis-request.dto.ts
  - backend/src/analysis/dto/analysis-response.dto.ts
  - backend/data/monad-docs/
key_decisions:
  - AnalysisController uses @Res() for both streaming and non-streaming paths to avoid TransformInterceptor conflict when stream=true; non-streaming path manually wraps response in { success, data } envelope
  - GeminiService lazy-initializes GoogleGenerativeAI client on first call; returns null if API key not configured (graceful degradation)
  - AI JSON parsing has two fallback layers: code-block extraction → regex extraction → heuristic fallback
patterns_established:
  - Pure utility modules (prompt-templates.ts, error-handler.ts) exported as plain functions — no @Injectable(), no DI
  - Service pipeline pattern: parse → RAG → prompt → AI → parse → fallback → optimize → persist
  - OptimizerService is a stateless class with no dependencies — can be instantiated directly in tests without TestingModule
observability_surfaces:
  - POST /api/analysis/error — returns { analysis: { summary, fixedCode, explanation, isMonadSpecific, category }, optimization: { score, deductions, suggestions } }
  - POST /api/analysis/error?stream=true — streams AI response chunks as text/plain
  - GeminiService logs model selection, primary/fallback attempts, and failures
  - AnalysisService logs RAG context loading count, AI success/fallback, and optional DB persistence
duration: 10m
verification_result: passed
completed_at: 2026-03-22
blocker_discovered: false
---

# T02: Build AnalysisModule with Gemini RAG error analysis, optimizer, and streaming

**Ported Vibe-Loom error analysis pipeline into NestJS AnalysisModule with GeminiService (model fallback + streaming), OptimizerService (6 detection patterns), RAG context loading, and AnalysisController supporting JSON and streaming modes; 13 tests passing.**

## What Happened

Copied 5 monad-docs RAG files to `backend/data/monad-docs/`. Ported prompt-templates.ts with `DeploymentErrorContext` interface, 3 keyword arrays, 2 prompt templates, `isLikelyMonadSpecificError()`, `buildErrorAnalysisPrompt()`, `buildRagFixPrompt()` as pure functions. Ported error-handler.ts with `parseProviderError()` and `handleDeploymentError()` as pure functions.

Created GeminiService wrapping `@google/generative-ai` with ConfigService injection for API key, lazy client initialization, `gemini-2.5-flash` default model with `gemini-pro` fallback, and both `generateContent()` (returns text or null) and `generateContentStream()` (async generator yielding chunks) methods.

Created OptimizerService porting all 6 detection patterns from Vibe-Loom: (1) loop storage access, (2) direct SLOAD/SSTORE, (3) repeated mapping access, (4) CEI pattern violation, (5) centralized storage bottleneck, (6) block/tx property dependency. Returns `{ score, deductions, suggestions }`.

Created AnalysisService orchestrating the full pipeline: error parsing → RAG context loading → prompt building → Gemini AI call → JSON parsing (with markdown code block handling) → heuristic fallback (gas, nonce, revert, opcode categories) → optimizer scoring → optional DB persistence. Also provides `analyzeErrorStream()` for chunked streaming.

Created AnalysisController with `POST /api/analysis/error` supporting both normal JSON response and streaming via `?stream=true` query parameter. Uses `@Res()` to bypass TransformInterceptor for streaming, and manually wraps non-streaming responses in the `{ success, data }` envelope.

Created DTOs: `AnalysisRequestDto` with class-validator decorations, `AnalysisResult` response interface. Registered AnalysisModule in AppModule.

## Verification

- `npm run build` — exit 0, zero TypeScript errors
- `npm test -- --testPathPattern="analysis|optimizer"` — 13 tests pass across 2 suites
- `npm test` — 56 total tests pass (34 existing + 9 auth + 13 new), no regressions
- `grep -q "AnalysisModule" backend/src/app.module.ts` — confirmed registered
- `test -d backend/data/monad-docs && ls backend/data/monad-docs/*.md | wc -l` — outputs 5
- `test -f backend/src/analysis/gemini.service.ts` — confirmed exists

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cd backend && npm run build` | 0 | ✅ pass | 3s |
| 2 | `cd backend && npm test -- --testPathPattern="analysis\|optimizer"` | 0 | ✅ pass | 4s |
| 3 | `cd backend && npm test` | 0 | ✅ pass | 8s |
| 4 | `grep -q "AnalysisModule" backend/src/app.module.ts` | 0 | ✅ pass | <1s |
| 5 | `test -d backend/data/monad-docs && ls backend/data/monad-docs/*.md \| wc -l` | 0 | ✅ pass (5) | <1s |
| 6 | `test -f backend/src/analysis/gemini.service.ts` | 0 | ✅ pass | <1s |

## Diagnostics

- **Analysis endpoint:** `POST /api/analysis/error` with `{ error: { message: "..." }, contractSource: "..." }` returns full analysis + optimization result. Add `?stream=true` for streaming.
- **GeminiService logging:** Logs model selection (`Using Gemini model: gemini-2.5-flash`), fallback attempts, and errors at WARN/ERROR levels. Returns null gracefully when API key not configured.
- **AnalysisService logging:** Logs RAG context file count, AI success/heuristic fallback path taken, and DB persistence status.
- **Optimizer inspection:** OptimizerService can be called independently for any Solidity code — returns score 0-100 with deductions array explaining each penalty.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `backend/data/monad-docs/00-index.md` — RAG context file (copied from Vibe-Loom)
- `backend/data/monad-docs/01-parallel-execution.md` — RAG context file
- `backend/data/monad-docs/02-rpc-and-gas.md` — RAG context file
- `backend/data/monad-docs/03-evm-and-pectra.md` — RAG context file
- `backend/data/monad-docs/04-consensus-and-mempool.md` — RAG context file
- `backend/src/analysis/prompt-templates.ts` — Prompt templates + builder functions (pure module)
- `backend/src/analysis/error-handler.ts` — Error parsing utilities (pure module)
- `backend/src/analysis/gemini.service.ts` — Gemini AI wrapper with model fallback + streaming
- `backend/src/analysis/optimizer.service.ts` — Rule-based Monad parallelism scorer (6 patterns)
- `backend/src/analysis/analysis.service.ts` — Orchestration service (RAG → AI → fallback → optimizer)
- `backend/src/analysis/analysis.controller.ts` — HTTP controller with JSON + streaming support
- `backend/src/analysis/analysis.module.ts` — NestJS module registering all analysis providers
- `backend/src/analysis/dto/analysis-request.dto.ts` — Request DTO with class-validator
- `backend/src/analysis/dto/analysis-response.dto.ts` — Response type interface
- `backend/src/app.module.ts` — Added AnalysisModule import
- `backend/test/optimizer.service.spec.ts` — 6 optimizer unit tests
- `backend/test/analysis.service.spec.ts` — 7 analysis service unit tests
