//! Solidity compiler wrapper using solc subprocess.
//!
//! Compiles Solidity source code by writing to a temp file and invoking
//! `solc --combined-json abi,bin,storage-layout --evm-version cancun`.
//!
//! # Observability
//!
//! - `CompileResult` contains abi_json, bytecode hex, and storage_layout JSON
//! - `CompileError::SolcFailed` preserves stderr for user-facing error messages
//! - `CompileError::SolcNotFound` distinguishes missing solc from compile failures

use std::process::Command;

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use thiserror::Error;

/// Errors during Solidity compilation.
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("solc not found — install via `solc-select install 0.8.28 && solc-select use 0.8.28`")]
    SolcNotFound,

    #[error("solc compilation failed: {stderr}")]
    SolcFailed { stderr: String },

    #[error("failed to write temp file: {0}")]
    TempFile(#[from] std::io::Error),

    #[error("failed to parse solc output: {0}")]
    ParseError(String),

    #[error("no contracts found in source")]
    NoContracts,
}

/// Result of a successful Solidity compilation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileResult {
    /// Contract name extracted from source.
    pub contract_name: String,
    /// ABI as raw JSON string (array of function/event descriptors).
    pub abi_json: String,
    /// Compiled bytecode as hex string (no 0x prefix).
    pub bytecode: String,
    /// Storage layout as raw JSON string (from solc --storage-layout).
    pub storage_layout: String,
}

/// Compile Solidity source code via solc subprocess.
///
/// Writes source to a temp file, runs solc with `--combined-json abi,bin,storage-layout`,
/// and parses the output to extract ABI, bytecode, and storage layout for the first
/// contract found.
///
/// # Errors
///
/// Returns `CompileError` if solc is not installed, compilation fails, or
/// the output cannot be parsed.
pub fn compile(source: &str) -> Result<CompileResult, CompileError> {
    use std::io::Write;

    // Write source to a temp .sol file
    let mut tmp = NamedTempFile::with_suffix(".sol")?;
    tmp.write_all(source.as_bytes())?;
    tmp.flush()?;

    let tmp_path = tmp.path().to_path_buf();

    // Run solc
    let output = Command::new("solc")
        .arg("--combined-json")
        .arg("abi,bin,storage-layout")
        .arg("--evm-version")
        .arg("cancun")
        .arg(&tmp_path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CompileError::SolcNotFound
            } else {
                CompileError::TempFile(e)
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(CompileError::SolcFailed { stderr });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse the combined-json output
    let combined: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| CompileError::ParseError(format!("invalid JSON from solc: {e}")))?;

    let contracts = combined
        .get("contracts")
        .and_then(|c| c.as_object())
        .ok_or_else(|| CompileError::ParseError("missing 'contracts' in solc output".into()))?;

    // Take the first contract (solc keys are "filename:ContractName")
    let (full_key, contract_obj) = contracts
        .iter()
        .next()
        .ok_or(CompileError::NoContracts)?;

    // Extract contract name from "path/to/file.sol:ContractName"
    let contract_name = full_key
        .rsplit(':')
        .next()
        .unwrap_or(full_key)
        .to_string();

    let abi_json = contract_obj
        .get("abi")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "[]".to_string());

    let bytecode = contract_obj
        .get("bin")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if bytecode.is_empty() {
        return Err(CompileError::ParseError(
            "empty bytecode — contract may be abstract or an interface".into(),
        ));
    }

    let storage_layout = contract_obj
        .get("storage-layout")
        .map(|v| v.to_string())
        .unwrap_or_else(|| r#"{"storage":[],"types":{}}"#.to_string());

    Ok(CompileResult {
        contract_name,
        abi_json,
        bytecode,
        storage_layout,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_solc() -> bool {
        Command::new("solc").arg("--version").output().is_ok()
    }

    #[test]
    fn test_compile_simple_counter() {
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
        let result = compile(source).expect("compilation should succeed");
        assert_eq!(result.contract_name, "Counter");
        assert!(!result.bytecode.is_empty(), "bytecode should not be empty");
        assert!(
            result.abi_json.contains("increment"),
            "ABI should contain 'increment'"
        );
        assert!(
            result.abi_json.contains("decrement"),
            "ABI should contain 'decrement'"
        );
        assert!(
            result.abi_json.contains("count"),
            "ABI should contain 'count'"
        );
    }

    #[test]
    fn test_compile_error_invalid_source() {
        if !has_solc() {
            eprintln!("SKIP: solc not installed");
            return;
        }

        let source = "this is not valid solidity";
        let result = compile(source);
        assert!(result.is_err());
        match result.unwrap_err() {
            CompileError::SolcFailed { stderr } => {
                assert!(!stderr.is_empty(), "stderr should contain error details");
            }
            other => panic!("expected SolcFailed, got: {other:?}"),
        }
    }

    #[test]
    fn test_compile_result_has_storage_layout() {
        if !has_solc() {
            eprintln!("SKIP: solc not installed");
            return;
        }

        let source = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Store {
    uint256 public value;
    mapping(address => uint256) public balances;

    function set(uint256 v) public { value = v; }
    function deposit() public { balances[msg.sender] += 1; }
}
"#;
        let result = compile(source).expect("compilation should succeed");
        assert!(
            result.storage_layout.contains("storage"),
            "storage_layout should contain 'storage' key"
        );
    }
}
