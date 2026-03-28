//! C++ monad-vibe-cli subprocess execution engine.
//!
//! Replaces the Rust-native `execute_block_parallel()` with a subprocess call
//! to the official C++ Monad engine (`monad-vibe-cli`), ensuring simulation
//! results match mainnet behavior exactly.
//!
//! # Pipeline
//!
//! 1. Convert `BuildResult` (transactions + block_env) → CLI JSON input
//! 2. Spawn `monad-vibe-cli` subprocess, pipe JSON via stdin
//! 3. Parse JSON stdout → `EngineOutput`
//! 4. Caller (api.rs) maps `EngineOutput` to `SimulateResponse`
//!
//! # Error Handling
//!
//! - Binary not found → `EngineError::BinaryNotFound`
//! - Non-zero exit → `EngineError::ProcessFailed` with stderr
//! - JSON parse failure → `EngineError::OutputParse`

use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;

use crate::block_builder::BuildResult;

/// Default binary name (overridable via MONAD_VIBE_CLI env var).
const DEFAULT_BINARY: &str = "monad-vibe-cli";

// ---------------------------------------------------------------------------
// CLI Input types (matches monad-vibe-cli stdin JSON schema)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct CliInput {
    transactions: Vec<CliTx>,
    block_env: CliBlockEnv,
}

#[derive(Debug, Serialize)]
struct CliTx {
    sender: String,
    to: serde_json::Value, // null for contract creation
    data: String,
    value: String,
    gas_limit: u64,
    nonce: u64,
    gas_price: String,
}

#[derive(Debug, Serialize)]
struct CliBlockEnv {
    number: u64,
    coinbase: String,
    timestamp: u64,
    gas_limit: u64,
    base_fee: String,
    difficulty: String,
}

// ---------------------------------------------------------------------------
// CLI Output types (matches monad-vibe-cli stdout JSON schema)
// ---------------------------------------------------------------------------

/// Parsed output from monad-vibe-cli stdout.
#[derive(Debug, Deserialize)]
pub struct EngineOutput {
    pub results: Vec<EngineResult>,
    pub incarnations: Vec<u32>,
    pub stats: EngineStats,
    pub conflict_details: EngineConflictDetails,
}

#[derive(Debug, Deserialize)]
pub struct EngineResult {
    pub success: bool,
    pub gas_used: u64,
    pub output: String,
    pub error: Option<String>,
    pub logs_count: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct EngineStats {
    pub total_gas: u64,
    pub num_transactions: usize,
    pub num_conflicts: u32,
    pub num_re_executions: u32,
    #[serde(default)]
    pub per_tx_exec_time_us: Vec<u64>,
}

#[derive(Debug, Deserialize)]
pub struct EngineConflictDetails {
    #[serde(default)]
    pub per_tx: Vec<serde_json::Value>,
    #[serde(default)]
    pub conflicts: Vec<EngineConflict>,
}

#[derive(Debug, Deserialize)]
pub struct EngineConflict {
    pub tx_a: usize,
    pub tx_b: usize,
    #[serde(default)]
    pub location_type: Option<String>,
    #[serde(default)]
    pub conflict_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("monad-vibe-cli binary not found: {0}")]
    BinaryNotFound(String),

    #[error("monad-vibe-cli process failed (exit {exit_code}): {stderr}")]
    ProcessFailed { exit_code: i32, stderr: String },

    #[error("monad-vibe-cli was killed by signal")]
    Killed,

    #[error("failed to parse monad-vibe-cli output: {0}")]
    OutputParse(String),

