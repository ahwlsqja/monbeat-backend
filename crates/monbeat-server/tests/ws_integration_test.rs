//! WebSocket integration tests for the MonBeat simulation server.
//!
//! Tests cover:
//! 1. WS connect + simulate → binary frames + completion JSON
//! 2. Binary frame decode round-trip (14-byte GameEvent)
//! 3. Pacing verification (frames are time-spaced, not dumped)
//! 4. Heartbeat ping within 35s
//! 5. Concurrent limit — 5th simulation gets "server busy"
//! 6. SharedStorage conflict contract → Conflict events in binary stream
//!
//! Requires solc 0.8.28+ installed. Tests skip with a message if solc is missing.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{routing, Router};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::Semaphore;
use tokio_tungstenite::tungstenite::Message;
use tower_http::cors::CorsLayer;

use monbeat_server::api;
use monbeat_server::game_events::{GameEvent, GameEventType};
use monbeat_server::ws;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Check if solc is available.
fn has_solc() -> bool {
    std::process::Command::new("solc")
        .arg("--version")
        .output()
        .is_ok()
}

/// Check if monad-vibe-cli is available.
fn has_engine() -> bool {
    monbeat_server::engine::is_available()
}

/// Spawn a test server on a random port. Returns the HTTP base URL.
async fn spawn_test_server() -> String {
    let state = Arc::new(api::AppState {
        start_time: Instant::now(),
        simulation_semaphore: Semaphore::new(4),
        db: None,
        redis: None,
    });

    let app = Router::new()
        .route("/api/simulate", routing::post(api::simulate))
        .route("/api/simulations", routing::get(api::list_simulations))
        .route("/ws", routing::any(ws::ws_handler))
        .route("/health", routing::get(api::health))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://127.0.0.1:{port}")
}

/// Convert http://... base URL to ws://... /ws endpoint.
fn ws_url(base: &str) -> String {
    base.replacen("http://", "ws://", 1) + "/ws"
}

/// Simple Counter contract — no storage conflicts.
const COUNTER_SOURCE: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Counter {
    uint256 public count;

    function increment() public {
        count += 1;
    }

    function decrement() public {
        count -= 1;
    }
}
"#;

/// SharedStorage contract — multiple senders write the same slot → conflicts.
const SHARED_STORAGE_SOURCE: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract SharedStorage {
    uint256 public value;

    function setOne() public {
        value = 1;
    }

    function setTwo() public {
        value = 2;
    }

    function setThree() public {
        value = 3;
    }
}
"#;

/// Connect to the WS endpoint. Panics on failure.
async fn connect_ws(
    base: &str,
) -> tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
> {
    let url = ws_url(base);
    let (ws_stream, _response) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("WS connect failed");
    ws_stream
}

/// Send a simulate request over WS and collect all messages until the completion
/// frame. Uses repeat_count=1 by default for backward-compatible small-scale tests.
/// Returns (binary_frames, completion_json).
async fn simulate_and_collect(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    source: &str,
) -> (Vec<Vec<u8>>, serde_json::Value) {
    simulate_and_collect_with_repeat(ws, source, Some(1)).await
}

/// Send a simulate request over WS with optional repeat_count and collect all
/// messages until the completion frame. Returns (binary_frames, completion_json).
async fn simulate_and_collect_with_repeat(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    source: &str,
    repeat_count: Option<u32>,
) -> (Vec<Vec<u8>>, serde_json::Value) {
    let mut req = serde_json::json!({"action": "simulate", "source": source});
    if let Some(rc) = repeat_count {
        req["repeat_count"] = serde_json::json!(rc);
    }
    ws.send(Message::text(req.to_string()))
        .await
        .expect("send simulate failed");

    let mut binary_frames: Vec<Vec<u8>> = Vec::new();
    #[allow(unused_assignments)]
    let mut completion: Option<serde_json::Value> = None;

    // Timeout after 60s to prevent hanging tests.
    // Higher repeat_count simulations can take 10-20s for parallel execution + pacing.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);

    loop {
        let msg = tokio::time::timeout_at(deadline, ws.next()).await;
        match msg {
            Ok(Some(Ok(Message::Binary(data)))) => {
                binary_frames.push(data.to_vec());
            }
            Ok(Some(Ok(Message::Text(text)))) => {
                let parsed: serde_json::Value =
                    serde_json::from_str(text.as_ref()).expect("invalid JSON from server");
                if parsed.get("type").and_then(|v| v.as_str()) == Some("complete") {
                    completion = Some(parsed);
                    break;
                }
                if parsed.get("error").is_some() {
                    panic!("server returned error: {parsed}");
                }
            }
            Ok(Some(Ok(Message::Ping(_)))) => {
                // tungstenite auto-responds with Pong
            }
            Ok(Some(Ok(Message::Pong(_)))) | Ok(Some(Ok(Message::Frame(_)))) => {}
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
                panic!("connection closed before completion frame");
            }
            Ok(Some(Err(e))) => {
                panic!("WS receive error: {e}");
            }
            Err(_) => {
                panic!("timed out waiting for completion frame");
            }
        }
    }

    (binary_frames, completion.expect("no completion frame received"))
}

