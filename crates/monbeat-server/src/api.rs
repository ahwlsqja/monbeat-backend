//! Axum REST API handlers for the MonBeat simulation server.
//!
//! # Endpoints
//!
//! - `POST /api/simulate` — Full pipeline: Solidity source → compile → build block
//!   → parallel execute → conflict detect → game event map → JSON response.
//! - `GET /health` — Liveness probe with uptime.
//!
//! # Error Handling
//!
//! - 400 Bad Request: Compile errors (with solc stderr), missing source field
//! - 500 Internal Server Error: Engine/scheduler failures
//!
//! # Observability
//!
//! - `tracing::info!` on each simulate request with contract name, tx count, event count
//! - `tracing::warn!` on compile errors (user-facing, not system failures)
//! - `tracing::error!` on engine failures (system issue)

use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::block_builder;
use crate::compiler;
use crate::engine;
use crate::game_events::{ConflictInput, GameEvent, GameEventMapper, TxResult};

// ---------------------------------------------------------------------------
// Application state (shared across requests via Axum State extractor)
// ---------------------------------------------------------------------------

/// Shared application state injected into handlers.
pub struct AppState {
    /// Server start time for uptime calculation.
    pub start_time: Instant,
    /// Limits concurrent simulation runs (prevents overload).
    pub simulation_semaphore: tokio::sync::Semaphore,
    /// Optional PostgreSQL connection pool (graceful degradation if None).
    pub db: Option<sqlx::PgPool>,
    /// Optional Redis connection manager (graceful degradation if None).
    pub redis: Option<tokio::sync::Mutex<redis::aio::ConnectionManager>>,
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// POST /api/simulate request body.
#[derive(Deserialize)]
pub struct SimulateRequest {
    /// Raw Solidity source code to compile and simulate.
    pub source: String,
    /// How many times each state-changing function is called.
    /// `None` → auto-compute to target ~300 TXs.
    pub repeat_count: Option<u32>,
}

/// Per-transaction execution result in the response.
#[derive(Debug, Serialize, Deserialize)]
pub struct TxResultOutput {
    pub success: bool,
    pub gas_used: u64,
    pub output: String,
    pub error: Option<String>,
    pub logs_count: usize,
}

/// Execution statistics summary.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_gas: u64,
    pub num_transactions: usize,
    pub num_conflicts: usize,
    pub num_re_executions: usize,
}

/// Conflict pair in the response.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConflictPairOutput {
    pub tx_a: usize,
    pub tx_b: usize,
    pub location_type: String,
    pub conflict_type: String,
}

/// Conflict details in the response.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConflictDetailsOutput {
    pub conflicts: Vec<ConflictPairOutput>,
}

/// POST /api/simulate response body.
#[derive(Serialize, Deserialize)]
pub struct SimulateResponse {
    pub results: Vec<TxResultOutput>,
    pub incarnations: Vec<u32>,
    pub stats: ExecutionStats,
    pub conflict_details: ConflictDetailsOutput,
    #[serde(rename = "gameEvents")]
    pub game_events: Vec<GameEvent>,
}

/// GET /health response body.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub uptime_secs: u64,
    pub engine_type: String,
    pub engine_available: bool,
    pub db_connected: bool,
    pub redis_connected: bool,
    pub pool_size: u32,
    pub pool_idle: u32,
}

/// Error response body (returned as JSON for 400/500).
#[derive(Serialize)]
pub struct ErrorBody {
    pub error: String,
}

/// Query parameters for GET /api/simulations.
#[derive(Deserialize)]
pub struct SimulationsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// A persisted simulation record returned by GET /api/simulations.
#[derive(Serialize)]
pub struct SimulationRecord {
    pub id: String,
    pub source_hash: String,
    pub response: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /health — liveness probe with connection pool health.
pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let (db_connected, pool_size, pool_idle) = if let Some(pool) = &state.db {
        let connected = !pool.is_closed();
        let size = pool.size();
        let idle = pool.num_idle() as u32;
        (connected, size, idle)
    } else {
        (false, 0, 0)
    };

