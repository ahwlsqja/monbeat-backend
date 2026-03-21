//! End-to-end NINE FORK integration tests through the EvmExecutor pipeline.
//!
//! These tests verify NINE FORK compliance (MIP-3, MIP-4, MIP-5) through
//! the full EVM execution stack: bytecode → interpreter → state changes.
//!
//! This file is extended by subsequent tasks in slice S02.

use monad_evm::EvmExecutor;
use monad_state::InMemoryState;
use monad_types::{
    AccountInfo, Address, BlockEnv, Bytes, Transaction, U256,
};

/// Sender address for NINE FORK integration tests.
fn sender() -> Address {
    Address::with_last_byte(0xA5)
}

/// Helper to create AccountInfo with code for a deployed contract.
fn account_with_code(code: Vec<u8>) -> AccountInfo {
    let code_bytes = Bytes::from(code);
    let code_hash = alloy_primitives::keccak256(&code_bytes);
    AccountInfo::new_contract(U256::ZERO, 1, code_hash.into(), code_bytes)
}

// ─── MIP-3: Linear Memory Pool Integration Tests ──────────────────────────

/// Builds a contract that MSTOREs a value at offset 0 and REVERTs.
fn build_mstore_revert(value: U256) -> Vec<u8> {
    let mut code = Vec::new();
    code.push(0x7F); // PUSH32
    code.extend_from_slice(&value.to_be_bytes::<32>());
    code.push(0x60); // PUSH1 0x00
    code.push(0x00);
    code.push(0x52); // MSTORE
    code.push(0x60); // PUSH1 0x00
    code.push(0x00);
    code.push(0x60); // PUSH1 0x00
    code.push(0x00);
    code.push(0xFD); // REVERT
    code
}

/// Builds a contract that MSTOREs a value at offset 0 and RETURNs empty.
fn build_mstore_return(value: U256) -> Vec<u8> {
    let mut code = Vec::new();
    code.push(0x7F); // PUSH32
    code.extend_from_slice(&value.to_be_bytes::<32>());
    code.push(0x60); // PUSH1 0x00
    code.push(0x00);
    code.push(0x52); // MSTORE
    code.push(0x60); // PUSH1 0x00
    code.push(0x00);
    code.push(0x60); // PUSH1 0x00
    code.push(0x00);
    code.push(0xF3); // RETURN
    code
}

/// Builds a parent contract that MSTOREs parent_value, CALLs target, then
/// MLOADs and SSTOREs for verification.
fn build_memory_test_parent(parent_value: U256, target_addr: Address) -> Vec<u8> {
    let mut code = Vec::new();

    // MSTORE parent_value at offset 0
    code.push(0x7F); // PUSH32
    code.extend_from_slice(&parent_value.to_be_bytes::<32>());
    code.push(0x60); // PUSH1 0x00
    code.push(0x00);
    code.push(0x52); // MSTORE

    // CALL(gas=100000, addr, value=0, argsOffset=0, argsLen=0, retOffset=0, retLen=0)
    code.push(0x60); code.push(0x00); // retLength
    code.push(0x60); code.push(0x00); // retOffset
    code.push(0x60); code.push(0x00); // argsLength
    code.push(0x60); code.push(0x00); // argsOffset
    code.push(0x60); code.push(0x00); // value
    code.push(0x73); // PUSH20 target_addr
    code.extend_from_slice(target_addr.as_slice());
    code.push(0x62); // PUSH3 100000
    code.push(0x01); code.push(0x86); code.push(0xA0);
    code.push(0xF1); // CALL

    // SSTORE call success at slot 1
    code.push(0x60); code.push(0x01); // slot 1
    code.push(0x55); // SSTORE

    // MLOAD offset 0
    code.push(0x60); code.push(0x00);
    code.push(0x51); // MLOAD

    // SSTORE at slot 0
    code.push(0x60); code.push(0x00);
    code.push(0x55); // SSTORE

    code.push(0x00); // STOP
    code
}

/// MIP-3 Integration: REVERT in sub-call restores parent memory.
/// Tests the full pipeline from TX submission through EvmExecutor.
#[test]
fn nine_fork_mip3_revert_memory_isolation() {
    let sub_addr = Address::with_last_byte(0x31);
    let parent_addr = Address::with_last_byte(0x30);

    let parent_value = U256::from(0xCAFEu64);
    let sub_value = U256::from(0xFACEu64);

    let state = InMemoryState::new()
        .with_account(sender(), AccountInfo::new(U256::from(10_000_000_000u64), 0))
        .with_account(parent_addr, account_with_code(build_memory_test_parent(parent_value, sub_addr)))
        .with_account(sub_addr, account_with_code(build_mstore_revert(sub_value)));

    let tx = Transaction {
        sender: sender(),
        to: Some(parent_addr),
        value: U256::ZERO,
        data: Bytes::new(),
        gas_limit: 1_000_000,
        nonce: 0,
        gas_price: U256::ZERO,
    };

    let (result, state_changes) =
        EvmExecutor::execute_tx_with_state_changes(&tx, &state, &BlockEnv::default())
            .expect("MIP-3 integration test should not error");

    assert!(result.is_success(), "TX should succeed: {:?}", result);

    let (_, storage) = state_changes
        .get(&parent_addr)
        .expect("Parent should have state changes");

    // Memory at offset 0 should still be parent_value (0xCAFE), not sub's 0xFACE
    let mem_readback = storage.get(&U256::ZERO).expect("Slot 0 should exist");
    assert_eq!(
        *mem_readback, parent_value,
        "Parent memory should be {:#x} after sub-REVERT, got {:#x}",
        parent_value, mem_readback
    );

    // CALL should have returned 0 (sub reverted)
    let call_ok = storage.get(&U256::from(1u64)).expect("Slot 1 should exist");
    assert_eq!(*call_ok, U256::ZERO, "CALL to reverting contract should return 0");
}

