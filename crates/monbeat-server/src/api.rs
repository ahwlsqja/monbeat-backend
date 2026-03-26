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

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use monad_mv_state::{LocationKey, ReadSet, WriteSet};
use monad_scheduler::execute_block_parallel;
use monad_state::InMemoryState;
use monad_types::{AccountInfo, ExecutionResult, Transaction};

use alloy_primitives::U256;

use crate::block_builder;
use crate::compiler;
use crate::game_events::{ConflictInput, GameEvent, GameEventMapper, TxResult};

// ---------------------------------------------------------------------------
// Application state (shared across requests via Axum State extractor)
// ---------------------------------------------------------------------------

/// Shared application state injected into handlers.
pub struct AppState {
    /// Server start time for uptime calculation.
    pub start_time: Instant,
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// POST /api/simulate request body.
#[derive(Deserialize)]
pub struct SimulateRequest {
    /// Raw Solidity source code to compile and simulate.
    pub source: String,
}

/// Per-transaction execution result in the response.
#[derive(Debug, Serialize)]
pub struct TxResultOutput {
    pub success: bool,
    pub gas_used: u64,
    pub output: String,
    pub error: Option<String>,
    pub logs_count: usize,
}

/// Execution statistics summary.
#[derive(Debug, Serialize)]
pub struct ExecutionStats {
    pub total_gas: u64,
    pub num_transactions: usize,
    pub num_conflicts: usize,
    pub num_re_executions: usize,
}

/// Conflict pair in the response.
#[derive(Debug, Serialize)]
pub struct ConflictPairOutput {
    pub tx_a: usize,
    pub tx_b: usize,
    pub location_type: String,
    pub conflict_type: String,
}

/// Conflict details in the response.
#[derive(Debug, Serialize)]
pub struct ConflictDetailsOutput {
    pub conflicts: Vec<ConflictPairOutput>,
}

/// POST /api/simulate response body.
#[derive(Serialize)]
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
}

/// Error response body (returned as JSON for 400/500).
#[derive(Serialize)]
pub struct ErrorBody {
    pub error: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /health — liveness probe.
pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_secs: state.start_time.elapsed().as_secs(),
    })
}