    let redis_connected = if let Some(redis_mtx) = &state.redis {
        let mut conn = redis_mtx.lock().await;
        let result: Result<String, redis::RedisError> = redis::cmd("PING")
            .query_async(&mut *conn)
            .await;
        result.is_ok()
    } else {
        false
    };

    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_secs: state.start_time.elapsed().as_secs(),
        engine_type: "cpp-monad-vibe-cli".to_string(),
        engine_available: engine::is_available(),
        db_connected,
        redis_connected,
        pool_size,
        pool_idle,
    })
}

/// POST /api/simulate — full simulation pipeline with caching + persistence.
pub async fn simulate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SimulateRequest>,
) -> Result<Json<SimulateResponse>, (StatusCode, Json<ErrorBody>)> {
    // Compute content hash for cache key (includes repeat_count to avoid serving stale results)
    let rc_tag = req.repeat_count.map_or("auto".to_string(), |n| n.to_string());
    let source_hash = hex::encode(Sha256::digest(req.source.as_bytes()));

    // 1. Check Redis cache
    if let Some(redis_mtx) = &state.redis {
        let cache_key = format!("sim:{source_hash}:rc:{rc_tag}");
        let cached: Result<Option<String>, redis::RedisError> = {
            let mut conn = redis_mtx.lock().await;
            redis::cmd("GET")
                .arg(&cache_key)
                .query_async(&mut *conn)
                .await
        };
        if let Ok(Some(json_str)) = cached {
            if let Ok(response) = serde_json::from_str::<SimulateResponse>(&json_str) {
                tracing::info!(source_hash = %source_hash, "cache hit");
                return Ok(Json(response));
            }
        }
    }

    // 2. Run simulation
    let result = run_simulation(&req.source, req.repeat_count).await?;

    // 3. Cache in Redis (fire-and-forget, don't fail on error)
    if let Some(redis_mtx) = &state.redis {
        let cache_key = format!("sim:{source_hash}:rc:{rc_tag}");
        if let Ok(json_str) = serde_json::to_string(&result.response) {
            let mut conn = redis_mtx.lock().await;
            let _: Result<(), redis::RedisError> = redis::cmd("SET")
                .arg(&cache_key)
                .arg(&json_str)
                .arg("EX")
                .arg(3600) // 1 hour TTL
                .query_async(&mut *conn)
                .await;
        }
    }

    // 4. Persist to PostgreSQL (fire-and-forget, don't fail on error)
    if let Some(pool) = &state.db {
        let id = uuid::Uuid::new_v4().to_string();
        if let Ok(json_val) = serde_json::to_value(&result.response) {
            let res = sqlx::query(
                "INSERT INTO simulations (id, source_hash, response, created_at) VALUES ($1, $2, $3, $4)"
            )
                .bind(&id)
                .bind(&source_hash)
                .bind(&json_val)
                .bind(chrono::Utc::now())
                .execute(pool)
                .await;
            if let Err(e) = res {
                tracing::warn!(error = %e, "failed to persist simulation — degraded mode");
            }
        }
    }

    Ok(Json(result.response))
}

/// GET /api/simulations — paginated simulation history.
pub async fn list_simulations(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SimulationsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let pool = match &state.db {
        Some(pool) => pool,
        None => {
            return Ok(Json(serde_json::json!({
                "simulations": [],
                "total": 0,
                "message": "database not connected"
            })));
        }
    };

    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0).max(0);

    let rows = sqlx::query_as::<_, (String, String, serde_json::Value, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, source_hash, response, created_at FROM simulations ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to query simulations");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody { error: "database query failed".to_string() }),
            )
        })?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM simulations")
        .fetch_one(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to count simulations");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody { error: "database query failed".to_string() }),
            )
        })?;

    let simulations: Vec<SimulationRecord> = rows
        .into_iter()
        .map(|(id, source_hash, response, created_at)| SimulationRecord {
            id,
            source_hash,
            response,
            created_at,
        })
        .collect();

    Ok(Json(serde_json::json!({
        "simulations": simulations,
        "total": total.0,
    })))
}

// ---------------------------------------------------------------------------
// Shared simulation pipeline (used by both REST and WebSocket handlers)
// ---------------------------------------------------------------------------

