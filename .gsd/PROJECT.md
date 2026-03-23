# Vibe-Room — Monad Smart Contract Security Platform

## What This Is

Monad 네트워크를 위한 스마트 컨트랙트 보안 검증 + 병렬 실행 최적화 플랫폼. monad-core Rust 병렬 EVM 엔진(298 tests, Block-STM OCC, NINE FORK 준수)을 NestJS 백엔드에 연동하여, Solidity 컨트랙트를 실제로 병렬 실행 시뮬레이션하고 충돌/재실행 기반 실측 vibe-score를 제공한다.

## Core Value

**사용자 유입 최우선** — Cursor/AI로 코딩하는 빌더들이 모나드에 가장 쉽게 안착하도록 '배포 에러 자동 수정 루프'와 '무료 배포'를 제공. 3회 초과 시 WalletConnect로 사용자 지갑 결제 전환하여 지속 가능한 운영.

## Current State

- **monad-core** (Vibe-Room-Core 레포): Rust 병렬 EVM 엔진 완성. 7 crates, 298 tests, Block-STM OCC, NINE FORK(MIP-3/4/5) 규정 준수. 200 tx 블록까지 parallel==sequential state root 검증. 모나드 메인넷 블록 replay 검증.
- **Vibe-Loom** (leejk206/Vibe-Loom): Next.js 프론트엔드 — 22 Playwright E2E 테스트로 전체 플로우 검증 완료 (로드→편집→컴파일→배포→AI 분석→인터랙션). refined technical aesthetic 디자인 적용 (M004). 모바일 반응형 검증, 4-contract 셀렉터 검증, 10 스크린샷 증거 파일.
- **Vibe-Room-Backend** (ahwlsqja/Vibe-Room-Backend): NestJS 백엔드 — 7개 API 엔드포인트 E2E 검증 완료 (health, contract-source, compile, deploy, vibe-score, analysis/error, paymaster/status).
- **Product Roadmap** (M005 리서치 기반): 3-phase 개발 로드맵 수립 — Phase 1 병렬 실행 차별화(M006), Phase 2 온보딩/유입(M007), Phase 3 리텐션/커뮤니티(M008). 25개 요구사항, 23개 결정 등록.

## Architecture / Key Patterns

- **별도 레포**: monad-core(Rust)는 Vibe-Room-Core, 백엔드(NestJS)는 Vibe-Room-Backend
- **엔진 연동**: monad-core Rust CLI 바이너리 + JSON stdin/stdout → NestJS subprocess 호출
- **인증**: GitHub OAuth (가입/식별) + WalletConnect (3회 초과 시 가스비 결제)
- **데이터**: PostgreSQL + Prisma (Railway 호스팅)
- **AI**: Gemini API (RAG 에러 분석, monad-docs 컨텍스트)
- **인프라**: Railway (NestJS 서버 + PostgreSQL)
- **프론트엔드**: Next.js (기존 Vibe-Loom UI 이관, API 대상만 NestJS로 전환)

## Strategic Positioning

- **제품 포지셔닝 (D022)**: "Monad 병렬 실행 전문 IDE" — 직접 경쟁자 없음. Remix/Tenderly/Cookbook.dev 모두 범용 EVM.
- **재단 그랜츠 타겟 (D023)**: NINE FORK 규정 준수 + 병렬 실행 최적화를 핵심 어필 포인트로. Monad AI Blueprint 프로그램 방향 일치.
- **개발 순서 (D021)**: Phase 1 차별화(병렬 최적화) → Phase 2 유입(온보딩/템플릿) → Phase 3 리텐션(커뮤니티/모니터링)
- **AI 지식 업데이트**: 모나드 최신 기술 문서를 실시간 반영하는 RAG 구조
- **지속 가능한 운영**: GitHub 인증 기반 초기 3회 가스비 완납 → 이후 WalletConnect 지갑 결제

## Capability Contract

See `.gsd/REQUIREMENTS.md` for the explicit capability contract, requirement status, and coverage mapping.

## Milestone Sequence

- [x] M001: Monad Core — Parallel EVM Execution Engine (completed, 298 tests)
- [ ] M002: Vibe-Room Backend — NestJS 백엔드 + 엔진 통합 + Railway 배포
- [x] M003: Full-Stack E2E QA — 시제품 수준 통합 검증 (completed, 22 Playwright E2E tests, 10 screenshot evidence files)
- [x] M004: Vibe-Loom UI Redesign — Refined Technical Aesthetic (impeccable 디자인 원칙 기반 전면 재설계)
- [x] M005: Monad Ecosystem UX Research — 모나드 생태계 리서치 → 3-phase 로드맵(M006/M007/M008), R017-R025 요구사항 + D021-D023 전략 결정 등록 완료
- [ ] M006: Parallel Execution Optimization Suggestions — Vibe Score를 "점수"에서 "처방전"으로. R/W set 충돌 데이터 + storage layout 디코딩 + 매트릭스 히트맵. **S01 완료** (CLI conflict_details JSON 출력, 25+7 tests pass), **S02 완료** (NestJS storage layout 디코딩 + actionable suggestion 생성, 43 tests pass), **S03 완료** (Vibe-Loom 매트릭스 히트맵 + suggestion 카드 UI, 16+11 tests pass), **S04 완료** (E2E 검증 — Backend 15 E2E pass, Frontend 63 unit pass, Playwright 23 E2E pass)
- [ ] M007: Onboarding & Template Gallery (Phase 2) — 컨트랙트 템플릿 갤러리, 온보딩 투어, 원클릭 검증, 가스 최적화
- [ ] M008: Monitoring, Workspace & Community (Phase 3) — 배포 후 모니터링, 워크스페이스, 커뮤니티