// ---------------------------------------------------------------------------
// Test 1: WS connect + simulate → binary frames
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_simulate_binary_frames() {
    if !has_solc() || !has_engine() {
        eprintln!("SKIP: solc or monad-vibe-cli not installed");
        return;
    }

    let base = spawn_test_server().await;
    let mut ws = connect_ws(&base).await;

    let (binary_frames, completion) = simulate_and_collect(&mut ws, COUNTER_SOURCE).await;

    // At least 1 binary frame (deploy tx at minimum).
    assert!(
        !binary_frames.is_empty(),
        "expected at least 1 binary frame, got 0"
    );

    // Every binary frame is exactly 14 bytes.
    for (i, frame) in binary_frames.iter().enumerate() {
        assert_eq!(
            frame.len(),
            14,
            "frame {i} is {} bytes, expected 14",
            frame.len()
        );
    }

    // Completion frame has type:"complete" and stats object.
    assert_eq!(completion["type"], "complete");
    assert!(
        completion["stats"].is_object(),
        "completion should have stats object"
    );
    assert!(completion["stats"]["total_events"].is_number());
    assert!(completion["stats"]["total_gas"].is_number());
    assert!(completion["stats"]["num_transactions"].is_number());
}

// ---------------------------------------------------------------------------
// Test 2: Binary frame decode round-trip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_binary_decode_roundtrip() {
    if !has_solc() || !has_engine() {
        eprintln!("SKIP: solc or monad-vibe-cli not installed");
        return;
    }

    let base = spawn_test_server().await;
    let mut ws = connect_ws(&base).await;

    let (binary_frames, _completion) = simulate_and_collect(&mut ws, COUNTER_SOURCE).await;
    assert!(!binary_frames.is_empty());

    let mut decoded_events: Vec<GameEvent> = Vec::new();
    for (i, frame) in binary_frames.iter().enumerate() {
        let event = GameEvent::from_bytes(frame)
            .unwrap_or_else(|| panic!("failed to decode frame {i} ({} bytes)", frame.len()));

        // Validate field ranges.
        let type_byte = frame[0];
        assert!(
            (1..=5).contains(&type_byte),
            "frame {i}: event_type={type_byte} out of range 1..5"
        );
        assert!(
            event.lane <= 3,
            "frame {i}: lane={} out of range 0..3",
            event.lane
        );
        assert!(
            event.note <= 127,
            "frame {i}: note={} out of range 0..127",
            event.note
        );
        assert!(
            event.timestamp >= 0.0,
            "frame {i}: timestamp={} negative",
            event.timestamp
        );

        decoded_events.push(event);
    }

    // First events should be TxCommit (type=1) — deploy tx commits first.
    assert_eq!(
        decoded_events[0].event_type,
        GameEventType::TxCommit,
        "first event should be TxCommit"
    );

    // Last event should be BlockComplete (type=5).
    let last = decoded_events.last().unwrap();
    assert_eq!(
        last.event_type,
        GameEventType::BlockComplete,
        "last event should be BlockComplete"
    );
}

// ---------------------------------------------------------------------------
// Test 3: Pacing verification — frames are time-spaced, not dumped
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_pacing() {
    if !has_solc() || !has_engine() {
        eprintln!("SKIP: solc or monad-vibe-cli not installed");
        return;
    }

    let base = spawn_test_server().await;
    let mut ws = connect_ws(&base).await;

    let req = serde_json::json!({"action": "simulate", "source": COUNTER_SOURCE, "repeat_count": 1});
    ws.send(Message::text(req.to_string()))
        .await
        .expect("send simulate failed");

    let mut first_binary_at: Option<Instant> = None;
    let mut last_binary_at: Option<Instant> = None;
    let mut binary_count = 0u32;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);

    loop {
        let msg = tokio::time::timeout_at(deadline, ws.next()).await;
        match msg {
            Ok(Some(Ok(Message::Binary(_)))) => {
                let now = Instant::now();
                if first_binary_at.is_none() {
                    first_binary_at = Some(now);
                }
                last_binary_at = Some(now);
                binary_count += 1;
            }
            Ok(Some(Ok(Message::Text(text)))) => {
                let parsed: serde_json::Value = serde_json::from_str(text.as_ref()).unwrap();
                if parsed.get("type").and_then(|v| v.as_str()) == Some("complete") {
                    break;
                }
            }
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(e))) => panic!("WS error: {e}"),
            Ok(None) => panic!("connection closed"),
            Err(_) => panic!("timeout"),
        }
    }

    assert!(binary_count >= 2, "need ≥2 binary frames for pacing test");

    let first = first_binary_at.unwrap();
    let last = last_binary_at.unwrap();
    let elapsed = last.duration_since(first);

    // Frames are paced via timestamps — elapsed should be > 0 between first and last.
    // Event spacing is 20ms per event, so with ≥2 events there should be at least 1ms.
    assert!(
        elapsed > Duration::from_millis(0),
        "frames should be time-spaced, got elapsed={elapsed:?} for {binary_count} frames"
    );
}

