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

/// Generate N sender addresses dynamically (0xE001..0xE0XX range).
/// More senders = more parallelism (fewer nonce conflicts per sender).
fn generate_senders(count: usize) -> Vec<Address> {
    (0..count)
        .map(|i| {
            let mut bytes = [0u8; 20];
            // Spread across 2 bytes for up to 65536 senders
            bytes[18] = ((i + 1) >> 8) as u8;
            bytes[19] = ((i + 1) & 0xFF) as u8;
            // Use 0xE0 prefix to distinguish from other addresses
            bytes[17] = 0xE0;
            Address::new(bytes)
        })
        .collect()
}

/// Number of unique senders for parallel execution.
/// More senders → fewer same-sender nonce conflicts → more parallelism.
const NUM_SENDERS: usize = 64;

/// Deployer address (sender of tx[0] = contract creation).
const DEPLOYER: Address = Address::new([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0x00, 0x01,
]);

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

/// Compute a default repeat_count that targets ~300 total TXs.
///
/// Given `n` state-changing functions, returns `ceil(300 / n)` so the
/// block contains at least 300 call TXs (plus 1 deploy TX).
/// Clamps to a minimum of 1 and maximum of 1000.
pub fn default_repeat_count(state_changing_fn_count: usize) -> u32 {
    if state_changing_fn_count == 0 {
        return 1;
    }
    let target = 300u32;
    let count = (target + state_changing_fn_count as u32 - 1) / state_changing_fn_count as u32;
    count.clamp(1, 1000)
}

