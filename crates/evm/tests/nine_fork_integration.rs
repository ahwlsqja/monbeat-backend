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

// ─── MIP-4: Reserve Balance Precompile Integration Tests ──────────────────

/// Builds a contract that STATICCALLs the MIP-4 precompile at address 0x20
/// with an ABI-encoded target address, then SSTOREs the result.
///
/// The contract:
/// 1. MSTOREs the ABI-encoded target address at memory offset 0
/// 2. STATICCALLs 0x20 with 32 bytes of input, receives 32 bytes of output
/// 3. SSTOREs the STATICCALL success flag at slot 1
/// 4. MCOPYs the return data from returndata buffer to memory (via RETURNDATACOPY)
/// 5. MLOADs the 32-byte result from memory offset 64
/// 6. SSTOREs the result at slot 0
/// 7. STOPs
fn build_mip4_caller(target_address: Address) -> Vec<u8> {
    let mut code = Vec::new();

    // Step 1: MSTORE the ABI-encoded address at memory offset 0
    // The ABI encoding is: 12 zero bytes + 20 address bytes = 32 bytes
    // We can PUSH32 the entire 32-byte word
    let mut abi_encoded = [0u8; 32];
    abi_encoded[12..32].copy_from_slice(target_address.as_slice());

    code.push(0x7F); // PUSH32
    code.extend_from_slice(&abi_encoded);
    code.push(0x60); code.push(0x00); // PUSH1 0 (memory offset)
    code.push(0x52); // MSTORE

    // Step 2: STATICCALL to 0x20
    // STATICCALL(gas, addr, argsOffset, argsLength, retOffset, retLength)
    // Stack order (bottom to top): gas, addr, argsOff, argsLen, retOff, retLen
    // But EVM pops from top: retLen, retOff, argsLen, argsOff, addr, gas
    code.push(0x60); code.push(0x20); // PUSH1 32 = retLength (32 bytes)
    code.push(0x60); code.push(0x40); // PUSH1 64 = retOffset (write at offset 64)
    code.push(0x60); code.push(0x20); // PUSH1 32 = argsLength (32 bytes)
    code.push(0x60); code.push(0x00); // PUSH1 0 = argsOffset
    code.push(0x73);                  // PUSH20 address 0x20
    code.extend_from_slice(Address::with_last_byte(0x20).as_slice());
    code.push(0x62); code.push(0x01); code.push(0x86); code.push(0xA0); // PUSH3 100000 = gas
    code.push(0xFA); // STATICCALL

    // Step 3: SSTORE call success at slot 1
    code.push(0x60); code.push(0x01); // PUSH1 1 = slot
    code.push(0x55); // SSTORE

    // Step 4: MLOAD the result from offset 64 (where STATICCALL wrote the return data)
    code.push(0x60); code.push(0x40); // PUSH1 64
    code.push(0x51); // MLOAD

    // Step 5: SSTORE result at slot 0
    code.push(0x60); code.push(0x00); // PUSH1 0 = slot
    code.push(0x55); // SSTORE

    code.push(0x00); // STOP
    code
}

/// MIP-4 Integration: STATICCALL to precompile 0x20 for an address that has NOT dipped.
/// Should return 0x01 (safe / not dipped).
#[test]
fn nine_fork_mip4_reserve_not_dipped() {
    use monad_nine_fork::mip4_reserve::reset_dipped_tracker;
    use monad_nine_fork::nine_fork_precompiles::execute_with_nine_fork_precompiles;

    // Reset the dipped tracker for clean test state
    reset_dipped_tracker();

    let target_addr = Address::with_last_byte(0xAA);
    let caller_addr = Address::with_last_byte(0x50);

    let state = InMemoryState::new()
        .with_account(sender(), AccountInfo::new(U256::from(10_000_000_000u64), 0))
        .with_account(caller_addr, account_with_code(build_mip4_caller(target_addr)));

    let tx = Transaction {
        sender: sender(),
        to: Some(caller_addr),
        value: U256::ZERO,
        data: Bytes::new(),
        gas_limit: 1_000_000,
        nonce: 0,
        gas_price: U256::ZERO,
    };

    let (result, state_changes) =
        execute_with_nine_fork_precompiles(&tx, &state, &BlockEnv::default())
            .expect("MIP-4 integration test should not error");

    assert!(result.is_success(), "TX should succeed: {:?}", result);

    let (_, storage) = state_changes
        .get(&caller_addr)
        .expect("Caller should have state changes");

    // STATICCALL should have succeeded (slot 1 = 1)
    let call_ok = storage.get(&U256::from(1u64)).expect("Slot 1 should exist");
    assert_eq!(
        *call_ok,
        U256::from(1u64),
        "STATICCALL to MIP-4 precompile should succeed (got {:#x})",
        call_ok
    );

    // Result should be 0x01 (NOT dipped = safe)
    let reserve_result = storage.get(&U256::ZERO).expect("Slot 0 should exist");
    assert_eq!(
        *reserve_result,
        U256::from(1u64),
        "Address that has NOT dipped should return 0x01, got {:#x}",
        reserve_result
    );
}

/// MIP-4 Integration: STATICCALL to precompile 0x20 for an address that HAS dipped.
/// Should return 0x00 (dipped / not safe).
#[test]
fn nine_fork_mip4_reserve_dipped() {
    use monad_nine_fork::mip4_reserve::{mark_address_dipped, reset_dipped_tracker};
    use monad_nine_fork::nine_fork_precompiles::execute_with_nine_fork_precompiles;

    // Reset tracker and mark the target as dipped
    reset_dipped_tracker();
    let target_addr = Address::with_last_byte(0xBB);
    mark_address_dipped(target_addr);

    let caller_addr = Address::with_last_byte(0x51);

    let state = InMemoryState::new()
        .with_account(sender(), AccountInfo::new(U256::from(10_000_000_000u64), 0))
        .with_account(caller_addr, account_with_code(build_mip4_caller(target_addr)));

    let tx = Transaction {
        sender: sender(),
        to: Some(caller_addr),
        value: U256::ZERO,
        data: Bytes::new(),
        gas_limit: 1_000_000,
        nonce: 0,
        gas_price: U256::ZERO,
    };

    let (result, state_changes) =
        execute_with_nine_fork_precompiles(&tx, &state, &BlockEnv::default())
            .expect("MIP-4 dipped integration test should not error");

    assert!(result.is_success(), "TX should succeed: {:?}", result);

    let (_, storage) = state_changes
        .get(&caller_addr)
        .expect("Caller should have state changes");

    // STATICCALL should have succeeded
    let call_ok = storage.get(&U256::from(1u64)).expect("Slot 1 should exist");
    assert_eq!(
        *call_ok,
        U256::from(1u64),
        "STATICCALL to MIP-4 precompile should succeed"
    );

    // Result should be 0x00 (HAS dipped)
    let reserve_result = storage.get(&U256::ZERO).expect("Slot 0 should exist");
    assert_eq!(
        *reserve_result,
        U256::ZERO,
        "Address that HAS dipped should return 0x00, got {:#x}",
        reserve_result
    );
}
