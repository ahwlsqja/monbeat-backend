---
estimated_steps: 5
estimated_files: 5
---

# T01: CompileService storageLayout 추출 + 전체 TypeScript 인터페이스 정의

**Slice:** S02 — NestJS — Storage Layout 디코딩 + Actionable Suggestion 생성
**Milestone:** M006

## Description

모든 downstream 로직의 기반을 구축한다. solc 컴파일러에서 `storageLayout`을 추출하고, S01 Rust CLI의 `conflict_details` JSON 스키마와 일치하는 TypeScript 인터페이스를 정의하고, 최종 API 응답의 `conflictAnalysis` 인터페이스를 정의한다. CompileService의 기존 테스트를 유지하면서 storageLayout 추출을 검증하는 테스트를 추가한다.

**Relevant skills:** `test` (Jest test generation)

## Steps

1. **CompileService에 storageLayout 추출 추가** — `Vibe-Room-Backend/src/contracts/compile.service.ts`:
   - `SolcOutput` 인터페이스의 contracts value 타입에 `storageLayout?: { storage: any[]; types: Record<string, any> }` 추가
   - `settings.outputSelection['*']['*']` 배열에 `'storageLayout'` 추가 (현재: `['abi', 'evm.bytecode.object']` → `['abi', 'evm.bytecode.object', 'storageLayout']`)
   - `compile()` 메서드 리턴 시 `storageLayout: contract.storageLayout` 포함

2. **CompileResultDto 확장** — `Vibe-Room-Backend/src/contracts/dto/compile-result.dto.ts`:
   ```typescript
   export interface StorageEntry {
     astId: number;
     contract: string;
     label: string;
     offset: number;
     slot: string;  // decimal string, e.g. "0", "1"
     type: string;  // typeId reference into types map
   }
   export interface StorageTypeInfo {
     encoding: string;  // "inplace", "mapping", "dynamic_array"
     label: string;     // e.g. "uint256", "mapping(address => uint256)"
     numberOfBytes: string;
     key?: string;      // typeId for mapping key type
     value?: string;    // typeId for mapping value type
     base?: string;     // typeId for dynamic array base type
     members?: any[];   // for struct types
   }
   export interface StorageLayout {
     storage: StorageEntry[];
     types: Record<string, StorageTypeInfo>;
   }
   export interface CompileResultDto {
     contractName: string;
     abi: any[];
     bytecode: string;
     storageLayout?: StorageLayout;
   }
   ```

3. **CliOutput 인터페이스 확장** — `Vibe-Room-Backend/src/engine/engine.service.ts`:
   ```typescript
   // S01 conflict_details schema — matches Rust CLI output exactly
   export interface LocationInfo {
     location_type: string;  // "Storage", "Balance", "Nonce", "CodeHash"
     address: string;        // lowercase hex with 0x prefix
     slot?: string;          // hex with 0x prefix, only for Storage type
   }
   export interface ConflictPair {
     location: LocationInfo;
     tx_a: number;
     tx_b: number;
     conflict_type: string;  // "write-write" | "read-write"
   }
   export interface TxAccessSummary {
     tx_index: number;
     reads: LocationInfo[];
     writes: LocationInfo[];
   }
   export interface ConflictDetails {
     per_tx: TxAccessSummary[];
     conflicts: ConflictPair[];
   }
   // Add to CliOutput:
   export interface CliOutput {
     results: TxResult[];
     incarnations: number[];
     stats: CliStats;
     conflict_details?: ConflictDetails;
   }
   ```

4. **VibeScoreResultDto 확장** — `Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts`:
   ```typescript
   export interface DecodedConflict {
     variableName: string;
     variableType: string;
     slot: string;
     functions: string[];
     conflictType: string;
     suggestion: string;
   }
   export interface ConflictMatrix {
     rows: string[];      // function names
     cols: string[];      // variable names
     cells: number[][];   // intensity values (conflict count)
   }
   export interface ConflictAnalysis {
     conflicts: DecodedConflict[];
     matrix: ConflictMatrix;
   }
   // Add to VibeScoreResultDto:
   conflictAnalysis?: ConflictAnalysis;
   ```

