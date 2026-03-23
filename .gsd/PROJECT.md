# Vibe-Room — Monad Smart Contract Security Platform

## What This Is

Monad 네트워크를 위한 스마트 컨트랙트 보안 검증 + 병렬 실행 최적화 플랫폼. monad-core Rust 병렬 EVM 엔진(298 tests, Block-STM OCC, NINE FORK 준수)을 NestJS 백엔드에 연동하여, Solidity 컨트랙트를 실제로 병렬 실행 시뮬레이션하고 충돌/재실행 기반 실측 vibe-score를 제공한다.

## Core Value

**사용자 유입 최우선** — Cursor/AI로 코딩하는 빌더들이 모나드에 가장 쉽게 안착하도록 '배포 에러 자동 수정 루프'와 '무료 배포'를 제공. 3회 초과 시 WalletConnect로 사용자 지갑 결제 전환하여 지속 가능한 운영.

## Current State

- **monad-core** (Vibe-Room-Core 레포): Rust 병렬 EVM 엔진 완성. 7 crates, 298 tests, Block-STM OCC, NINE FORK(MIP-3/4/5) 규정 준수. 200 tx 블록까지 parallel==sequential state root 검증. 모나드 메인넷 블록 replay 검증.
- **Vibe-Loom** (leejk206/Vibe-Loom): Next.js 프론트엔드 — 22 Playwright E2E 테스트로 전체 플로우 검증 완료 (로드→편집→컴파일→배포→AI 분석→인터랙션). 모바일 반응형 검증, 4-contract 셀렉터 검증, 10 스크린샷 증거 파일.
- **Vibe-Room-Backend** (ahwlsqja/Vibe-Room-Backend): NestJS 백엔드 — 7개 API 엔드포인트 E2E 검증 완료 (health, contract-source, compile, deploy, vibe-score, analysis/error, paymaster/status).

## Architecture / Key Patterns

- **별도 레포**: monad-core(Rust)는 Vibe-Room-Core, 백엔드(NestJS)는 Vibe-Room-Backend
- **엔진 연동**: monad-core Rust CLI 바이너리 + JSON stdin/stdout → NestJS subprocess 호출
- **인증**: GitHub OAuth (가입/식별) + WalletConnect (3회 초과 시 가스비 결제)
- **데이터**: PostgreSQL + Prisma (Railway 호스팅)
- **AI**: Gemini API (RAG 에러 분석, monad-docs 컨텍스트)
- **인프라**: Railway (NestJS 서버 + PostgreSQL)
- **프론트엔드**: Next.js (기존 Vibe-Loom UI 이관, API 대상만 NestJS로 전환)

## Strategic Positioning

- **재단 그랜츠 타겟**: NINE FORK 규정 준수 + 병렬 처리 최적화 로직을 엔진에 탑재
- **AI 지식 업데이트**: 모나드 최신 기술 문서를 실시간 반영하는 RAG 구조
- **지속 가능한 운영**: GitHub 인증 기반 초기 3회 가스비 완납 → 이후 WalletConnect 지갑 결제

## Capability Contract

See `.gsd/REQUIREMENTS.md` for the explicit capability contract, requirement status, and coverage mapping.

## Milestone Sequence

- [x] M001: Monad Core — Parallel EVM Execution Engine (completed, 298 tests)
- [ ] M002: Vibe-Room Backend — NestJS 백엔드 + 엔진 통합 + Railway 배포
- [x] M003: Full-Stack E2E QA — 시제품 수준 통합 검증 (completed, 22 Playwright E2E tests, 10 screenshot evidence files)
