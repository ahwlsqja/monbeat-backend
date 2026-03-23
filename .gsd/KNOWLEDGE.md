
### 레포 매핑 — 3개 레포 절대 섞지 않기
**Discovered:** M002 milestone closeout (실수로 Core에 frontend/backend 푸시)
**Context:** 프로젝트는 3개의 독립 레포로 구성된다:
- **Core (Rust 엔진)**: `https://github.com/ahwlsqja/Vibe-Room-Core.git` → `/home/ahwlsqja/monad-core`
- **Frontend (Next.js)**: `https://github.com/leejk206/Vibe-Loom.git` → `/home/ahwlsqja/Vibe-Loom`
- **Backend (NestJS)**: `https://github.com/ahwlsqja/Vibe-Room-Backend.git`
**Rule:** Core 레포에는 Rust crates + Cargo.toml만. frontend/, backend/ 디렉토리가 절대 들어가면 안 된다. GSD 워크트리가 Core 레포 안에 생성되지만, 그 안의 frontend/backend 코드는 각각의 레포에 별도 푸시해야 한다.

### Vibe-Loom 모바일 UI — 중복 텍스트 요소 주의
**Discovered:** M003-S01-T01 E2E 테스트 작성 시
**Context:** Vibe-Loom 프론트엔드의 모바일 뷰포트(375×812)에서 `Results`, `Console` 등의 텍스트가 탭 버튼(`<button>`)과 섹션 헤더(`<h2>`) 양쪽에 존재한다.
**Rule:** Playwright에서 `getByText('Results', { exact: true })` 사용 시 strict-mode 위반 발생. 반드시 `getByRole('button', { name: 'Results' })` 형태로 role을 지정해야 한다.

### Vibe-Room Backend — analysis/error API 페이로드 형식
**Discovered:** M003-S01-T01 API 테스트 작성 시
**Context:** `/api/analysis/error` 엔드포인트의 `error` 필드는 문자열이 아니라 `{message: string, severity: string}` 객체여야 한다. 문자열로 보내면 400 Bad Request ("error must be an object") 반환.
**Rule:** 에러 분석 API 호출 시 `error: { message: '...', severity: 'error' }` 형태 사용.

### Monad Testnet 배포 타이밍 — E2E 테스트 방어 설계 필수
**Discovered:** M003-S01-T02 Contract Interaction 테스트 작성 시
**Context:** Monad testnet 배포는 30-90초 소요되며, 잔고 부족이나 네트워크 상태에 따라 60초 내 타임아웃이 빈번하다. Deploy 성공에 의존하는 테스트는 항상 타임아웃을 고려해야 한다.
**Rule:** Deploy 의존 테스트에는 반드시 `test.skip()` 가드 사용. `Promise.race`로 성공/실패/타임아웃 모두 유효한 결과로 허용. 60s 이상 timeout 설정 필수.

### Playwright E2E 테스트 — Promise.race 다중 결과 패턴
**Discovered:** M003-S01-T02 AI Error Analysis Flow 테스트
**Context:** 라이브 서비스에 의존하는 UI 테스트는 AI 응답, 에러 표시, 성공 배포 등 여러 유효한 결과가 있을 수 있다. 단일 결과만 기대하면 서비스 상태에 따라 불안정해진다.
**Rule:** `Promise.race([waitForAI, waitForError, waitForDeploy])` 패턴으로 다중 유효 결과를 수용. 각 결과에 대해 독립적인 assertion 분기 작성.

### E2E 테스트 스위트 — 라이브 서비스 대상 테스트 설계 원칙
**Discovered:** M003 마일스톤 전체 경험
**Context:** 라이브 서비스(vibe-loom.xyz + Railway 백엔드 + Monad testnet)를 대상으로 E2E 테스트를 작성할 때, 서비스 상태가 테스트 결과에 직접 영향을 미친다. 테스트넷 지연, API 키 미설정, 잔고 부족 등 외부 요인이 많다.
**Rule:** (1) 서비스 의존 테스트는 항상 방어적으로 설계 — 다중 유효 상태코드 허용, test.skip() 가드, Promise.race. (2) 스크린샷 증거는 모든 주요 단계에서 캡처 — 실패 시 원인 파악에 필수. (3) 단일 monolithic 테스트 파일에 describe 블록으로 구분하면 실행 순서 제어와 공유 헬퍼 관리가 용이. (4) Monaco editor 등 비동기 UI 컴포넌트는 고정 타임아웃보다 waitForSelector가 안정적.

### CLI conflict_details — 코인베이스 주소가 거의 모든 tx 쌍에서 충돌로 나타남
**Discovered:** M006-S01 integration check
**Context:** EVM 실행에서 모든 tx는 gas fee 처리를 위해 coinbase 주소의 Balance/Nonce/CodeHash를 읽고 쓴다. 따라서 2개 이상 tx가 있는 블록에서는 coinbase 관련 read-write/write-write 충돌이 항상 나타난다.
**Rule:** S02(NestJS)에서 conflict_details를 분석할 때 coinbase 주소 관련 충돌은 필터링하거나 우선순위를 낮춰야 한다. 이는 사용자가 수정할 수 없는 EVM 내재적 동작이며, actionable suggestion 대상이 아니다. `block_env.coinbase` 값과 conflict location의 address를 비교하여 필터링.