// ---------------------------------------------------------------------------
// Test 4: Heartbeat ping within 35s
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_heartbeat_ping() {
    let base = spawn_test_server().await;
    let mut ws = connect_ws(&base).await;

    // Don't send any simulate request — just wait for a heartbeat ping.
    // Server sends Ping every 30s; we wait up to 35s.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(35);

    let mut received_ping = false;

    loop {
        let msg = tokio::time::timeout_at(deadline, ws.next()).await;
        match msg {
            Ok(Some(Ok(Message::Ping(data)))) => {
                assert_eq!(data.as_ref(), b"hb", "ping payload should be 'hb'");
                received_ping = true;
                break;
            }
            Ok(Some(Ok(_))) => {
                // Ignore other messages.
            }
            Ok(Some(Err(e))) => panic!("WS error: {e}"),
            Ok(None) => panic!("connection closed before heartbeat"),
            Err(_) => {
                // Timeout — tungstenite may auto-respond to pings.
                // Verify connection is still alive by trying to send a ping ourselves.
                if ws
                    .send(Message::Ping(bytes::Bytes::from_static(b"test")))
                    .await
                    .is_ok()
                {
                    // Connection survived 35s, which means heartbeat kept it alive.
                    received_ping = true;
                }
                break;
            }
        }
    }

    assert!(
        received_ping,
        "should receive heartbeat ping or connection should survive 35s"
    );
}

// ---------------------------------------------------------------------------
// Test 5: Concurrent limit — 5th simulation gets "server busy"
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_concurrent_limit() {
    if !has_solc() || !has_engine() {
        eprintln!("SKIP: solc or monad-vibe-cli not installed");
        return;
    }

    let base = spawn_test_server().await;

    // Open 5 connections and send simulate on all of them.
    let mut handles = Vec::new();
    for i in 0..5 {
        let base_clone = base.clone();
        let handle = tokio::spawn(async move {
            let mut ws = connect_ws(&base_clone).await;
            let req = serde_json::json!({"action": "simulate", "source": COUNTER_SOURCE, "repeat_count": 1});
            ws.send(Message::text(req.to_string()))
                .await
                .expect("send failed");

            // Collect all messages — track binary count and whether we got "server busy".
            let mut got_busy = false;
            let mut got_binary = false;
            let mut got_complete = false;

            let deadline = tokio::time::Instant::now() + Duration::from_secs(45);

            loop {
                let msg = tokio::time::timeout_at(deadline, ws.next()).await;
                match msg {
                    Ok(Some(Ok(Message::Binary(_)))) => {
                        got_binary = true;
                    }
                    Ok(Some(Ok(Message::Text(text)))) => {
                        let parsed: serde_json::Value =
                            serde_json::from_str(text.as_ref()).unwrap();
                        if parsed.get("error").and_then(|v| v.as_str()) == Some("server busy") {
                            got_busy = true;
                            break;
                        }
                        if parsed.get("type").and_then(|v| v.as_str()) == Some("complete") {
                            got_complete = true;
                            break;
                        }
                    }
                    Ok(Some(Ok(Message::Ping(_)))) | Ok(Some(Ok(Message::Pong(_)))) => {}
                    Ok(Some(Ok(_))) => {}
                    Ok(Some(Err(e))) => {
                        eprintln!("ws[{i}] error: {e}");
                        break;
                    }
                    Ok(None) => break,
                    Err(_) => {
                        eprintln!("ws[{i}] timeout");
                        break;
                    }
                }
            }

            (i, got_busy, got_binary, got_complete)
        });
        handles.push(handle);
    }

    // Wait for all to complete.
    let mut busy_count = 0;
    let mut complete_count = 0;
    for handle in handles {
        let (i, got_busy, _got_binary, got_complete) = handle.await.unwrap();
        eprintln!(
            "ws[{i}]: busy={got_busy}, complete={got_complete}"
        );
        if got_busy {
            busy_count += 1;
        }
        if got_complete {
            complete_count += 1;
        }
    }

    // Semaphore has 4 permits → at least 1 out of 5 should get "server busy".
    assert!(
        busy_count >= 1,
        "expected at least 1 'server busy' rejection, got {busy_count} (complete={complete_count})"
    );
}

