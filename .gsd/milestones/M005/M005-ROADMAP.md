# M005: Monad Ecosystem UX Research - 사용자가 진짜 필요한 것 찾기

**Vision:** Vibe-Loom의 다음 개발 방향을 데이터 기반으로 결정한다. 모나드 생태계 현황, 경쟁 제품, 개발자 pain point, 킬러 기능 후보를 조사하고, 리서치 결과를 기능 우선순위 매트릭스 + 구체적 다음 마일스톤 제안으로 정리한다.

## Success Criteria

- M005-RESEARCH.md에 모나드 생태계 현황, 경쟁 분석, pain point, 킬러 기능 후보, UX 개선 기회가 출처와 함께 문서화되어 있다
- M005-ROADMAP.md에 3-phase 기능 우선순위 매트릭스와 구체적 다음 마일스톤 제안(슬라이스 수준 스코프)이 포함되어 있다
- 리서치에서 발견된 후보 요구사항이 REQUIREMENTS.md에 등록되어 있고 각각 우선순위/상태가 지정되어 있다
- 리서치 과정에서 내린 전략적 결정이 DECISIONS.md에 기록되어 있다
- 기존 요구사항(R006, R015, R016) 중 리서치로 인해 이해가 변경된 것에 대한 업데이트/메모가 추가되어 있다

## Key Risks / Unknowns

- **리서치 근거의 시효성** — 모나드 생태계는 빠르게 변하고 있으며 (메인넷 2025-11 런칭, NINE FORK 2026-02), 수집된 데이터의 유효기간이 짧다. 로드맵이 실행 단계에서 outdated될 수 있다.
- **병렬 실행 최적화의 실용성 미검증** — 리서치에서 P0 기능으로 제안하지만, 실제 개발자들이 이 기능을 원하는지는 코드를 작성하고 배포해봐야 검증된다. 리서치만으로는 수요를 확정할 수 없다.

## Proof Strategy

- 리서치 시효성 → M005-RESEARCH.md에 모든 출처를 날짜와 함께 기록하여 향후 재검증 가능하게 함
- 병렬 실행 최적화 수요 → M005-ROADMAP.md에서 다음 마일스톤(M006) Phase 1의 첫 슬라이스를 이 기능으로 배치하여, 실제 코드 작성을 통해 가장 빨리 검증하도록 설계

## Verification Classes

- Contract verification: `test -f .gsd/milestones/M005/M005-ROADMAP.md` + section count + YAML validity checks
- Integration verification: none (research milestone, no runtime components)
- Operational verification: none
- UAT / human verification: 로드맵 문서의 전략적 판단은 사람이 읽고 승인해야 함

## Milestone Definition of Done

This milestone is complete only when all are true:

- M005-RESEARCH.md가 완성되어 있고 6개 리서치 질문 모두에 대한 답변 + 출처가 포함됨
- M005-ROADMAP.md가 완성되어 있고 3-phase 기능 우선순위, 다음 마일스톤 제안, 요구사항 매핑이 포함됨
- 리서치에서 도출된 새 요구사항(R017+)이 REQUIREMENTS.md에 등록됨
- 전략적 결정(D021+)이 DECISIONS.md에 기록됨
- 기존 요구사항 중 리서치로 인사이트가 변경된 것(R006 notes)이 업데이트됨

## Requirement Coverage

- Covers: R006 (notes update — Vibe Score에 최적화 제안 추가 필요성 근거 문서화)
- Partially covers: R015 (리서치에서 재평가 — 재단 그랜츠 어필에 가치 있으나 구현 복잡도 높아 Phase 3 유지)
- Leaves for later: R001-R005, R007-R014 (M002 스코프, 이 리서치 마일스톤과 무관)
- Orphan risks: none

## Slices

- [x] **S01: Formalize Research into Actionable Roadmap** `risk:low` `depends:[]`
  > After this: M005-ROADMAP.md에 3-phase 기능 우선순위 매트릭스, 다음 3개 마일스톤 제안(M006/M007/M008 스코프), 9개 신규 요구사항(R017-R025), 3개 전략 결정(D021-D023)이 문서화되어 있다. R006 notes가 업데이트되어 있다.

