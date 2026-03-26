# Vibe Room Core — Product Requirements Document v2.0

**Monad-Native Parallel Execution Analyzer**
**Official Execution Engine Integration Roadmap**

Version 2.0 | March 2026
Target: Monad Blitz Seoul 4th & Beyond

---

## 1. Executive Summary

Vibe Room Core는 솔리디티 개발자가 스마트 컨트랙트의 **병렬 실행 동작을 배포 전에 분석**할 수 있는 Monad-native SaaS 개발 도구입니다. 이 PRD는 현재 자체 구축한 Rust Block-STM 엔진에서 **공식 Monad 실행 엔진(category-labs/monad, GPL-3.0) 통합**으로 전환하는 로드맵을 정의합니다.

### 1.1 Why transition

- 공식 실행 엔진(C++, 코드베이스의 91.8%)에는 프로덕션급 병렬 스케줄러, MonadDB, JIT 컴파일러, MIP-3/4/5 지원이 포함되어 있으며 Zellic과 Spearbit의 보안 감사를 받았음
- 현재 Rust 엔진은 정확한 Block-STM 재구현(14,368 LOC, 295 테스트)이지만 MonadDB, JIT 컴파일, 프로덕션 수준 최적화가 부재
- 공식 엔진 포크 시 충돌 분석 결과가 실제 메인넷 동작과 정확히 일치
- GPL-3.0 라이센스는 상용 SaaS 목적의 포크를 허용 (파생 저작물도 GPL-3.0 필요)

### 1.2 Product vision

**One-liner:** Remix tells you *IF* your contract works. Vibe Room tells you *HOW WELL* it works on Monad.

개발자가 Web IDE에서 솔리디티를 작성하고, 시뮬레이션 블록의 트랜잭션을 제출하면, 어떤 트랜잭션이 충돌하는지, 재실행이 몇 번 발생하는지, 어떤 스토리지 슬롯이 경합을 유발하는지 즉시 확인할 수 있습니다. 이를 통해 첫날부터 병렬 친화적 컨트랙트 설계가 가능해집니다.

---

## 2. Current State Analysis

### 2.1 Existing Rust engine (Vibe-Room-Core)

| Attribute | Detail |
|-----------|--------|
| **Language** | Rust |
| **LOC** | 14,368 lines across 8 crates |
| **Tests** | 295 tests, 0 failures, `cargo check` clean |
| **Algorithm** | Block-STM (Aptos paper) with MVHashMap, OCC validation |
| **EVM backend** | revm (Rust EVM) with alloy-primitives |
| **MIP support** | MIP-3 (linear memory), MIP-4 (reserve balance), MIP-5 (CLZ/Fusaka) |
| **Interface** | JSON stdin/stdout CLI (`monad-cli` crate) |
| **Strengths** | Correctness proven (`parallel state_root == sequential`), clean architecture, fast compilation |
| **Gaps** | No MonadDB, no JIT compiler, SeqCst atomic ordering, O(n) `mark_estimate`, ecrecover cache not connected, ESTIMATE extraction via string parsing |

### 2.2 Official Monad execution engine (category-labs/monad)

| Attribute | Detail |
|-----------|--------|
| **Language** | C++ (91.8%), Rust (2.6%), C (2.5%) |
| **Commits** | 3,815 commits, 35 contributors |
| **License** | GPL-3.0 (permits forking, derivative must be GPL-3.0) |
| **Build system** | CMake + ninja, gcc-15 or clang-19, x86-64-v3 minimum |
| **Architecture** | `libmonad_execution.a` static library with EVM, triedb, scheduler |
| **Key dirs** | `category/core` (scheduler, glue), `category/vm` (EVM + JIT), `category/triedb` (MonadDB) |
| **Modes** | Daemon (production), Interactive (block replay), Hosted (shared lib) |
| **Latest** | v0.13.0 (March 4, 2026) — MONAD_NINE hard fork support |
| **Docker** | Full Docker support with single-node setup via monad-bft repo |
| **Audits** | Zellic, Spearbit, Code4rena (September 2025) |

