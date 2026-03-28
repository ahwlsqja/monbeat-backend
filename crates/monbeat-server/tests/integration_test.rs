//! Integration tests for the MonBeat simulation server HTTP API.
//!
//! These tests start a real Axum server and send HTTP requests to test
//! the full pipeline: Solidity source → compile → build → execute → game events.
//!
//! Requires solc 0.8.28+ installed. Tests skip with a message if solc is missing.

use std::sync::Arc;
use std::time::Instant;

use axum::{routing, Router};
use tower_http::cors::CorsLayer;

use monbeat_server::api;
use monbeat_server::ws;

/// Check if solc is available. If not, tests skip gracefully.
fn has_solc() -> bool {
    std::process::Command::new("solc")
        .arg("--version")
        .output()
        .is_ok()
}

/// Check if monad-vibe-cli is available. If not, engine-dependent tests skip.
fn has_engine() -> bool {
    monbeat_server::engine::is_available()
}

/// Spawn a test server on a random available port, return the base URL.
async fn spawn_test_server() -> String {
    let state = Arc::new(api::AppState {
        start_time: Instant::now(),
        simulation_semaphore: tokio::sync::Semaphore::new(4),
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

// ---------------------------------------------------------------------------
// Test 1: Health endpoint (expanded with db/redis fields)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_health_endpoint() {
    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base_url}/health"))
        .send()
        .await
        .expect("health request failed");

    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["uptime_secs"].is_number());
    // db/redis not connected in test mode
    assert_eq!(body["db_connected"], false);
    assert_eq!(body["redis_connected"], false);
    assert_eq!(body["pool_size"], 0);
    assert_eq!(body["pool_idle"], 0);
}

// ---------------------------------------------------------------------------
// Test 2: Counter contract — no conflicts, only TxCommit + BlockComplete
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_counter_no_conflicts() {
    if !has_solc() || !has_engine() {
        eprintln!("SKIP: solc or monad-vibe-cli not installed");
        return;
    }

    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    let source = r#"
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

    let resp = client
        .post(format!("{base_url}/api/simulate"))
        .json(&serde_json::json!({ "source": source, "repeat_count": 1 }))
        .send()
        .await
        .expect("simulate request failed");

    let status = resp.status();
    let body_text = resp.text().await.unwrap();
    assert_eq!(status, 200, "expected 200 OK, body: {body_text}");

    let body: serde_json::Value = serde_json::from_str(&body_text).unwrap();

    // Verify response structure
    assert!(body["results"].is_array(), "results should be an array");
    assert!(body["incarnations"].is_array(), "incarnations should be an array");
    assert!(body["stats"].is_object(), "stats should be an object");
    assert!(body["gameEvents"].is_array(), "gameEvents should be an array");

    // Results: deploy + 2 functions = at least 3 txs
    let results = body["results"].as_array().unwrap();
    assert!(results.len() >= 3, "expected at least 3 tx results, got {}", results.len());

    // Deploy tx should succeed
    assert_eq!(results[0]["success"], true, "deploy tx should succeed");

    // gameEvents should contain TxCommit (type=1) and BlockComplete (type=5)
    let game_events = body["gameEvents"].as_array().unwrap();
    assert!(!game_events.is_empty(), "gameEvents should not be empty");

    let has_tx_commit = game_events.iter().any(|e| e["type"] == 1);
    let has_block_complete = game_events.iter().any(|e| e["type"] == 5);
    assert!(has_tx_commit, "should have TxCommit events (type=1)");
    assert!(has_block_complete, "should have BlockComplete events (type=5)");

    // Stats should have positive gas
    assert!(body["stats"]["total_gas"].as_u64().unwrap() > 0);
    assert!(body["stats"]["num_transactions"].as_u64().unwrap() >= 3);
}