/// MIP-3 Integration: RETURN in sub-call does not leak memory.
#[test]
fn nine_fork_mip3_return_memory_isolation() {
    let sub_addr = Address::with_last_byte(0x33);
    let parent_addr = Address::with_last_byte(0x32);

    let parent_value = U256::from(0x1234u64);
    let sub_value = U256::from(0x5678u64);

    let state = InMemoryState::new()
        .with_account(sender(), AccountInfo::new(U256::from(10_000_000_000u64), 0))
        .with_account(parent_addr, account_with_code(build_memory_test_parent(parent_value, sub_addr)))
        .with_account(sub_addr, account_with_code(build_mstore_return(sub_value)));

    let tx = Transaction {
        sender: sender(),
        to: Some(parent_addr),
        value: U256::ZERO,
        data: Bytes::new(),
        gas_limit: 1_000_000,
        nonce: 0,
        gas_price: U256::ZERO,
    };

    let (result, state_changes) =
        EvmExecutor::execute_tx_with_state_changes(&tx, &state, &BlockEnv::default())
            .expect("MIP-3 return integration test should not error");

    assert!(result.is_success(), "TX should succeed: {:?}", result);

    let (_, storage) = state_changes
        .get(&parent_addr)
        .expect("Parent should have state changes");

    // Memory at offset 0 should still be parent_value
    let mem_readback = storage.get(&U256::ZERO).expect("Slot 0 should exist");
    assert_eq!(
        *mem_readback, parent_value,
        "Parent memory should be {:#x} after sub-RETURN, got {:#x}",
        parent_value, mem_readback
    );

    // CALL should have returned 1 (sub returned successfully)
    let call_ok = storage.get(&U256::from(1u64)).expect("Slot 1 should exist");
    assert_eq!(*call_ok, U256::from(1u64), "CALL to returning contract should return 1");
}

/// MIP-3 Integration: Deep nesting (6 levels) with innermost REVERT.
/// Proves memory isolation through the full EvmExecutor pipeline at depth.
#[test]
fn nine_fork_mip3_deep_nesting_via_executor() {
    // Build a chain: A→B→C→D→E→F where F REVERTs.
    // Each intermediate stores its own MSTORE value and verifies MLOAD.
    let addrs: Vec<Address> = (0..6)
        .map(|i| Address::with_last_byte(0x40 + i))
        .collect();

    let values: Vec<U256> = (0..6)
        .map(|i| U256::from(0xCC00u64 + i as u64))
        .collect();

    // F: MSTORE and REVERT
    let innermost = build_mstore_revert(values[5]);

    // E through A: MSTORE, CALL next, MLOAD, SSTORE, RETURN
    let mut state = InMemoryState::new()
        .with_account(sender(), AccountInfo::new(U256::from(10_000_000_000u64), 0))
        .with_account(addrs[5], account_with_code(innermost));

    for i in (0..5).rev() {
        let mut code = Vec::new();
        // MSTORE values[i] at offset 0
        code.push(0x7F);
        code.extend_from_slice(&values[i].to_be_bytes::<32>());
        code.push(0x60); code.push(0x00);
        code.push(0x52);

        // CALL addrs[i+1] with gas 500000
        code.push(0x60); code.push(0x00); // retLen
        code.push(0x60); code.push(0x00); // retOff
        code.push(0x60); code.push(0x00); // argsLen
        code.push(0x60); code.push(0x00); // argsOff
        code.push(0x60); code.push(0x00); // value
        code.push(0x73);
        code.extend_from_slice(addrs[i + 1].as_slice());
        code.push(0x62); code.push(0x07); code.push(0xA1); code.push(0x20);
        code.push(0xF1);

        // Store call success at slot 1
        code.push(0x60); code.push(0x01);
        code.push(0x55);

        // MLOAD 0 → SSTORE slot 0
        code.push(0x60); code.push(0x00);
        code.push(0x51);
        code.push(0x60); code.push(0x00);
        code.push(0x55);

        // RETURN empty (except level 0 which STOPs — doesn't matter, both work)
        code.push(0x60); code.push(0x00);
        code.push(0x60); code.push(0x00);
        code.push(0xF3);

        state.insert_account(addrs[i], account_with_code(code));
    }

    let tx = Transaction {
        sender: sender(),
        to: Some(addrs[0]),
        value: U256::ZERO,
        data: Bytes::new(),
        gas_limit: 16_000_000,
        nonce: 0,
        gas_price: U256::ZERO,
    };

    let (result, state_changes) =
        EvmExecutor::execute_tx_with_state_changes(&tx, &state, &BlockEnv::default())
            .expect("Deep nesting integration test should not error");

    assert!(result.is_success(), "TX should succeed: {:?}", result);

    // Verify each level's memory remained isolated
    for i in 0..5 {
        let (_, storage) = state_changes
            .get(&addrs[i])
            .unwrap_or_else(|| panic!("Level {} should have state changes", i));

        let mem_val = storage.get(&U256::ZERO)
            .unwrap_or_else(|| panic!("Level {} slot 0 should exist", i));

        assert_eq!(
            *mem_val, values[i],
            "Level {} memory should be {:#x}, got {:#x}",
            i, values[i], mem_val
        );
    }
}