### 2.3 Gap analysis

| Capability | Rust engine (current) | Official engine (target) |
|------------|----------------------|--------------------------|
| Parallel scheduler | Custom Block-STM (correct, not optimized) | Production Block-STM with relaxed merge |
| State database | InMemoryState (HashMap) | MonadDB (native MPT, async SSD I/O) |
| EVM | revm interpreter | Custom EVM + JIT compiler |
| Conflict detection | Field-level (Balance, Nonce, Storage) | Same + reserve balance, relaxed merge |
| Performance | ~100 TPS (dev machine, no optimization) | 10,000 TPS (production hardware) |
| Mainnet fidelity | Approximate (same algorithm, different impl) | Exact (same binary as validators) |

---

## 3. Milestone Plan

총 타임라인: **18주 (4.5개월)**. 각 마일스톤은 명확한 진입 기준, 산출물, 종료 기준을 가집니다. 모든 마일스톤은 순차적이며 M(n)은 M(n-1) 완료에 의존합니다.

| ID | Milestone | Timeline | Key deliverables | Exit criteria |
|----|-----------|----------|-------------------|---------------|
| **M0** | Foundation | W1-W2 (2주) | category-labs/monad fork, build env, Docker / C++ codebase audit, key module mapping | Build succeeds on CI, `ctest` passes on fork |
| **M1** | Headless execution mode | W3-W5 (3주) | Strip consensus deps from execution daemon / Build `libmonad_execution.a` standalone / JSON stdin/stdout CLI wrapper | CLI accepts JSON block input, outputs execution results, no consensus daemon needed |
| **M2** | Instrumentation layer | W6-W8 (3주) | Hook into scheduler: per-tx incarnation counter / Hook into MV state: conflict location tracking / Hook into VM: read/write set capture / Emit structured telemetry JSON | Telemetry shows incarnation per tx, conflict LocationKeys reported, re-execution count matches expectations |
| **M3** | Analysis API server | W9-W11 (3주) | REST API wrapping CLI / Solidity compile endpoint / Conflict heatmap data generation / Parallel vs sequential comparison endpoint | `POST /analyze` returns conflict report, Swagger docs live, latency < 5s for 100-tx block |
| **M4** | Web IDE frontend | W12-W15 (4주) | Monaco editor with Solidity syntax / Conflict heatmap visualization / Incarnation timeline view / Deploy-to-Monad integration | User writes Solidity, sees conflicts, heatmap renders correctly, E2E flow works on staging |
| **M5** | Production & launch | W16-W18 (3주) | Cloud infra (Docker + K8s) / Auth, rate limiting, billing skeleton / Docs, landing page, onboarding flow / Monad ecosystem outreach | SaaS live on custom domain, handles 10 concurrent users, monitoring + alerting active |

---

## 4. Milestone Details

### 4.1 M0: Foundation (Week 1-2)

#### 4.1.1 Objective

`category-labs/monad`를 포크하고, 빌드 파이프라인을 구축하며, 소스 코드를 매핑하여 계측에 필요한 정확한 파일/API를 식별합니다.

#### 4.1.2 Tasks

- **T0.1 Fork and build:** `category-labs/monad` (v0.13.0 tag) 포크. gcc-15, CMake 3.27+, x86-64-v3 타겟으로 Docker 기반 빌드 환경 구축. 전체 빌드 성공 및 `ctest` 통과 확인
- **T0.2 Source code audit:** `category/` 디렉토리 구조 매핑. 식별 대상: (a) `category/core/scheduler.*` — 병렬 실행 스케줄러 진입점, (b) `category/core/block_executor.*` — 블록 처리 파이프라인, (c) `category/vm/` — EVM 실행 및 JIT, (d) `category/triedb/` — MonadDB 상태 접근 레이어
- **T0.3 Identify instrumentation points:** incarnation 카운터가 관리되는 곳, read/write set이 추적되는 곳, 충돌 감지(OCC validation)가 발생하는 곳, ESTIMATE marker가 설정/확인되는 곳의 정확한 함수/클래스 위치 파악. 함수 시그니처와 호출 체인 문서화
- **T0.4 CI pipeline:** 모든 push에서 포크를 빌드하고 `ctest`를 실행하는 GitHub Actions 워크플로우. Docker 이미지를 컨테이너 레지스트리에 게시

