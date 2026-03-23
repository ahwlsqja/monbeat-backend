# Requirements

## Active

### R001 — NestJS 프로젝트 구조 + 모듈 아키텍처
- Class: core-capability
- Status: active
- Description: NestJS 프로젝트가 모듈별로 분리된 구조(deploy, analysis, vibe-score, auth, engine)로 구성되고 Railway에 배포 가능한 형태여야 한다
- Why it matters: 백엔드의 기반 구조
- Source: inferred
- Primary owning slice: M002/S01
- Supporting slices: none
- Validation: unmapped
- Notes: Prisma + PostgreSQL 포함

### R002 — 컨트랙트 소스 관리 API
- Class: core-capability
- Status: active
- Description: 컨트랙트 소스 조회(GET) 및 사용자 코드 업로드를 지원하는 API
- Why it matters: 모든 기능(배포, 분석, vibe-score)의 입력이 컨트랙트 소스
- Source: user
- Primary owning slice: M002/S02
- Supporting slices: none
- Validation: unmapped
- Notes: Vibe-Loom /api/contract-source 이관

### R003 — Hardhat 기반 컨트랙트 컴파일 + 모나드 배포 API
- Class: core-capability
- Status: active
- Description: Solidity 소스를 Hardhat으로 컴파일하고 모나드 테스트넷/메인넷에 배포하는 API
- Why it matters: 핵심 사용자 루프 — 코드 → 배포
- Source: user
- Primary owning slice: M002/S02
- Supporting slices: none
- Validation: unmapped
- Notes: Vibe-Loom /api/deploy 이관. ethers.js + Hardhat toolbox

### R004 — Gemini AI RAG 기반 배포 에러 분석 + 수정 제안 API
- Class: core-capability
- Status: active
- Description: 배포 에러 발생 시 monad-docs를 컨텍스트로 Gemini AI에 전달하여 수정 코드와 설명을 반환하는 API. 스트리밍 응답 지원
- Why it matters: Vibe-Loom의 핵심 차별화 — AI 기반 자동 수정 루프
- Source: user
- Primary owning slice: M002/S03
- Supporting slices: none
- Validation: unmapped
- Notes: Vibe-Loom /api/analyze-deployment-error 이관. 규칙 기반 휴리스틱 폴백 포함

### R005 — monad-core Rust 엔진 CLI 바이너리 + JSON 인터페이스
- Class: core-capability
- Status: active
- Description: monad-core 엔진을 CLI 바이너리로 빌드하고, JSON stdin/stdout으로 트랜잭션 블록을 받아 실행 결과를 반환
- Why it matters: NestJS와 Rust 엔진 브릿지. 모나드 생태계(C++/Rust)에 맞는 연동 방식
- Source: inferred
- Primary owning slice: M002/S04
- Supporting slices: none
- Validation: unmapped
- Notes: Monode 패턴(Rust 백엔드)과 일치. subprocess 호출

### R006 — 실제 EVM 병렬 실행 시뮬레이션 기반 Vibe-Score API
- Class: differentiator
- Status: active
- Description: Solidity 소스를 컴파일 → 바이트코드 추출 → monad-core 엔진으로 병렬 실행 시뮬레이션 → 충돌 횟수, 재실행 비율, gas 효율 기반 실측 점수 반환
- Why it matters: 정적 분석 대신 실제 EVM 실행 기반 점수. 프로젝트 핵심 차별화
- Source: user
- Primary owning slice: M002/S04
- Supporting slices: M002/S02
- Validation: unmapped
- Notes: Hardhat 컴파일 → ABI + bytecode → 엔진에 트랜잭션 블록 구성 → 실행 → 결과 파싱

### R007 — PostgreSQL + Prisma 데이터 레이어
- Class: continuity
- Status: active
- Description: 배포 이력, 분석 결과, 사용자 데이터를 PostgreSQL에 저장. Prisma ORM
- Why it matters: 파일 기반 저장에서 탈피. 다중 인스턴스, Railway 배포 대응
- Source: user
- Primary owning slice: M002/S01
- Supporting slices: M002/S02, M002/S03
- Validation: unmapped
- Notes: Railway PostgreSQL 플러그인

### R008 — GitHub OAuth 인증 + Paymaster (3회 무료 → WalletConnect 전환)
- Class: primary-user-loop
- Status: active
- Description: GitHub OAuth로 로그인, 사용자별 배포 횟수 추적. 3회까지 서버 지갑 대리 배포, 3회 초과 시 WalletConnect로 사용자 지갑 연결하여 가스비 직접 결제. AI 피드백 인프라는 무료 유지
- Why it matters: 진입 장벽 제거 + 지속 가능한 운영 모델
- Source: user
- Primary owning slice: M002/S03
- Supporting slices: none
- Validation: unmapped
- Notes: 3회 초과 시 프론트엔드에서 WalletConnect 모달 표시 → 사용자 서명 → 백엔드가 tx relay

### R009 — Next.js 프론트엔드 API 엔드포인트 전환
- Class: integration
- Status: active
- Description: 기존 Vibe-Loom Next.js 프론트엔드의 API 호출 대상을 NestJS 백엔드로 전환. UI 변경 최소화
- Why it matters: 프론트엔드/백엔드 분리 완성
- Source: user
- Primary owning slice: M002/S05
- Supporting slices: none
- Validation: unmapped
- Notes: CORS 설정, 환경변수 기반 API URL, WalletConnect 통합