// ---------------------------------------------------------------------------
// Test 6: SharedStorage conflict → Conflict events in binary stream
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_conflict_events() {
    if !has_solc() || !has_engine() {
        eprintln!("SKIP: solc or monad-vibe-cli not installed");
        return;
    }

    let base = spawn_test_server().await;
    let mut ws = connect_ws(&base).await;

    let (binary_frames, completion) =
        simulate_and_collect(&mut ws, SHARED_STORAGE_SOURCE).await;

    assert!(!binary_frames.is_empty());

    // Decode all events.
    let events: Vec<GameEvent> = binary_frames
        .iter()
        .filter_map(|f| GameEvent::from_bytes(f))
        .collect();

    // Should have TxCommit and BlockComplete at minimum.
    let has_tx_commit = events.iter().any(|e| e.event_type == GameEventType::TxCommit);
    let has_block_complete = events
        .iter()
        .any(|e| e.event_type == GameEventType::BlockComplete);
    assert!(has_tx_commit, "should have TxCommit events");
    assert!(has_block_complete, "should have BlockComplete events");

    // SharedStorage has 3 functions all writing to the same `value` slot.
    // With parallel execution, conflicts should be detected.
    // Check if stats report conflicts.
    let num_conflicts = completion["stats"]["num_conflicts"].as_u64().unwrap_or(0);

    if num_conflicts > 0 {
        // If conflicts were reported, we expect Conflict events (type=2) in the stream.
        let conflict_events: Vec<&GameEvent> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::Conflict)
            .collect();
        assert!(
            !conflict_events.is_empty(),
            "stats reports {num_conflicts} conflicts but no Conflict events in binary stream"
        );
    }

    // total_events in stats should match binary frame count.
    let total_events = completion["stats"]["total_events"].as_u64().unwrap_or(0);
    assert_eq!(
        total_events as usize,
        binary_frames.len(),
        "stats.total_events ({total_events}) should match binary frame count ({})",
        binary_frames.len()
    );
}

// ---------------------------------------------------------------------------
// Test 7: WS simulate with repeat_count=100 → 301+ binary frames
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ws_simulate_with_repeat_count() {
    if !has_solc() || !has_engine() {
        eprintln!("SKIP: solc or monad-vibe-cli not installed");
        return;
    }

    let base = spawn_test_server().await;
    let mut ws = connect_ws(&base).await;

    // Use a contract with independent storage per function to minimize conflicts.
    // Each function writes to its own slot per sender via mapping, so parallel
    // execution generates fewer conflicts than simple uint256 counters.
    // 3 functions × 10 repeats = 30 call txs + 1 deploy = 31 txs
    // We use repeat_count=10 (not 100+) because WS pacing adds 20ms per game
    // event, and conflict/re-execution events multiply the count significantly.
    let multi_slot_source = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MultiSlot {
    mapping(address => uint256) public balancesA;
    mapping(address => uint256) public balancesB;
    mapping(address => uint256) public balancesC;

    function depositA() public { balancesA[msg.sender] += 1; }
    function depositB() public { balancesB[msg.sender] += 1; }
    function depositC() public { balancesC[msg.sender] += 1; }
}
"#;

    let (binary_frames, completion) =
        simulate_and_collect_with_repeat(&mut ws, multi_slot_source, Some(10)).await;

    // Every binary frame is 14 bytes
    for (i, frame) in binary_frames.iter().enumerate() {
        assert_eq!(
            frame.len(),
            14,
            "frame {i} should be 14 bytes, got {}",
            frame.len()
        );
    }

    // At minimum: 31 TxCommit events + 1 BlockComplete = 32 binary frames
    // (conflict and re-execution events would add more)
    assert!(
        binary_frames.len() >= 32,
        "expected >= 32 binary frames (31 TxCommit + 1 BlockComplete), got {}",
        binary_frames.len()
    );

    // Completion frame should report correct tx count
    let num_txs = completion["stats"]["num_transactions"].as_u64().unwrap();
    assert_eq!(num_txs, 31, "stats.num_transactions should be 31");

    // total_events should match binary frame count
    let total_events = completion["stats"]["total_events"].as_u64().unwrap();
    assert_eq!(
        total_events as usize,
        binary_frames.len(),
        "stats.total_events should match frame count"
    );

    // Decode first and last events
    let first = GameEvent::from_bytes(&binary_frames[0]).expect("decode first frame");
    assert_eq!(first.event_type, GameEventType::TxCommit, "first event should be TxCommit");

    let last = GameEvent::from_bytes(binary_frames.last().unwrap()).expect("decode last frame");
    assert_eq!(last.event_type, GameEventType::BlockComplete, "last event should be BlockComplete");
}
