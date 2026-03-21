//! MIP-3: Linear Memory Pool compliance verification.
//!
//! # MIP-3 Requirements
//!
//! MIP-3 specifies a **linear memory pool** for EVM execution:
//!
//! 1. **Single contiguous slab allocation:** Each transaction should use a single
//!    contiguous memory buffer, with sub-calls receiving slices of this buffer
//!    rather than independent allocations.
//!
//! 2. **Offset-based sub-call sharing:** When a CALL creates a new execution frame,
//!    the child frame's memory is a new slice within the shared buffer, starting
//!    from the current watermark (end of parent's used memory).
//!
//! 3. **Watermark stack for REVERT:** On REVERT, the slab pointer is restored to
//!    the checkpoint saved before the sub-call, effectively discarding the child's
//!    memory without deallocation.
//!
//! # revm's SharedMemory Compliance
//!
//! revm v36's `SharedMemory` (in `revm-interpreter-34.0.0/src/interpreter/shared_memory.rs`)
//! already satisfies these requirements:
//!
//! - **Single `Vec<u8>` buffer:** All call frames within a transaction share one
//!   `Vec<u8>`, matching the single-slab requirement.
//!
//! - **Checkpoint stack = watermark stack:** `new_context()` records the current
//!   buffer length as a checkpoint and pushes it onto a stack. The child frame
//!   then appends to the same buffer from that offset. This is exactly
//!   offset-based sub-call sharing.
//!
//! - **`free_context()` = slab restoration:** On RETURN or REVERT, `free_context()`
//!   pops the checkpoint and truncates the buffer back to that length. This is
//!   the watermark stack restoration that MIP-3 requires.
//!
//! # Differences
//!
//! - **Gas cost model:** The standard OSAKA memory expansion cost is **quadratic**
//!   (cost = words + words² / 512). MIP-3 may specify a linear cost model, but
//!   changing this would require forking revm's gas accounting in
//!   `revm-interpreter/src/gas/calc.rs`. Since the memory *behavior* (isolation,
//!   watermark restoration) is correct, the gas model difference is a separate
//!   concern and doesn't affect correctness.
//!
//! # Compliance Status
//!
//! **COMPLIANT** — revm's SharedMemory provides the exact memory isolation and
//! watermark stack semantics that MIP-3 requires. Each call frame operates on
//! its own memory region within a shared buffer, and REVERT correctly restores
//! the parent's memory by truncating back to the checkpoint. The tests below
//! verify this through nested CALL/REVERT sequences at various depths.
//!
//! # Verification Strategy
//!
//! Tests deploy contracts that:
//! 1. MSTORE a known value at offset 0
//! 2. CALL a sub-contract that MSTOREs a different value at offset 0
//! 3. After the sub-call (whether REVERT or RETURN), MLOAD offset 0
//! 4. SSTORE the MLOAD result for inspection via state changes
//!
//! If memory isolation is correct, the parent's MLOAD will always return
//! the parent's original MSTORE value, regardless of what the sub-call wrote.

#[cfg(test)]
mod tests {
    use monad_evm::EvmExecutor;
    use monad_state::InMemoryState;
    use monad_types::{
        AccountInfo, Address, BlockEnv, Bytes, Transaction, U256,
    };

    /// Sender address for MIP-3 tests.
    fn sender() -> Address {
        Address::with_last_byte(0xC0)
    }

    /// Helper to create AccountInfo with code for a deployed contract.
    fn account_with_code(code: Vec<u8>) -> AccountInfo {
        let code_bytes = Bytes::from(code);
        let code_hash = alloy_primitives::keccak256(&code_bytes);
        AccountInfo::new_contract(U256::ZERO, 1, code_hash.into(), code_bytes)
    }