#### 4.1.3 Risks

- 빌드에 특정 하드웨어(x86-64-v3) 필요. **완화:** Haswell+ 인스턴스(AWS c5/c6i)로 클라우드 CI 사용
- C++ 코드베이스가 대규모(~3,800 commits). **완화:** `category/core/`에만 집중 오디트, triedb 내부는 제외

#### 4.1.4 Deliverables

```
vibe-room-monad-fork/
├── .github/workflows/ci.yml    # Build + ctest on every push
├── docker/Dockerfile.dev        # Dev build environment
├── docs/
│   ├── source-map.md            # category/ directory structure analysis
│   ├── instrumentation-points.md # Function signatures for hooks
│   └── call-chain.md            # Block execution call chain trace
└── scripts/
    └── build-headless.sh        # Build without consensus deps
```

---

### 4.2 M1: Headless Execution Mode (Week 3-5)

#### 4.2.1 Objective

실행 엔진을 데몬에서 추출하여 합의 없이 독립 블록 실행을 가능하게 합니다. 실행과 풀 노드를 분리하는 핵심 단계입니다.

#### 4.2.2 Architecture options

공식 코드베이스는 이미 `cmd/monad`가 커맨드라인에서 블록을 리플레이할 수 있는 "Interactive mode"를 지원합니다. 전략은 이 기존 모드를 활용하고 확장하는 것입니다:

| Option | Approach | Pros | Cons | Effort |
|--------|----------|------|------|--------|
| **A (recommended)** | C++ CLI wrapper | 최소 코드 변경, 최대 충실도, interactive mode 확장 | C++ JSON 파싱 필요 (nlohmann/json 사용 가능) | 2주 |
| **B** | Rust FFI bridge | Rust-native API 서버 가능 (M3에서 유리), 타입 안전 | `extern "C"` 바인딩 복잡, 빌드 체인 이중화 | 3주 |
| **C** | Docker sidecar | 가장 간단한 통합, 공식 바이너리 그대로 사용 | 가장 높은 latency, 운영 복잡도, IPC 오버헤드 | 1주 |

**권장:** M1에서 Option A로 시작 (동작하는 CLI까지 최단 경로), M3에서 Rust FFI가 API 서버에 더 나은 인체공학을 제공하는지 평가.

#### 4.2.3 Input/Output specification

CLI 인터페이스는 기존 Rust 엔진의 monad-cli JSON 포맷과 **하위 호환**되어야 합니다:

**Input:**
```json
{
  "transactions": [
    {
      "sender": "0x...",
      "to": "0x...",
      "data": "0x...",
      "value": "0",
      "gas_limit": 2000000,
      "nonce": 0,
      "gas_price": "1000000000"
    }
  ],
  "block_env": {
    "number": 1,
    "coinbase": "0x...",
    "timestamp": 1700000000,
    "gas_limit": 30000000,
    "base_fee": "0",
    "difficulty": "0"
  }
}
```

**Output:**
```json
{
  "results": [
    {
      "success": true,
      "gas_used": 21000,
      "output": "0x",
      "error": null,
      "logs_count": 0
    }
  ],
  "incarnations": [0, 1, 0],
  "stats": {
    "total_gas": 63000,
    "num_transactions": 3,
    "num_conflicts": 1,
    "num_re_executions": 1
  }
}
```

이를 통해 기존 Rust 엔진의 테스트 케이스가 수정 없이 새 CLI를 검증할 수 있습니다.

#### 4.2.4 Tasks