## Boundary Map

### S01 (standalone — no downstream slices)

Produces:
- `M005-ROADMAP.md` — 3-phase feature priority matrix + next milestone proposals (M006/M007/M008)
- Updated `REQUIREMENTS.md` — 9 new candidate requirements (R017-R025) with status, priority, proposed owner
- Updated `DECISIONS.md` — 3 strategic decisions (D021-D023) from research phase
- Updated R006 notes — Vibe Score 최적화 제안 추가 필요성 근거

Consumes:
- `M005-RESEARCH.md` — completed research findings (ecosystem, competition, pain points, feature matrix)
- `M005-CONTEXT.md` — research questions and scope
- `REQUIREMENTS.md` — existing requirement registry (R001-R016)
- `DECISIONS.md` — existing decision registry (D001-D011)

---

## Feature Priority Roadmap (Research Output)

### Phase 1 — 핵심 차별화 강화 (Proposed M006)

**테마:** 병렬 실행 최적화 — 다른 도구가 제공하지 못하는 유일한 가치

| Priority | Feature | Requirement | 구현 복잡도 | 차별화 | 재단 어필 |
|----------|---------|-------------|-----------|--------|----------|
| **P0** | 병렬 실행 최적화 제안 (Vibe Score → actionable suggestions) | R017 | 중 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **P0** | Read/Write Set 충돌 시각화 | R018 | 중 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |

**예상 슬라이스 구성:**
1. CLI에서 read/write set 분석 데이터 추가 반환 + NestJS 충돌 원인 분석 로직
2. VibeScoreDashboard에 actionable suggestion 카드 + 충돌 시각화 UI
3. E2E 검증 — Vibe Score API가 구체적 코드 수정 제안을 반환하는지

**왜 먼저?** Vibe-Loom의 유일한 차별화. Remix, Tenderly, Cookbook.dev 모두 범용 EVM 도구이며, 모나드 병렬 실행 최적화 기능을 제공하는 도구는 시장에 없다. 재단 그랜츠 + 개발자 관심 확보의 핵심.

### Phase 2 — 사용자 유입 최적화 (Proposed M007)

**테마:** 온보딩 + 템플릿 — 차별화 기능이 있어야 유입된 사용자가 남음

| Priority | Feature | Requirement | 구현 복잡도 | 차별화 | 재단 어필 |
|----------|---------|-------------|-----------|--------|----------|
| **P1** | 컨트랙트 템플릿 갤러리 (10-15개 Monad 최적화 템플릿) | R019 | 저 | ⭐⭐⭐ | ⭐⭐⭐ |
| **P1** | 온보딩 가이드 투어 (5단계: 편집→컴파일→배포→분석→최적화) | R020 | 저 | ⭐⭐ | ⭐⭐ |
| **P2** | 원클릭 컨트랙트 검증 (Sourcify/Monadscan API) | R021 | 중 | ⭐⭐ | ⭐⭐ |
| **P2** | Monad 가스 최적화 제안 (via_ir, Cancun EVM 설정) | R022 | 중 | ⭐⭐⭐ | ⭐⭐⭐ |

**예상 슬라이스 구성:**
1. 컨트랙트 템플릿 갤러리 API + UI (기존 4개 하드코딩 교체)
2. 온보딩 가이드 투어 (react-joyride/driver.js 기반)
3. 원클릭 검증 + 가스 최적화 제안
4. E2E — 새 사용자가 5분 이내에 온보딩→첫 배포 완료

**왜 두 번째?** Phase 1에서 차별화 기능을 만든 후, 그 기능을 경험할 사용자를 유입해야 한다. 템플릿 갤러리는 현재 4개 하드코딩에서 10-15개로 확장하여 신규 사용자 유입 bottleneck을 해소한다.

### Phase 3 — 리텐션 + 생태계 확장 (Proposed M008+)

