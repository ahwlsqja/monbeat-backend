//! Transaction block construction from compiled Solidity contracts.
//!
//! Analyzes the ABI for state-changing functions, then constructs a block of
//! transactions: 1 deploy tx + N call txs with rotating senders. This mirrors
//! the NestJS VibeScoreService `constructTransactionBlock` pattern.
//!
//! # Sender Rotation
//!
//! 8 pre-funded sender addresses (0xE1..0xE8) rotate across call transactions
//! to create realistic multi-sender execution patterns for conflict detection.
//!
//! # TxFunctionMap
//!
//! A mapping from transaction index to function name, used by downstream
//! game event mapping and conflict analysis.
//!
//! # Observability
//!
//! - `BuildResult.tx_function_map` tracks which function each tx calls
//! - `BuildError` variants distinguish ABI parsing failures from encoding errors

use std::collections::HashMap;

use alloy_json_abi::{JsonAbi, StateMutability};
use alloy_primitives::{Address, Bytes, U256};
use monad_types::{BlockEnv, Transaction};
use thiserror::Error;

use crate::compiler::CompileResult;

/// Errors during transaction block construction.
#[derive(Debug, Error)]
pub enum BuildError {
    #[error("failed to parse ABI: {0}")]
    AbiParse(String),

    #[error("no state-changing functions found in contract")]
    NoStateChangingFunctions,
}

/// Result of building a transaction block.
#[derive(Debug)]
pub struct BuildResult {
    /// Ordered transactions: tx[0] = deploy, tx[1..] = function calls.
    pub transactions: Vec<Transaction>,
    /// Block environment for execution.
    pub block_env: BlockEnv,
    /// Map from tx index → function name ("constructor" for deploy tx).
    pub tx_function_map: HashMap<usize, String>,
}

/// The 8 rotating sender addresses used for call transactions.
/// Matching the NestJS VibeScoreService pattern (0xE1..0xE8 range).
const SENDER_ADDRESSES: [Address; 8] = [
    Address::new([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0xE1,
    ]),
    Address::new([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0xE2,
    ]),
    Address::new([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0xE3,
    ]),
    Address::new([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0xE4,
    ]),
    Address::new([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0xE5,
    ]),
    Address::new([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0xE6,
    ]),
    Address::new([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0xE7,
    ]),
    Address::new([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0xE8,
    ]),
];

/// Deployer address (sender of tx[0] = contract creation).
const DEPLOYER: Address = SENDER_ADDRESSES[0];

/// Default gas limit per transaction (2M — sufficient for most contracts).
const GAS_LIMIT: u64 = 2_000_000;

/// Default gas price (1 gwei).
fn default_gas_price() -> U256 {
    U256::from(1_000_000_000u64)
}

/// Compute the CREATE address for a contract deployed by `sender` with `nonce`.
///
/// Uses the standard Ethereum CREATE formula: keccak256(rlp([sender, nonce]))[12..].
fn compute_create_address(sender: Address, nonce: u64) -> Address {
    // RLP encode [sender, nonce]
    // sender is 20 bytes → RLP: 0x94 ++ sender (21 bytes)
    // nonce encoding depends on value
    let nonce_rlp = rlp_encode_u64(nonce);
    let list_len = 21 + nonce_rlp.len();

    let mut buf = Vec::with_capacity(1 + list_len_prefix_size(list_len) + list_len);

    // List prefix
    if list_len < 56 {
        buf.push(0xc0 + list_len as u8);
    } else {
        let len_bytes = min_bytes_u64(list_len as u64);
        buf.push(0xf7 + len_bytes.len() as u8);
        buf.extend_from_slice(&len_bytes);
    }

    // sender (20 bytes, RLP string)
    buf.push(0x94); // 0x80 + 20
    buf.extend_from_slice(sender.as_slice());

    // nonce
    buf.extend_from_slice(&nonce_rlp);

    // keccak256 and take last 20 bytes
    let hash = alloy_primitives::keccak256(&buf);
    Address::from_slice(&hash[12..])
}

fn rlp_encode_u64(val: u64) -> Vec<u8> {
    if val == 0 {
        vec![0x80] // empty string
    } else if val < 128 {
        vec![val as u8]
    } else {
        let bytes = min_bytes_u64(val);
        let mut out = Vec::with_capacity(1 + bytes.len());
        out.push(0x80 + bytes.len() as u8);
        out.extend_from_slice(&bytes);
        out
    }
}

fn min_bytes_u64(val: u64) -> Vec<u8> {
    let bytes = val.to_be_bytes();
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(7);
    bytes[start..].to_vec()
}

fn list_len_prefix_size(len: usize) -> usize {
    if len < 56 {
        1
    } else {
        1 + min_bytes_u64(len as u64).len()
    }
}