- **T1.1:** `cmd/monad` interactive mode 코드 경로 학습. `main()`에서 블록 실행까지 추적
- **T1.2:** JSON 입력 파서 구현 (`nlohmann/json` 또는 `simdjson`, 둘 다 `third_party/`에서 사용 가능)
- **T1.3:** JSON 입력을 블록 실행 API에 연결. 입력에서 sender 계정 자금 충당 (기존 `InMemoryState` 패턴과 동일)
- **T1.4:** 실행 결과 캡처 및 JSON 출력으로 직렬화
- **T1.5:** 교차 검증: 동일한 테스트 블록을 Rust CLI와 C++ CLI 양쪽으로 실행, 출력 비교

#### 4.2.5 Cross-validation strategy

기존 Rust 엔진의 295개 테스트에서 추출한 "golden block" 세트를 만듭니다:

```
test-vectors/
├── independent_8tx.json       # 8 independent transfers, 0 conflicts
├── serial_3tx.json            # Same sender nonce 0-2, max conflict
├── mixed_8tx.json             # 2 conflicting pairs + 4 independent
├── stress_100_independent.json
├── stress_50_serial.json
└── stress_100_mixed.json
```

각 벡터에 대해 `expected_state_root`, `expected_gas_used`, `expected_incarnations`를 Rust 엔진으로 생성하고, C++ CLI 출력과 비교합니다.

---

### 4.3 M2: Instrumentation Layer (Week 6-8)

#### 4.3.1 Objective

이것이 **핵심 가치 추가**입니다. 공식 실행 엔진에 hook을 걸어 기존 어떤 도구도 제공하지 않는 병렬 실행 텔레메트리를 추출합니다.

#### 4.3.2 Telemetry data model

각 블록 실행에 대해 계측 레이어가 캡처해야 하는 데이터:

| Data point | Type | Source in official engine |
|------------|------|--------------------------|
| Incarnation per tx | `Vec<u32>` | Scheduler incarnation counter (`category/core/scheduler`) |
| Conflict locations | `Vec<LocationKey>` | OCC validation failure — 어떤 스토리지 슬롯이 충돌을 유발했는지 |
| Read set per tx | `HashMap<LocationKey, Version>` | MV state read tracking during EVM execution |
| Write set per tx | `HashMap<LocationKey, Value>` | State diffs after execution (pre-merge) |
| Execution time per tx | `Duration (ns)` | Wall-clock time per `execute_transaction` call |
| Blocking tx dependency | `Option<TxIndex>` | ESTIMATE hit source — 어떤 선행 tx가 이 tx를 블로킹했는지 |
| Gas fee per tx | `U256` | `gas_used * gas_price` (beneficiary tracker) |
| Conflict graph | `Vec<(TxIndex, TxIndex)>` | Derived: write location을 공유하는 tx 쌍 |

#### 4.3.3 Implementation strategy

코드베이스 복잡도에 따른 두 가지 계측 접근법:

**Approach A (compile-time hooks):** 텔레메트리 수집 코드 주변에 `#ifdef VIBE_ROOM_INSTRUMENTATION` 가드 추가. 비활성화 시 오버헤드 제로. 공식 소스 파일 수정 필요하지만 변경은 최소한이고 잘 격리됨.

```cpp
// category/core/scheduler.cpp
void Scheduler::finish_validation(TxIndex tx_index, bool valid) {
    if (!valid) {
        // existing abort logic...
        state.incarnation++;

#ifdef VIBE_ROOM_INSTRUMENTATION
        telemetry_.record_conflict(tx_index, state.incarnation, conflicting_location);
#endif
    }
}
```

**Approach B (callback injection):** `TelemetryCallback` 인터페이스 클래스 정의. 스케줄러/executor 생성 시 전달. 공식 코드가 핵심 지점에서 콜백 호출. 더 깔끔한 분리지만 생성자 리팩토링이 더 많이 필요.

```cpp
class TelemetryCallback {
public:
    virtual void on_execution_complete(TxIndex, Incarnation, ReadSet, WriteSet) = 0;
    virtual void on_validation_failure(TxIndex, LocationKey conflicting_key) = 0;
    virtual void on_estimate_hit(TxIndex blocked, TxIndex blocking) = 0;
};
```

#### 4.3.4 Key instrumentation points