**테마:** 커뮤니티 + 고급 기능 — 네트워크 효과로 장기 성장

| Priority | Feature | Requirement | 구현 복잡도 | 차별화 | 재단 어필 |
|----------|---------|-------------|-----------|--------|----------|
| **P2** | 배포 후 컨트랙트 모니터링 (이벤트/트랜잭션 대시보드) | R023 | 고 | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| **P3** | 사용자 워크스페이스 (코드 저장/불러오기) | R024 | 고 | ⭐⭐ | ⭐⭐ |
| **P3** | 커뮤니티 공유/포크 | R025 | 고 | ⭐⭐⭐ | ⭐⭐ |
| **P3** | 실시간 Monad 블록 리플레이 대시보드 | R015 (재평가) | 고 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |

**왜 마지막?** 사용자 기반 없이 소셜 기능과 고급 모니터링을 만들면 텅 빈 플랫폼이 된다. Phase 1-2가 사용자를 확보한 후에 의미 있다.

---

## Next Milestone Proposals

### M006: Parallel Execution Optimization Suggestions (Phase 1)

**Scope:** monad-core CLI에서 read/write set 충돌 분석 데이터를 추가 반환하고, NestJS에서 충돌 원인 분석 + 구체적 코드 수정 제안 로직을 구현하고, Vibe-Loom VibeScoreDashboard에 actionable suggestion 카드와 충돌 시각화를 추가한다.

**Covers:** R017 (Parallel Optimization Suggestions), R018 (R/W Set Conflict Visualization), R006 (Vibe Score 강화)

**Key files:**
- `crates/cli/src/main.rs` — R/W set 분석 데이터 추가 반환
- `crates/mv-state/src/read_write_sets.rs` — 충돌 원인 분석 데이터 구조
- NestJS vibe-score 모듈 — 충돌 분석 로직 추가
- `src/components/ide/VibeScoreDashboard.tsx` — suggestion 카드 + 충돌 시각화

**Estimated:** 3-4 slices, ~6-8h

### M007: Onboarding & Template Gallery (Phase 2)

**Scope:** Monad 최적화 컨트랙트 템플릿 10-15개 갤러리, 첫 방문 온보딩 가이드 투어, 원클릭 컨트랙트 검증, 가스 최적화 제안.

**Covers:** R019 (Template Gallery), R020 (Onboarding Tour), R021 (Contract Verification), R022 (Gas Optimization)

**Estimated:** 3-4 slices, ~6-8h

### M008: Monitoring, Workspace & Community (Phase 3)

**Scope:** 배포 후 컨트랙트 모니터링, 사용자 워크스페이스, 커뮤니티 공유/포크. 사용자 기반 확보 후 진행.

**Covers:** R023 (Post-deploy Monitoring), R024 (User Workspace), R025 (Community Share/Fork), R015 (Block Replay — 재평가)

**Estimated:** 4-5 slices, ~10-12h

---

## New Requirements Summary (R017-R025)

| ID | Class | Status | Description | Priority | Proposed Owner |
|---|---|---|---|---|---|
| R017 | differentiator | active | 병렬 실행 최적화 제안 — Vibe Score에 충돌 원인 분석 + 구체적 코드 수정 제안 추가 | P0 | M006 |
| R018 | differentiator | active | Read/Write Set 충돌 시각화 — 어떤 storage slot에서 충돌이 발생하는지 시각적으로 표시 | P0 | M006 |
| R019 | primary-user-loop | active | 컨트랙트 템플릿 갤러리 — Monad 최적화 10-15개 표준 템플릿 (ERC20, ERC721, DEX 등) + 각 Vibe Score | P1 | M007 |
| R020 | primary-user-loop | active | 온보딩 가이드 투어 — 첫 방문 시 5단계 가이드 (편집→컴파일→배포→분석→최적화) | P1 | M007 |
| R021 | integration | active | 원클릭 컨트랙트 검증 — Sourcify/Monadscan API 연동 자동 검증 | P2 | M007 |
| R022 | quality-attribute | active | Monad 가스 최적화 제안 — via_ir, Cancun EVM 설정 등 Monad 특화 최적화 가이드 | P2 | M007 |
| R023 | differentiator | active | 배포 후 컨트랙트 모니터링 — 이벤트/트랜잭션 실시간 대시보드 | P2 | M008 |
| R024 | continuity | active | 사용자 워크스페이스 — 컨트랙트 코드 저장/불러오기 (PostgreSQL 백엔드) | P3 | M008 |
| R025 | primary-user-loop | active | 커뮤니티 공유/포크 — 컨트랙트 공유, 다른 사용자 코드 포크 | P3 | M008 |

