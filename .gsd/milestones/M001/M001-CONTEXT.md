# M001: Vibe-Room Backend — NestJS 백엔드 + 엔진 통합 + Railway 배포

**Gathered:** 2026-03-22
**Status:** Ready for planning

## Project Description

Vibe-Loom(스마트 컨트랙트 보안 검증 플랫폼)의 API를 NestJS 백엔드로 재구성하고, monad-core Rust 병렬 EVM 엔진을 연동하여 실측 기반 병렬 실행 시뮬레이션 + vibe-score를 제공. Railway에 서버와 DB를 배포하고, 프론트엔드는 기존 Next.js를 유지하며 API 호출 대상만 전환.

## Why This Milestone

monad-core 엔진(298 tests, Block-STM OCC)이 완성됐지만 아직 어디에도 연결되지 않았음. Vibe-Loom은 정적 분석(regex)으로만 vibe-score를 매기고, Next.js 안에 서버 로직이 섞여있어 스케일과 독립 배포가 불가능. 이 마일스톤이 엔진 → 백엔드 → 프론트엔드 → 사용자까지의 전체 파이프라인을 완성.

## User-Visible Outcome

### When this milestone is complete, the user can:

- GitHub OAuth로 로그인하고, Solidity 컨트랙트를 입력하면 **실제 EVM 병렬 실행 기반** vibe-score를 받는다
- 모나드 테스트넷에 3회까지 무료 배포하고, 4회째부터는 WalletConnect로 자기 지갑에서 가스비를 지불한다
- 배포 에러 발생 시 Gemini AI가 모나드 문서 기반으로 수정 코드를 제안받는다
- 이 모든 것이 Railway에 배포된 라이브 서비스에서 동작한다

### Entry point / environment

- Entry point: https://{railway-domain}/api (NestJS) + https://{frontend-domain} (Next.js)
- Environment: Railway (production-like)
- Live dependencies: 모나드 RPC (테스트넷/메인넷), Gemini AI API, GitHub OAuth, PostgreSQL (Railway), WalletConnect

## Completion Class

- Contract complete means: NestJS API 13개 엔드포인트가 동작하고 유닛 테스트 통과
- Integration complete means: 프론트엔드 → NestJS → monad-core 엔진 → 모나드 RPC 전체 플로우 동작
- Operational complete means: Railway에 배포되어 외부 접근 가능

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- 프론트엔드에서 Solidity 코드를 입력하고 vibe-score를 받고, 배포를 시도하고, 에러 분석을 받는 전체 플로우가 Railway 배포 환경에서 동작
- 3회 무료 배포 후 4회째에 WalletConnect 지갑 연결 프롬프트가 표시되고, 사용자 서명으로 배포 진행
- monad-core 엔진 기반 vibe-score가 정적 분석 대비 다른(더 정확한) 결과를 반환

## Risks and Unknowns

- **Solidity → EVM 바이트코드 → monad-core 파이프라인** — Hardhat 컴파일 결과에서 바이트코드를 추출하고 monad-core Transaction 형태로 변환하는 과정이 검증 안 됨
- **병렬 실행 시뮬레이션의 의미** — 스토리지 레이아웃 없이 deploy tx만으로 의미 있는 충돌 패턴을 감지할 수 있는지
- **WalletConnect 서버 사이드 통합** — 일반적으로 클라이언트 전용인 WalletConnect를 NestJS 백엔드 relay와 결합
- **Railway에서 Rust 바이너리 실행** — monad-core CLI 바이너리를 Railway 컨테이너에 포함시키는 빌드 파이프라인

## Existing Codebase / Prior Art

- `monad-core/crates/evm/src/block_executor.rs` — execute_block_parallel(), execute_block_sequential()
- `monad-core/crates/evm/src/tracer.rs` — FailureTracer, TraceResult
- `monad-core/crates/scheduler/src/parallel_executor.rs` — execute_block_parallel()
- `/tmp/vibe-loom/src/app/api/` — 5개 Next.js API Routes (이관 대상)
- `/tmp/vibe-loom/src/lib/` — AI, optimizer, paymaster, error handler, prompt templates
- `/tmp/vibe-loom/contracts/` — 4개 테스트 컨트랙트 (FailingContract, FixedContract, PectraTest, ParallelConflict)
- `/tmp/vibe-loom/data/monad-docs/` — RAG 컨텍스트 문서 5개

> See `.gsd/DECISIONS.md` for all architectural and pattern decisions.

## Relevant Requirements

- R001~R013 — 전부 이 마일스톤에서 다룸

## Scope

### In Scope

- NestJS 프로젝트 스캐폴딩 (별도 레포: Vibe-Room-Backend)
- Vibe-Loom API 5개 NestJS 모듈로 이관
- PostgreSQL + Prisma 데이터 레이어
- GitHub OAuth 인증
- WalletConnect 지갑 연결 (3회 초과 시)
- monad-core CLI 바이너리 빌드 + JSON 인터페이스
- 실측 EVM 시뮬레이션 기반 vibe-score
- 엔진 트레이스 기반 에러 분석 강화
- Next.js 프론트엔드 API 전환
- Railway 배포 (서버 + DB)
- E2E 통합 테스트

### Out of Scope / Non-Goals

- 프론트엔드 UI 재디자인
- Docker/Kubernetes 인프라 (Railway가 담당)
- CachedStateProvider 파이프라인 통합 (deferred)
- 체인 replay 대시보드 (deferred)

## Technical Constraints

- 백엔드는 Vibe-Room-Backend 별도 레포에서 관리
- monad-core는 Vibe-Room-Core 레포에서 CLI 바이너리로 빌드
- Railway free tier 또는 starter plan 범위 내
- Gemini API key 필요 (환경변수)
- 모나드 테스트넷 private key 필요 (Paymaster 서버 지갑)

## Integration Points

- **모나드 RPC** — ethers.js로 테스트넷/메인넷 배포
- **Gemini AI API** — RAG 에러 분석
- **GitHub OAuth** — 사용자 인증
- **WalletConnect** — 3회 초과 시 지갑 연결
- **monad-core CLI** — subprocess로 병렬 실행 시뮬레이션
- **Railway** — 서버 + PostgreSQL 호스팅

## Open Questions

- monad-core CLI 바이너리를 Railway Docker 이미지에 포함시키는 최적 방법 — multi-stage build?
- WalletConnect v2 서버사이드 relay 패턴 — 프론트엔드에서 서명 → 백엔드에서 raw tx broadcast?