- **`Scheduler::finish_execution()`:** incarnation, read_set, write_set, execution result, gas fee 캡처
- **`Scheduler::finish_validation(false)`:** 어떤 LocationKey가 충돌을 유발했는지 기록. `(tx_index, blocking_tx)` 쌍 로깅
- **`MV state mark_estimate()`:** 어떤 tx가 ESTIMATE를 유발했고 어떤 location이 마킹되었는지 추적
- **Block execution completion:** per-tx 데이터를 블록 레벨 요약으로 집계: 총 충돌 수, 총 재실행 수, conflict graph, hot slot 랭킹

#### 4.3.5 Output: conflict report

```json
{
  "block_summary": {
    "total_transactions": 100,
    "total_conflicts": 12,
    "total_re_executions": 18,
    "parallelism_ratio": 0.82
  },
  "per_tx": [
    {
      "tx_index": 0,
      "incarnation": 0,
      "gas_used": 21000,
      "execution_time_ns": 450000,
      "read_set_size": 3,
      "write_set_size": 2,
      "conflicts": []
    },
    {
      "tx_index": 1,
      "incarnation": 2,
      "gas_used": 21000,
      "execution_time_ns": 380000,
      "read_set_size": 3,
      "write_set_size": 2,
      "conflicts": [
        {
          "with_tx": 0,
          "location": "Storage(0xContractAddr, slot_0x07)",
          "type": "write-write"
        }
      ],
      "blocked_by": 0
    }
  ],
  "hot_slots": [
    {
      "location": "Storage(0xDEX, slot_reserve0)",
      "conflict_count": 8,
      "involved_txs": [1, 3, 5, 7, 9, 11, 13, 15]
    }
  ],
  "conflict_graph": {
    "edges": [[1, 0], [3, 2], [5, 4]],
    "clusters": [[0, 1], [2, 3], [4, 5]]
  }
}
```

---

### 4.4 M3: Analysis API Server (Week 9-11)

#### 4.4.1 API endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/analyze` | Solidity 소스 + tx 리스트 제출. 전체 충돌 분석 보고서(incarnations, conflict graph, hot slots) 반환 |
| `POST` | `/api/v1/compile` | Solidity 소스 컴파일. ABI + bytecode 반환. `solc` 바이너리 사용 |
| `POST` | `/api/v1/simulate` | 주어진 트랜잭션으로 단일 블록 실행. 실행 결과 + 상태 변경 반환 |
| `GET` | `/api/v1/compare/:id` | 이전에 제출한 블록의 병렬 vs 순차 실행 비교 |
| `GET` | `/api/v1/report/:id` | ID로 이전에 생성한 분석 보고서 조회 |

#### 4.4.2 Tech stack

- **API server:** Rust (Axum) 또는 Go (Gin) — M1의 FFI 접근법에 따라 결정
- **Execution backend:** M1의 공식 Monad CLI (subprocess 또는 FFI로 호출)
- **Solidity compiler:** `solc` 바이너리 (solc-select으로 버전 관리)
- **Queue:** Redis — 비동기 분석 작업 (100+ tx 블록)
- **Storage:** PostgreSQL — 분석 보고서, S3 — 대용량 텔레메트리 데이터

#### 4.4.3 Request flow

```
Client
  │
  ▼
POST /api/v1/analyze
  { solidity_source, transactions, block_env }
  │
  ├─ 1. solc compile → ABI + bytecode
  │
  ├─ 2. Build transaction list with bytecode
  │
  ├─ 3. Call instrumented Monad CLI
  │     (stdin: JSON block, stdout: telemetry)
  │
  ├─ 4. Parse telemetry → conflict report
  │
  ├─ 5. Store report in PostgreSQL
  │
  └─ 6. Return report JSON to client
```

---

### 4.5 M4: Web IDE Frontend (Week 12-15)

#### 4.5.1 Core features

