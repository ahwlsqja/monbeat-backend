# Vibe Room — Frontend Performance Optimization PRD

**60fps Rhythm Game on Blockchain Data**

Version 1.0 | March 2026
Infra: Vercel (Frontend) + Railway Paid (Backend)

---

## 1. Why This Document Exists

Vibe Room is a rhythm-game-style visualizer rendering 10K TPS blockchain data as real-time audio-visual experience. The frontend is the product — if it stutters, the music glitches, or the health bar lags, the entire experience breaks. This is not a dashboard where 2-second load time is acceptable. This is a game where 1 dropped frame = 1 broken beat.

### 1.1 Performance budget

| Metric | Target | Rationale |
|--------|--------|-----------|
| Frame rate | 60fps sustained (16.67ms/frame) | Below 45fps, rhythm game feel collapses |
| Audio latency | < 50ms event-to-sound | Above 100ms, user perceives desync |
| WebSocket-to-render latency | < 100ms | Live mode must feel real-time |
| First Contentful Paint | < 1.5s | Vercel edge + static assets |
| Time to Interactive | < 3s | Canvas + Tone.js init |
| JS bundle size | < 200KB gzipped (initial) | Tone.js alone is ~150KB; lazy-load non-critical |
| Memory usage | < 150MB sustained | Mobile devices have ~2-4GB total |
| Mobile frame rate | 30fps minimum | Graceful degradation, not crash |

### 1.2 Performance tiers

| Tier | Device | Target FPS | Features | Particle cap |
|------|--------|-----------|----------|--------------|
| **High** | Desktop, 120Hz+ | 60fps | Full audio + particles + trails + glow | 500 |
| **Medium** | Laptop, 60Hz | 60fps | Full audio + particles, reduced trails | 200 |
| **Low** | Mobile / tablet | 30fps | Audio optional, minimal particles | 50 |
| **Minimal** | Low-end mobile | 20fps | Visual only, no audio, no particles | 0 |

Auto-detect on first load via `navigator.hardwareConcurrency` + initial FPS measurement during 2-second calibration phase.

---

## 2. Infrastructure Architecture

### 2.1 Vercel (Frontend)

```
Vercel Edge Network
│
├─ Next.js 14+ (App Router)
│  ├─ Static pages (/, /about, /docs) → ISR or SSG
│  ├─ Dynamic route (/live) → Client-side only (no SSR for Canvas)
│  └─ API routes → NOT used (all API on Railway)
│
├─ Edge Functions (if needed)
│  └─ WebSocket proxy? No — Vercel doesn't support persistent WebSocket
│     → Client connects directly to Railway WebSocket endpoint
│
└─ CDN-served assets
   ├─ JS bundles (code-split per route)
   ├─ Tone.js (lazy-loaded on /live route only)
   └─ Static assets (fonts, icons)
```

**Vercel limitations that affect architecture:**
- No persistent WebSocket support on Vercel Functions (max 25s edge, 300s serverless)
- Client must connect to Railway WebSocket directly (CORS configured)
- Canvas rendering is 100% client-side — SSR is pointless and harmful (hydration mismatch)

### 2.2 Railway Paid (Backend)

```
Railway (Pro Plan)
│
├─ WebSocket Server (Rust or Node.js)
│  ├─ Connects to Monad node (libmonad_event SDK)
│  ├─ Processes raw events → game events
│  ├─ Broadcasts to connected clients via WS
│  └─ Scales horizontally with Railway service replicas
│
├─ Simulation API (Rust)
│  ├─ Vibe-Room-Core engine (14K LOC)
│  ├─ Accepts Solidity + tx list → returns conflict report
│  └─ Stateless — each request runs independent simulation
│
└─ REST API (historical data)
   ├─ PostgreSQL for block/conflict history
   └─ Redis for caching recent blocks
```

**Railway considerations:**
- Paid plan = no sleep, persistent processes, custom domains
- WebSocket server must handle connection lifecycle (reconnect, heartbeat)
- Railway has ~100ms cold start for new instances — pre-warm with health checks
- Egress bandwidth: monitor for cost — each WS client receives ~1-5KB/s of game events

