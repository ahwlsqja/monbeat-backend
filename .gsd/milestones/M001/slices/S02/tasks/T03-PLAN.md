---
estimated_steps: 4
estimated_files: 4
---

# T03: Build ContractsController, ContractsModule, wire into AppModule, and add controller tests

**Slice:** S02 — Contract & Deploy Module
**Milestone:** M001

## Description

Create the ContractsController with 3 HTTP endpoints (GET /contracts/source, POST /contracts/compile, POST /contracts/deploy), create the ContractsModule registering the controller + services and exporting CompileService for S04, add ContractsModule to AppModule imports, and write controller unit tests.

This is the integration task that makes the slice demo true — after T03, all S02 endpoints are live and the build + full test suite passes.

**S01 patterns to follow:**
- TransformInterceptor auto-wraps all controller returns as `{ success: true, data: ... }` — controllers return raw data objects
- Global ValidationPipe handles DTO validation — just use `@Body()` with DTO types
- All API endpoints are under `/api` prefix (set in main.ts)
- PrismaModule is @Global() — no need to import in ContractsModule
- ConfigModule is isGlobal: true — inject ConfigService directly

**Relevant skills:** None required.

## Steps

1. **Create ContractsController:** Create `backend/src/contracts/contracts.controller.ts`:
   - `@Controller('contracts')` decorator (produces /api/contracts/* with the global prefix)
   - **GET /source** (`@Get('source') @Query('type') type: string`):
     - Validate `type` is one of: 'FixedContract', 'FailingContract', 'ParallelConflict', 'PectraTest'
     - If invalid/missing: throw `BadRequestException('Invalid contract type')`
     - Read the corresponding .sol file from disk using `fs.readFileSync`
     - File paths: `FixedContract` → `contracts/FixedContract.sol`, others → `contracts/test/{Type}.sol`
     - Return `{ contractType: type, source: fileContent }`
   - **POST /compile** (`@Post('compile') @Body() dto: CompileRequestDto`):
     - Call `compileService.compile(dto.source)`
     - Return the CompileResultDto directly
   - **POST /deploy** (`@Post('deploy') @Body() dto: DeployRequestDto`):
     - Call `deployService.deploy(dto.source)`
     - Return the deploy result (address, txHash, contractName, status)
   - Use `path.join(process.cwd(), 'contracts', ...)` or `path.join(__dirname, '../../contracts', ...)` for contract file paths — test both to see which resolves correctly in the NestJS runtime context. The contracts directory is at `backend/contracts/` (peer to `backend/src/`).

2. **Create ContractsModule:** Create `backend/src/contracts/contracts.module.ts`:
   - `@Module({ controllers: [ContractsController], providers: [CompileService, DeployService], exports: [CompileService] })`
   - The `exports: [CompileService]` is critical — S04's EngineModule will import ContractsModule to access CompileService
   - No need to import PrismaModule or ConfigModule (both are global)

3. **Wire into AppModule:** Edit `backend/src/app.module.ts`:
   - Add `import { ContractsModule } from './contracts/contracts.module';`
   - Add `ContractsModule` to the `imports` array

4. **Write controller tests:** Create `backend/test/contracts.controller.spec.ts`:
   - Mock CompileService and DeployService
   - Test: GET /source with type=FixedContract → returns source string (mock fs or use real file from T01)
   - Test: GET /source with type=InvalidType → throws BadRequestException
   - Test: GET /source with no type parameter → throws BadRequestException
   - Test: POST /compile with valid source → calls compileService.compile and returns result
   - Test: POST /deploy with valid source → calls deployService.deploy and returns result
   - Run full test suite to verify no regressions: `cd backend && npm test`

## Must-Haves

- [ ] ContractsController exposes GET /contracts/source, POST /contracts/compile, POST /contracts/deploy
- [ ] GET /source reads .sol files by contract type and returns source string
- [ ] GET /source returns 400 for invalid/missing contract type
- [ ] ContractsModule exports CompileService (for S04)
- [ ] ContractsModule registered in AppModule imports
- [ ] `npm run build` succeeds with zero errors
- [ ] `npm test` passes all tests (existing 5 + new tests from T01, T02, T03)

## Verification

- `cd backend && npm run build` — zero TypeScript errors
- `cd backend && npm test` — all tests pass (existing 5 + all new S02 tests)
- `grep -q "ContractsModule" backend/src/app.module.ts` — module registered
- `grep -q "exports:" backend/src/contracts/contracts.module.ts` — CompileService exported

## Inputs

- `backend/src/contracts/compile.service.ts` — CompileService from T01
- `backend/src/contracts/deploy.service.ts` — DeployService from T02
- `backend/src/contracts/dto/compile-request.dto.ts` — DTO from T01
- `backend/src/contracts/dto/deploy-request.dto.ts` — DTO from T02
- `backend/src/contracts/dto/compile-result.dto.ts` — response type from T01
- `backend/src/app.module.ts` — AppModule to add ContractsModule import
- `backend/contracts/FixedContract.sol` — sample contract from T01 (for source endpoint)

## Expected Output

- `backend/src/contracts/contracts.controller.ts` — 3-endpoint HTTP controller
- `backend/src/contracts/contracts.module.ts` — NestJS module with CompileService export
- `backend/src/app.module.ts` — updated with ContractsModule import
- `backend/test/contracts.controller.spec.ts` — controller unit tests
