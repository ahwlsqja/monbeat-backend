//! MonBeat simulation server binary.
//!
//! Starts an Axum HTTP server on PORT (default 3000) with:
//! - POST /api/simulate — Full Solidity simulation pipeline (with Redis cache + PG persist)
//! - GET /api/simulations — Paginated simulation history from PostgreSQL
//! - GET /health — Liveness probe with uptime and connection pool health
//! - WebSocket /ws — Binary event streaming
//! - CORS middleware (allow all origins for dev)
//! - tracing subscriber for structured logging
//!
//! PostgreSQL and Redis are optional — the server degrades gracefully when
//! DATABASE_URL / REDIS_URL are not set.

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

    // Optional PostgreSQL pool (with 5s connect timeout)
    let db = match std::env::var("DATABASE_URL") {
        Ok(url) => {
            match sqlx::postgres::PgPoolOptions::new()
                .max_connections(10)
                .acquire_timeout(std::time::Duration::from_secs(5))
                .connect(&url)
                .await
            {
                Ok(pool) => {
                    tracing::info!("PostgreSQL connected");
                    // Run migrations — execute each statement separately
                    // (sqlx::query doesn't support multi-statement execution)
                    let migration_stmts = [
                        "CREATE TABLE IF NOT EXISTS simulations (
                            id          TEXT PRIMARY KEY,
                            source_hash TEXT NOT NULL,
                            response    JSONB NOT NULL,
                            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
                        )",
                        "CREATE INDEX IF NOT EXISTS idx_simulations_created_at ON simulations (created_at DESC)",
                        "CREATE INDEX IF NOT EXISTS idx_simulations_source_hash ON simulations (source_hash)",
                    ];
                    for stmt in &migration_stmts {
                        if let Err(e) = sqlx::query(stmt).execute(&pool).await {
                            tracing::warn!(error = %e, "migration statement failed");
                        }
                    }
                    Some(pool)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "PostgreSQL connection failed — running without persistence");
                    None
                }
            }
        }
        Err(_) => {
            tracing::info!("DATABASE_URL not set — running without persistence");
            None
        }
    };

    // Optional Redis connection (with 5s timeout)
    let redis = match std::env::var("REDIS_URL") {
        Ok(url) => {
            match redis::Client::open(url.as_str()) {
                Ok(client) => {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        redis::aio::ConnectionManager::new(client),
                    )
                    .await
                    {
                        Ok(Ok(mgr)) => {
                            tracing::info!("Redis connected");
                            Some(tokio::sync::Mutex::new(mgr))
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(error = %e, "Redis connection failed — running without cache");
                            None
                        }
                        Err(_) => {
                            tracing::warn!("Redis connection timed out (5s) — running without cache");
                            None
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Redis client creation failed — running without cache");
                    None
                }
            }
        }
        Err(_) => {
            tracing::info!("REDIS_URL not set — running without cache");
            None
        }
    };

    let state = Arc::new(api::AppState {
        start_time: Instant::now(),
        simulation_semaphore: tokio::sync::Semaphore::new(4),
        db,
        redis,
    });

    let app = Router::new()
        .route("/api/simulate", routing::post(api::simulate))
        .route("/api/simulations", routing::get(api::list_simulations))
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