### 2.3 Data flow: Railway → Vercel client

```
Monad Node
  │ (libmonad_event, ~1μs latency)
  ▼
Railway: Event Processor
  │ (filter, normalize, assign notes)
  │ Output: GameEvent { type, lane, note, timestamp, ... }
  │ ~50 events/second per client
  ▼
WebSocket (wss://api.viberoom.xyz/ws)
  │ (Railway → Client, ~20-50ms network)
  ▼
Vercel-hosted Client (Browser)
  │
  ├─ WS Message Handler (main thread)
  │  ├─ Parse GameEvent
  │  ├─ Push to game state queue
  │  └─ Trigger audio (Tone.js, separate audio thread)
  │
  └─ requestAnimationFrame loop (main thread)
     ├─ Drain game state queue
     ├─ Update positions (delta-time based)
     ├─ Detect commit-zone hits
     ├─ Update particles
     ├─ Render frame to Canvas
     └─ Update HUD (score, health, stats)
```

---

## 3. Canvas Rendering Optimization

### 3.1 Architecture: dual canvas layers

```html
<div class="game-container">
  <!-- Layer 1: Static background (lanes, labels, commit zone) -->
  <!-- Redrawn only on resize -->
  <canvas id="bg-canvas" />

  <!-- Layer 2: Dynamic game elements (tx blocks, particles, flashes) -->
  <!-- Redrawn every frame at 60fps -->
  <canvas id="game-canvas" />

  <!-- Layer 3: HUD overlay (HTML, not Canvas) -->
  <!-- Updated via DOM, not Canvas redraws -->
  <div id="hud">
    <div class="health-bar" />
    <div class="score" />
    <div class="stats" />
  </div>
</div>
```

**Why dual canvas:**
- Background (lane lines, labels, commit zone line) doesn't change every frame
- Redrawing static elements 60x/second wastes ~30% of frame budget
- Separate canvas = background drawn once, composited by GPU for free
- HUD as HTML DOM instead of Canvas = browser handles text rendering optimally

### 3.2 Game loop: fixed timestep with interpolation

```javascript
// WRONG: variable timestep (physics tied to frame rate)
function badLoop() {
  update();  // physics runs faster on 120Hz monitors
  render();
  requestAnimationFrame(badLoop);
}

// CORRECT: fixed physics timestep, interpolated rendering
const PHYSICS_HZ = 60;
const PHYSICS_DT = 1000 / PHYSICS_HZ;
let accumulator = 0;
let lastTime = performance.now();

function gameLoop(currentTime) {
  requestAnimationFrame(gameLoop);

  const elapsed = Math.min(currentTime - lastTime, 100); // cap at 100ms (10fps)
  lastTime = currentTime;
  accumulator += elapsed;

  // Fixed-step physics updates
  while (accumulator >= PHYSICS_DT) {
    updatePhysics(PHYSICS_DT / 1000); // seconds
    accumulator -= PHYSICS_DT;
  }

  // Interpolated rendering (smooth between physics steps)
  const alpha = accumulator / PHYSICS_DT;
  render(alpha);
}
```

**Why fixed timestep matters:**
- Transaction fall speed must be identical on 60Hz and 120Hz monitors
- Without fixed step, conflict detection timing varies by device
- `Math.min(elapsed, 100)` prevents spiral of death when tab is backgrounded

### 3.3 Object pooling (zero garbage collection)

GC pauses cause visible stutters. In a rhythm game, a 50ms GC pause = missed beat.

```javascript
class ObjectPool {
  constructor(Factory, initialSize = 200) {
    this.pool = [];
    this.active = [];
    this.Factory = Factory;
    for (let i = 0; i < initialSize; i++) {
      this.pool.push(new Factory());
    }
  }

  acquire() {
    const obj = this.pool.length > 0
      ? this.pool.pop()
      : new this.Factory(); // only allocates if pool exhausted
    this.active.push(obj);
    return obj;
  }

  release(obj) {
    obj.reset(); // reset state, don't destroy
    const idx = this.active.indexOf(obj);
    if (idx !== -1) this.active.splice(idx, 1);
    this.pool.push(obj);
  }
}

// Pre-allocate all pools at init
const txPool = new ObjectPool(Transaction, 200);
const particlePool = new ObjectPool(Particle, 500);
const flashPool = new ObjectPool(Flash, 50);
```