/// POST /api/simulate — full simulation pipeline.
pub async fn simulate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SimulateRequest>,
) -> Result<Json<SimulateResponse>, (StatusCode, Json<ErrorBody>)> {
    let _ = state; // AppState available for future use (rate limiting, etc.)

    // 1. Compile
    let compile_result = compiler::compile(&req.source).map_err(|e| {
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
    let build_result = block_builder::build(&compile_result).map_err(|e| {
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

    // 3. Set up pre-funded in-memory state
    let base_state = build_prefunded_state(&build_result.transactions);

    // 4. Execute in parallel via Block-STM
    let par_result = execute_block_parallel(
        &build_result.transactions,
        base_state,
        &build_result.block_env,
        4, // worker threads
    );

    // 5. Detect conflicts from ReadSet/WriteSet
    let (conflict_details, conflict_inputs) =
        detect_conflicts_from_results(&par_result.tx_results, &build_result.block_env.coinbase);

    // 6. Build per-tx results for response + mapper input
    let mut total_gas = 0u64;
    let mut tx_result_outputs = Vec::with_capacity(par_result.tx_results.len());
    let mut mapper_tx_results = Vec::with_capacity(par_result.tx_results.len());

    for (exec_result, _write_set, _read_set) in &par_result.tx_results {
        let (success, gas_used, output, error, logs_count) = match exec_result {
            ExecutionResult::Success {
                gas_used,
                output,
                logs,
            } => {
                total_gas += gas_used;
                (
                    true,
                    *gas_used,
                    format!("0x{}", hex::encode(output)),
                    None,
                    logs.len(),
                )
            }
            ExecutionResult::Revert { gas_used, output } => {
                total_gas += gas_used;
                (
                    false,
                    *gas_used,
                    format!("0x{}", hex::encode(output)),
                    Some("revert".to_string()),
                    0,
                )
            }
            ExecutionResult::Halt { gas_used, reason } => {
                total_gas += gas_used;
                (false, *gas_used, "0x".to_string(), Some(reason.clone()), 0)
            }
        };

        tx_result_outputs.push(TxResultOutput {
            success,
            gas_used,
            output,
            error,
            logs_count,
        });
        mapper_tx_results.push(TxResult { success, gas_used });
    }

    // 7. Map to game events
    let game_events = GameEventMapper::map_to_events(
        &mapper_tx_results,
        &par_result.incarnations,
        &conflict_inputs,
    );

    let num_conflicts = par_result
        .incarnations
        .iter()
        .filter(|&&inc| inc > 0)
        .count();
    let num_re_executions: usize = par_result
        .incarnations
        .iter()
        .map(|&inc| if inc > 0 { inc as usize } else { 0 })
        .sum();

    tracing::info!(
        events = game_events.len(),
        conflicts = conflict_details.conflicts.len(),
        re_executions = num_re_executions,
        "simulation complete"
    );

    Ok(Json(SimulateResponse {
        results: tx_result_outputs,
        incarnations: par_result.incarnations,
        stats: ExecutionStats {
            total_gas,
            num_transactions: num_txs,
            num_conflicts,
            num_re_executions,
        },
        conflict_details,
        game_events,
    }))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build an in-memory state provider with all unique senders pre-funded.
///
/// Each sender gets 1000 ETH (same as CLI) so contract deployment and
/// function calls don't fail due to insufficient balance.
fn build_prefunded_state(transactions: &[Transaction]) -> Arc<dyn monad_state::StateProvider> {
    let mut state = InMemoryState::new();
    let prefund = U256::from(1_000_000_000_000_000_000_000u128); // 1000 ETH

    // Also pre-fund the coinbase address (block_env.coinbase = 0xFF) to
    // avoid account-not-found issues during gas fee processing.
    let coinbase = alloy_primitives::Address::with_last_byte(0xFF);
    state.insert_account(coinbase, AccountInfo::new(U256::ZERO, 0));

    for tx in transactions {
        state.insert_account(tx.sender, AccountInfo::new(prefund, 0));
    }

    Arc::new(state)
}

/// Detect conflicts from parallel execution results.
///
/// Returns both the serializable conflict details (for the JSON response)
/// and the ConflictInput list (for the game event mapper).
///
/// Filters out coinbase-address conflicts (EVM-inherent, not actionable).
fn detect_conflicts_from_results(
    tx_results: &[(ExecutionResult, WriteSet, ReadSet)],
    coinbase: &alloy_primitives::Address,
) -> (ConflictDetailsOutput, Vec<ConflictInput>) {
    let mut conflict_outputs = Vec::new();
    let mut conflict_inputs = Vec::new();
    let mut seen_pairs: HashSet<(usize, usize)> = HashSet::new();

    for tx_a in 0..tx_results.len() {
        for tx_b in (tx_a + 1)..tx_results.len() {
            let (_, write_set_a, read_set_a) = &tx_results[tx_a];
            let (_, write_set_b, read_set_b) = &tx_results[tx_b];

            let write_keys_a: HashSet<LocationKey> =
                write_set_a.iter().map(|(k, _)| k.clone()).collect();
            let write_keys_b: HashSet<LocationKey> =
                write_set_b.iter().map(|(k, _)| k.clone()).collect();
            let read_keys_a: HashSet<LocationKey> =
                read_set_a.iter().map(|(k, _)| k.clone()).collect();
            let read_keys_b: HashSet<LocationKey> =
                read_set_b.iter().map(|(k, _)| k.clone()).collect();

            // Check all conflict types
            let mut pair_conflicts: Vec<(LocationKey, &str)> = Vec::new();

            // Write-write
            for key in write_keys_a.intersection(&write_keys_b) {
                pair_conflicts.push((key.clone(), "write-write"));
            }
            // Read-write: a reads, b writes
            for key in read_keys_a.intersection(&write_keys_b) {
                pair_conflicts.push((key.clone(), "read-write"));
            }
            // Read-write: a writes, b reads
            for key in write_keys_a.intersection(&read_keys_b) {
                pair_conflicts.push((key.clone(), "read-write"));
            }

            // Filter coinbase conflicts and emit
            for (key, conflict_type) in pair_conflicts {
                if is_coinbase_location(&key, coinbase) {
                    continue;
                }

                let (location_type, slot_byte) = location_info(&key);

                conflict_outputs.push(ConflictPairOutput {
                    tx_a,
                    tx_b,
                    location_type,
                    conflict_type: conflict_type.to_string(),
                });

                // Dedup for game event mapper (one conflict per pair)
                let pair_key = (tx_a, tx_b);
                if seen_pairs.insert(pair_key) {
                    conflict_inputs.push(ConflictInput {
                        tx_a,
                        tx_b,
                        slot_byte,
                    });
                }
            }
        }
    }

    (
        ConflictDetailsOutput {
            conflicts: conflict_outputs,
        },
        conflict_inputs,
    )
}

/// Check if a location key involves the coinbase address.
fn is_coinbase_location(key: &LocationKey, coinbase: &alloy_primitives::Address) -> bool {
    match key {
        LocationKey::Balance(addr) | LocationKey::Nonce(addr) | LocationKey::CodeHash(addr) => {
            addr == coinbase
        }
        LocationKey::Storage(addr, _) => addr == coinbase,
    }
}

/// Extract location type string and low slot byte from a LocationKey.
fn location_info(key: &LocationKey) -> (String, u8) {
    match key {
        LocationKey::Storage(_, slot) => {
            let slot_bytes: [u8; 32] = slot.to_be_bytes();
            ("Storage".to_string(), slot_bytes[31])
        }
        LocationKey::Balance(_) => ("Balance".to_string(), 0),
        LocationKey::Nonce(_) => ("Nonce".to_string(), 0),
        LocationKey::CodeHash(_) => ("CodeHash".to_string(), 0),
    }
}