/// Encode a function call with default arguments.
///
/// For each parameter type, generates a zero/default value:
/// - uint/int variants → 0 (or 1 for amounts to be non-trivial)
/// - address → sender address
/// - bool → true
/// - bytes/string → empty
/// - arrays → empty
///
/// Returns the 4-byte selector + ABI-encoded arguments.
fn encode_function_call(func: &alloy_json_abi::Function, sender: Address) -> Option<Vec<u8>> {
    let selector = func.selector();

    // ABI-encode each parameter as a 32-byte word
    let mut calldata = selector.to_vec();

    for param in &func.inputs {
        let word = default_abi_word(&param.ty, sender);
        calldata.extend_from_slice(&word);
    }

    Some(calldata)
}

/// Generate a default 32-byte ABI word for a Solidity type.
///
/// This is intentionally simple — we're generating test transactions, not
/// production calldata. Complex types (tuples, nested arrays) get zero-filled.
fn default_abi_word(ty: &str, sender: Address) -> [u8; 32] {
    let mut word = [0u8; 32];

    if ty == "address" {
        // Left-pad address to 32 bytes
        word[12..].copy_from_slice(sender.as_slice());
    } else if ty == "bool" {
        word[31] = 1; // true
    } else if ty.starts_with("uint") || ty.starts_with("int") {
        // Use 1 as default for amounts (avoids zero-value edge cases)
        word[31] = 1;
    }
    // bytes, string, arrays, tuples → all zeros (valid empty encoding for fixed types)

    word
}