/// Build a transaction block from a compiled contract.
///
/// Creates:
/// - tx[0]: Deploy the contract (sender = DEPLOYER, nonce = 0)
/// - tx[1..N]: Call each state-changing function `repeat_count` times
///   with rotating senders
///
/// If `repeat_count` is `None`, auto-computes a value targeting ~300 TXs.
///
/// State-changing functions are those with mutability `NonPayable` or `Payable`
/// (i.e., not `Pure` or `View`).
///
/// # Errors
///
/// Returns `BuildError::AbiParse` if the ABI JSON is malformed, or
/// `BuildError::NoStateChangingFunctions` if the contract has no callable
/// state-changing functions.
pub fn build(compile_result: &CompileResult, repeat_count: Option<u32>) -> Result<BuildResult, BuildError> {
    // Generate sender addresses for parallel execution
    let senders = generate_senders(NUM_SENDERS);

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

    // Resolve repeat_count: use provided value, or auto-compute to target ~300 TXs.
    let repeats = repeat_count
        .unwrap_or_else(|| default_repeat_count(state_changing_fns.len()));

    let estimated_txs = 1 + state_changing_fns.len() * repeats as usize;
    let mut transactions = Vec::with_capacity(estimated_txs);
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

    // tx[1..N]: Call each state-changing function `repeats` times with rotating senders
    let mut sender_idx = 1usize; // Start from sender[1] for calls (sender[0] = deployer)
    let mut nonce_map: HashMap<Address, u64> = HashMap::new();
    // Deployer already used nonce 0 for deploy
    nonce_map.insert(DEPLOYER, 1);

    for _round in 0..repeats {
        for func in &state_changing_fns {
            let sender = senders[sender_idx % senders.len()];
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
        let addr = compute_create_address(generate_senders(NUM_SENDERS)[0], 0);
        // Just verify it's deterministic and non-zero
        assert_ne!(addr, Address::ZERO);

        // Same inputs → same output
        let addr2 = compute_create_address(generate_senders(NUM_SENDERS)[0], 0);
        assert_eq!(addr, addr2);

        // Different nonce → different address
        let addr3 = compute_create_address(generate_senders(NUM_SENDERS)[0], 1);
        assert_ne!(addr, addr3);
    }

    #[test]
    fn test_default_abi_word_address() {
        let sender = generate_senders(NUM_SENDERS)[0];
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
        let mut addrs: Vec<Address> = generate_senders(NUM_SENDERS);
        addrs.sort();
        addrs.dedup();
        assert_eq!(addrs.len(), NUM_SENDERS, "all senders should be unique");
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
        let build_result = build(&compile_result, Some(1)).expect("build should succeed");

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

        let result = build(&compile_result, Some(1));
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
        let build_result = build(&compile_result, Some(1)).expect("build");

        assert_eq!(build_result.block_env.number, 1);
        assert_eq!(build_result.block_env.gas_limit, 30_000_000);
        assert_eq!(build_result.block_env.coinbase, Address::with_last_byte(0xFF));
    }

    #[test]
    fn test_default_repeat_count() {
        // 2 functions → ceil(300/2) = 150
        assert_eq!(default_repeat_count(2), 150);
        // 3 functions → ceil(300/3) = 100
        assert_eq!(default_repeat_count(3), 100);
        // 1 function → 300
        assert_eq!(default_repeat_count(1), 300);
        // 0 functions → 1 (edge case)
        assert_eq!(default_repeat_count(0), 1);
        // 600 functions → ceil(300/600) = 1
        assert_eq!(default_repeat_count(600), 1);
    }

    #[test]
    fn test_build_with_repeat_count() {
        if !has_solc() {
            eprintln!("SKIP: solc not installed");
            return;
        }

        let source = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Counter {
    uint256 public count;
    function increment() public { count += 1; }
    function decrement() public { count -= 1; }
}
"#;
        let compile_result = crate::compiler::compile(source).expect("compile");

        // repeat_count=5 → 2 functions × 5 repeats = 10 call txs + 1 deploy = 11 total
        let result = build(&compile_result, Some(5)).expect("build");
        assert_eq!(result.transactions.len(), 11, "1 deploy + 2*5 = 11 txs");
        assert!(result.transactions[0].to.is_none(), "tx[0] is deploy");
        for tx in &result.transactions[1..] {
            assert!(tx.to.is_some(), "call txs should have target address");
        }

        // All call txs should have valid function names in the map
        for i in 1..result.transactions.len() {
            let name = result.tx_function_map.get(&i).expect("missing tx_function_map entry");
            assert!(
                name == "increment" || name == "decrement",
                "unexpected function name: {name}"
            );
        }
    }

    #[test]
    fn test_build_with_none_repeat_count_auto_computes() {
        if !has_solc() {
            eprintln!("SKIP: solc not installed");
            return;
        }

        let source = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Counter {
    uint256 public count;
    function increment() public { count += 1; }
    function decrement() public { count -= 1; }
}
"#;
        let compile_result = crate::compiler::compile(source).expect("compile");

        // None → auto-compute: 2 functions → repeat=150 → 300 call txs + 1 deploy = 301
        let result = build(&compile_result, None).expect("build");
        assert_eq!(
            result.transactions.len(),
            301,
            "auto repeat_count for 2 fns should produce 301 txs"
        );
    }

    #[test]
    fn test_build_nonce_correctness_with_repeat() {
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

        // 1 function × 10 repeats = 10 call txs + 1 deploy
        let result = build(&compile_result, Some(10)).expect("build");
        assert_eq!(result.transactions.len(), 11);

        // Verify nonces are sequential per sender.
        // DEPLOYER (0xE1) starts at nonce=1 because nonce=0 was used for deploy.
        // Other senders start at nonce=0.
        let mut nonce_check: HashMap<Address, Vec<u64>> = HashMap::new();
        // Skip deploy tx (index 0)
        for tx in &result.transactions[1..] {
            nonce_check.entry(tx.sender).or_default().push(tx.nonce);
        }
        for (addr, nonces) in &nonce_check {
            let base_nonce = if *addr == DEPLOYER { 1u64 } else { 0u64 };
            for (i, &nonce) in nonces.iter().enumerate() {
                assert_eq!(
                    nonce,
                    base_nonce + i as u64,
                    "sender {addr}: nonce at position {i} should be {}, got {nonce}",
                    base_nonce + i as u64
                );
            }
        }
    }
}