- **Solidity editor:** Monaco Editor with Solidity syntax highlighting, autocomplete, `solc` 진단 에러 underline
- **Transaction builder:** 테스트 트랜잭션 구성 GUI 폼 (sender, to, value, calldata, gas). 공통 패턴 템플릿 (ERC-20 transfer, Uniswap swap, NFT mint)
- **Conflict heatmap:** 행 = 트랜잭션, 열 = 스토리지 슬롯인 매트릭스 시각화. 색상 강도 = 충돌 빈도. 클릭하면 충돌 세부 정보 표시
- **Incarnation timeline:** 트랜잭션별 실행 시도 횟수 바 차트. 초록 = 첫 시도 성공, 노랑 = 1회 재실행, 빨강 = 2회 이상 재실행
- **Dependency graph:** 공유 스토리지 접근 기반 tx-to-tx 의존성을 보여주는 D3 force-directed 그래프. 클러스터 = 병렬화 그룹
- **Parallel vs sequential comparison:** state_root 일치 확인, 총 가스 비교, wall-clock 시간 차이를 보여주는 사이드바이사이드 뷰

#### 4.5.2 Tech stack

- **Framework:** Next.js 14+ (App Router)
- **Editor:** Monaco Editor (`@monaco-editor/react`)
- **Charts:** Recharts (bar/line), D3.js (heatmap, force graph)
- **State:** Zustand or Jotai
- **Styling:** Tailwind CSS

#### 4.5.3 Key screens

```
┌─────────────────────────────────────────────────────┐
│  Vibe Room                               [Deploy ▾] │
├──────────────────┬──────────────────────────────────┤
│                  │  CONFLICT HEATMAP                │
│  // Solidity     │  ┌──┬──┬──┬──┬──┬──┐            │
│  contract DEX {  │  │  │██│  │██│  │  │ tx0        │
│    uint reserve; │  │██│  │██│  │██│  │ tx1        │
│    ...           │  │  │██│  │  │  │██│ tx2        │
│                  │  └──┴──┴──┴──┴──┴──┘            │
│                  │   s0  s1  s2  s3  s4  s5        │
│  [Analyze ▶]     │                                  │
│                  ├──────────────────────────────────┤
│  TX BUILDER      │  INCARNATION TIMELINE            │
│  ┌────────────┐  │  tx0 ████                    [0] │
│  │ sender: 0x │  │  tx1 ████████████            [2] │
│  │ to:     0x │  │  tx2 ████                    [0] │
│  │ value:  0  │  │  tx3 ████████████████████    [4] │
│  └────────────┘  │                                  │
│  [+ Add tx]      │  4 conflicts, 6 re-executions   │
└──────────────────┴──────────────────────────────────┘
```

---

### 4.6 M5: Production and Launch (Week 16-18)

- Containerized deployment (Docker + Kubernetes on AWS/GCP)
- Authentication (GitHub OAuth for developer identity)
- Rate limiting (10 analyses/minute for free tier)
- Monitoring: Prometheus + Grafana for API latency, error rates, queue depth
- Landing page with product demo video
- Documentation site (Docusaurus or Nextra)
- Monad ecosystem outreach: Monad Foundation developer relations, Discord presence

---

## 5. Technical Risks and Mitigations

| ID | Risk | Impact | Mitigation |
|----|------|--------|------------|
| **R1** | C++ codebase complexity — `category/core` scheduler may be deeply coupled to consensus | HIGH | M0 audit identifies coupling points early. Interactive mode already proves execution can run without consensus. Fallback: Docker sidecar (Option C) if decoupling fails. |
| **R2** | Build environment portability — requires x86-64-v3 CPU and gcc-15 | MEDIUM | Docker-based build from day one. Cloud CI on c5.2xlarge (Haswell+). Production runs on x86 cloud instances only. |
| **R3** | GPL-3.0 license contamination — SaaS wrapping GPL code | MEDIUM | GPL-3.0 applies to the execution engine fork, not the SaaS layer. API server and frontend are separate works communicating via CLI/IPC. Legal review before M3 completion. |
| **R4** | Upstream breaking changes — category-labs/monad may refactor scheduler APIs | LOW | Pin to v0.13.0 tag. Instrumentation via `#ifdef` guards minimizes diff surface. Rebase quarterly. |
| **R5** | Execution latency — full MonadDB + JIT may be slow for interactive SaaS use | MEDIUM | Use in-memory state (skip MonadDB persistence) for analysis mode. JIT compilation can be disabled for small contracts. Target < 5s for 100-tx block. |
| **R6** | Monad Foundation relationship — using their engine commercially | LOW | Tool benefits Monad ecosystem (better contracts = better chain performance). Engage Monad DevRel early. Blitz Seoul is the introduction point. |

