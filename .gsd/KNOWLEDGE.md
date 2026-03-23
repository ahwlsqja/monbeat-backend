
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