5. **CompileService 테스트 추가** — `Vibe-Room-Backend/test/compile.service.spec.ts`:
   - 기존 ParallelConflict.sol 테스트 describe 블록 내에 새 test: `it('should include storageLayout with counter variable')` — result.storageLayout이 존재하고, storage 배열에 label === 'counter'인 entry가 있는지 확인
   - FixedContract.sol 테스트에도 storageLayout 존재 확인 추가 가능

## Must-Haves

- [ ] solc outputSelection에 'storageLayout' 추가
- [ ] CompileResultDto에 storageLayout?: StorageLayout 필드 + StorageLayout/StorageEntry/StorageTypeInfo 인터페이스
- [ ] CliOutput에 conflict_details?: ConflictDetails + LocationInfo/ConflictPair/TxAccessSummary/ConflictDetails 인터페이스
- [ ] VibeScoreResultDto에 conflictAnalysis?: ConflictAnalysis + DecodedConflict/ConflictMatrix/ConflictAnalysis 인터페이스
- [ ] 기존 compile.service 테스트 전부 통과
- [ ] ParallelConflict.sol 컴파일 시 storageLayout에 "counter" 변수 포함 테스트 통과

## Verification

- `cd Vibe-Room-Backend && npx jest test/compile.service.spec.ts` — 기존 6개 + 새 1개 이상 테스트 통과
- `cd Vibe-Room-Backend && npx jest test/engine.service.spec.ts` — 기존 테스트 깨지지 않음 (CliOutput 확장은 optional field이므로)

## Observability Impact

- **Signals changed:** `CompileResultDto` now carries optional `storageLayout` field — downstream can inspect whether solc produced layout data by checking field presence.
- **Inspection:** compile test output verifies `storageLayout.storage[].label` values. In runtime, if `storageLayout` is undefined (solc failure or unsupported version), downstream phases silently skip conflict analysis with no error.
- **Failure visibility:** solc compilation errors are already surfaced via `BadRequestException`. The new `storageLayout` field is optional — its absence is the only visible signal of extraction failure (no additional error thrown).

## Inputs

- `Vibe-Room-Backend/src/contracts/compile.service.ts` — 현재 solc outputSelection에 storageLayout 미포함. SolcOutput 타입에 storageLayout 필드 없음
- `Vibe-Room-Backend/src/contracts/dto/compile-result.dto.ts` — 현재 contractName, abi, bytecode만 포함
- `Vibe-Room-Backend/src/engine/engine.service.ts` — 현재 CliOutput에 results, incarnations, stats만 포함
- `Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts` — 현재 vibeScore, conflicts, reExecutions, gasEfficiency, engineBased, suggestions, traceResults만 포함
- `Vibe-Room-Backend/test/compile.service.spec.ts` — 현재 6개 테스트
- `Vibe-Room-Backend/contracts/test/ParallelConflict.sol` — 테스트에 사용할 컨트랙트 소스

## Expected Output

- `Vibe-Room-Backend/src/contracts/compile.service.ts` — storageLayout 추출 추가
- `Vibe-Room-Backend/src/contracts/dto/compile-result.dto.ts` — StorageLayout, StorageEntry, StorageTypeInfo 인터페이스 + CompileResultDto 확장
- `Vibe-Room-Backend/src/engine/engine.service.ts` — LocationInfo, ConflictPair, TxAccessSummary, ConflictDetails 인터페이스 + CliOutput 확장
- `Vibe-Room-Backend/src/vibe-score/dto/vibe-score-result.dto.ts` — DecodedConflict, ConflictMatrix, ConflictAnalysis 인터페이스 + VibeScoreResultDto 확장
- `Vibe-Room-Backend/test/compile.service.spec.ts` — storageLayout 추출 테스트 추가