---

## 6. Existing Asset Reuse Plan

현재 Rust 엔진(Vibe-Room-Core)은 상당한 엔지니어링 투자를 나타내며 폐기되어서는 안 됩니다. 다음 구성 요소가 계속 가치를 가집니다:

| Asset | Reuse plan |
|-------|------------|
| **monad-cli JSON interface** | 입출력 포맷이 API 서버와 실행 백엔드 간의 계약이 됨. 같은 포맷, 새로운 백엔드. |
| **295 test cases** | 교차 검증 스위트: 동일한 테스트 블록을 Rust 엔진과 C++ 엔진 양쪽으로 실행, 동일한 결과 확인. 회귀 감지. |
| **Stress test scenarios** | 7개 스트레스 테스트(independent, serial, mixed, revert, interleaved, determinism)가 새 엔진의 벤치마크 스위트가 됨. |
| **Block-STM knowledge** | 스케줄러 내부에 대한 깊은 이해가 C++ 코드베이스의 정확한 계측 포인트 식별을 가능하게 함. |
| **MIP-3/4/5 tests** | Nine-fork 통합 테스트가 계측된 엔진이 여전히 Monad 전용 EVM 확장을 올바르게 처리하는지 검증. |
| **Benchmark harness** | Criterion 벤치마크(`parallel_vs_sequential.rs`)가 새 CLI의 성능 기준선을 설정. |

---

## 7. Success Metrics

### 7.1 Technical metrics

- 충돌 분석 결과가 동일한 입력 블록에 대해 실제 Monad 메인넷 실행과 일치
- 분석 latency < 5초 (최대 100 트랜잭션 블록)
- API 가용률 > 99.5% (M5 런칭 이후)
- 교차 검증: 기존 Rust 엔진 테스트 케이스의 100%가 새 C++ 백엔드에서 통과

### 7.2 Product metrics (런칭 후 3개월)

- 등록 개발자 100명 이상
- 주간 분석 실행 500회 이상
- 최소 1개의 Monad-native DeFi 프로토콜이 CI/CD 파이프라인에서 Vibe Room 사용
- Monad 개발자 문서 또는 에코시스템 페이지에 소개

### 7.3 Blitz Seoul immediate goal

Monad Blitz Seoul 4th 해커톤에서의 목표는 기존 Rust 엔진으로 충돌 분석을 실행하는 웹 프론트엔드가 포함된 **동작하는 프로토타입**을 시연하고, 공식 엔진 통합 **로드맵을 발표**하는 것입니다. 프로토타입과 로드맵이 함께 설득력 있는 스토리를 전달합니다: *"문제를 깊이 이해하기 위해 처음부터 만들었고, 이제 실제 엔진으로 프로덕션에 나갑니다."*

---

## 8. Open Questions

| # | Question | Decision needed by |
|---|----------|-------------------|
| **Q1** | FFI approach: C++ CLI (Option A) vs Rust FFI (Option B) vs Docker sidecar (Option C)? | End of M0 (after source audit) |
| **Q2** | GPL-3.0 implications for commercial SaaS — frontend/API에 별도 라이센스 필요? | Before M3 (legal review) |
| **Q3** | MonadDB in analysis mode: real MonadDB 사용 vs 속도를 위한 in-memory state? | M1 (based on latency testing) |
| **Q4** | Pricing model: rate limit 기반 freemium vs 유료? | Before M5 (launch prep) |
| **Q5** | Monad Foundation 공식 참여(grant 신청) vs 독립 유지? | After Blitz Seoul feedback |

---

*End of Document*