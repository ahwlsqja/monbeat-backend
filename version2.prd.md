# Vibe Room — Product Requirements Document v3.0

**"Monad's Parallel Execution, Made Audible and Visible"**

Rhythm Game-Style Parallel Execution Visualizer & Monitoring Platform

Version 3.0 | March 2026
Target: Monad Blitz Seoul 4th → Nitro Accelerator → Grant/Business

---

## 1. Executive Summary

### 1.1 What changed from v2

PRD v2 defined a "Parallel Execution Profiler" — a developer tool that analyzes conflict patterns in Monad's Block-STM engine. Through rigorous market validation, we identified critical weaknesses in that direction:

- Monad's official position is "developers don't need to change anything for parallel execution." This undermines demand for a profiler.
- Conflict analysis is a one-time activity, not a daily-use tool. Ponder (acqui-hired by Monad Foundation) succeeded because hundreds of apps used it daily.
- Every tool built on top of the GPL-3.0 open-source engine has zero technical moat. The only defensible moats are ecosystem position (official endorsement) and adoption (switching cost).

### 1.2 The pivot

Vibe Room pivots from "developer profiler" to **"experiential parallel execution visualizer"** — a rhythm-game-style real-time dashboard that makes Monad's parallel execution audible and visible.

**One-liner:** "Ponder turns smart contract events into APIs. Vibe Room turns parallel execution into music."

### 1.3 Why this works

Monad's core innovation (parallel execution) is invisible. No existing tool shows it happening. Tenderly and Phalcon market "parallel execution debugging" but actually provide standard per-transaction opcode traces — the same thing they do on Ethereum. Block explorers show final results, not the parallel execution process.

Vibe Room makes the invisible visible — and audible. Successful transactions produce pentatonic harmony. Conflicts produce dissonance. Re-executions create rhythmic tension and resolution. The network's health becomes something you can feel, not just read.

### 1.4 Three-layer product strategy

| Layer | Product | Audience | Revenue | Timeline |
|-------|---------|----------|---------|----------|
| **Show** | Live rhythm visualizer | Everyone (viral, educational) | Free / Grant-funded | Blitz Seoul (April 2026) |
| **Understand** | Protocol-specific dashboard | Protocol teams, validators | Freemium | Post-grant (Q3 2026) |
| **Optimize** | Parallel execution data API | Indexers, analytics platforms, MEV researchers | Subscription | Q4 2026+ |

Layer 1 (Show) is the grant play. Layer 3 (Optimize) is the business play. They share the same data pipeline.

---

## 2. Market Context

### 2.1 What exists on Monad

| Category | Tools | What they show |
|----------|-------|----------------|
| Block explorers | MonadScan, MonadVision, NadScan, Hoodscan | Standard tx/block data |
| Tx debuggers | Tenderly, Phalcon (BlockSec) | Per-tx opcode trace, call graph, balance changes |
| Network dashboards | gmonads.com | Validator performance, staking |
| Analytics | Dune Analytics | On-chain analytics (SQL queries) |
| Gamified visualizers | Dragon Ball Z visualizer, Monanimals | Standard data (TPS, gas) with game aesthetics |

### 2.2 What nobody shows

No existing tool visualizes the parallel execution process itself:

- Which transactions were executed simultaneously across which cores
- Which transactions conflicted with each other (and on which storage slot)
- How many times each transaction was re-executed (incarnation count)
- What the effective parallelism ratio of each block was
- How re-execution cascades propagated through the block

This data exists only inside the execution engine. It is not exposed via Monad's RPC (standard Ethereum JSON-RPC). The official `libmonad_event` SDK streams execution events (`BlockStart`, `TxnEnd`, `TxnCallFrame`, `AccountAccess`) but does not include incarnation, conflict, or re-execution metadata.

### 2.3 Competitive positioning

**Tenderly's "parallel execution debugging" claim:** Tenderly's Monad support page says "Navigate parallel execution with advanced debugging tools" and "Step through transactions line-by-line." In practice, this is standard per-transaction EVM tracing — the same product they ship for every EVM chain. They cannot show inter-transaction conflict data because it doesn't exist in the RPC.

**Existing Monad visualizers:** gmonads.com is linked from official Monad docs. Dragon Ball Z and Monanimals visualizers were built during Monad developer missions. These prove the ecosystem values gamified visualization — but they all visualize standard blockchain metrics (TPS, gas, blocks). None visualize parallel execution internals.

