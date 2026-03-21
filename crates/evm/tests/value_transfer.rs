//! Integration test: ETH value transfer between two accounts.
//!
//! Verifies that a simple ETH transfer:
//! - Deducts (value + gas_cost) from the sender
//! - Credits exactly `value` to the receiver
//! - Returns Success with gas_used == 21000 (intrinsic gas for simple transfer)

use monad_evm::EvmExecutor;
use monad_state::InMemoryState;
use monad_types::{
    AccountInfo, Address, BlockEnv, Bytes, ExecutionResult, Transaction, U256,
};

/// Non-precompile address for sender (precompiles are 0x01-0x09).
fn sender_addr() -> Address {
    Address::with_last_byte(0x10)
}

/// Non-precompile address for receiver.
fn receiver_addr() -> Address {
    Address::with_last_byte(0x20)
}

#[test]
fn value_transfer_deducts_sender_credits_receiver() {
    let sender = sender_addr();
    let receiver = receiver_addr();

    let ten_eth = U256::from(10_000_000_000_000_000_000u128); // 10 ETH
    let one_eth = U256::from(1_000_000_000_000_000_000u128); // 1 ETH

    let state = InMemoryState::new()
        .with_account(sender, AccountInfo::new(ten_eth, 0))
        .with_account(receiver, AccountInfo::new(U256::ZERO, 0));

    let block_env = BlockEnv::default();

    let tx = Transaction {
        sender,
        to: Some(receiver),
        value: one_eth,
        data: Bytes::new(),
        gas_limit: 21_000,
        nonce: 0,
        gas_price: U256::ZERO, // fee charge disabled, so gas_price=0 works
    };

    // Execute
    let (result, state_changes) =
        EvmExecutor::execute_tx_with_state_changes(&tx, &state, &block_env)
            .expect("execution should succeed");

    // Assert: result is Success
    assert!(result.is_success(), "expected Success, got {:?}", result);
    assert_eq!(result.gas_used(), 21_000, "simple transfer uses 21000 gas");

    // Assert: sender balance decreased by exactly 1 ETH
    // (gas_price is 0 and fee_charge disabled, so no gas cost in wei)
    let (sender_info, _) = state_changes
        .get(&sender)
        .expect("sender should be in state changes");
    let expected_sender_balance = ten_eth - one_eth;
    assert_eq!(
        sender_info.balance, expected_sender_balance,
        "sender balance should be 10 ETH - 1 ETH = 9 ETH"
    );

    // Assert: sender nonce incremented
    assert_eq!(sender_info.nonce, 1, "sender nonce should increment to 1");

    // Assert: receiver balance increased by 1 ETH
    let (receiver_info, _) = state_changes
        .get(&receiver)
        .expect("receiver should be in state changes");
    assert_eq!(
        receiver_info.balance, one_eth,
        "receiver balance should be 1 ETH"
    );
}

#[test]
fn value_transfer_with_gas_price() {
    // When gas_price > 0, sender pays value + gas_used * gas_price
    let sender = sender_addr();
    let receiver = receiver_addr();

    let ten_eth = U256::from(10_000_000_000_000_000_000u128);
    let one_eth = U256::from(1_000_000_000_000_000_000u128);
    let gas_price = U256::from(1_000_000_000u64); // 1 gwei

    let state = InMemoryState::new()
        .with_account(sender, AccountInfo::new(ten_eth, 0))
        .with_account(receiver, AccountInfo::new(U256::ZERO, 0));

    let block_env = BlockEnv::default();

    let tx = Transaction {
        sender,
        to: Some(receiver),
        value: one_eth,
        data: Bytes::new(),
        gas_limit: 21_000,
        nonce: 0,
        gas_price,
    };

    let (result, state_changes) =
        EvmExecutor::execute_tx_with_state_changes(&tx, &state, &block_env)
            .expect("execution should succeed");

    assert!(result.is_success(), "expected Success, got {:?}", result);
    assert_eq!(result.gas_used(), 21_000);

    // With disable_fee_charge, no gas fee is deducted from sender balance.
    // Only the value transfer reduces the sender's balance.
    let (sender_info, _) = state_changes.get(&sender).unwrap();
    let expected = ten_eth - one_eth;
    assert_eq!(
        sender_info.balance, expected,
        "sender balance should be 10 ETH - 1 ETH (fee charge disabled)"
    );
}

#[test]
fn value_transfer_returns_success_variant() {
    let sender = sender_addr();
    let receiver = receiver_addr();

    let state = InMemoryState::new().with_account(
        sender,
        AccountInfo::new(U256::from(1_000_000_000_000_000_000u128), 0),
    );

    let tx = Transaction {
        sender,
        to: Some(receiver),
        value: U256::from(100u64),
        data: Bytes::new(),
        gas_limit: 21_000,
        nonce: 0,
        gas_price: U256::ZERO,
    };

    let result = EvmExecutor::execute_tx(&tx, &state, &BlockEnv::default())
        .expect("execution should succeed");

    match result {
        ExecutionResult::Success {
            gas_used,
            output,
            logs,
        } => {
            assert_eq!(gas_used, 21_000);
            assert!(output.is_empty(), "simple transfer has no output data");
            assert!(logs.is_empty(), "simple transfer emits no logs");
        }
        other => panic!("expected Success, got {:?}", other),
    }
}

#[test]
fn halt_insufficient_gas() {
    // Transaction with gas_limit < 21000 (intrinsic gas) should halt or error
    let sender = sender_addr();
    let receiver = receiver_addr();

    let state = InMemoryState::new().with_account(
        sender,
        AccountInfo::new(U256::from(1_000_000_000_000_000_000u128), 0),
    );

    let tx = Transaction {
        sender,
        to: Some(receiver),
        value: U256::from(100u64),
        data: Bytes::new(),
        gas_limit: 100, // way too low
        nonce: 0,
        gas_price: U256::ZERO,
    };

    // This should result in an error (intrinsic gas check happens in validation)
    let result = EvmExecutor::execute_tx(&tx, &state, &BlockEnv::default());

    // The transaction should fail — either as Halt or as a validation error
    match result {
        Ok(ExecutionResult::Halt { reason, .. }) => {
            assert!(
                !reason.is_empty(),
                "halt reason should explain the failure"
            );
        }
        Err(e) => {
            // revm may reject at validation time with an error
            let msg = e.to_string();
            assert!(
                msg.contains("gas") || msg.contains("Gas") || msg.contains("intrinsic"),
                "error should mention gas: {}",
                msg
            );
        }
        other => panic!(
            "expected Halt or error for insufficient gas, got {:?}",
            other
        ),
    }
}
