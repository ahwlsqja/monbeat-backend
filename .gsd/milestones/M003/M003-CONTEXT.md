# M003: Full-Stack E2E QA — 시제품 수준 통합 검증

**Vision:** 브라우저에서 라이브 서비스(vibe-loom.xyz + Railway 백엔드 + monad-core 엔진)까지 관통하는 Playwright E2E 테스트로 전체 사용자 플로우를 시제품 QA 수준으로 자동 검증한다.

## Success Criteria

- 백엔드 API 전체 엔드포인트 (health, compile, deploy, vibe-score, analysis, contract source, paymaster) 응답 검증
- 프론트엔드: 페이지 로드 → Monaco Editor 렌더 → Solidity 코드 로드
- 컨트랙트 셀렉터 4종(FailingContract, FixedContract, PectraTest, ParallelConflict) 전환 동작
- 컴파일 성공 → TransactionConsole에 "Compiled:" 기록
- 컴파일 에러 → Monaco 인라인 에러 마커 표시 + 소스 변경 시 마커 클리어
- Vibe-Score 분석 → SVG 게이지 렌더 + suggestions 텍스트 표시 + 엔진 기반(engineBased=true) 확인
- 배포(서버 페이마스터) → 성공 시 0x 주소 표시 + TransactionConsole 기록
- AI 에러 분석 → 에러 발생 시 수정 제안 diff 표시
- 배포된 컨트랙트 ABI read 함수 호출 → 결과값 표시
- 모바일 반응형: 768px 이하에서 탭 전환 레이아웃 동작
- 전체 플로우 스크린샷 증거 (단계별 캡처)
- 모든 테스트가 headless Chromium에서 자동 PASS

## Key Risks

- **Deploy 테스트**: 서버 페이마스터 지갑에 MON 필요 (DB 테이블 미생성 문제 수정 배포 중)
- **AI Analysis**: GEMINI_API_KEY 설정 여부에 따라 AI vs 휴리스틱 폴백
- **모나드 테스트넷 불안정**: RPC 지연/실패 가능성

## Slices

- [ ] **S01: 시제품 QA 수준 Full E2E 테스트 스위트** `risk:medium` `depends:[]`
  > After this: Playwright 기반 20+ E2E 테스트가 라이브 서비스 대상으로 전부 통과, 단계별 스크린샷 증거 포함

## 3개 레포 매핑
- **Core (Rust)**: `ahwlsqja/Vibe-Room-Core` → `/home/ahwlsqja/monad-core` — .gsd 메타데이터만
- **Frontend (Next.js)**: `leejk206/Vibe-Loom` → `/home/ahwlsqja/Vibe-Loom` — E2E 테스트 코드
- **Backend (NestJS)**: `ahwlsqja/Vibe-Room-Backend` → `/home/ahwlsqja/Vibe-Room-Backend`