**Pool sizes by performance tier:**

| Pool | High | Medium | Low | Minimal |
|------|------|--------|-----|---------|
| Transaction | 200 | 150 | 100 | 50 |
| Particle | 500 | 200 | 50 | 0 |
| Flash | 50 | 30 | 10 | 5 |
| Trail point | 1000 | 400 | 0 | 0 |

### 3.4 Rendering optimizations (per-frame)

**Batch draw calls by state:**

```javascript
// WRONG: interleaved draw calls (context switches per object)
txs.forEach(tx => {
  ctx.fillStyle = tx.state === 'conflict' ? RED : BLUE;
  ctx.fillRect(tx.x, tx.y, tx.w, tx.h);
});

// CORRECT: batch by fill color (minimize state changes)
ctx.fillStyle = BLUE;
activeTxs.forEach(tx => {
  if (tx.state === 'falling') ctx.fillRect(tx.x, tx.y, tx.w, tx.h);
});

ctx.fillStyle = RED;
activeTxs.forEach(tx => {
  if (tx.state === 'conflict') ctx.fillRect(tx.x, tx.y, tx.w, tx.h);
});

ctx.fillStyle = AMBER;
activeTxs.forEach(tx => {
  if (tx.state === 'reexec') ctx.fillRect(tx.x, tx.y, tx.w, tx.h);
});
```

**Integer coordinates (avoid sub-pixel anti-aliasing):**

```javascript
// WRONG: sub-pixel coordinates trigger expensive anti-aliasing
ctx.fillRect(tx.x, tx.y, tx.w, tx.h);

// CORRECT: round to integers
ctx.fillRect(tx.x | 0, tx.y | 0, tx.w, tx.h);
// Or for positions calculated with delta time:
ctx.fillRect(Math.round(tx.x), Math.round(tx.y), tx.w, tx.h);
```

**Avoid per-frame text rendering:**

```javascript
// WRONG: rendering tx hash text 60x/second per transaction
ctx.font = '10px monospace';
ctx.fillText(tx.hash, tx.x, tx.y); // text rendering is EXPENSIVE

// CORRECT: pre-render hash to offscreen canvas, drawImage
class Transaction {
  constructor() {
    this.labelCanvas = document.createElement('canvas');
    this.labelCanvas.width = 60;
    this.labelCanvas.height = 16;
  }
  setHash(hash) {
    const lctx = this.labelCanvas.getContext('2d');
    lctx.clearRect(0, 0, 60, 16);
    lctx.font = '10px monospace';
    lctx.fillStyle = '#fff';
    lctx.fillText(hash, 2, 12);
  }
  draw(ctx) {
    // drawImage is 5-10x faster than fillText
    ctx.drawImage(this.labelCanvas, this.x - 30, this.y - 8);
  }
}
```

**Dirty-rect rendering for HUD:**

```javascript
// HUD updates (score, health) happen via DOM, not Canvas
// Only update DOM when values actually change
let prevScore = -1;
function updateHUD(score, health) {
  if (score !== prevScore) {
    scoreEl.textContent = score;
    prevScore = score;
  }
  // health bar: CSS width transition handles animation
  healthFill.style.width = health + '%';
}
```

### 3.5 Canvas DPI handling

```javascript
function setupCanvas(canvas) {
  const dpr = Math.min(window.devicePixelRatio || 1, 2); // cap at 2x
  const rect = canvas.getBoundingClientRect();

  canvas.width = rect.width * dpr;
  canvas.height = rect.height * dpr;

  const ctx = canvas.getContext('2d');
  ctx.scale(dpr, dpr);

  // Return logical dimensions for game logic
  return { width: rect.width, height: rect.height, dpr };
}
```

