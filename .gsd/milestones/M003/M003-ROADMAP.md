# M003: Full-Stack E2E QA — 시제품 수준 통합 검증

**Vision:** 브라우저에서 라이브 서비스까지 관통하는 Playwright E2E 테스트로 전체 사용자 플로우를 시제품 QA 수준으로 자동 검증한다.

## Success Criteria

- Playwright E2E 20+ 테스트 전부 PASS
- 백엔드 API 7개 엔드포인트 전부 응답 검증
- 프론트엔드 IDE 전체 플로우 (로드→편집→컴파일→배포→인터랙션→분석) 동작
- 모바일 반응형 레이아웃 검증
- 단계별 스크린샷 증거 캡처
- Deploy 실제 성공 (0x 주소 확인)

## Slices

- [x] **S01: 시제품 QA 수준 Full E2E 테스트 스위트** `risk:medium` `depends:[]`
  > After this: 20+ Playwright E2E 테스트 전부 PASS, 라이브 서비스 전체 플로우 검증 완료, 스크린샷 증거 캡처