**Our differentiation:** We show the one thing that makes Monad different from every other EVM chain — and we make it an experience, not a spreadsheet.

---

## 3. Product Vision

### 3.1 Core concept: Blockchain as Music

Every Monad block is a musical composition. Transactions are notes. Parallel execution lanes are instruments. Conflicts are dissonance. Resolution (successful commit) is harmony.

The fundamental insight: **developers and users respond to sensory feedback faster than to numbers.** A parallelism ratio of 78% means nothing to most people. But when the music suddenly becomes harsh and discordant, everyone in the room knows something is wrong.

### 3.2 Game mechanics

#### Sound design

| Event | Sound | Scale/Note system |
|-------|-------|-------------------|
| Successful tx commit | Pentatonic note (C, D, E, G, A) | Each core gets a base note; tx position adds variation |
| Conflict detected | Two dissonant notes (semitone interval: C+Db, E+F) + noise burst | Tritone and minor-second intervals create visceral discomfort |
| Re-execution | Quick ascending arpeggio (tension) | Same note as original, played faster and higher |
| Re-execution success | Resolution chord | Returns to pentatonic harmony |
| Block complete | Chord progression resolving to tonic | Satisfying musical conclusion |

**Why pentatonic:** Any combination of pentatonic notes sounds harmonious. This means a healthy network (few conflicts) always produces pleasant music, regardless of transaction composition. No special tuning needed — the math works automatically.

**Why dissonance for conflicts:** Semitone intervals (C-Db, E-F) create psychoacoustic roughness that humans instinctively perceive as "wrong." This is not a design choice — it's physics. The frequency beating between close notes triggers an aversive response. A developer hearing this while testing their contract will instinctively want to fix it.

#### Visual design

| Element | Visual representation |
|---------|----------------------|
| 4 parallel execution cores | 4 vertical lanes (like rhythm game note highways) |
| Transaction | Colored block falling down a lane |
| Commit zone | Horizontal line near bottom (like rhythm game hit zone) |
| Successful commit | Green flash + particle burst at commit zone |
| Conflict | Red glow + shake + particle explosion |
| Re-execution | Block bounces upward, then falls again |
| Hot storage slot | Slot label glows amber at bottom of lane |
| Network health bar | Depletes on conflicts, recovers on successes |
| Score | Accumulates on successful commits, penalized on conflicts |
| Block number | Increments periodically, triggers block-complete chord |

#### Behavioral incentive design

The health bar and score create a gamification loop:

1. Developer deploys contract → runs simulation → sees/hears the result
2. High conflict rate → health bar drops, music becomes harsh, score decreases
3. Developer modifies contract (e.g., splits storage slots) → re-runs
4. Lower conflict rate → health bar recovers, music is pleasant, score increases
5. Developer has now optimized for parallel execution — without reading a single doc about Block-STM

This is "nudge architecture" applied to smart contract development. The game mechanics guide behavior toward network-healthy outcomes.

---

## 4. Technical Architecture

### 4.1 Data pipeline

```
Data Source (choose one per mode)
│
├─ Mode A: Live mainnet
│  └─ libmonad_event SDK → Event Ring → real-time tx events
│     (BlockStart, TxnEnd, TxnCallFrame, AccountAccess)
│     Note: Does NOT include incarnation/conflict data
│     → Derive parallelism from timing (block time vs sum of tx times)
│     → Derive hot slots from AccountAccess frequency
│
├─ Mode B: Simulation (our Rust engine)
│  └─ User submits Solidity + tx list → Rust Block-STM engine
│     → Full conflict/incarnation data from instrumented execution
│     → Exact conflict graph, read/write sets, ESTIMATE events
│
└─ Mode C: Hybrid (future, post-upstream-merge)
   └─ Modified libmonad_event with incarnation/conflict events
      → Best of both: live mainnet data + full parallel execution metadata

         ↓

Processing Layer (Rust backend)
│
├─ Event ingestion & normalization
├─ Conflict graph construction
├─ Parallelism ratio calculation
├─ Hot slot ranking
├─ Sound event mapping (tx → note assignment)
│
         ↓

Presentation Layer
│
├─ WebSocket server → real-time event stream
├─ Web frontend (Canvas + Tone.js)
│  ├─ Rhythm game visualization
│  ├─ Sound synthesis
│  └─ Score/health tracking
├─ REST API (historical data)
└─ Embeddable widget (for third-party dashboards)
```