**Cap DPI at 2x:** 3x Retina renders 9x the pixels. On mobile, this kills performance. 2x is visually sufficient for game elements.

### 3.6 Performance monitoring (built-in FPS counter)

```javascript
class PerfMonitor {
  constructor() {
    this.frames = 0;
    this.lastFPSUpdate = performance.now();
    this.currentFPS = 60;
    this.frameTimeHistory = new Float32Array(60);
    this.frameTimeIdx = 0;
  }

  beginFrame() {
    this.frameStart = performance.now();
  }

  endFrame() {
    const frameTime = performance.now() - this.frameStart;
    this.frameTimeHistory[this.frameTimeIdx++ % 60] = frameTime;
    this.frames++;

    const now = performance.now();
    if (now - this.lastFPSUpdate >= 1000) {
      this.currentFPS = this.frames;
      this.frames = 0;
      this.lastFPSUpdate = now;

      // Auto-downgrade tier if FPS drops
      if (this.currentFPS < 40) {
        this.requestTierDowngrade();
      }
    }
  }

  getAvgFrameTime() {
    let sum = 0;
    for (let i = 0; i < 60; i++) sum += this.frameTimeHistory[i];
    return sum / 60;
  }

  requestTierDowngrade() {
    // Reduce particle count, disable trails, etc.
    window.dispatchEvent(new CustomEvent('perf-downgrade'));
  }
}
```

---

## 4. Audio Performance (Tone.js)

### 4.1 Core audio architecture

```javascript
// Initialize once, reuse forever
class AudioEngine {
  constructor() {
    this.ready = false;
    this.synths = new Map(); // core -> synth
    this.noiseSynth = null;
  }

  async init() {
    await Tone.start(); // requires user gesture

    // CRITICAL: set latencyHint before creating synths
    Tone.context.latencyHint = 'interactive'; // lowest latency
    // Alternative: 'playback' for stability, 'balanced' for middle ground

    // Pre-create all synths (avoid allocation during gameplay)
    for (let core = 0; core < 4; core++) {
      this.synths.set(core, new Tone.PolySynth(Tone.Synth, {
        maxPolyphony: 4, // limit simultaneous voices per core
        oscillator: { type: CORE_TIMBRES[core] },
        envelope: { attack: 0.01, decay: 0.2, sustain: 0.05, release: 0.3 },
        volume: -18 // headroom to prevent clipping
      }).toDestination());
    }

    this.noiseSynth = new Tone.NoiseSynth({
      noise: { type: 'pink' },
      envelope: { attack: 0.005, decay: 0.1, sustain: 0, release: 0.05 },
      volume: -24
    }).toDestination();

    this.ready = true;
  }

  playNote(core, note, duration = '16n', velocity = 0.3) {
    if (!this.ready) return;
    const synth = this.synths.get(core);
    // Use Tone.now() for precise scheduling
    synth.triggerAttackRelease(note, duration, Tone.now(), velocity);
  }

  playConflict(note1, note2) {
    if (!this.ready) return;
    // Use core 0's synth for conflict (any synth works)
    this.synths.get(0).triggerAttackRelease(
      [note1, note2], '8n', Tone.now(), 0.35
    );
    this.noiseSynth.triggerAttackRelease('16n', Tone.now(), 0.2);
  }

  dispose() {
    this.synths.forEach(s => s.dispose());
    this.noiseSynth?.dispose();
  }
}
```

### 4.2 Audio performance rules

**Rule 1: Never create synths during gameplay.**

```javascript
// WRONG: creates new synth per event (GC pressure + audio glitch)
function onTxCommit(tx) {
  const synth = new Tone.Synth().toDestination();
  synth.triggerAttackRelease('C4', '8n');
}

// CORRECT: reuse pre-created PolySynth
function onTxCommit(tx) {
  audioEngine.playNote(tx.core, PENTA[tx.core], '16n');
}
```

**Rule 2: Limit total polyphony.**