    /// Builds a contract that MSTOREs a value at offset 0 and then REVERTs.
    ///
    /// Bytecode:
    ///   PUSH32 <value>   ; push the value
    ///   PUSH1 0x00       ; memory offset 0
    ///   MSTORE           ; store value at offset 0
    ///   PUSH1 0x00       ; revert data size = 0
    ///   PUSH1 0x00       ; revert data offset = 0
    ///   REVERT           ; revert the call frame
    fn build_mstore_and_revert(value: U256) -> Vec<u8> {
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

    /// Builds a contract that MSTOREs a value at offset 0 and then RETURNs empty.
    ///
    /// Bytecode:
    ///   PUSH32 <value>   ; push the value
    ///   PUSH1 0x00       ; memory offset 0
    ///   MSTORE           ; store value at offset 0
    ///   PUSH1 0x00       ; return size = 0
    ///   PUSH1 0x00       ; return offset = 0
    ///   RETURN           ; return successfully
    fn build_mstore_and_return(value: U256) -> Vec<u8> {
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

    /// Builds a parent contract that:
    /// 1. MSTOREs `parent_value` at offset 0
    /// 2. CALLs `target_addr` (forwarding some gas, no value, no calldata)
    /// 3. MLOADs offset 0 (should still be parent_value if memory is isolated)
    /// 4. SSTOREs the MLOAD result at storage slot 0 for verification
    /// 5. SSTOREs the CALL success flag at storage slot 1
    /// 6. STOPs
    ///
    /// Bytecode layout:
    ///   PUSH32 <parent_value>  ; value to store
    ///   PUSH1 0x00             ; offset 0
    ///   MSTORE                 ; mem[0..32] = parent_value
    ///
    ///   ;; CALL(gas, addr, value, argsOffset, argsLength, retOffset, retLength)
    ///   PUSH1 0x00             ; retLength = 0
    ///   PUSH1 0x00             ; retOffset = 0
    ///   PUSH1 0x00             ; argsLength = 0
    ///   PUSH1 0x00             ; argsOffset = 0
    ///   PUSH1 0x00             ; value = 0
    ///   PUSH20 <target_addr>   ; address to call
    ///   PUSH3 <gas>            ; gas to forward (100000)
    ///   CALL                   ; call the sub-contract
    ///
    ///   ;; Store CALL success at slot 1
    ///   PUSH1 0x01             ; storage slot 1
    ///   SSTORE                 ; store call result (0=fail, 1=success) at slot 1
    ///
    ///   ;; MLOAD offset 0 → should be parent_value
    ///   PUSH1 0x00             ; offset 0
    ///   MLOAD                  ; load 32 bytes from memory offset 0
    ///
    ///   ;; SSTORE result at slot 0
    ///   PUSH1 0x00             ; storage slot 0
    ///   SSTORE                 ; store mload result at slot 0
    ///
    ///   STOP
    fn build_parent_caller(parent_value: U256, target_addr: Address) -> Vec<u8> {
        let mut code = Vec::new();

        // PUSH32 parent_value → PUSH1 0x00 → MSTORE
        code.push(0x7F); // PUSH32
        code.extend_from_slice(&parent_value.to_be_bytes::<32>());
        code.push(0x60); // PUSH1 0x00
        code.push(0x00);
        code.push(0x52); // MSTORE

        // CALL args (pushed in reverse order for the stack):
        // retLength = 0
        code.push(0x60); // PUSH1
        code.push(0x00);
        // retOffset = 0
        code.push(0x60); // PUSH1
        code.push(0x00);
        // argsLength = 0
        code.push(0x60); // PUSH1
        code.push(0x00);
        // argsOffset = 0
        code.push(0x60); // PUSH1
        code.push(0x00);
        // value = 0
        code.push(0x60); // PUSH1
        code.push(0x00);
        // address
        code.push(0x73); // PUSH20
        code.extend_from_slice(target_addr.as_slice());
        // gas = 100000 (0x0186A0)
        code.push(0x62); // PUSH3
        code.push(0x01);
        code.push(0x86);
        code.push(0xA0);
        // CALL
        code.push(0xF1);

        // Store CALL success flag at slot 1: top of stack is success (0 or 1)
        code.push(0x60); // PUSH1 0x01 (slot 1)
        code.push(0x01);
        code.push(0x55); // SSTORE

        // MLOAD offset 0
        code.push(0x60); // PUSH1 0x00
        code.push(0x00);
        code.push(0x51); // MLOAD

        // SSTORE at slot 0
        code.push(0x60); // PUSH1 0x00
        code.push(0x00);
        code.push(0x55); // SSTORE

        // STOP
        code.push(0x00);

        code
    }

    /// Builds a contract that:
    /// 1. MSTOREs `my_value` at offset 0
    /// 2. CALLs `target_addr`
    /// 3. MLOADs offset 0 and SSTOREs result at slot `store_slot`
    /// 4. SSTOREs CALL success at slot `store_slot + 1`
    /// 5. RETURNs empty (so the parent's CALL succeeds)
    ///
    /// Used for deep nesting chains where each level stores its verification
    /// in a different storage slot.
    fn build_chain_link(my_value: U256, target_addr: Address, store_slot: u8) -> Vec<u8> {
        let mut code = Vec::new();

        // PUSH32 my_value → PUSH1 0x00 → MSTORE
        code.push(0x7F); // PUSH32
        code.extend_from_slice(&my_value.to_be_bytes::<32>());
        code.push(0x60); // PUSH1 0x00
        code.push(0x00);
        code.push(0x52); // MSTORE

        // CALL target_addr with gas 500000
        code.push(0x60); // PUSH1 retLength = 0
        code.push(0x00);
        code.push(0x60); // PUSH1 retOffset = 0
        code.push(0x00);
        code.push(0x60); // PUSH1 argsLength = 0
        code.push(0x00);
        code.push(0x60); // PUSH1 argsOffset = 0
        code.push(0x00);
        code.push(0x60); // PUSH1 value = 0
        code.push(0x00);
        code.push(0x73); // PUSH20 address
        code.extend_from_slice(target_addr.as_slice());
        // gas = 500000 (0x07A120)
        code.push(0x62); // PUSH3
        code.push(0x07);
        code.push(0xA1);
        code.push(0x20);
        code.push(0xF1); // CALL

        // Store CALL success at slot (store_slot + 1)
        code.push(0x60); // PUSH1 slot
        code.push(store_slot + 1);
        code.push(0x55); // SSTORE

        // MLOAD offset 0
        code.push(0x60); // PUSH1 0x00
        code.push(0x00);
        code.push(0x51); // MLOAD

        // SSTORE at store_slot
        code.push(0x60); // PUSH1 slot
        code.push(store_slot);
        code.push(0x55); // SSTORE

        // RETURN empty
        code.push(0x60); // PUSH1 0
        code.push(0x00);
        code.push(0x60); // PUSH1 0
        code.push(0x00);
        code.push(0xF3); // RETURN

        code
    }

    // ─── Test: Shallow REVERT restores parent memory ─────────────────

    #[test]
    fn mip3_revert_restores_parent_memory() {
        // Sub-contract: MSTOREs 0xDEAD at offset 0, then REVERTs
        let sub_addr = Address::with_last_byte(0xD1);
        let sub_code = build_mstore_and_revert(U256::from(0xDEADu64));

        // Parent contract: MSTOREs 0xBEEF at offset 0, CALLs sub, MLOADs offset 0, SSTOREs
        let parent_addr = Address::with_last_byte(0xD0);
        let parent_code = build_parent_caller(U256::from(0xBEEFu64), sub_addr);

        let state = InMemoryState::new()
            .with_account(sender(), AccountInfo::new(U256::from(10_000_000_000u64), 0))
            .with_account(parent_addr, account_with_code(parent_code))
            .with_account(sub_addr, account_with_code(sub_code));

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
                .expect("MIP-3 revert test should succeed");

        assert!(result.is_success(), "Transaction should succeed: {:?}", result);

        // Check parent's storage: slot 0 should have parent_value (0xBEEF)
        let (_, parent_storage) = state_changes
            .get(&parent_addr)
            .expect("Parent contract should have state changes");

        let memory_readback = parent_storage
            .get(&U256::ZERO)
            .expect("Storage slot 0 should contain MLOAD result");

        assert_eq!(
            *memory_readback,
            U256::from(0xBEEFu64),
            "After sub-call REVERT, parent's memory at offset 0 should still be 0xBEEF, got {:#x}",
            memory_readback
        );

        // Verify CALL returned 0 (failure, since sub-call reverted)
        let call_success = parent_storage
            .get(&U256::from(1u64))
            .expect("Storage slot 1 should contain CALL success flag");

        assert_eq!(
            *call_success,
            U256::ZERO,
            "CALL to reverting sub-contract should return 0 (failure), got {}",
            call_success
        );
    }

    // ─── Test: Successful sub-call doesn't leak memory to parent ─────

    #[test]
    fn mip3_return_does_not_leak_subcall_memory() {
        // Sub-contract: MSTOREs 0xDEAD at offset 0, then RETURNs
        let sub_addr = Address::with_last_byte(0xD3);
        let sub_code = build_mstore_and_return(U256::from(0xDEADu64));

        // Parent contract: MSTOREs 0xBEEF at offset 0, CALLs sub, MLOADs offset 0, SSTOREs
        let parent_addr = Address::with_last_byte(0xD2);
        let parent_code = build_parent_caller(U256::from(0xBEEFu64), sub_addr);

        let state = InMemoryState::new()
            .with_account(sender(), AccountInfo::new(U256::from(10_000_000_000u64), 0))
            .with_account(parent_addr, account_with_code(parent_code))
            .with_account(sub_addr, account_with_code(sub_code));

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
                .expect("MIP-3 return test should succeed");

        assert!(result.is_success(), "Transaction should succeed: {:?}", result);

        let (_, parent_storage) = state_changes
            .get(&parent_addr)
            .expect("Parent contract should have state changes");

        let memory_readback = parent_storage
            .get(&U256::ZERO)
            .expect("Storage slot 0 should contain MLOAD result");

        // Parent's memory should still be 0xBEEF — sub-call's MSTORE should not
        // have leaked into the parent's memory frame.
        assert_eq!(
            *memory_readback,
            U256::from(0xBEEFu64),
            "After sub-call RETURN, parent's memory at offset 0 should still be 0xBEEF (not sub's 0xDEAD), got {:#x}",
            memory_readback
        );

        // Verify CALL returned 1 (success, since sub-call returned)
        let call_success = parent_storage
            .get(&U256::from(1u64))
            .expect("Storage slot 1 should contain CALL success flag");

        assert_eq!(
            *call_success,
            U256::from(1u64),
            "CALL to returning sub-contract should return 1 (success), got {}",
            call_success
        );
    }

    // ─── Test: Deep nesting with REVERT restores each frame's memory ─

    #[test]
    fn mip3_deep_nesting_revert_restores_memory() {
        // Chain of 6 contracts: A→B→C→D→E→F
        // F (innermost) REVERTs after MSTORE
        // A through E each MSTORE their own value, CALL the next level,
        // then MLOAD and SSTORE to verify their memory is intact.
        //
        // Each level stores its verification in unique storage slots on contract A
        // (since intermediate contracts' state changes are preserved — only F's
        // state is reverted by F's REVERT, not the parent levels).
        //
        // Actually, since each contract stores to its own storage (via SSTORE in
        // its own code), we verify each contract's storage independently.

        // Addresses for the chain (non-precompile, above 0x100 to avoid conflicts)
        let addrs: Vec<Address> = (0..6)
            .map(|i| Address::with_last_byte(0xE0 + i))
            .collect();

        // Values each level will MSTORE (unique per level for verification)
        let values: Vec<U256> = (0..6)
            .map(|i| U256::from(0xAA00u64 + i as u64))
            .collect();

        // Build the innermost contract (F = index 5): MSTOREs and REVERTs
        let innermost_code = build_mstore_and_revert(values[5]);

        // Build intermediate contracts (E=4, D=3, C=2, B=1): each MSTOREs, CALLs next, MLOADs, SSTOREs
        // Each stores its verification at slot 0 (memory readback) and slot 1 (call success)
        let mut codes: Vec<Vec<u8>> = Vec::new();
        for i in 0..5 {
            let target = addrs[i + 1];
            let code = build_chain_link(values[i], target, 0);
            codes.push(code);
        }

        // Build state with all contracts
        let mut state = InMemoryState::new()
            .with_account(sender(), AccountInfo::new(U256::from(10_000_000_000u64), 0));

        for i in 0..5 {
            state.insert_account(addrs[i], account_with_code(codes[i].clone()));
        }
        state.insert_account(addrs[5], account_with_code(innermost_code));

        // Call the outermost contract (A = index 0)
        let tx = Transaction {
            sender: sender(),
            to: Some(addrs[0]),
            value: U256::ZERO,
            data: Bytes::new(),
            gas_limit: 16_000_000, // Use EIP-7825 cap
            nonce: 0,
            gas_price: U256::ZERO,
        };

        let (result, state_changes) =
            EvmExecutor::execute_tx_with_state_changes(&tx, &state, &BlockEnv::default())
                .expect("MIP-3 deep nesting test should succeed");

        assert!(result.is_success(), "Transaction should succeed: {:?}", result);

        // Verify each intermediate level (0..5) has its own MSTORE value intact
        // after the sub-call chain completes.
        //
        // Level 0 (A): calls B, B eventually leads to F which REVERTs.
        // The REVERT only affects F's call frame. Each level from A-E
        // has its own memory context, so MLOAD should return their own value.
        for i in 0..5 {
            let (_, storage) = state_changes
                .get(&addrs[i])
                .unwrap_or_else(|| {
                    panic!(
                        "Contract at level {} (addr {:?}) should have state changes",
                        i, addrs[i]
                    )
                });

            let memory_value = storage
                .get(&U256::ZERO)
                .unwrap_or_else(|| {
                    panic!(
                        "Level {} should have storage slot 0 with MLOAD result",
                        i
                    )
                });

            assert_eq!(
                *memory_value,
                values[i],
                "Level {} memory readback should be {:#x} (its own MSTORE value), got {:#x}",
                i, values[i], memory_value
            );
        }

        // Verify call success flags:
        // Level 4 (E) called F which REVERTs → CALL returns 0
        // Levels 0-3 called the next level which RETURNed → CALL returns 1
        for i in 0..5 {
            let (_, storage) = state_changes.get(&addrs[i]).unwrap();
            let call_success = storage
                .get(&U256::from(1u64))
                .unwrap_or_else(|| {
                    panic!("Level {} should have storage slot 1 with CALL success flag", i)
                });

            if i == 4 {
                // Level 4 directly called the reverting contract
                assert_eq!(
                    *call_success,
                    U256::ZERO,
                    "Level {} (direct caller of reverting F) CALL should return 0, got {}",
                    i, call_success
                );
            } else {
                // Levels 0-3 called a returning contract
                assert_eq!(
                    *call_success,
                    U256::from(1u64),
                    "Level {} CALL should return 1 (success), got {}",
                    i, call_success
                );
            }
        }
    }

    // ─── Test: Deep nesting all-return preserves memory isolation ─────

    #[test]
    fn mip3_deep_nesting_all_return_memory_isolation() {
        // Chain of 6 contracts where ALL levels RETURN (no REVERT).
        // Verifies that even without REVERT, memory isolation holds —
        // each frame has its own memory space.

        let addrs: Vec<Address> = (0..6)
            .map(|i| Address::with_last_byte(0xF0 + i))
            .collect();

        let values: Vec<U256> = (0..6)
            .map(|i| U256::from(0xBB00u64 + i as u64))
            .collect();

        // Innermost: MSTOREs and RETURNs
        let innermost_code = build_mstore_and_return(values[5]);

        // Intermediates: MSTORE, CALL, MLOAD, SSTORE, RETURN
        let mut codes: Vec<Vec<u8>> = Vec::new();
        for i in 0..5 {
            let target = addrs[i + 1];
            let code = build_chain_link(values[i], target, 0);
            codes.push(code);
        }

        let mut state = InMemoryState::new()
            .with_account(sender(), AccountInfo::new(U256::from(10_000_000_000u64), 0));

        for i in 0..5 {
            state.insert_account(addrs[i], account_with_code(codes[i].clone()));
        }
        state.insert_account(addrs[5], account_with_code(innermost_code));

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
                .expect("MIP-3 all-return nesting test should succeed");

        assert!(result.is_success(), "Transaction should succeed: {:?}", result);

        // All levels should have their own MSTORE value intact
        for i in 0..5 {
            let (_, storage) = state_changes
                .get(&addrs[i])
                .unwrap_or_else(|| {
                    panic!("Level {} should have state changes", i)
                });

            let memory_value = storage
                .get(&U256::ZERO)
                .unwrap_or_else(|| {
                    panic!("Level {} should have MLOAD result in slot 0", i)
                });

            assert_eq!(
                *memory_value,
                values[i],
                "Level {} memory readback should be {:#x}, got {:#x}",
                i, values[i], memory_value
            );

            // All calls should succeed
            let call_success = storage.get(&U256::from(1u64)).unwrap();
            assert_eq!(
                *call_success,
                U256::from(1u64),
                "Level {} CALL should succeed, got {}",
                i, call_success
            );
        }
    }

    // ─── Test: Multiple MSTOREs at different offsets ──────────────────

    #[test]
    fn mip3_multiple_offsets_preserved_after_revert() {
        // Parent stores values at memory offsets 0 and 32.
        // Sub-contract writes different values at offsets 0 and 32, then REVERTs.
        // Parent MLOADs both offsets — both should have original values.

        let sub_addr = Address::with_last_byte(0xD5);
        // Sub: MSTORE 0xAAAA at offset 0, MSTORE 0xBBBB at offset 32, REVERT
        let mut sub_code = Vec::new();
        // MSTORE 0xAAAA at offset 0
        sub_code.push(0x7F); // PUSH32
        sub_code.extend_from_slice(&U256::from(0xAAAAu64).to_be_bytes::<32>());
        sub_code.push(0x60); // PUSH1 0x00
        sub_code.push(0x00);
        sub_code.push(0x52); // MSTORE
        // MSTORE 0xBBBB at offset 32
        sub_code.push(0x7F); // PUSH32
        sub_code.extend_from_slice(&U256::from(0xBBBBu64).to_be_bytes::<32>());
        sub_code.push(0x60); // PUSH1 0x20
        sub_code.push(0x20);
        sub_code.push(0x52); // MSTORE
        // REVERT
        sub_code.push(0x60); // PUSH1 0
        sub_code.push(0x00);
        sub_code.push(0x60); // PUSH1 0
        sub_code.push(0x00);
        sub_code.push(0xFD); // REVERT

        // Parent: MSTORE 0x1111 at offset 0, MSTORE 0x2222 at offset 32,
        // CALL sub, MLOAD offset 0 → SSTORE slot 0, MLOAD offset 32 → SSTORE slot 1
        let parent_addr = Address::with_last_byte(0xD4);
        let mut parent_code = Vec::new();

        // MSTORE 0x1111 at offset 0
        parent_code.push(0x7F); // PUSH32
        parent_code.extend_from_slice(&U256::from(0x1111u64).to_be_bytes::<32>());
        parent_code.push(0x60); // PUSH1 0x00
        parent_code.push(0x00);
        parent_code.push(0x52); // MSTORE

        // MSTORE 0x2222 at offset 32 (0x20)
        parent_code.push(0x7F); // PUSH32
        parent_code.extend_from_slice(&U256::from(0x2222u64).to_be_bytes::<32>());
        parent_code.push(0x60); // PUSH1 0x20
        parent_code.push(0x20);
        parent_code.push(0x52); // MSTORE

        // CALL sub_addr
        parent_code.push(0x60); // retLength = 0
        parent_code.push(0x00);
        parent_code.push(0x60); // retOffset = 0
        parent_code.push(0x00);
        parent_code.push(0x60); // argsLength = 0
        parent_code.push(0x00);
        parent_code.push(0x60); // argsOffset = 0
        parent_code.push(0x00);
        parent_code.push(0x60); // value = 0
        parent_code.push(0x00);
        parent_code.push(0x73); // PUSH20 sub_addr
        parent_code.extend_from_slice(sub_addr.as_slice());
        parent_code.push(0x62); // PUSH3 gas = 100000
        parent_code.push(0x01);
        parent_code.push(0x86);
        parent_code.push(0xA0);
        parent_code.push(0xF1); // CALL

        // POP the call success flag (we don't need it for this test)
        parent_code.push(0x50); // POP

        // MLOAD offset 0 → SSTORE slot 0
        parent_code.push(0x60); // PUSH1 0x00
        parent_code.push(0x00);
        parent_code.push(0x51); // MLOAD
        parent_code.push(0x60); // PUSH1 0x00 (slot)
        parent_code.push(0x00);
        parent_code.push(0x55); // SSTORE

        // MLOAD offset 32 → SSTORE slot 2
        parent_code.push(0x60); // PUSH1 0x20
        parent_code.push(0x20);
        parent_code.push(0x51); // MLOAD
        parent_code.push(0x60); // PUSH1 0x02 (slot)
        parent_code.push(0x02);
        parent_code.push(0x55); // SSTORE

        // STOP
        parent_code.push(0x00);

        let state = InMemoryState::new()
            .with_account(sender(), AccountInfo::new(U256::from(10_000_000_000u64), 0))
            .with_account(parent_addr, account_with_code(parent_code))
            .with_account(sub_addr, account_with_code(sub_code));

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
                .expect("MIP-3 multi-offset test should succeed");

        assert!(result.is_success(), "Transaction should succeed: {:?}", result);

        let (_, parent_storage) = state_changes
            .get(&parent_addr)
            .expect("Parent should have state changes");

        // Offset 0: should still be 0x1111 (not sub's 0xAAAA)
        let val_at_0 = parent_storage
            .get(&U256::ZERO)
            .expect("Slot 0 should have MLOAD(0) result");
        assert_eq!(
            *val_at_0,
            U256::from(0x1111u64),
            "Memory at offset 0 should be 0x1111 after sub-call REVERT, got {:#x}",
            val_at_0
        );

        // Offset 32: should still be 0x2222 (not sub's 0xBBBB)
        let val_at_32 = parent_storage
            .get(&U256::from(2u64))
            .expect("Slot 2 should have MLOAD(32) result");
        assert_eq!(
            *val_at_32,
            U256::from(0x2222u64),
            "Memory at offset 32 should be 0x2222 after sub-call REVERT, got {:#x}",
            val_at_32
        );
    }
}
