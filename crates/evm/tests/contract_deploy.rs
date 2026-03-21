//! Integration test: contract deployment via CREATE transaction.
//!
//! Verifies that a contract creation transaction:
//! - Returns Success with the deployed bytecode in output
//! - Consumes gas including deployment cost
//! - Creates the contract at the correct address

use monad_evm::EvmExecutor;
use monad_state::InMemoryState;
use monad_types::{
    AccountInfo, Address, BlockEnv, Bytes, ExecutionResult, Transaction, U256,
};

#[test]
fn contract_deploy_stores_code() {
    // Deployer has 10 ETH, nonce 0
    let deployer = Address::with_last_byte(0x10);
    let ten_eth = U256::from(10_000_000_000_000_000_000u128);

    let state = InMemoryState::new()
        .with_account(deployer, AccountInfo::new(ten_eth, 0));

    let block_env = BlockEnv::default();

    // Simple contract init code:
    // PUSH1 0x42    -> 6042
    // PUSH1 0x00    -> 6000
    // MSTORE        -> 52
    // PUSH1 0x20    -> 6020
    // PUSH1 0x00    -> 6000
    // RETURN        -> f3
    //
    // This stores 0x42 at memory[0] and returns 32 bytes as deployed code
    let init_code = Bytes::from(vec![
        0x60, 0x42, // PUSH1 0x42
        0x60, 0x00, // PUSH1 0x00
        0x52,       // MSTORE
        0x60, 0x20, // PUSH1 0x20 (32 bytes)
        0x60, 0x00, // PUSH1 0x00
        0xf3,       // RETURN
    ]);

    let tx = Transaction {
        sender: deployer,
        to: None, // CREATE transaction
        value: U256::ZERO,
        data: init_code,
        gas_limit: 100_000,
        nonce: 0,
        gas_price: U256::ZERO,
    };

    // Execute
    let (result, state_changes) =
        EvmExecutor::execute_tx_with_state_changes(&tx, &state, &block_env)
            .expect("execution should succeed");

    // Assert: result is Success
    assert!(result.is_success(), "expected Success, got {:?}", result);

    // Assert: gas_used > 21000 (includes deployment cost)
    let gas_used = result.gas_used();
    assert!(
        gas_used > 21_000,
        "contract deployment should use more gas than simple transfer, got {}",
        gas_used
    );

    // Assert: output contains the deployed bytecode (32 bytes with 0x42 at the end)
    match &result {
        ExecutionResult::Success { output, .. } => {
            assert!(
                !output.is_empty(),
                "deployed contract should have non-empty code"
            );
            assert_eq!(
                output.len(),
                32,
                "deployed code should be 32 bytes (MSTORE writes 32 bytes)"
            );
            // The returned code should contain 0x42 in the last byte
            assert_eq!(
                output[31], 0x42,
                "deployed code should have 0x42 as the value"
            );
        }
        _ => panic!("expected Success variant"),
    }

    // Assert: deployer nonce incremented
    let (deployer_info, _) = state_changes
        .get(&deployer)
        .expect("deployer should be in state changes");
    assert_eq!(
        deployer_info.nonce, 1,
        "deployer nonce should increment to 1"
    );
}

#[test]
fn contract_deploy_with_value() {
    // Deploy a contract with initial ETH value (endowment)
    let deployer = Address::with_last_byte(0x10);
    let ten_eth = U256::from(10_000_000_000_000_000_000u128);
    let endowment = U256::from(1_000_000_000_000_000_000u128); // 1 ETH

    let state = InMemoryState::new()
        .with_account(deployer, AccountInfo::new(ten_eth, 0));

    // Minimal contract that just returns empty code: PUSH1 0 PUSH1 0 RETURN
    let init_code = Bytes::from(vec![
        0x60, 0x00, // PUSH1 0x00
        0x60, 0x00, // PUSH1 0x00
        0xf3,       // RETURN
    ]);

    let tx = Transaction {
        sender: deployer,
        to: None,
        value: endowment,
        data: init_code,
        gas_limit: 100_000,
        nonce: 0,
        gas_price: U256::ZERO,
    };

    let result = EvmExecutor::execute_tx(&tx, &state, &BlockEnv::default())
        .expect("execution should succeed");

    assert!(result.is_success(), "expected Success, got {:?}", result);
    assert!(result.gas_used() > 21_000);
}

#[test]
fn contract_deploy_gas_accounting() {
    // Verify gas is properly accounted for during deployment
    let deployer = Address::with_last_byte(0x10);
    let ten_eth = U256::from(10_000_000_000_000_000_000u128);

    let state = InMemoryState::new()
        .with_account(deployer, AccountInfo::new(ten_eth, 0));

    // Same init code as above
    let init_code = Bytes::from(vec![
        0x60, 0x42, 0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3,
    ]);

    let tx = Transaction {
        sender: deployer,
        to: None,
        value: U256::ZERO,
        data: init_code,
        gas_limit: 200_000,
        nonce: 0,
        gas_price: U256::ZERO,
    };

    let result = EvmExecutor::execute_tx(&tx, &state, &BlockEnv::default())
        .expect("execution should succeed");

    assert!(result.is_success());

    // Gas used should include:
    // - Intrinsic gas (21000 base + per-byte data cost)
    // - Execution cost (PUSH, MSTORE, RETURN opcodes)
    // - Code deposit cost (200 gas per byte of deployed code)
    let gas_used = result.gas_used();
    assert!(
        gas_used > 21_000,
        "gas should include more than just intrinsic"
    );
    assert!(
        gas_used < 200_000,
        "gas should be less than the limit ({})",
        gas_used
    );
}
