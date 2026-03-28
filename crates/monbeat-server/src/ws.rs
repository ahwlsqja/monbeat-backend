//! WebSocket handler for real-time binary event streaming.
//!
//! # Protocol
//!
//! 1. Client connects to `/ws` (HTTP upgrade → WebSocket)
//! 2. Client sends JSON: `{"action": "simulate", "source": "<solidity>"}`
//! 3. Server streams binary frames — each 14 bytes (one `GameEvent`)
//! 4. Server sends JSON completion: `{"type": "complete", "stats": {...}}`
//! 5. Connection stays open for more simulations
//!
//! # Pacing
//!
//! Binary frames are paced to match event timestamps from the simulation.
//! Each frame is delayed by `event.timestamp - elapsed` from stream start,
//! so conflicts arrive at the right moment relative to commits.
//!
//! # Heartbeat
//!
//! A ping is sent every 30 seconds to keep the connection alive through
//! proxies and load balancers. If the client disappears, the next send
//! will fail and the loop exits cleanly.
//!
//! # Concurrency
//!
//! A semaphore in `AppState` limits concurrent simulations. If all permits
//! are taken, the client receives `{"error": "server busy"}` immediately.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::time::{self, Instant};

use crate::api::AppState;

// ---------------------------------------------------------------------------
// Protocol types
// ---------------------------------------------------------------------------

/// Inbound JSON message from the client.
#[derive(Deserialize)]
struct ClientMessage {
    action: String,
    source: Option<String>,
    /// How many times each state-changing function is called.
    /// `None` → auto-compute to target ~300 TXs.
    repeat_count: Option<u32>,
}

/// Completion frame sent after all binary events have been streamed.
#[derive(Serialize)]
struct CompletionFrame {
    #[serde(rename = "type")]
    msg_type: &'static str,
    stats: CompletionStats,
}

/// Stats included in the completion frame.
#[derive(Serialize)]
struct CompletionStats {
    total_events: usize,
    total_gas: u64,
    num_transactions: usize,
    num_conflicts: usize,
    num_re_executions: usize,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Axum handler — upgrades HTTP to WebSocket, then delegates to `handle_ws`.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

/// Main WebSocket loop — reads messages, runs simulations, streams binary frames.
async fn handle_ws(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Heartbeat ping every 30 seconds.
    let mut heartbeat = time::interval(Duration::from_secs(30));
    // First tick fires immediately — skip it so the first ping is after 30s.
    heartbeat.tick().await;

    tracing::info!("websocket client connected");

    loop {
        tokio::select! {
            // Branch 1: incoming message from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(()) = handle_text_message(&text, &state, &mut sender).await {
                            break; // send failed → client gone
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!("websocket client disconnected");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        // Respond to client-initiated pings.
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {
                        // Binary/Pong from client — ignore silently
                    }
                    Some(Err(e)) => {
                        tracing::warn!(error = %e, "websocket receive error");
                        break;
                    }
                }
            }
            // Branch 2: heartbeat ping
            _ = heartbeat.tick() => {
                if sender.send(Message::Ping(Bytes::from_static(b"hb"))).await.is_err() {
                    tracing::info!("heartbeat send failed — client gone");
                    break;
                }
            }
        }
    }
}

/// Process a text message from the client.
///
/// Returns `Ok(())` if the response was sent successfully (or there was nothing to send),
/// `Err(())` if the send failed (caller should break the loop).
async fn handle_text_message(
    text: &str,
    state: &Arc<AppState>,
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
) -> Result<(), ()> {
    let parsed: ClientMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            let err_json = serde_json::json!({"error": format!("invalid JSON: {e}")});
            return send_text(sender, &err_json.to_string()).await;
        }
    };

    if parsed.action != "simulate" {
        let err_json = serde_json::json!({"error": format!("unknown action: {}", parsed.action)});
        return send_text(sender, &err_json.to_string()).await;
    }

    let source = match parsed.source {
        Some(ref s) if !s.trim().is_empty() => s.as_str(),
        _ => {
            let err_json = serde_json::json!({"error": "missing or empty source field"});
            return send_text(sender, &err_json.to_string()).await;
        }
    };

    // Try to acquire a simulation permit (non-blocking).
    let permit = match state.simulation_semaphore.try_acquire() {
        Ok(p) => p,
        Err(_) => {
            let err_json = serde_json::json!({"error": "server busy"});
            return send_text(sender, &err_json.to_string()).await;
        }
    };

    // Run the simulation pipeline.
    let result = match crate::api::run_simulation(source, parsed.repeat_count).await {
        Ok(r) => r,
        Err((_status, json_body)) => {
            let err_json = serde_json::json!({"error": json_body.0.error});
            drop(permit);
            return send_text(sender, &err_json.to_string()).await;
        }
    };

    // Stream binary frames with pacing.
    let stream_start = Instant::now();
    for event in &result.game_events {
        // Pace: sleep until the event's target time relative to stream start.
        let target = Duration::from_secs_f64(event.timestamp);
        let elapsed = stream_start.elapsed();
        if target > elapsed {
            time::sleep(target - elapsed).await;
        }

        let frame = Message::Binary(Bytes::from(event.to_bytes().to_vec()));
        if sender.send(frame).await.is_err() {
            drop(permit);
            return Err(()); // client gone
        }
    }

    // Send completion frame.
    let completion = CompletionFrame {
        msg_type: "complete",
        stats: CompletionStats {
            total_events: result.game_events.len(),
            total_gas: result.response.stats.total_gas,
            num_transactions: result.response.stats.num_transactions,
            num_conflicts: result.response.stats.num_conflicts,
            num_re_executions: result.response.stats.num_re_executions,
        },
    };
    let completion_json = serde_json::to_string(&completion).unwrap_or_default();
    let send_result = send_text(sender, &completion_json).await;

    drop(permit);
    send_result
}

/// Send a text message, mapping send errors to `Err(())`.
async fn send_text(
    sender: &mut (impl SinkExt<Message, Error = axum::Error> + Unpin),
    text: &str,
) -> Result<(), ()> {
    sender
        .send(Message::Text(text.into()))
        .await
        .map_err(|_| ())
}