PolySynth with `maxPolyphony: 4` per core = max 16 simultaneous voices total. At 10K TPS, not every tx gets a sound. Rate-limit to ~20 sounds/second max.

```javascript
class AudioRateLimiter {
  constructor(maxPerSecond = 20) {
    this.tokens = maxPerSecond;
    this.maxTokens = maxPerSecond;
    this.lastRefill = performance.now();
  }

  canPlay() {
    const now = performance.now();
    const elapsed = (now - this.lastRefill) / 1000;
    this.tokens = Math.min(this.maxTokens, this.tokens + elapsed * this.maxTokens);
    this.lastRefill = now;
    return this.tokens >= 1;
  }

  consume() {
    this.tokens -= 1;
  }
}
```

**Rule 3: Use Tone.now() for scheduling, not setTimeout.**

Web Audio API runs on a separate high-priority thread. `Tone.now()` uses the audio clock, which is more precise than `performance.now()` for scheduling sounds.

**Rule 4: Lazy-load Tone.js.**

Tone.js is ~150KB gzipped. Don't load it on page load. Dynamic import on first user interaction:

```javascript
let audioEngine = null;

async function initAudio() {
  const Tone = await import('tone'); // dynamic import
  audioEngine = new AudioEngine(Tone);
  await audioEngine.init();
}

// Called on first click/tap (required by browser autoplay policy)
startButton.addEventListener('click', () => {
  initAudio();
});
```

### 4.3 Mobile audio considerations

- iOS Safari: AudioContext must be created inside user gesture handler
- Chrome Android: `latencyHint: 'playback'` is more stable than `'interactive'`
- Low-end mobile: disable audio entirely, visual-only mode
- Battery: `Tone.Transport.stop()` when tab is backgrounded

```javascript
document.addEventListener('visibilitychange', () => {
  if (document.hidden) {
    Tone.Transport.pause();
    // Also pause game loop to save battery
  } else {
    Tone.Transport.start();
  }
});
```

---

## 5. WebSocket Optimization

### 5.1 Connection management

```javascript
class GameSocket {
  constructor(url) {
    this.url = url;
    this.ws = null;
    this.reconnectDelay = 1000;
    this.maxReconnectDelay = 30000;
    this.eventQueue = []; // buffer events during reconnect
    this.messageBuffer = []; // batch incoming messages
    this.flushInterval = null;
  }

  connect() {
    this.ws = new WebSocket(this.url);
    this.ws.binaryType = 'arraybuffer'; // binary > JSON for performance

    this.ws.onopen = () => {
      this.reconnectDelay = 1000; // reset on successful connect
      // Start message batching: flush every 16ms (one frame)
      this.flushInterval = setInterval(() => this.flushMessages(), 16);
    };

    this.ws.onmessage = (event) => {
      // Don't process immediately — buffer and batch
      this.messageBuffer.push(event.data);
    };

    this.ws.onclose = () => {
      clearInterval(this.flushInterval);
      this.scheduleReconnect();
    };
  }

  flushMessages() {
    if (this.messageBuffer.length === 0) return;

    // Process all buffered messages at once
    const batch = this.messageBuffer;
    this.messageBuffer = [];

    for (const data of batch) {
      const event = this.decode(data);
      this.onGameEvent(event); // push to game state
    }
  }

  scheduleReconnect() {
    setTimeout(() => {
      this.reconnectDelay = Math.min(
        this.reconnectDelay * 2,
        this.maxReconnectDelay
      );
      this.connect();
    }, this.reconnectDelay);
  }

  // Binary protocol: much smaller than JSON
  decode(buffer) {
    const view = new DataView(buffer);
    return {
      type: view.getUint8(0),     // 1 byte: event type
      lane: view.getUint8(1),     // 1 byte: core/lane
      txIndex: view.getUint16(2), // 2 bytes: tx index
      note: view.getUint8(4),     // 1 byte: note index
      slot: view.getUint8(5),     // 1 byte: slot identifier
      timestamp: view.getFloat64(6) // 8 bytes: high-res timestamp
    }; // Total: 14 bytes per event vs ~200 bytes JSON
  }
}
```

