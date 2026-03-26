//! MonBeat simulation server binary.
//!
//! Starts an Axum HTTP server on PORT (default 3000) with:
//! - POST /api/simulate — Full Solidity simulation pipeline
//! - GET /health — Liveness probe with uptime
//! - CORS middleware (allow all origins for dev)
//! - tracing subscriber for structured logging

use std::sync::Arc;
use std::time::Instant;

use axum::{routing, Router};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

use monbeat_server::api;
use monbeat_server::ws;

#[tokio::main]
async fn main() {
    // Initialize structured logging (RUST_LOG env var, default info).
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    let state = Arc::new(api::AppState {
        start_time: Instant::now(),
        simulation_semaphore: tokio::sync::Semaphore::new(4),
    });

    let app = Router::new()
        .route("/api/simulate", routing::post(api::simulate))
        .route("/ws", routing::any(ws::ws_handler))
        .route("/health", routing::get(api::health))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind listener");

    tracing::info!("monbeat-server listening on 0.0.0.0:{port}");

    axum::serve(listener, app).await.expect("server error");
}