/// Build a transaction block from a compiled contract.
///
/// Creates:
/// - tx[0]: Deploy the contract (sender = DEPLOYER, nonce = 0)
/// - tx[1..N]: Call each state-changing function with rotating senders
///
/// State-changing functions are those with mutability `NonPayable` or `Payable`
/// (i.e., not `Pure` or `View`).
///
/// # Errors
///
/// Returns `BuildError::AbiParse` if the ABI JSON is malformed, or
/// `BuildError::NoStateChangingFunctions` if the contract has no callable
/// state-changing functions.
pub fn build(compile_result: &CompileResult) -> Result<BuildResult, BuildError> {
    // Parse ABI
    let abi: JsonAbi = serde_json::from_str(&compile_result.abi_json)
        .map_err(|e| BuildError::AbiParse(format!("{e}")))?;

    // Decode bytecode
    let bytecode_bytes = hex::decode(&compile_result.bytecode)
        .map_err(|e| BuildError::AbiParse(format!("invalid bytecode hex: {e}")))?;

    // Find state-changing functions (not view/pure)
    let state_changing_fns: Vec<&alloy_json_abi::Function> = abi
        .functions()
        .filter(|f| {
            matches!(
                f.state_mutability,
                StateMutability::NonPayable | StateMutability::Payable
            )
        })
        .collect();

    if state_changing_fns.is_empty() {
        return Err(BuildError::NoStateChangingFunctions);
    }

    let mut transactions = Vec::new();
    let mut tx_function_map = HashMap::new();

    // tx[0]: Deploy transaction
    transactions.push(Transaction {
        sender: DEPLOYER,
        to: None,
        value: U256::ZERO,
        data: Bytes::from(bytecode_bytes),
        gas_limit: GAS_LIMIT,
        nonce: 0,
        gas_price: default_gas_price(),
    });
    tx_function_map.insert(0, "constructor".to_string());

    // Compute deployed contract address (DEPLOYER nonce=0)
    let contract_addr = compute_create_address(DEPLOYER, 0);

    // tx[1..N]: Call each state-changing function with rotating senders
    let mut sender_idx = 1usize; // Start from sender[1] for calls (sender[0] = deployer)
    let mut nonce_map: HashMap<Address, u64> = HashMap::new();
    // Deployer already used nonce 0 for deploy
    nonce_map.insert(DEPLOYER, 1);

    for func in &state_changing_fns {
        let sender = SENDER_ADDRESSES[sender_idx % SENDER_ADDRESSES.len()];
        let nonce = nonce_map.entry(sender).or_insert(0);

        if let Some(calldata) = encode_function_call(func, sender) {
            let tx_idx = transactions.len();
            transactions.push(Transaction {
                sender,
                to: Some(contract_addr),
                value: U256::ZERO,
                data: Bytes::from(calldata),
                gas_limit: GAS_LIMIT,
                nonce: *nonce,
                gas_price: default_gas_price(),
            });
            tx_function_map.insert(tx_idx, func.name.clone());
            *nonce += 1;
        }

        sender_idx += 1;
    }

    let block_env = BlockEnv {
        number: 1,
        coinbase: Address::with_last_byte(0xFF),
        timestamp: 1_700_000_000,
        gas_limit: 30_000_000,
        base_fee: U256::ZERO,
        difficulty: U256::ZERO,
    };

    Ok(BuildResult {
        transactions,
        block_env,
        tx_function_map,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_solc() -> bool {
        std::process::Command::new("solc")
            .arg("--version")
            .output()
            .is_ok()
    }

    #[test]
    fn test_compute_create_address() {
        // Known test vector: sender=0x0...E1, nonce=0
        let addr = compute_create_address(SENDER_ADDRESSES[0], 0);
        // Just verify it's deterministic and non-zero
        assert_ne!(addr, Address::ZERO);

        // Same inputs → same output
        let addr2 = compute_create_address(SENDER_ADDRESSES[0], 0);
        assert_eq!(addr, addr2);

        // Different nonce → different address
        let addr3 = compute_create_address(SENDER_ADDRESSES[0], 1);
        assert_ne!(addr, addr3);
    }

    #[test]
    fn test_default_abi_word_address() {
        let sender = SENDER_ADDRESSES[0];
        let word = default_abi_word("address", sender);
        // Last 20 bytes should be the sender address
        assert_eq!(&word[12..], sender.as_slice());
    }

    #[test]
    fn test_default_abi_word_uint256() {
        let word = default_abi_word("uint256", Address::ZERO);
        // Should be 1 (non-zero default for amounts)
        assert_eq!(word[31], 1);
        assert!(word[..31].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_default_abi_word_bool() {
        let word = default_abi_word("bool", Address::ZERO);
        assert_eq!(word[31], 1); // true
    }

    #[test]
    fn test_sender_rotation() {
        // Verify senders are distinct
        let mut addrs: Vec<Address> = SENDER_ADDRESSES.to_vec();
        addrs.sort();
        addrs.dedup();
        assert_eq!(addrs.len(), 8, "all 8 senders should be unique");
    }

    #[test]
    fn test_build_counter_contract() {
        if !has_solc() {
            eprintln!("SKIP: solc not installed");
            return;
        }

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
        let compile_result = crate::compiler::compile(source).expect("compilation should succeed");
        let build_result = build(&compile_result).expect("build should succeed");

        // tx[0] = deploy (no `to` address)
        assert!(build_result.transactions[0].to.is_none(), "tx[0] should be deploy");
        assert_eq!(
            build_result.tx_function_map.get(&0).unwrap(),
            "constructor"
        );

        // tx[1..] = function calls
        assert!(
            build_result.transactions.len() >= 3,
            "should have deploy + at least 2 function calls (increment, decrement)"
        );

        // All call txs should target the same contract address
        let contract_addr = build_result.transactions[1].to.unwrap();
        for tx in &build_result.transactions[1..] {
            assert_eq!(tx.to.unwrap(), contract_addr, "all calls should target deployed contract");
        }

        // tx_function_map should have entries for all txs
        assert_eq!(
            build_result.tx_function_map.len(),
            build_result.transactions.len()
        );

        // Verify sender rotation — call txs should use different senders
        let call_senders: Vec<Address> = build_result.transactions[1..]
            .iter()
            .map(|tx| tx.sender)
            .collect();
        // With 2 functions and starting at sender[1], we get sender[1] and sender[2]
        assert_ne!(
            call_senders[0], call_senders[1],
            "different functions should use different senders"
        );
    }

    #[test]
    fn test_build_no_state_changing_functions() {
        // A contract with only view functions should fail
        let compile_result = CompileResult {
            contract_name: "ViewOnly".to_string(),
            abi_json: r#"[{"type":"function","name":"get","inputs":[],"outputs":[{"type":"uint256"}],"stateMutability":"view"}]"#.to_string(),
            bytecode: "6080604052".to_string(),
            storage_layout: r#"{"storage":[],"types":{}}"#.to_string(),
        };

        let result = build(&compile_result);
        assert!(result.is_err());
        match result.unwrap_err() {
            BuildError::NoStateChangingFunctions => {}
            other => panic!("expected NoStateChangingFunctions, got: {other:?}"),
        }
    }

    #[test]
    fn test_build_block_env_defaults() {
        if !has_solc() {
            eprintln!("SKIP: solc not installed");
            return;
        }

        let source = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;
contract Simple { function set() public {} }
"#;
        let compile_result = crate::compiler::compile(source).expect("compile");
        let build_result = build(&compile_result).expect("build");

        assert_eq!(build_result.block_env.number, 1);
        assert_eq!(build_result.block_env.gas_limit, 30_000_000);
        assert_eq!(build_result.block_env.coinbase, Address::with_last_byte(0xFF));
    }
}