### 5.2 Binary vs JSON protocol

| Protocol | Size per event | Parse time | Bandwidth at 50 events/s |
|----------|---------------|------------|--------------------------|
| JSON | ~200 bytes | ~0.05ms (JSON.parse) | ~10 KB/s |
| Binary (DataView) | 14 bytes | ~0.005ms | ~0.7 KB/s |
| MessagePack | ~40 bytes | ~0.02ms | ~2 KB/s |

**Recommendation:** Start with JSON for development speed. Switch to binary when optimizing for production. The bandwidth savings matter on Railway (egress cost) and mobile (data usage).

### 5.3 Railway WebSocket scaling

```
Railway Service: ws-server
├─ Handles WebSocket connections
├─ Subscribes to Monad events (single upstream connection)
├─ Fans out to N clients
│
├─ Memory: ~1KB per client connection
├─ CPU: event processing + serialization
│
├─ Scaling strategy:
│  ├─ Single instance handles ~5,000 concurrent WS connections
│  ├─ Beyond that: Railway service replicas + Redis pub/sub
│  └─ Client connects to load-balanced endpoint
│
└─ Cost estimate (Railway Pro):
   ├─ vCPU: $10/vCPU/month
   ├─ RAM: $5/GB/month
   ├─ Egress: $0.10/GB (first 100GB free)
   └─ For 100 concurrent users at 10KB/s:
      ~86 GB/month egress = ~$0 (within free tier)
```

---

## 6. Next.js / Vercel Optimization

### 6.1 Route structure

```
app/
├─ page.tsx           → Landing page (SSG, static)
├─ live/
│  └─ page.tsx        → Live visualizer (client-only, no SSR)
├─ simulate/
│  └─ page.tsx        → "Hear your contract" (client-only)
├─ docs/
│  └─ page.tsx        → API docs (SSG)
└─ api/               → NOT USED (all API on Railway)
```

**Critical: disable SSR for game pages.**

```typescript
// app/live/page.tsx
'use client';

import dynamic from 'next/dynamic';

// Lazy-load the entire game component (includes Canvas + Tone.js)
const GameView = dynamic(() => import('@/components/GameView'), {
  ssr: false, // CRITICAL: Canvas + Web Audio = client only
  loading: () => <GameSkeleton />, // show lanes skeleton during load
});

export default function LivePage() {
  return <GameView />;
}
```

### 6.2 Bundle splitting strategy

```javascript
// next.config.js
module.exports = {
  experimental: {
    optimizePackageImports: ['tone'],
  },
  webpack: (config) => {
    config.optimization.splitChunks = {
      chunks: 'all',
      cacheGroups: {
        tone: {
          test: /[\\/]node_modules[\\/]tone[\\/]/,
          name: 'tone',
          chunks: 'all',
          priority: 20,
        },
        gameEngine: {
          test: /[\\/]components[\\/]game[\\/]/,
          name: 'game-engine',
          chunks: 'all',
          priority: 10,
        },
      },
    };
    return config;
  },
};
```

**Expected bundle sizes:**

| Chunk | Size (gzipped) | Loaded on |
|-------|---------------|-----------|
| Framework (React + Next.js) | ~85KB | All pages |
| Landing page | ~15KB | / only |
| Game engine (Canvas + game logic) | ~30KB | /live, /simulate |
| Tone.js | ~150KB | /live, /simulate (lazy, after user click) |
| Total initial load (landing) | ~100KB | / |
| Total interactive (game) | ~280KB | /live (progressive) |

### 6.3 Vercel deployment optimization

```json
// vercel.json
{
  "headers": [
    {
      "source": "/(.*)",
      "headers": [
        {
          "key": "Cache-Control",
          "value": "public, max-age=31536000, immutable"
        }
      ]
    },
    {
      "source": "/live",
      "headers": [
        {
          "key": "Cache-Control",
          "value": "no-cache"
        }
      ]
    }
  ],
  "rewrites": [
    {
      "source": "/ws",
      "destination": "https://ws.viberoom.xyz/ws"
    }
  ]
}
```