## Existing Requirement Updates

| ID | Change | Rationale |
|---|---|---|
| R006 | Notes 업데이트: 현재 generic suggestions → CR-01/R017로 구체적 코드 수정 제안 필요 | 리서치에서 "점수만 보여주기" 함정 발견. 실행 가능한 제안이 핵심 차별화 |
| R015 | Status 유지: deferred, 그러나 재단 그랜츠 어필 최상위 가치 기록 | 구현 복잡도 높으나 Phase 3에서 재평가 가치 있음 |
| R016 | Status: validated (M004 완료) | M004-SUMMARY.md 참조 |

## Strategic Decisions (D021-D023)

| # | When | Scope | Decision | Choice | Rationale | Revisable? |
|---|------|-------|----------|--------|-----------|------------|
| D021 | M005 | strategy | 기능 개발 3-Phase 순서 | Phase 1: 차별화(병렬 최적화) → Phase 2: 유입(온보딩/템플릿) → Phase 3: 리텐션(커뮤니티/모니터링) | 차별화 기능이 먼저 있어야 유입된 사용자가 남고, 사용자 기반이 있어야 커뮤니티 기능이 의미 있음 | Yes — 사용자 피드백에 따라 Phase 순서 조정 가능 |
| D022 | M005 | strategy | 제품 포지셔닝 | "Monad 병렬 실행 전문 IDE"로 포지셔닝 강화 | 직접 경쟁자 없음. Remix/Tenderly/Cookbook.dev 모두 범용 EVM. monad-core Rust 엔진 기반 실제 병렬 실행 시뮬레이션은 시장 유일 | Yes — 생태계 변화에 따라 피벗 가능 |
| D023 | M005 | strategy | 재단 그랜츠 어필 포인트 | NINE FORK 준수 + 병렬 실행 최적화를 핵심 어필 포인트로 | Monad AI Blueprint 프로그램과 방향 일치. 다른 도구가 제공하지 못하는 유일한 가치 | Yes |

## "Don't Hand-Roll" Registry

| Problem | Solution | Source |
|---------|----------|--------|
| 온보딩 투어 UI | react-joyride 또는 driver.js | M005 리서치 |
| 컨트랙트 템플릿 관리 | OpenZeppelin Contracts Wizard API | M005 리서치 |
| 가스 최적화 분석 | Foundry `forge snapshot` 패턴 | M005 리서치 |
| 실시간 이벤트 모니터링 | Envio HyperIndex / Goldsky Streams | M005 리서치 |
| 컨트랙트 검증 | Sourcify API / Monadscan API | M005 리서치 |

## Open Risks (carried forward from research)

1. **재단 그랜츠 승인 불확실** — Monad Foundation 구체적 조건 미확인. AI Blueprint 적합성 추가 조사 필요
2. **Monad Foundry 경쟁/협력** — 공식 Foundry 포크 강화 시 CLI 사용자 이탈 가능. Foundry 프로젝트 임포트로 시너지 전환
3. **Tenderly Monad 지원 확대** — 병렬 실행 디버깅 추가 시 차별화 약화. 선점 중요
4. **테스트넷→메인넷 전환** — 실제 MON 비용 발생. Paymaster 경제 모델 재검토 필요
5. **병렬 실행 최적화 실용성** — 단순 컨트랙트는 충돌 없을 수 있음. 복잡한 DeFi 프로토콜 타겟팅 필요할 수 있으나 초보자 타겟과 충돌