### 4.2 Two operating modes

**Mode A: Live network monitor (Grant/free tier)**

Connects to Monad mainnet via `libmonad_event` SDK. Uses available events (timing, account access) to derive approximate parallel execution metrics. Cannot show exact incarnation counts or conflict locations (this data isn't in the SDK), but can show:

- Approximate parallelism ratio (from block time vs sum of tx times)
- Hot accounts/slots (from AccountAccess frequency)
- Transaction flow across blocks (timing and ordering)
- Sound mapping based on available metrics

This mode is sufficient for the "experiential visualizer" — the music and visuals work with approximate data. The exact conflict data isn't needed for the aesthetic experience.

**Mode B: Simulation profiler (developer tool)**

Uses the existing Rust Block-STM engine (14,368 LOC, 295 tests) to execute user-submitted transaction batches. Produces exact parallel execution data:

- Incarnation count per transaction
- Conflict graph with storage slot identification
- Read/write sets
- ESTIMATE marker events
- Re-execution cascade paths

This mode powers the "hear your contract" feature — developers submit their Solidity code and hear whether it produces harmony or dissonance.

### 4.3 Technology stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Execution engine | Rust (existing Vibe-Room-Core, 14K LOC) | Already built, tested, proven |
| Backend API | Rust (Axum) | Same language as engine, low latency |
| WebSocket server | Rust (tokio-tungstenite) | Real-time event streaming |
| Frontend | Next.js 14+ / React | Modern web framework |
| Canvas rendering | HTML5 Canvas (2D context) | Performant real-time animation |
| Audio synthesis | Tone.js | Web Audio API wrapper, PolySynth + NoiseSynth |
| Charts | Recharts or D3.js | Historical data visualization |
| Solidity compiler | solc (via solc-select) | Multi-version Solidity compilation |

### 4.4 Existing asset reuse

| v2 Asset | v3 Reuse |
|----------|----------|
| Rust Block-STM engine (14K LOC) | Mode B simulation backend — unchanged |
| 295 test cases | Validation suite for simulation accuracy |
| monad-cli JSON interface | API contract between frontend and engine |
| Stress test scenarios | Demo scenarios for Blitz Seoul |
| MIP-3/4/5 support | MONAD_NINE fork compatibility |

---

## 5. Grant Strategy

### 5.1 Why Monad Foundation funds this

**Marketing asset:** "Monad's parallel execution, experienced live" is a more powerful message than any documentation page. A 10-second GIF of the visualizer on Twitter communicates what Monad does better than a 10-page technical paper.

**Precedent:** Monad Foundation already links gmonads.com from official docs. The Dragon Ball Z and Monanimals visualizers were funded through developer missions. The Foundation values visual/experiential tools.

**Education tool:** DevRel team can use the visualizer in workshops. "Watch what happens when these 100 transactions hit the parallel executor" is a powerful live demo.

**Ecosystem health metric:** "Monad's average parallelism ratio this week was 87%" becomes a quotable network health statistic — like L2Beat's data availability metrics.

**Unique content:** Every other Monad visualizer could exist on any EVM chain. This visualizer can only exist on Monad because it shows parallel execution — the thing that makes Monad different.

### 5.2 Grant positioning

**One-liner for grant application:**

"We make Monad's parallel execution visible and audible. gmonads.com shows validator performance. MonadScan shows transaction results. Vibe Room shows the thing that makes Monad different — parallel execution, happening live, in real-time, as music."

**Why us:**

"We built Monad's Block-STM execution engine from scratch in Rust — 14,000 lines of code, 295 tests, zero failures. We understand parallel execution at the deepest level because we implemented it ourselves. This is not a wrapper around an RPC endpoint. This is instrumented execution."

### 5.3 Pipeline: Blitz Seoul → Nitro

| Stage | Timeline | Objective | Deliverable |
|-------|----------|-----------|-------------|
| **Blitz Seoul** | April 2026 | Demo + network | Working rhythm visualizer with sound, connected to Rust engine |
| **Ship logs** | April-May 2026 | Nitro qualification | 4 weekly public ship logs (required for Nitro interview) |
| **Nitro application** | May 2026 | Apply | Application + interview |
| **Nitro cohort** | Q3 2026 | $500K + VC access | 1 month NYC + 2 months remote, demo day |

### 5.4 Critical question for Blitz Seoul

At the hackathon, ask Monad Foundation team directly:

"We want to extend `libmonad_event` with incarnation and conflict data. Would you accept an upstream PR for this? Or is there a reason this data is intentionally not exposed?"

This answer determines the entire product direction:

- "Yes, submit a PR" → Ponder path (upstream contributor → potential acqui-hire)
- "No, it's intentional" → Must work with derived/approximate data only
- "Not a priority yet" → Best case — we build it first, propose as standard

---

## 6. Milestone Plan

Total timeline: **16 weeks (4 months)** from Blitz Seoul to public launch.

### M0: Blitz Seoul Demo (Week 0 — Hackathon day)

**Objective:** Working prototype that demonstrates the concept. Connected to existing Rust engine in simulation mode.

**Deliverables:**
- Canvas-based rhythm game visualization (4 lanes, falling transactions, commit zone)
- Tone.js audio engine (pentatonic success, dissonant conflicts, re-execution arpeggios)
- Health bar, score counter, parallelism ratio display
- Conflict rate slider for live adjustment during demo
- 3 pre-built demo scenarios (independent txs, serial conflicts, mixed workload)
- 5-minute live demo script

**Exit criteria:** Audience reaction. Foundation team engagement. Nitro pipeline conversation started.

### M1: Ship Log Foundation (Week 1-4)

**Objective:** 4 consecutive public ship logs to qualify for Nitro interview.

| Week | Ship log content | Technical milestone |
|------|-----------------|---------------------|
| W1 | "Connected to Monad mainnet via libmonad_event" | Event Ring integration, real-time tx streaming |
| W2 | "Live parallelism ratio from mainnet blocks" | Timing-based parallelism derivation, live mode operational |
| W3 | "Sound design v2: per-contract musical themes" | Contract address → instrument mapping, richer audio |
| W4 | "Public beta: viberoom.xyz live" | Deployed frontend, WebSocket streaming, public URL |

### M2: Live Network Monitor (Week 5-8)

**Objective:** Production-quality live visualizer connected to Monad mainnet.

**Deliverables:**
- Stable WebSocket connection to Monad node via libmonad_event SDK
- Real-time rhythm visualization of mainnet activity
- Historical parallelism ratio charts (per-block, hourly, daily)
- Hot account/slot ranking (derived from AccountAccess events)
- Embeddable widget (iframe) for third-party dashboards
- Open-source repository (MIT license)

**Exit criteria:** Stable 24/7 operation, < 2s latency from block to visualization.

### M3: Simulation Mode + Developer Features (Week 9-12)

**Objective:** "Hear your contract" — developers submit Solidity and experience the parallel execution result.

**Deliverables:**
- Web-based Solidity editor (Monaco) connected to Rust Block-STM backend
- Transaction builder GUI (common patterns: ERC-20 transfer, DEX swap, NFT mint)
- Full conflict report with exact incarnation data (from simulation, not mainnet)
- Side-by-side comparison: "before optimization" vs "after optimization"
- CI/CD integration: `vibe-room analyze --json` for headless use

**Exit criteria:** End-to-end flow works: write Solidity → submit → hear result → modify → re-submit → hear improvement.

### M4: API Layer + Launch (Week 13-16)

**Objective:** Data API for programmatic access. Public launch with marketing push.

**Deliverables:**
- REST API: `/v1/blocks/{id}/parallelism`, `/v1/contracts/{addr}/conflict-history`
- WebSocket API: real-time event stream for custom integrations
- API documentation (OpenAPI/Swagger)
- Landing page with embedded live visualizer
- Launch blog post + Twitter thread with GIF/video demos
- Monad ecosystem outreach (DevRel, docs team, community)

**Exit criteria:** API serving external requests, 3+ third-party integrations.

---

## 7. Sound Design Specification

### 7.1 Musical system

**Base scale:** C major pentatonic (C4, D4, E4, G4, A4) — no dissonance possible between any combination of these notes.

**Core-to-instrument mapping:**

| Core | Base note | Timbre |
|------|-----------|--------|
| Core 0 | C4 | Triangle wave (warm, fundamental) |
| Core 1 | E4 | Sine wave (pure, clear) |
| Core 2 | G4 | Square wave (bright, digital) |
| Core 3 | A4 | Sawtooth wave (rich, textured) |

**Transaction position modulation:** Within each core, the transaction's position in the block shifts the note by pentatonic intervals. tx0 = base note, tx1 = +2 semitones, tx2 = +4, etc. This creates melodic variation while maintaining harmonic consistency.

### 7.2 Event-to-sound mapping

| Event | Sound design | Duration | Volume |
|-------|-------------|----------|--------|
| Tx enters lane | Subtle click (filtered noise) | 10ms | 0.1 |
| Tx reaches commit zone (success) | Pentatonic note from core's instrument | 200ms | 0.3 |
| Conflict detected | Two semitone-adjacent notes + pink noise burst | 400ms | 0.4 |
| Re-execution triggered | Quick ascending 3-note arpeggio | 150ms | 0.2 |
| Re-execution resolved | Descending resolution to pentatonic note | 250ms | 0.3 |
| Block complete | Root chord (C-E-G) with slight reverb | 500ms | 0.5 |
| Health critical (< 30%) | Sustained low drone underneath all sounds | Continuous | 0.15 |

### 7.3 Adaptive music system

**Healthy network (parallelism > 80%):**
- Major pentatonic harmony
- Moderate tempo
- Clean, bright timbres
- Occasional melodic phrases emerge from transaction patterns

**Stressed network (parallelism 50-80%):**
- Minor pentatonic shifts
- Faster tempo (more re-executions = more notes)
- Slight detuning on conflict notes
- More rhythmic complexity

**Critical network (parallelism < 50%):**
- Dissonant cluster chords
- Erratic tempo
- Noise floor increases
- Musical tension builds until resolution

---

## 8. Business Model

### 8.1 Revenue structure

| Tier | Product | Price | Target |
|------|---------|-------|--------|
| **Free** | Live mainnet visualizer + basic API (100 req/day) | $0 | Everyone (adoption driver) |
| **Pro** | "Hear your contract" simulation + historical data + CI integration | $49/month | Protocol teams |
| **Enterprise** | Full API access + custom dashboards + dedicated support | Custom | Indexers, analytics platforms, validators |
| **Grant** | Open-source visualizer as public good | Monad Foundation grant | Ecosystem-wide |

### 8.2 Grant vs business balance

**Open-source (grant-funded):** Live visualizer, basic API, embeddable widget. This is the "show" layer — viral, educational, marketing asset for Monad.

**Proprietary (business):** Simulation engine API, historical conflict database, CI/CD integration, custom protocol dashboards. This is the "understand" and "optimize" layers — value-add for paying customers.

**The Ponder model:** Ponder was MIT-licensed, used by hundreds of apps, and acqui-hired by Monad Foundation. If we achieve similar adoption with the open-source visualizer, the same outcome is possible. The business model doesn't need to be a pure SaaS success — ecosystem position is the primary asset.

### 8.3 Exit scenarios

| Scenario | Probability | Path |
|----------|------------|------|
| Acqui-hire by Monad Foundation | Medium | Follow Ponder: build useful open-source infra → Foundation brings team in-house |
| Nitro → VC seed round | Medium | $500K Nitro + demo day → Paradigm/Dragonfly seed |
| Standalone SaaS | Low | Requires significant Monad DeFi growth to generate enough paying customers |
| Upstream merge → standard | Medium | Our instrumentation code becomes part of official monad client |

---

## 9. Technical Risks

| ID | Risk | Impact | Likelihood | Mitigation |
|----|------|--------|------------|------------|
| R1 | `libmonad_event` doesn't provide enough data for meaningful live visualization | HIGH | MEDIUM | Mode A uses derived metrics (timing, access frequency). Mode B (simulation) provides full data. Hybrid mode depends on upstream merge. |
| R2 | Monad Foundation considers parallel execution internals "intentionally opaque" | HIGH | LOW | Ask directly at Blitz Seoul. If confirmed, pivot to derived-data-only approach. |
| R3 | Tenderly adds genuine parallel execution features | MEDIUM | LOW | Tenderly operates at RPC level — they can't access engine internals without forking. Our engine-level instrumentation is fundamentally different. |
| R4 | Audio/visual novelty wears off, no sustained engagement | MEDIUM | MEDIUM | Layer 2-3 (dashboard, API) provide utilitarian value beyond novelty. The game is the hook; the data is the retention. |
| R5 | Monad mainnet performance makes derived metrics inaccurate | LOW | LOW | Cross-validate derived metrics against simulation results. If they diverge significantly, flag as "approximate." |
| R6 | Web Audio API browser compatibility issues | LOW | LOW | Tone.js abstracts browser differences. Fallback to visual-only mode if audio fails. |

---

## 10. Success Metrics

### 10.1 Blitz Seoul (immediate)

- Demo completed without crashes
- Foundation team engages in 1:1 conversation post-demo
- Answer to "will you accept upstream PR for incarnation data" obtained
- Nitro application timeline confirmed

### 10.2 Post-launch (3 months)

| Metric | Target | Rationale |
|--------|--------|-----------|
| Unique visitors (live visualizer) | 5,000+ | Viral potential from crypto Twitter |
| Embedded widgets on third-party sites | 10+ | Proves ecosystem value |
| "Hear your contract" simulations run | 500+ | Developer engagement |
| API consumers | 5+ | Platform stickiness |
| Twitter impressions (demo videos) | 100K+ | Marketing asset value for Foundation |
| Monad docs/blog mention | 1+ | Ecosystem endorsement signal |

### 10.3 Grant/business validation

| Signal | Meaning |
|--------|---------|
| Foundation links us from docs | Ecosystem position secured (like gmonads.com) |
| Nitro acceptance | $500K + VC pipeline activated |
| Upstream PR accepted | Our code becomes part of every Monad node |
| Protocol team pays for Pro tier | PMF confirmed for developer tool layer |
| Acqui-hire conversation initiated | Maximum outcome (Ponder path) |

---

## 11. Open Questions

| # | Question | Decision needed by | Impact |
|---|----------|-------------------|--------|
| Q1 | Does Monad Foundation want incarnation/conflict data exposed? | Blitz Seoul (ask directly) | Determines Mode A data richness |
| Q2 | Upstream merge: will Category Labs accept instrumentation PR? | Post-Blitz Seoul | Determines long-term moat strategy |
| Q3 | Live mode data quality: is timing-derived parallelism accurate enough? | M2 (Week 8) | Determines if live mode is viable |
| Q4 | Sound design: real-time audio synthesis vs pre-rendered samples? | M1 (Week 2) | Performance trade-off |
| Q5 | Pricing: freemium vs fully grant-funded? | Nitro (if accepted) | Business model validation |

---

## Appendix A: Blitz Seoul Demo Script (5 minutes)

**[0:00-0:30] Hook**
"Every blockchain shows you what happened. We show you how it happened — in parallel — and you can hear it."
→ Start the visualizer. Transactions begin falling. Music begins playing.

**[0:30-1:30] Healthy network**
Conflict rate at 10%. Pentatonic harmony. Green flashes. Health bar full.
"This is a healthy Monad block. 100 transactions, 4 cores, beautiful harmony. Parallelism ratio 92%."

**[1:30-2:30] Introduce stress**
Slide conflict rate to 50%. Music becomes harsh. Red flashes. Health bar drops.
"Now imagine a poorly designed DEX contract. Every swap touches the same storage slot. Hear that? That's conflict. That's re-execution. That's wasted compute."

**[2:30-3:30] The insight**
"Tenderly will tell you your transaction succeeded. What it won't tell you is that it was re-executed 4 times before succeeding. We show you that — and you can hear the difference."

**[3:30-4:30] Developer use case**
"Developers submit their Solidity. If it sounds like this [play dissonant clip] — they know to optimize. If it sounds like this [play harmonious clip] — they're good to deploy. No docs needed. Your ears tell you."

**[4:30-5:00] The ask**
"We built Monad's Block-STM engine from scratch — 14,000 lines of Rust, 295 tests. We understand parallel execution because we implemented it ourselves. We're building the tool that makes Monad's core innovation visible and audible. We'd love your feedback on this direction."

---

## Appendix B: Comparison with Ponder (Acqui-hire Model)

| Dimension | Ponder | Vibe Room |
|-----------|--------|-----------|
| Core function | Smart contract events → API | Parallel execution events → experience + API |
| Pain solved | Indexing is slow and complex | Parallel execution is invisible |
| Daily use driver | Every dApp needs indexed data | Live monitoring + developer tool |
| Open-source | MIT license, open from day one | MIT license (visualizer), proprietary (simulation API) |
| Team size | 3 people | 2 people |
| Monad-specific value | "High-throughput chains need faster indexing" | "Only Monad has parallel execution to visualize" |
| Acqui-hire trigger | Hundreds of apps using it | Official docs link + ecosystem standard |

---

*End of Document*