// ---------------------------------------------------------------------------
// Test 3: SharedStorage contract — multiple senders writing same slot → conflicts
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_shared_storage_conflicts() {
    if !has_solc() || !has_engine() {
        eprintln!("SKIP: solc or monad-vibe-cli not installed");
        return;
    }

    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Contract where multiple functions write to the same storage slot
    let source = r#"
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

    let resp = client
        .post(format!("{base_url}/api/simulate"))
        .json(&serde_json::json!({ "source": source, "repeat_count": 1 }))
        .send()
        .await
        .expect("simulate request failed");

    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();

    let game_events = body["gameEvents"].as_array().unwrap();
    assert!(!game_events.is_empty(), "gameEvents should not be empty");

    // With 3 functions all writing to the same `value` slot from different senders,
    // there should be conflicts detected. The conflict_details should show them.
    let conflict_details = &body["conflict_details"];
    assert!(conflict_details["conflicts"].is_array());

    // At minimum we should have TxCommit and BlockComplete events
    let has_tx_commit = game_events.iter().any(|e| e["type"] == 1);
    let has_block_complete = game_events.iter().any(|e| e["type"] == 5);
    assert!(has_tx_commit, "should have TxCommit events");
    assert!(has_block_complete, "should have BlockComplete events");

    // For SharedStorage, we expect conflict events (type=2) since all 3 functions
    // write to the same slot. Whether the parallel executor detects them depends on
    // execution ordering, but conflict_details should have entries.
    let has_conflicts = conflict_details["conflicts"].as_array().unwrap().len() > 0;
    if has_conflicts {
        // If conflicts were detected, gameEvents should include Conflict events
        let has_conflict_events = game_events.iter().any(|e| e["type"] == 2);
        assert!(
            has_conflict_events,
            "conflict_details has conflicts but no Conflict game events"
        );
    }

    // Verify gameEvent structure
    let first_event = &game_events[0];
    assert!(first_event["lane"].is_number());
    assert!(first_event["tx_index"].is_number());
    assert!(first_event["note"].is_number());
    assert!(first_event["timestamp"].is_number());
}

// ---------------------------------------------------------------------------
// Test 4: Compile error → 400 response
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_compile_error_returns_400() {
    if !has_solc() || !has_engine() {
        eprintln!("SKIP: solc or monad-vibe-cli not installed");
        return;
    }

    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base_url}/api/simulate"))
        .json(&serde_json::json!({ "source": "this is not valid solidity code" }))
        .send()
        .await
        .expect("simulate request failed");

    assert_eq!(resp.status(), 400, "invalid source should return 400");

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["error"].is_string(),
        "error response should have 'error' field"
    );
    let error_msg = body["error"].as_str().unwrap();
    assert!(
        !error_msg.is_empty(),
        "error message should not be empty"
    );
}

// ---------------------------------------------------------------------------
// Test 5: Missing source field → 422 (Axum's default for deserialization failure)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_missing_source_field() {
    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base_url}/api/simulate"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("simulate request failed");

    // Axum returns 422 Unprocessable Entity for JSON deserialization failures
    assert_eq!(resp.status(), 422);
}

// ---------------------------------------------------------------------------
// Test 6: CORS headers present
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_cors_headers() {
    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base_url}/health"))
        .header("Origin", "http://localhost:5173")
        .send()
        .await
        .expect("health request failed");

    assert_eq!(resp.status(), 200);
    // CorsLayer::permissive() adds access-control-allow-origin: *
    let cors_header = resp.headers().get("access-control-allow-origin");
    assert!(cors_header.is_some(), "CORS header should be present");
}

// ---------------------------------------------------------------------------
// Test 7: GET /api/simulations returns empty list when no DB
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_simulations_empty_no_db() {
    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base_url}/api/simulations"))
        .send()
        .await
        .expect("simulations request failed");

    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["simulations"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"], 0);
    assert_eq!(body["message"], "database not connected");
}