### TxState.read_set — 이미 Option<ReadSet> 필드가 존재했음
**Discovered:** M006-S01 T01 구현 시
**Context:** S01 계획에서 "ReadSet 보존이 가장 큰 리스크"로 평가되었으나, 실제로는 `TxState`에 이미 `read_set: Option<ReadSet>` 필드가 존재하고 `take_read_set()`이 구현되어 있었다. `return_read_set()` 역방향 메서드 추가와 validation success path에서의 호출만으로 완료.
**Rule:** monad-core scheduler의 `TxState`는 mutex로 보호되며, `take_*()` / `return_*()` 패턴으로 thread-safe 접근. 새 필드 추가 시에도 같은 패턴을 따르면 됨.

### solc storageLayout — slot 값이 decimal string, CLI conflict_details는 hex string
**Discovered:** M006-S02 T02 storage-layout-decoder 구현 시
**Context:** solc `storageLayout.storage[].slot`은 `"0"`, `"1"` 등 decimal string이고, monad-core CLI `conflict_details`의 slot은 `"0x0"`, `"0x1"` 등 hex string이다. 직접 문자열 비교하면 매칭 실패.
**Rule:** slot 비교 시 반드시 양쪽 모두 `BigInt()` 변환 후 비교. hex는 `BigInt("0x0")`, decimal은 `BigInt("0")`. `parseInt`는 큰 수(keccak256-derived slot)에서 정밀도 손실되므로 사용 금지.

### Mapping runtime slot — keccak256 기반 slot은 선언 범위를 크게 초과
**Discovered:** M006-S02 T02 mapping heuristic 구현 시
**Context:** Solidity mapping의 runtime storage slot은 `keccak256(key . base_slot)`으로 계산되어, 선언된 slot 범위(0, 1, 2, ...)를 크게 초과하는 값이 된다. 예: `mapping(address => uint256) balances`의 base slot이 3이면, 특정 key의 runtime slot은 `0x4e0c...` 같은 큰 수.
**Rule:** Runtime slot이 storageLayout의 max declared slot보다 크면 mapping/dynamic array의 runtime slot으로 간주. 단일 mapping이면 확정 귀속, 복수 mapping이면 후보 목록 보고. 100% 정확도 불가능하지만 단일 mapping 컨트랙트(대부분의 케이스)에서는 정확함.

### constructTransactionBlock — txFunctionMap 패턴
**Discovered:** M006-S02 T03 VibeScoreService wiring 시
**Context:** conflict_details의 tx_a/tx_b는 정수 인덱스인데, suggestion 생성에는 함수명이 필요. tx index → function name 매핑은 트랜잭션 블록 구성 시점에 생성해야 정확함 (deploy tx=0="constructor", 이후 tx=stateChangingFns 순서).
**Rule:** `constructTransactionBlock()`에서 txFunctionMap을 함께 빌드하여 반환. `{ transactions, blockEnv, txFunctionMap }` triple로 반환하는 패턴 사용. function encode 실패로 skip된 함수도 있으므로 인덱스를 직접 추적해야 함.

### NestJS E2E mock conflict_details — 반드시 non-coinbase 주소 사용
**Discovered:** M006-S04 T01 E2E 테스트 작성 시
**Context:** `buildConflictAnalysis`는 coinbase 주소 관련 충돌을 자동 필터링한다 (EVM 내재적 동작이므로 actionable 아님). E2E 테스트에서 EngineService mock의 `conflict_details`에 coinbase 주소(예: `0x0000...0000`)를 사용하면 모든 충돌이 필터링되어 `conflictAnalysis.conflicts`가 빈 배열이 된다.
**Rule:** E2E 테스트 mock 데이터의 conflict address는 반드시 non-coinbase 주소(예: `0x1234...abcd`)를 사용해야 한다. `block_env.coinbase` 값과 다른 주소를 선택.

### NestJS E2E — describe 블록별 TestingModule 격리 패턴
**Discovered:** M006-S04 T01 EngineService mock 격리 설계 시
**Context:** 기존 E2E 테스트와 새로운 Conflict Analysis E2E 테스트가 서로 다른 EngineService 동작을 필요로 한다 (기존: real service, 신규: mock with conflict_details). 단일 TestingModule에서 override하면 모든 테스트에 영향.
**Rule:** Provider override가 다른 E2E describe 블록은 각자 독립적인 TestingModule(`createTestingModule → compile → createNestApplication`)을 부트한다. `beforeAll`에서 생성, `afterAll`에서 close. 격리 검증을 위해 `jest.fn()` 타입 체크 테스트 추가 권장.

### VibeScoreDashboard 테스트 — within() 스코핑 필수
**Discovered:** M006-S03 T02 heatmap + suggestion card 테스트 작성 시
**Context:** VibeScoreDashboard에 heatmap과 suggestion card가 추가되면서, 함수명(transfer, approve 등)과 변수명(balances, counter 등)이 heatmap 테이블 행/열과 suggestion card 양쪽에 동시 존재한다. `screen.getByText('transfer')`로 찾으면 multiple-elements 에러 발생.
**Rule:** heatmap/suggestion card 관련 테스트에서는 반드시 `within(screen.getByTestId('conflict-matrix'))` 또는 `within(screen.getAllByTestId('conflict-card')[index])` 형태로 스코프를 한정해야 한다. `@testing-library/react`의 `within` import 필요.