---

## 7. Mobile Optimization

### 7.1 Adaptive rendering pipeline

```javascript
class AdaptiveRenderer {
  constructor() {
    this.tier = this.detectTier();
    this.config = TIER_CONFIGS[this.tier];

    window.addEventListener('perf-downgrade', () => {
      this.downgrade();
    });
  }

  detectTier() {
    const cores = navigator.hardwareConcurrency || 2;
    const isMobile = /Mobi|Android|iPhone/i.test(navigator.userAgent);
    const memory = navigator.deviceMemory || 4; // GB

    if (isMobile && memory <= 2) return 'minimal';
    if (isMobile) return 'low';
    if (cores >= 8) return 'high';
    return 'medium';
  }

  downgrade() {
    const tiers = ['high', 'medium', 'low', 'minimal'];
    const currentIdx = tiers.indexOf(this.tier);
    if (currentIdx < tiers.length - 1) {
      this.tier = tiers[currentIdx + 1];
      this.config = TIER_CONFIGS[this.tier];
      console.log(`Performance downgrade: ${this.tier}`);
    }
  }
}

const TIER_CONFIGS = {
  high: {
    maxParticles: 500,
    trailLength: 8,
    dprCap: 2,
    enableGlow: true,
    enableAudio: true,
    targetFPS: 60,
    enableTrails: true,
  },
  medium: {
    maxParticles: 200,
    trailLength: 4,
    dprCap: 2,
    enableGlow: false,
    enableAudio: true,
    targetFPS: 60,
    enableTrails: true,
  },
  low: {
    maxParticles: 50,
    trailLength: 0,
    dprCap: 1.5,
    enableGlow: false,
    enableAudio: false, // opt-in on mobile
    targetFPS: 30,
    enableTrails: false,
  },
  minimal: {
    maxParticles: 0,
    trailLength: 0,
    dprCap: 1,
    enableGlow: false,
    enableAudio: false,
    targetFPS: 20,
    enableTrails: false,
  },
};
```

### 7.2 Touch interactions

```javascript
// Mobile: tap to toggle sound (not automatic)
canvas.addEventListener('touchstart', (e) => {
  e.preventDefault(); // prevent scroll
  if (!audioEngine) {
    initAudio(); // first tap initializes audio
  }
}, { passive: false });

// Prevent pull-to-refresh on mobile
document.body.style.overscrollBehavior = 'none';
```

### 7.3 Viewport handling

```css
.game-container {
  width: 100%;
  height: 100dvh; /* dynamic viewport height (handles mobile address bar) */
  touch-action: none; /* prevent browser gestures */
  overflow: hidden;
  -webkit-overflow-scrolling: auto;
}

canvas {
  display: block;
  width: 100%;
  height: 100%;
  /* GPU hint */
  will-change: contents;
  transform: translateZ(0);
}
```

---

## 8. Memory Management

### 8.1 Memory budget

| Component | Budget | Notes |
|-----------|--------|-------|
| Canvas backbuffer | ~20-40MB | Depends on resolution and DPI |
| Object pools (tx + particles) | ~5MB | Pre-allocated, stable |
| Tone.js AudioContext | ~10-20MB | Synth nodes + buffers |
| WebSocket buffers | ~1MB | Message queue |
| Game state (last 100 blocks) | ~5MB | Circular buffer |
| React/Next.js overhead | ~15MB | Framework baseline |
| **Total target** | **< 100MB** | |

### 8.2 Leak prevention

```javascript
// Circular buffer for block history (prevents unbounded growth)
class CircularBuffer {
  constructor(maxSize) {
    this.buffer = new Array(maxSize);
    this.head = 0;
    this.size = 0;
    this.maxSize = maxSize;
  }

  push(item) {
    this.buffer[this.head] = item;
    this.head = (this.head + 1) % this.maxSize;
    if (this.size < this.maxSize) this.size++;
  }
}

// Use for block history, event log, etc.
const blockHistory = new CircularBuffer(1000); // keep last 1000 blocks
```