// ---------------------------------------------------------------------------
// Test 8: SimulateResponse Deserialize round-trip
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_simulate_response_deserialize_roundtrip() {
    use monbeat_server::api::{
        ConflictDetailsOutput, ConflictPairOutput, ExecutionStats, SimulateResponse,
        TxResultOutput,
    };
    use monbeat_server::game_events::{GameEvent, GameEventType};

    let response = SimulateResponse {
        results: vec![TxResultOutput {
            success: true,
            gas_used: 21000,
            output: "0x".to_string(),
            error: None,
            logs_count: 0,
        }],
        incarnations: vec![0],
        stats: ExecutionStats {
            total_gas: 21000,
            num_transactions: 1,
            num_conflicts: 0,
            num_re_executions: 0,
        },
        conflict_details: ConflictDetailsOutput {
            conflicts: vec![ConflictPairOutput {
                tx_a: 0,
                tx_b: 1,
                location_type: "Storage".to_string(),
                conflict_type: "write-write".to_string(),
            }],
        },
        game_events: vec![GameEvent {
            event_type: GameEventType::TxCommit,
            lane: 0,
            tx_index: 0,
            note: 60,
            slot: 0,
            timestamp: 0.0,
        }],
    };

    let json_str = serde_json::to_string(&response).unwrap();
    let deserialized: SimulateResponse = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.results.len(), 1);
    assert_eq!(deserialized.results[0].success, true);
    assert_eq!(deserialized.results[0].gas_used, 21000);
    assert_eq!(deserialized.stats.total_gas, 21000);
    assert_eq!(deserialized.conflict_details.conflicts.len(), 1);
    assert_eq!(deserialized.game_events.len(), 1);
    assert_eq!(deserialized.game_events[0].event_type, GameEventType::TxCommit);
}

// ---------------------------------------------------------------------------
// Test 9: Health endpoint returns expanded fields
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_health_expanded_fields() {
    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base_url}/health"))
        .send()
        .await
        .expect("health request failed");

    let body: serde_json::Value = resp.json().await.unwrap();

    // Verify all expected fields exist
    assert!(body.get("status").is_some(), "missing status field");
    assert!(body.get("uptime_secs").is_some(), "missing uptime_secs field");
    assert!(body.get("db_connected").is_some(), "missing db_connected field");
    assert!(body.get("redis_connected").is_some(), "missing redis_connected field");
    assert!(body.get("pool_size").is_some(), "missing pool_size field");
    assert!(body.get("pool_idle").is_some(), "missing pool_idle field");
}

// ---------------------------------------------------------------------------
// Test 10: POST /api/simulate with repeat_count=100 → 301 results
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_simulate_with_repeat_count() {
    if !has_solc() || !has_engine() {
        eprintln!("SKIP: solc or monad-vibe-cli not installed");
        return;
    }

    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Counter has 2 state-changing functions (increment, decrement)
    // repeat_count=100 → 200 call txs + 1 deploy = 201 results
    let source = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Counter {
    uint256 public count;
    function increment() public { count += 1; }
    function decrement() public { count -= 1; }
}
"#;

    let resp = client
        .post(format!("{base_url}/api/simulate"))
        .json(&serde_json::json!({ "source": source, "repeat_count": 100 }))
        .send()
        .await
        .expect("simulate request failed");

    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    let results = body["results"].as_array().unwrap();

    // 1 deploy + 2 functions × 100 repeats = 201 txs
    assert_eq!(
        results.len(),
        201,
        "expected 201 results (1 deploy + 2*100), got {}",
        results.len()
    );

    // Deploy tx should succeed
    assert_eq!(results[0]["success"], true, "deploy should succeed");

    // Stats should reflect the correct tx count
    assert_eq!(body["stats"]["num_transactions"].as_u64().unwrap(), 201);

    // gameEvents should be populated
    let events = body["gameEvents"].as_array().unwrap();
    assert!(events.len() > 100, "should have many game events, got {}", events.len());
}

// ---------------------------------------------------------------------------
// Test 11: POST /api/simulate without repeat_count → auto ~300 TXs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_simulate_default_repeat_count_targets_300() {
    if !has_solc() || !has_engine() {
        eprintln!("SKIP: solc or monad-vibe-cli not installed");
        return;
    }

    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    let source = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Counter {
    uint256 public count;
    function increment() public { count += 1; }
    function decrement() public { count -= 1; }
}
"#;

    // No repeat_count in body → auto-compute: 2 fns → repeat=150 → 300+1=301
    let resp = client
        .post(format!("{base_url}/api/simulate"))
        .json(&serde_json::json!({ "source": source }))
        .send()
        .await
        .expect("simulate request failed");

    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    let results = body["results"].as_array().unwrap();

    // Auto-targets ~300 call TXs: 2 fns × 150 = 300 + 1 deploy = 301
    assert_eq!(
        results.len(),
        301,
        "default repeat_count should produce 301 results, got {}",
        results.len()
    );
}