### R010 — E2E 통합 테스트
- Class: quality-attribute
- Status: active
- Description: 전체 파이프라인(로그인→소스→vibe-score→배포→에러 분석→수정 제안) E2E 테스트
- Why it matters: 통합이 실제로 동작하는지 검증
- Source: inferred
- Primary owning slice: M002/S06
- Supporting slices: M003/S01
- Validation: M003-S01: 22 Playwright E2E tests (20 passed, 1 skipped, 1 flaky/retry pass). Covers page load→compile→deploy→AI analysis→7 API endpoints→mobile responsive→4 contract selector. Missing: GitHub OAuth, WalletConnect.
- Notes: NestJS e2e testing + supertest + Playwright full-stack E2E

### R011 — 엔진 트레이스 기반 실패 분석
- Class: differentiator
- Status: active
- Description: monad-core FailureTracer를 연동하여 실행 실패 시 JSON 트레이스(PC, gas, revert reason)를 에러 분석에 활용
- Why it matters: AI 분석 정확도 향상 — 실제 실행 트레이스가 컨텍스트로 들어감
- Source: inferred
- Primary owning slice: M002/S04
- Supporting slices: M002/S03
- Validation: unmapped
- Notes: FailureTracer는 monad-core S06에서 구현 완료

### R012 — Railway 배포 (NestJS 서버 + PostgreSQL)
- Class: operability
- Status: active
- Description: NestJS 서버와 PostgreSQL을 Railway에 배포. 환경변수 관리, health check, 자동 배포
- Why it matters: 실제 서비스 운영
- Source: user
- Primary owning slice: M002/S06
- Supporting slices: M002/S01
- Validation: unmapped
- Notes: Railway CLI 또는 GitHub 연동

### R013 — WalletConnect 지갑 연결 (3회 초과 시 가스비 결제)
- Class: primary-user-loop
- Status: active
- Description: 3회 무료 배포 초과 시 WalletConnect로 사용자 지갑을 연결하고, 사용자가 직접 트랜잭션에 서명하여 가스비 결제
- Why it matters: 지속 가능한 운영 — 무한 무료 배포 방지하면서 AI 피드백은 계속 제공
- Source: user
- Primary owning slice: M002/S03
- Supporting slices: M002/S05
- Validation: unmapped
- Notes: 프론트엔드에서 WalletConnect 모달, 백엔드에서 서명 검증 + tx relay

## Validated

(none yet — M001 requirements are in monad-core repo)

## Deferred

### R014 — CachedStateProvider 파이프라인 통합
- Class: core-capability
- Status: deferred
- Description: monad-core CachedStateProvider를 parallel executor에 실제 연결
- Why it matters: 재실행 성능 개선
- Source: inferred
- Primary owning slice: none
- Validation: unmapped
- Notes: 인프라 완성됨. 연결만 하면 됨

### R015 — 실 체인 블록 replay 대시보드
- Class: differentiator
- Status: deferred
- Description: 모나드 메인넷 블록을 가져와서 병렬 실행 replay 시각화
- Why it matters: 엔진의 실 체인 호환성 시연
- Source: inferred
- Primary owning slice: none
- Validation: unmapped
- Notes: chain_replay 테스트가 이미 검증됨

## Out of Scope

### R016 — 프론트엔드 UI 재디자인
- Class: anti-feature
- Status: out-of-scope
- Description: 기존 Vibe-Loom UI를 새로 디자인하는 것은 하지 않음. 이관만 함
- Why it matters: 스코프 제한 — 백엔드 통합에 집중
- Source: user
- Primary owning slice: none
- Validation: n/a

## Traceability

| ID | Class | Status | Primary owner | Supporting | Proof |
|---|---|---|---|---|---|
| R001 | core-capability | active | M002/S01 | none | unmapped |
| R002 | core-capability | active | M002/S02 | none | unmapped |
| R003 | core-capability | active | M002/S02 | none | unmapped |
| R004 | core-capability | active | M002/S03 | none | unmapped |
| R005 | core-capability | active | M002/S04 | none | unmapped |
| R006 | differentiator | active | M002/S04 | M002/S02 | unmapped |
| R007 | continuity | active | M002/S01 | M002/S02, M002/S03 | unmapped |
| R008 | primary-user-loop | active | M002/S03 | none | unmapped |
| R009 | integration | active | M002/S05 | none | unmapped |
| R010 | quality-attribute | active | M002/S06 | none | unmapped |
| R011 | differentiator | active | M002/S04 | M002/S03 | unmapped |
| R012 | operability | active | M002/S06 | M002/S01 | unmapped |
| R013 | primary-user-loop | active | M002/S03 | M002/S05 | unmapped |
| R014 | core-capability | deferred | none | none | unmapped |
| R015 | differentiator | deferred | none | none | unmapped |
| R016 | anti-feature | out-of-scope | none | none | n/a |

## Coverage Summary

- Active requirements: 13
- Mapped to slices: 13
- Validated: 0
- Unmapped active requirements: 0