    #[error("failed to serialize input: {0}")]
    InputSerialize(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check if monad-vibe-cli is available (native binary or Docker image).
pub fn is_available() -> bool {
    which_binary().is_some() || has_docker_image()
}

/// Execute transactions via monad-vibe-cli subprocess.
///
/// Tries native binary first, falls back to Docker if not available.
/// Converts `BuildResult` to CLI JSON input, spawns the process, and parses
/// the JSON output. Returns `EngineOutput` for further mapping.
pub async fn execute(build_result: &BuildResult) -> Result<EngineOutput, EngineError> {
    // Convert BuildResult → CLI JSON input
    let cli_input = build_cli_input(build_result);
    let input_json = serde_json::to_string(&cli_input)
        .map_err(|e| EngineError::InputSerialize(e.to_string()))?;

    let start = std::time::Instant::now();

    // Choose execution mode: native binary or Docker
    let (mode, mut child) = if let Some(binary) = which_binary() {
        tracing::info!(
            binary = %binary,
            num_txs = build_result.transactions.len(),
            "spawning monad-vibe-cli (native)"
        );
        let child = Command::new(&binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        ("native", child)
    } else if has_docker_image() {
        let image = docker_image_name();
        tracing::info!(
            image = %image,
            num_txs = build_result.transactions.len(),
            "spawning monad-vibe-cli (docker)"
        );
        let child = Command::new("docker")
            .args(["run", "--rm", "-i", &image, "monad-vibe-cli"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        ("docker", child)
    } else {
        return Err(EngineError::BinaryNotFound(
            "monad-vibe-cli not found (checked PATH and Docker)".to_string(),
        ));
    };

    // Write input to stdin
    {
        use tokio::io::AsyncWriteExt;
        let stdin = child.stdin.as_mut().expect("stdin piped");
        stdin.write_all(input_json.as_bytes()).await?;
        stdin.shutdown().await?;
    }

    // Wait for process to complete
    let output = child.wait_with_output().await?;
    let elapsed = start.elapsed();

    tracing::info!(
        mode = mode,
        exit_code = output.status.code().unwrap_or(-1),
        elapsed_ms = elapsed.as_millis() as u64,
        stdout_bytes = output.stdout.len(),
        stderr_bytes = output.stderr.len(),
        "monad-vibe-cli completed"
    );

    // Check exit code
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        tracing::error!(exit_code, stderr = %stderr, "monad-vibe-cli failed");
        if exit_code == -1 {
            return Err(EngineError::Killed);
        }
        return Err(EngineError::ProcessFailed { exit_code, stderr });
    }

    // Parse stdout JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    let engine_output: EngineOutput = serde_json::from_str(&stdout)
        .map_err(|e| {
            tracing::error!(
                error = %e,
                stdout_preview = &stdout[..stdout.len().min(500)],
                "failed to parse monad-vibe-cli output"
            );
            EngineError::OutputParse(e.to_string())
        })?;

    Ok(engine_output)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn binary_name() -> String {
    std::env::var("MONAD_VIBE_CLI").unwrap_or_else(|_| DEFAULT_BINARY.to_string())
}

fn docker_image_name() -> String {
    std::env::var("MONAD_VIBE_CLI_IMAGE").unwrap_or_else(|_| "monad-vibe-cli:latest".to_string())
}

fn has_docker_image() -> bool {
    let image = docker_image_name();
    if let Ok(output) = std::process::Command::new("docker")
        .args(["image", "inspect", &image])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
    {
        return output.status.success();
    }
    false
}

fn which_binary() -> Option<String> {
    let name = binary_name();
    // Check if it's an absolute path that exists
    if std::path::Path::new(&name).is_absolute() && std::path::Path::new(&name).exists() {
        return Some(name);
    }
    // Check PATH
    if let Ok(output) = std::process::Command::new("which")
        .arg(&name)
        .output()
    {
        if output.status.success() {
            return Some(name);
        }
    }
    None
}

/// Convert `BuildResult` to the JSON structure expected by monad-vibe-cli.
fn build_cli_input(build_result: &BuildResult) -> CliInput {
    let transactions = build_result
        .transactions
        .iter()
        .map(|tx| CliTx {
            sender: format!("{:#x}", tx.sender),
            to: match &tx.to {
                Some(addr) => serde_json::Value::String(format!("{:#x}", addr)),
                None => serde_json::Value::Null,
            },
            data: format!("0x{}", hex::encode(&tx.data)),
            value: tx.value.to_string(),
            gas_limit: tx.gas_limit,
            nonce: tx.nonce,
            gas_price: tx.gas_price.to_string(),
        })
        .collect();

    let env = &build_result.block_env;
    let block_env = CliBlockEnv {
        number: env.number,
        coinbase: format!("{:#x}", env.coinbase),
        timestamp: env.timestamp,
        gas_limit: env.gas_limit,
        base_fee: env.base_fee.to_string(),
        difficulty: env.difficulty.to_string(),
    };

    CliInput {
        transactions,
        block_env,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, Bytes, U256};
    use monad_types::{BlockEnv, Transaction};
    use std::collections::HashMap;

    fn sample_build_result() -> BuildResult {
        BuildResult {
            transactions: vec![
                Transaction {
                    sender: Address::with_last_byte(0xE1),
                    to: None,
                    data: Bytes::from(vec![0x60, 0x80, 0x60, 0x40]),
                    value: U256::ZERO,
                    gas_limit: 2_000_000,
                    nonce: 0,
                    gas_price: U256::from(1_000_000_000u64),
                },
                Transaction {
                    sender: Address::with_last_byte(0xE2),
                    to: Some(Address::with_last_byte(0xAA)),
                    data: Bytes::from(vec![0xd0, 0x9d, 0xe0, 0x8a]),
                    value: U256::ZERO,
                    gas_limit: 2_000_000,
                    nonce: 0,
                    gas_price: U256::from(1_000_000_000u64),
                },
            ],
            block_env: BlockEnv {
                number: 1,
                coinbase: Address::with_last_byte(0xFF),
                timestamp: 1_700_000_000,
                gas_limit: 30_000_000,
                base_fee: U256::ZERO,
                difficulty: U256::ZERO,
            },
            tx_function_map: HashMap::from([(0, "constructor".to_string()), (1, "increment".to_string())]),
        }
    }

    #[test]
    fn test_build_cli_input_format() {
        let build = sample_build_result();
        let cli_input = build_cli_input(&build);

        assert_eq!(cli_input.transactions.len(), 2);

        // Deploy tx: to should be null
        assert_eq!(cli_input.transactions[0].to, serde_json::Value::Null);
        assert!(cli_input.transactions[0].sender.contains("e1"));

        // Call tx: to should be address string
        assert!(cli_input.transactions[1].to.is_string());
        assert!(cli_input.transactions[1].sender.contains("e2"));

        // Block env
        assert_eq!(cli_input.block_env.number, 1);
        assert!(cli_input.block_env.coinbase.contains("ff"));
        assert_eq!(cli_input.block_env.timestamp, 1_700_000_000);
    }

    #[test]
    fn test_build_cli_input_serializes_to_json() {
        let build = sample_build_result();
        let cli_input = build_cli_input(&build);
        let json = serde_json::to_string(&cli_input).unwrap();

        // Verify it's valid JSON with expected fields
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["transactions"].is_array());
        assert!(parsed["block_env"].is_object());
        assert_eq!(parsed["transactions"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_engine_output_deserialize() {
        let json = r#"{
            "results": [
                {"success": true, "gas_used": 100000, "output": "0x", "error": null, "logs_count": 0}
            ],
            "incarnations": [0],
            "stats": {
                "total_gas": 100000,
                "num_transactions": 1,
                "num_conflicts": 0,
                "num_re_executions": 0
            },
            "conflict_details": {
                "per_tx": [],
                "conflicts": []
            }
        }"#;

        let output: EngineOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.results.len(), 1);
        assert!(output.results[0].success);
        assert_eq!(output.incarnations, vec![0]);
        assert_eq!(output.stats.num_transactions, 1);
    }

    #[test]
    fn test_engine_output_with_conflicts() {
        let json = r#"{
            "results": [
                {"success": true, "gas_used": 50000, "output": "0x", "error": null, "logs_count": 0},
                {"success": true, "gas_used": 50000, "output": "0x", "error": null, "logs_count": 0}
            ],
            "incarnations": [0, 1],
            "stats": {
                "total_gas": 100000,
                "num_transactions": 2,
                "num_conflicts": 1,
                "num_re_executions": 1,
                "per_tx_exec_time_us": [120, 340]
            },
            "conflict_details": {
                "per_tx": [
                    {"incarnation_count": 1, "read_set_size": 3, "write_set_size": 2},
                    {"incarnation_count": 2, "read_set_size": 4, "write_set_size": 1}
                ],
                "conflicts": [
                    {"tx_a": 0, "tx_b": 1, "location_type": "StorageSlot", "conflict_type": "read-write"}
                ]
            }
        }"#;

        let output: EngineOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.incarnations, vec![0, 1]);
        assert_eq!(output.stats.num_conflicts, 1);
        assert_eq!(output.conflict_details.conflicts.len(), 1);
        assert_eq!(output.conflict_details.conflicts[0].tx_a, 0);
        assert_eq!(output.conflict_details.conflicts[0].tx_b, 1);
    }
}
