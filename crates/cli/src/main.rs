//! monad-cli — JSON stdin/stdout bridge to monad-core parallel EVM engine.
//!
//! Reads a JSON block from stdin:
//! ```json
//! {
//!   "transactions": [
//!     { "sender": "0x...", "to": "0x..." | null, "data": "0x...", "value": "0", "gas_limit": 2000000, "nonce": 0, "gas_price": "1000000000" }
//!   ],
//!   "block_env": { "number": 1, "coinbase": "0x...", "timestamp": 1700000000, "gas_limit": 30000000, "base_fee": "0", "difficulty": "0" }
//! }
//! ```
//!
//! Writes a JSON result to stdout:
//! ```json
//! {
//!   "results": [ { "success": true, "gas_used": 21000, "output": "0x...", "error": null, "logs_count": 0 } ],
//!   "incarnations": [0, 1, 0],
//!   "stats": { "total_gas": 63000, "num_transactions": 3, "num_conflicts": 1, "num_re_executions": 1 }
//! }
//! ```

use std::io::{self, Read};
use std::sync::Arc;

use alloy_primitives::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};

use monad_scheduler::execute_block_parallel;
use monad_state::InMemoryState;
use monad_types::{AccountInfo, BlockEnv, ExecutionResult, Transaction};

mod conflict;
use conflict::{detect_conflicts, ConflictDetails};

// ── Input types (matching NestJS VibeScoreService output) ──

#[derive(Deserialize)]
struct CliInput {
    transactions: Vec<InputTx>,
    block_env: InputBlockEnv,
}

#[derive(Deserialize)]
struct InputTx {
    sender: String,
    to: Option<String>,
    data: String,
    value: String,
    gas_limit: u64,
    nonce: u64,
    gas_price: String,
}

#[derive(Deserialize)]
struct InputBlockEnv {
    number: u64,
    coinbase: String,
    timestamp: u64,
    gas_limit: u64,
    base_fee: String,
    difficulty: String,
}

// ── Output types (matching NestJS EngineService expected format) ──

#[derive(Serialize)]
struct CliOutput {
    results: Vec<TxResultOutput>,
    incarnations: Vec<u32>,
    stats: CliStats,
    conflict_details: ConflictDetails,
}

#[derive(Serialize)]
struct TxResultOutput {
    success: bool,
    gas_used: u64,
    output: String,
    error: Option<String>,
    logs_count: usize,
}

#[derive(Serialize)]
struct CliStats {
    total_gas: u64,
    num_transactions: usize,
    num_conflicts: usize,
    num_re_executions: usize,
}

// ── Helpers ──

fn parse_address(s: &str) -> Address {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let mut bytes = [0u8; 20];
    // Pad left with zeros if shorter than 40 hex chars
    let padded = format!("{:0>40}", s);
    hex::decode_to_slice(&padded, &mut bytes).unwrap_or_default();
    Address::from(bytes)
}

fn parse_u256(s: &str) -> U256 {
    if s.starts_with("0x") || s.starts_with("0X") {
        U256::from_str_radix(s.strip_prefix("0x").unwrap_or(s.strip_prefix("0X").unwrap_or(s)), 16)
            .unwrap_or(U256::ZERO)
    } else {
        U256::from_str_radix(s, 10).unwrap_or(U256::ZERO)
    }
}

fn parse_bytes(s: &str) -> Bytes {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.is_empty() {
        return Bytes::new();
    }
    Bytes::from(hex::decode(s).unwrap_or_default())
}

fn main() {
    // Read all of stdin
    let mut input_str = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input_str) {
        eprintln!("Failed to read stdin: {}", e);
        std::process::exit(1);
    }

    // Parse input JSON
    let input: CliInput = match serde_json::from_str(&input_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse input JSON: {}", e);
            std::process::exit(1);
        }
    };

    // Convert transactions
    let transactions: Vec<Transaction> = input
        .transactions
        .iter()
        .map(|tx| Transaction {
            sender: parse_address(&tx.sender),
            to: tx.to.as_ref().map(|s| parse_address(s)),
            value: parse_u256(&tx.value),
            data: parse_bytes(&tx.data),
            gas_limit: tx.gas_limit,
            nonce: tx.nonce,
            gas_price: parse_u256(&tx.gas_price),
        })
        .collect();

    // Convert block env
    let block_env = BlockEnv {
        number: input.block_env.number,
        coinbase: parse_address(&input.block_env.coinbase),
        timestamp: input.block_env.timestamp,
        gas_limit: input.block_env.gas_limit,
        base_fee: parse_u256(&input.block_env.base_fee),
        difficulty: parse_u256(&input.block_env.difficulty),
    };

    // Build pre-funded state: all unique senders get 1000 ETH + high nonce allowance
    let mut state = InMemoryState::new();
    let prefund = U256::from(1_000_000_000_000_000_000_000u128); // 1000 ETH
    for tx in &transactions {
        state.insert_account(
            tx.sender,
            AccountInfo::new(prefund, 0),
        );
    }

    // Execute in parallel via Block-STM
    let par_result = execute_block_parallel(
        &transactions,
        Arc::new(state),
        &block_env,
        4, // worker threads
    );

    // Detect conflicts from ReadSet/WriteSet data
    let conflict_details = detect_conflicts(&par_result.tx_results);

    // Convert results to output format
    let mut total_gas = 0u64;
    let results: Vec<TxResultOutput> = par_result
        .tx_results
        .iter()
        .map(|(exec_result, _write_set, _read_set)| {
            let (success, gas_used, output, error, logs_count) = match exec_result {
                ExecutionResult::Success {
                    gas_used,
                    output,
                    logs,
                } => {
                    total_gas += gas_used;
                    (true, *gas_used, format!("0x{}", hex::encode(output)), None, logs.len())
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
            TxResultOutput {
                success,
                gas_used,
                output,
                error,
                logs_count,
            }
        })
        .collect();

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

    let output = CliOutput {
        results,
        incarnations: par_result.incarnations,
        stats: CliStats {
            total_gas,
            num_transactions: transactions.len(),
            num_conflicts,
            num_re_executions,
        },
        conflict_details,
    };

    // Write output JSON to stdout
    match serde_json::to_string(&output) {
        Ok(json) => println!("{}", json),
        Err(e) => {
            eprintln!("Failed to serialize output: {}", e);
            std::process::exit(1);
        }
    }
}
