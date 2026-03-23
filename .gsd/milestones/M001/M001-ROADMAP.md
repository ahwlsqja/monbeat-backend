# M001: Vibe-Room Backend - NestJS 백엔드 + 엔진 통합 + Railway 배포

**Vision:** Vibe-Loom의 스마트 컨트랙트 보안 검증 API를 NestJS로 재구성하고, monad-core Rust 병렬 EVM 엔진을 연동하여 실측 기반 vibe-score를 제공하며, GitHub OAuth + WalletConnect 인증과 Paymaster를 갖춘 라이브 서비스를 Railway에 배포한다.

## Success Criteria

- NestJS 서버가 Railway에서 기동되고 모든 API 엔드포인트가 응답한다
- Solidity 소스를 입력하면 monad-core 엔진이 병렬 실행 시뮬레이션하여 실측 vibe-score를 반환한다
- 모나드 테스트넷에 컨트랙트 배포가 동작한다 (3회 무료, 이후 WalletConnect)
- 배포 에러 시 Gemini AI RAG가 수정 코드를 제안한다
- Next.js 프론트엔드가 NestJS 백엔드만 사용하며 기존 UX와 동일하게 동작한다

## Key Risks / Unknowns

- **Solidity → monad-core 파이프라인** — Hardhat 컴파일 바이트코드를 monad-core Transaction으로 변환하는 과정이 검증 안 됨
- **병렬 시뮬레이션 의미** — deploy tx만으로 의미 있는 충돌 패턴을 감지할 수 있는지
- **Railway Rust 바이너리** — monad-core CLI를 Railway 컨테이너에 포함시키는 빌드 방법
- **WalletConnect 서버 통합** — 프론트엔드 서명 → 백엔드 relay 패턴

## Proof Strategy

- Solidity → monad-core 파이프라인 → S04에서 ParallelConflict.sol 컴파일 → 엔진 실행 → 충돌 감지 증명
- 병렬 시뮬레이션 의미 → S04에서 GlobalCounter(병목) vs mapping(분산) 패턴의 점수 차이 증명
- Railway Rust 바이너리 → S06에서 Dockerfile multi-stage build로 증명
- WalletConnect → S03에서 프론트엔드 서명 + 백엔드 broadcast 증명

## Verification Classes

- Contract verification: `npm run test` — NestJS 유닛 테스트
- Integration verification: 프론트엔드 → NestJS → monad-core → 모나드 RPC 전체 플로우
- Operational verification: Railway 배포 + health check + 외부 접근
- UAT / human verification: 프론트엔드에서 전체 사용자 플로우 수동 검증

## Milestone Definition of Done

- 모든 6개 슬라이스 완료
- NestJS 서버가 Railway에서 동작하며 health check 응답
- monad-core 엔진 기반 vibe-score가 ParallelConflict vs FixedContract에서 다른 점수 반환
- GitHub OAuth 로그인 → 3회 무료 배포 → 4회째 WalletConnect 지갑 연결
- 프론트엔드가 NestJS API만 사용하며 기존 Vibe-Loom UX 동작
- E2E 테스트가 전체 플로우 검증

## Requirement Coverage

- Covers: R001~R013
- Partially covers: none
- Leaves for later: R014, R015
- Orphan risks: none

## Slices

- [x] **S01: NestJS Foundation + Database** `risk:medium` `depends:[]`
  > After this: `npm run start:dev` → NestJS 서버 기동, Prisma migration 완료, health/readiness API 응답, 빈 Vibe-Room-Backend 레포에 push

- [x] **S02: Contract & Deploy Module** `risk:medium` `depends:[S01]`
  > After this: POST /api/contracts/deploy → 모나드 테스트넷에 FixedContract 배포 성공, GET /api/contracts/source → 소스 반환, 배포 이력 DB 저장

- [x] **S03: Auth + Analysis + Paymaster Module** `risk:high` `depends:[S01]`
  > After this: GitHub OAuth 로그인 → JWT 발급 → 배포 상태 조회 (3회 제한), 에러 분석 API가 Gemini RAG로 수정 코드 반환, WalletConnect로 4회째 배포 시 사용자 서명 요구

