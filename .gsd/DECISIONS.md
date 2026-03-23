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