/// Result of a simulation run, including both the JSON-serializable response
/// and the raw game events for binary streaming.
pub struct SimulationResult {
    /// Full JSON response (same as the REST endpoint returns).
    pub response: SimulateResponse,
    /// Game events for binary streaming (same data as response.game_events,
    /// but owned separately so the WS handler can consume them without cloning).
    pub game_events: Vec<GameEvent>,
}

/// Run the full Solidity simulation pipeline:
/// compile → build block → execute via C++ monad-vibe-cli → map game events.
///
/// Shared between the REST handler and the WebSocket handler.
/// `repeat_count`: `None` auto-targets ~300 TXs; `Some(n)` repeats each function n times.
pub async fn run_simulation(source: &str, repeat_count: Option<u32>) -> Result<SimulationResult, (StatusCode, Json<ErrorBody>)> {
    // 1. Compile
    let compile_result = compiler::compile(source).map_err(|e| {
        tracing::warn!(error = %e, "compilation failed");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: e.to_string(),
            }),
        )
    })?;

    tracing::info!(
        contract = %compile_result.contract_name,
        bytecode_len = compile_result.bytecode.len(),
        "compilation succeeded"
    );

    // 2. Build transaction block
    let build_result = block_builder::build(&compile_result, repeat_count).map_err(|e| {
        tracing::warn!(error = %e, "block build failed");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: e.to_string(),
            }),
        )
    })?;

    let num_txs = build_result.transactions.len();
    tracing::info!(num_txs, "transaction block built");

    // 3. Execute via C++ monad-vibe-cli subprocess
    let engine_output = engine::execute(&build_result).await.map_err(|e| {
        tracing::error!(error = %e, "engine execution failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: format!("Engine error: {e}"),
            }),
        )
    })?;

    // 4. Convert engine output → response types
    let mut tx_result_outputs = Vec::with_capacity(engine_output.results.len());
    let mut mapper_tx_results = Vec::with_capacity(engine_output.results.len());

    for r in &engine_output.results {
        tx_result_outputs.push(TxResultOutput {
            success: r.success,
            gas_used: r.gas_used,
            output: r.output.clone(),
            error: r.error.clone(),
            logs_count: r.logs_count.unwrap_or(0),
        });
        mapper_tx_results.push(TxResult {
            success: r.success,
            gas_used: r.gas_used,
        });
    }

    // 5. Convert conflict details for game event mapper
    let conflict_inputs: Vec<ConflictInput> = engine_output
        .conflict_details
        .conflicts
        .iter()
        .map(|c| ConflictInput {
            tx_a: c.tx_a,
            tx_b: c.tx_b,
            slot_byte: 0, // C++ engine doesn't provide slot byte detail
        })
        .collect();

    let conflict_outputs: Vec<ConflictPairOutput> = engine_output
        .conflict_details
        .conflicts
        .iter()
        .map(|c| ConflictPairOutput {
            tx_a: c.tx_a,
            tx_b: c.tx_b,
            location_type: c.location_type.clone().unwrap_or_else(|| "Unknown".to_string()),
            conflict_type: c.conflict_type.clone().unwrap_or_else(|| "unknown".to_string()),
        })
        .collect();

    // 6. Map to game events
    let game_events = GameEventMapper::map_to_events(
        &mapper_tx_results,
        &engine_output.incarnations,
        &conflict_inputs,
    );

    tracing::info!(
        events = game_events.len(),
        conflicts = engine_output.stats.num_conflicts,
        re_executions = engine_output.stats.num_re_executions,
        "simulation complete (C++ engine)"
    );

    let response = SimulateResponse {
        results: tx_result_outputs,
        incarnations: engine_output.incarnations,
        stats: ExecutionStats {
            total_gas: engine_output.stats.total_gas,
            num_transactions: num_txs,
            num_conflicts: engine_output.stats.num_conflicts as usize,
            num_re_executions: engine_output.stats.num_re_executions as usize,
        },
        conflict_details: ConflictDetailsOutput {
            conflicts: conflict_outputs,
        },
        game_events: game_events.clone(),
    };

    Ok(SimulationResult {
        response,
        game_events,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

// No Rust-engine-specific helpers needed — execution delegated to C++ monad-vibe-cli via engine.rs