- [x] **S04: Engine Bridge + Vibe-Score** `risk:high` `depends:[S01,S02]`
  > After this: POST /api/vibe-score → Solidity 컴파일 → monad-core CLI 병렬 실행 → 실측 충돌/재실행 기반 점수 반환. ParallelConflict(병목)이 FixedContract(정상)보다 낮은 점수

- [x] **S05: Frontend Integration** `risk:low` `depends:[S02,S03,S04]`
  > After this: Next.js 프론트엔드가 NestJS 백엔드 API를 호출, 기존 Vibe-Loom과 동일한 UX. WalletConnect 모달 포함

- [x] **S06: Railway Deploy + E2E Validation** `risk:medium` `depends:[S05]`
  > After this: Railway에 NestJS + PostgreSQL 배포 완료, E2E 테스트 통과, 외부에서 전체 플로우 동작

## Boundary Map

### S01 → S02, S03, S04

Produces:
- NestJS AppModule with ConfigModule, PrismaModule, HealthModule
- PrismaService with User, Deployment, Analysis 스키마
- Prisma migration infrastructure (Railway PostgreSQL 대응)
- BaseController pattern (표준 응답 형식, 에러 핸들링)
- `Vibe-Room-Backend` 레포 초기 구조

Consumes:
- nothing (first slice)

### S02 → S04, S05

Produces:
- ContractsModule: GET /api/contracts/source, POST /api/contracts/compile, POST /api/contracts/deploy
- HardhatService: 컴파일(solc) + 배포(ethers.js) 로직
- DeploymentRecord: DB에 배포 이력 저장 (address, tx hash, status, user_id)
- 컴파일된 바이트코드 + ABI 반환 인터페이스 (S04 엔진 연동용)

Consumes from S01:
- PrismaService (배포 이력 저장)
- ConfigModule (MONAD_RPC_URL, MONAD_PRIVATE_KEY)

### S03 → S05

Produces:
- AuthModule: GitHub OAuth callback → JWT 발급, /api/auth/me
- AnalysisModule: POST /api/analysis/error → Gemini RAG 수정 제안 (스트리밍 지원)
- PaymasterModule: GET /api/paymaster/status, 배포 횟수 체크 + 서버 지갑 relay
- WalletConnect 서명 검증 엔드포인트: POST /api/paymaster/relay-signed
- JwtAuthGuard: 인증 필요 엔드포인트 보호

Consumes from S01:
- PrismaService (User, DeployCount 저장)
- ConfigModule (GITHUB_CLIENT_ID/SECRET, GEMINI_API_KEY, JWT_SECRET)

### S04 → S05

Produces:
- EngineModule: monad-core CLI subprocess 관리
- VibeScoreModule: POST /api/vibe-score → 컴파일 + 엔진 실행 + 점수 계산
- EngineService: spawnSync/spawn으로 monad-core CLI 호출, JSON 파싱
- SimulationResult 타입: { conflicts, reExecutions, gasEfficiency, vibeScore, traceResults }

Consumes from S01:
- ConfigModule (ENGINE_BINARY_PATH)

Consumes from S02:
- HardhatService.compile() → 바이트코드 + ABI

### S05 → S06

Produces:
- Next.js 프론트엔드 (Vibe-Loom에서 이관, API URL만 NestJS로 변경)
- WalletConnect 통합 UI (3회 초과 시 지갑 연결 모달)
- CORS 설정 완료
- 환경변수 기반 API URL 구성

Consumes from S02, S03, S04:
- 전체 NestJS API 엔드포인트

### S06 (final)

Produces:
- Dockerfile (multi-stage: Rust CLI 빌드 + Node.js NestJS)
- railway.json / Procfile 설정
- E2E 테스트 스위트 (supertest)
- Health check + readiness probe
- 환경변수 관리 (Railway dashboard)

Consumes from S05:
- 완성된 NestJS + 프론트엔드
