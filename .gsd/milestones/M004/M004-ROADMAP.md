# M004: Vibe-Loom UI Redesign - Refined Technical Aesthetic

**Vision:** impeccable 디자인 원칙을 적용하여 Vibe-Loom을 Bloomberg Terminal 수준의 정보 밀도와 세련된 심미성을 갖춘 Refined Technical IDE로 전면 재설계한다. 기능은 동일하게 유지하면서 시각적으로 "AI가 만든 것 같지 않은" 수준의 디자인 퀄리티를 달성한다.

## Success Criteria

- 디자인 시스템(타이포그래피 스케일, 컬러 팔레트, 스페이싱, 모션 토큰)이 전체 컴포넌트에 일관 적용
- impeccable 안티패턴 제로: gray text on colored bg, pure black/white, Inter/system fonts, cards-in-cards, gradient text 없음
- 페이지 로드 시 orchestrated entry animation (staggered reveals)
- 인터랙션마다 의도적인 모션 피드백 (버튼, 패널 전환, 상태 변화)
- Monaco Editor에 커스텀 테마 적용 (기존 vs-dark가 아닌)
- 데스크톱 + 모바일(375×812) 양쪽에서 레이아웃이 의도대로 렌더링
- 기존 22개 E2E 테스트 전부 PASS (기능 회귀 없음)
- UX 카피 일관성 (한/영 혼용 해결)

## Key Risks / Unknowns

- **E2E 셀렉터 깨짐** — DOM 구조 변경 시 기존 Playwright 셀렉터 실패 가능. 최종 슬라이스에서 수정.
- **Monaco 테마 커스터마이징 한계** — Monaco는 자체 JSON 테마 시스템. CSS 변수와 완전 통합 어려울 수 있음.
- **모션 성능** — 과도한 애니메이션은 IDE 사용성을 해칠 수 있음. 절제된 high-impact 모션만.

## Proof Strategy

- E2E 셀렉터 깨짐 → retire in S04 by proving 22/22 E2E 테스트 pass
- Monaco 테마 한계 → retire in S02 by proving 커스텀 테마가 디자인 시스템 컬러와 조화

## Verification Classes

- Contract verification: 기존 E2E 22개 테스트 PASS, impeccable 안티패턴 수동 점검
- Integration verification: 라이브 서비스에서 full flow 동작 확인
- Operational verification: none
- UAT / human verification: 시각적 품질은 스크린샷 대비로 판단

## Milestone Definition of Done

This milestone is complete only when all are true:

- 전체 13 컴포넌트 + page.tsx가 새 디자인 시스템으로 렌더링
- 디자인 토큰(font, color, spacing, motion)이 globals.css/tailwind에 정의
- 기존 22개 E2E 테스트가 새 UI에서 전부 PASS
- 데스크톱 + 모바일 스크린샷 증거 캡처
- impeccable 안티패턴 체크리스트 전부 클리어

## Requirement Coverage

- Covers: R016 (프론트엔드 UI 재디자인 — 사용자 요청으로 scope 변경)
- Partially covers: R009 (Next.js 프론트엔드 — 시각적 개선)
- Leaves for later: R008, R013 (인증/WalletConnect 기능 — 스타일만 변경)

## Slices

- [x] **S01: Design Foundation — 디자인 시스템 + 레이아웃 셸** `risk:high` `depends:[]`
  > After this: 디자인 토큰(폰트, 컬러, 스페이싱, 모션), globals.css, 커스텀 Monaco 테마가 정의되고 IDELayout이 새 디자인으로 렌더링됨. 브라우저에서 새 레이아웃 셸 확인 가능.

- [x] **S02: Core Components — 에디터 + 사이드바 + 콘솔 리팩토링** `risk:medium` `depends:[S01]`
  > After this: EditorPanel(툴바 포함), SidebarPanel(배포/AI분석/바이브스코어), ConsolePanel(트랜잭션 로그)이 새 디자인 시스템으로 렌더링. 전체 IDE가 시각적으로 일관됨.

- [x] **S03: Motion + Polish — 애니메이션, 모바일, UX 카피** `risk:medium` `depends:[S02]`
  > After this: 페이지 로드 entry animation, 인터랙션 모션, 모바일 적응형 레이아웃, 한/영 UX 카피 일관성 완료. 모든 시각적 디테일 마무리.

- [x] **S04: Regression — E2E 테스트 호환 + 최종 검증** `risk:low` `depends:[S03]`
  > After this: 기존 22개 E2E 테스트가 새 UI에서 전부 PASS. 데스크톱/모바일 스크린샷 증거. impeccable 안티패턴 체크리스트 클리어.

## Boundary Map

### S01 → S02

Produces:
- CSS custom properties (design tokens): `--font-display`, `--font-body`, `--color-*`, `--space-*`, `--ease-*`
- Tailwind theme extension with custom values
- Monaco custom theme JSON
- IDELayout 새 구조 (시각적 셸)

Consumes:
- nothing (first slice)

### S02 → S03

Produces:
- 리팩토링된 전체 IDE 컴포넌트 (EditorPanel, SidebarPanel, ConsolePanel, VibeScoreDashboard, ContractInteraction, etc.)
- 일관된 디자인 토큰 사용

Consumes:
- S01의 디자인 토큰, 레이아웃 셸, Monaco 테마

### S03 → S04

Produces:
- 완성된 UI (모션, 모바일, UX 카피 포함)

Consumes:
- S02의 리팩토링된 컴포넌트
- S01의 모션 토큰
