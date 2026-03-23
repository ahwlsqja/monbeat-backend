# Decisions Register

<!-- Append-only. Never edit or remove existing rows.
     To reverse a decision, add a new row that supersedes it.
     Read this file at the start of any planning or research phase. -->

| # | When | Scope | Decision | Choice | Rationale | Revisable? |
|---|------|-------|----------|--------|-----------|------------|
| D001 | M002 | arch | 백엔드 레포 분리 | Vibe-Room-Backend 별도 레포 (ahwlsqja/Vibe-Room-Backend) | monad-core(Rust 엔진)와 독립적인 릴리스/배포 사이클. Railway 배포 단위 분리 | No |
| D002 | M002 | arch | monad-core 엔진 연동 방식 | Rust CLI 바이너리 + JSON stdin/stdout (subprocess) | Monode 패턴(Rust 백엔드)과 일치. NAPI/FFI는 빌드 복잡도 높음, WASM은 crossbeam 멀티스레드 제약. subprocess는 프로세스 격리 + 언어 독립적 | Yes — NAPI로 전환 가능하지만 복잡도 대비 이점 불분명 |
| D003 | M002 | arch | 데이터 저장소 | PostgreSQL + Prisma ORM (Railway 호스팅) | 파일 기반(Vibe-Loom 현재)에서 탈피. 다중 인스턴스, 구조화된 쿼리, Railway 플러그인 지원 | No |
| D004 | M002 | arch | 인증 체계 | GitHub OAuth (식별/가입) + WalletConnect (3회 초과 시 가스비 결제) | Cursor/AI 빌더 대상이므로 GitHub이 자연스러운 진입점. WalletConnect로 지속 가능한 운영 모델 | Yes — 추후 Wallet 단독 인증 가능 |
| D005 | M002 | arch | AI 에러 분석 | Gemini API + monad-docs RAG + FailureTracer 트레이스 | Vibe-Loom 패턴 유지하면서 monad-core 트레이스로 정확도 강화. 규칙 기반 휴리스틱 폴백 유지 | Yes — 모델 교체 가능 |
| D006 | M002 | arch | 인프라/배포 | Railway (NestJS + PostgreSQL) | 빠른 배포, GitHub 연동, PostgreSQL 플러그인, free tier 가능 | Yes — AWS/GCP로 전환 가능 |
| D007 | M002 | arch | 프론트엔드 전략 | 기존 Vibe-Loom Next.js UI 이관, API URL만 NestJS로 전환 | UI 재작업 스코프 제외. 최소 변경으로 프론트/백 분리 완성 | Yes — UI 재디자인은 별도 마일스톤 |
| D008 | M003-S01 | testing | Deploy 의존 E2E 테스트 실패 처리 | test.skip() + 방어적 다중 상태코드 assertion | Monad testnet 30-90s 소요, 타임아웃 빈번. 스위트를 항상 green 유지하면서 testnet 가용 시 실제 검증 수행 | Yes — pre-funded 테스트 월렛 확보 시 skip 제거 가능 |
| D009 | M003-S01 | testing | Playwright 모바일 셀렉터 전략 | getByRole('button', { name }) 사용, getByText 금지 | 모바일 뷰포트에서 탭 버튼(button)과 섹션 헤더(h2)에 동일 텍스트 존재하여 strict-mode 위반 | No |
| D010 | M004 | design | Vibe-Loom UI 디자인 방향 | Refined Technical — Bloomberg Terminal 수준 정보 밀도 + 세련된 타이포/컬러. impeccable 디자인 원칙 적용 | 현재 UI는 전형적 AI 슬롭. 모나드 생태계 차별화 위해 프로페셔널 품질 필요 | Yes |
| D011 | M004 | design | R016 스코프 변경 | out-of-scope → active. 사용자 명시적 요청으로 프론트엔드 UI 재디자인 진행 | M002에서는 백엔드 통합 집중을 위해 제외했으나, M003 완료 후 사용자가 디자인 개선 요청 | No |
| D012 | M004 | design | oklch 컬러 스페이스 선택 | oklch for perceptual uniformity across surface hierarchy | 3-tier surface hierarchy에서 색상 간 인지적 균일성 보장 | No |
| D013 | M004 | design | 디자인 토큰 단일 소스 | Tailwind v4 @theme block as CSS-first single source of truth | CSS 변수 기반으로 모든 컴포넌트에서 일관된 토큰 참조 | No |
| D014 | M004 | design | Monaco 테마 hex 동기화 | Monaco hex constants with JSDoc CSS variable mapping for manual sync | defineTheme API가 CSS 변수를 지원하지 않으므로 hex 상수 + JSDoc 매핑으로 수동 동기화 | Yes |
| D015 | M004 | design | 3-tier 서피스 계층 | base/raised/overlay surface hierarchy | 정보 계층을 시각적으로 구분하는 표준 패턴 | No |
| D016 | M004 | design | Accent vs Amber 시맨틱 구분 | Accent for read/structural, Amber for write/caution/warning | 두 색상이 서로 다른 의미를 갖고 있어 토큰화하지 않고 독립 유지 | No |
| D017 | M004 | design | Compile 버튼 색상 통일 | bg-accent (was bg-blue-600) | 디자인 토큰 시스템에 맞춰 통일 | No |
| D018 | M004 | design | 버튼 피드백 패턴 | CSS utility .btn-press for centralized button feedback | inline Tailwind 대신 중앙화된 모션 유틸리티 | No |
| D019 | M004 | design | UX 텍스트 언어 통일 | All user-facing text unified to English | 한/영 혼용 제거, html lang="en" 설정 | No |
| D020 | M004 | testing | Jest/Playwright 공존 | testPathIgnorePatterns added for e2e/ | 두 프레임워크가 같은 레포에 공존할 때 Jest가 Playwright 스펙을 로드하지 않도록 | No |
| D021 | M005 | strategy | 기능 개발 3-Phase 순서 | Phase 1: 차별화(병렬 최적화) → Phase 2: 유입(온보딩/템플릿) → Phase 3: 리텐션(커뮤니티/모니터링) | 차별화 기능이 먼저 있어야 유입된 사용자가 남고, 사용자 기반이 있어야 커뮤니티 기능이 의미 있음. M005 리서치 결과 | Yes — 사용자 피드백에 따라 Phase 순서 조정 가능 |
| D022 | M005 | strategy | 제품 포지셔닝 | "Monad 병렬 실행 전문 IDE"로 포지셔닝 강화 | 직접 경쟁자 없음. Remix/Tenderly/Cookbook.dev 모두 범용 EVM. monad-core 기반 실제 병렬 실행 시뮬레이션은 시장 유일 | Yes — 생태계 변화에 따라 피벗 가능 |
| D023 | M005 | strategy | 재단 그랜츠 어필 포인트 | NINE FORK 준수 + 병렬 실행 최적화를 핵심 어필 포인트로 | Monad AI Blueprint 프로그램과 방향 일치. 다른 도구가 제공하지 못하는 유일한 가치 | Yes |
| D024 | M006 | arch | R/W Set 충돌 시각화 형태 | 함수×변수명 매트릭스 히트맵 | 정보 밀도 높음. 그래프/네트워크는 복잡한 충돌에서 혼잡해질 수 있음. 매트릭스는 함수-변수 교차점에서 충돌 강도를 색상으로 직관적 표현 | Yes — 사용자 피드백에 따라 그래프 뷰 추가 가능 |
| D025 | M006 | arch | Storage layout 디코딩 범위 | solc storageLayout 기반 full 디코딩 (단순 변수 + mapping base slot + dynamic array) | raw slot 번호 대신 변수명/mapping명으로 표시하면 제안의 실용성이 크게 향상. solc가 이미 layout 정보를 제공하므로 추가 비용 낮음 | No |
| D026 | M006-S01 | arch | ReadSet 보존 전략 | return_read_set() on validation success only; failure drops ReadSet (tx re-executes with fresh one) | Validation failure = tx 재실행 → 새 ReadSet 생성. stale ReadSet 보존은 불필요하고 misleading | No |
| D027 | M006-S01 | arch | CLI conflict_details 직렬화 경계 | CLI-specific types (LocationInfo, ConflictPair) via pattern-matching, mv-state에 Serialize 추가 안 함 | mv-state는 hot execution path. serde derives 추가 시 전체 consumer에 컴파일 비용 증가. 직렬화 경계를 CLI에 한정 | Yes — 다수 consumer가 필요 시 중앙화 가능 |
| D028 | M006-S02 | arch | Storage layout decoder 모듈 아키텍처 | Pure function module (no NestJS DI) — 4 exported functions (decodeSlotToVariable, buildConflictAnalysis, generateSuggestion, buildMatrix) | DI 없는 순수 함수로 구현하면 단위 테스트가 mock 없이 가능하고, downstream wiring이 유연함 | Yes — 캐시/로깅 등 cross-cutting concern 필요 시 Service로 전환 가능 |
| D029 | M006-S02 | arch | Hex→decimal slot 비교 방식 | BigInt normalization — CLI hex ("0x0")과 solc decimal ("0") 모두 BigInt 변환 후 비교 | solc는 decimal, CLI는 hex. parseInt는 큰 수에서 정밀도 손실. BigInt가 정확하고 안전 | No |
| D030 | M006-S02 | arch | Mapping/dynamic array slot attribution heuristic | Runtime slot > max declared slot → mapping base 귀속. 단일 mapping → 정확 귀속, 복수 → "unknown (possibly X or Y)" | keccak256 기반 runtime slot은 선언된 범위 초과. 단일 mapping 케이스(가장 흔함)에서 확실, 복수는 후보 목록 제공 | Yes — keccak256 preimage DB로 개선 가능 |
| D031 | M006-S03 | design | Heatmap cell color scale | 4-tier oklch: 0→surface-base, 1→amber-500/30, 2→amber-500/60, 3+→red-400/60. Badge: amber=write-write, red=read-write | M004 디자인 토큰 일관성 유지. amber=낮은 심각도(write-write), red=높은 심각도(read-write). 4단계로 충돌 밀도 구분 | No |
| D032 | M006-S03 | design | Structured suggestion cards replace plain cards | conflictAnalysis.conflicts 비어있지 않으면 structured card가 plain 💡 card를 완전 대체 (병렬 표시 아님) | 둘 다 보여주면 중복/혼란. Structured card가 변수명/함수/타입/배지까지 포함하므로 정보량 상위호환. Plain card는 conflict analysis 없을 때만 fallback | No |