**Common leak sources to watch:**

- Particle arrays growing unbounded → use pool with max size
- WebSocket message handlers accumulating closures → clear on disconnect
- Tone.js nodes not disposed → call `synth.dispose()` on page leave
- Canvas offscreen buffers → reuse, don't recreate
- `addEventListener` without `removeEventListener` → use AbortController

```javascript
// Clean component unmount (React)
useEffect(() => {
  const controller = new AbortController();

  socket.connect();
  gameLoop.start();

  return () => {
    controller.abort();
    socket.disconnect();
    gameLoop.stop();
    audioEngine?.dispose();
    txPool.clear();
    particlePool.clear();
  };
}, []);
```

---

## 9. Testing & Profiling

### 9.1 Performance testing checklist

| Test | Tool | Pass criteria |
|------|------|---------------|
| Sustained FPS (desktop) | Chrome DevTools Performance | > 58fps for 5 minutes |
| Sustained FPS (mobile) | Chrome Remote Debug + Android | > 28fps for 2 minutes |
| Memory stability | DevTools Memory tab | No growth over 5 minutes |
| Audio latency | Manual test (tap-to-sound) | < 100ms perceived |
| Bundle size | `next build --analyze` | < 200KB initial gzip |
| Lighthouse score | Lighthouse CI | > 90 Performance |
| WebSocket reconnect | Kill Railway process | Reconnects within 5s |
| 120Hz display | High-refresh monitor | Consistent 60fps (not 120) |

### 9.2 Automated performance budget (CI)

```yaml
# .github/workflows/perf.yml
- name: Bundle size check
  run: |
    npm run build
    BUNDLE_SIZE=$(du -sk .next/static/chunks | awk '{sum+=$1} END {print sum}')
    if [ $BUNDLE_SIZE -gt 500 ]; then
      echo "Bundle too large: ${BUNDLE_SIZE}KB"
      exit 1
    fi

- name: Lighthouse CI
  uses: treosh/lighthouse-ci-action@v12
  with:
    urls: |
      https://viberoom.xyz/
    budgetPath: ./lighthouse-budget.json
```

```json
// lighthouse-budget.json
[{
  "resourceSizes": [
    { "resourceType": "script", "budget": 300 },
    { "resourceType": "total", "budget": 500 }
  ],
  "timings": [
    { "metric": "first-contentful-paint", "budget": 1500 },
    { "metric": "interactive", "budget": 3000 }
  ]
}]
```

---

## 10. Implementation Priority

| Priority | Task | Impact | Effort |
|----------|------|--------|--------|
| **P0** | Fixed timestep game loop | Correctness on all devices | 1 day |
| **P0** | Dual canvas layer (static bg / dynamic game) | 30% frame time reduction | 1 day |
| **P0** | Object pooling (tx + particles) | Eliminates GC stutters | 1 day |
| **P0** | SSR disabled for game routes | Prevents hydration errors | 0.5 day |
| **P1** | Tone.js lazy loading | -150KB initial bundle | 0.5 day |
| **P1** | Binary WebSocket protocol | 14x bandwidth reduction | 2 days |
| **P1** | Performance tier auto-detection | Mobile support | 1 day |
| **P1** | Offscreen text pre-rendering | 5-10x text performance | 1 day |
| **P2** | Batch Canvas draw calls by state | 15% frame time reduction | 0.5 day |
| **P2** | DPI cap at 2x | Mobile GPU relief | 0.5 day |
| **P2** | WebSocket reconnect + buffering | Reliability | 1 day |
| **P2** | Memory leak guards (circular buffer, AbortController) | Long-session stability | 1 day |
| **P3** | CI performance budget | Prevent regressions | 0.5 day |
| **P3** | Audio rate limiter | Prevent audio overload at 10K TPS | 0.5 day |

**Total estimated effort: ~12 days** for P0-P2, ~1 day for P3.

---

*End of Document*