//! # MIP-4: Reserve Balance Precompile
//!
//! MIP-4 introduces a Monad-specific precompile at address `0x20` that reports
//! whether an account has dipped into its reserve balance during the current
//! transaction context.
//!
//! ## Precompile Specification
//!
//! - **Address:** `0x0000000000000000000000000000000000000020`
//! - **Gas cost:** 100 (flat, state-reading precompile)
//! - **Input:** 32 bytes — ABI-encoded address (left-padded with 12 zero bytes)
//! - **Output:** 32 bytes — `0x01` if the address has NOT dipped into reserve
//!   (safe), `0x00` if it HAS dipped
//!
//! ## dippedIntoReserve Tracking
//!
//! For S02 (sequential-only execution), tracking uses a thread-local
//! `RefCell<HashMap<Address, bool>>`. This will evolve to per-transaction-index
//! tracking in S03/S04 for parallel execution.
//!
//! ## Init-Selfdestruct Bypass
//!
//! If a transaction is a contract creation that immediately selfdestructs,
//! the reserve balance check is skipped. This bypass prevents false positives
//! from ephemeral contract deployment patterns.
//!
//! ## Integration
//!
//! Use [`create_mip4_precompile()`] to get a `revm::precompile::Precompile`
//! ready for registration with a custom `PrecompileProvider`. The precompile
//! can also be tested standalone by calling [`reserve_balance_check()`] directly.

use alloy_primitives::{Address, Bytes};
use revm::precompile::{
    Precompile, PrecompileError, PrecompileId, PrecompileOutput, PrecompileResult,
};
use std::cell::RefCell;
use std::collections::HashMap;

/// Address of the MIP-4 reserve balance precompile.
///
/// Located at `0x20` (decimal 32), in the Monad custom precompile space,
/// outside the standard Ethereum range (0x01-0x13) and OSAKA extensions.
pub const MIP4_RESERVE_ADDRESS: Address = {
    let mut bytes = [0u8; 20];
    bytes[19] = 0x20;
    Address::new(bytes)
};

/// Gas cost for the reserve balance precompile.
///
/// Set to 100 — a reasonable cost for a state-reading precompile that
/// performs a map lookup (comparable to BALANCE opcode at 100 gas post-EIP-2929
/// warm access).
pub const MIP4_GAS_COST: u64 = 100;

/// Required input length: 32 bytes (ABI-encoded address, left-padded).
pub const MIP4_INPUT_LENGTH: usize = 32;

/// Configuration for the reserve balance system.
///
/// Holds the reserve balance threshold per account. In S02, this is a simple
/// fixed threshold. In production, this would be derived from account-specific
/// reserve requirements.
#[derive(Debug, Clone)]
pub struct ReserveBalanceConfig {
    /// The default reserve balance threshold in wei.
    /// Accounts with balance below this are considered to have "dipped".
    pub default_reserve_threshold: u64,
}

impl Default for ReserveBalanceConfig {
    fn default() -> Self {
        Self {
            // Default: 1 ETH reserve threshold
            default_reserve_threshold: 1_000_000_000_000_000_000,
        }
    }
}

/// Per-transaction tracking of which addresses have dipped into their reserve.
///
/// For S02 (sequential execution), this uses a thread-local `RefCell<HashMap>`.
/// S03/S04 will replace this with per-transaction-index tracking for parallel
/// execution safety.
#[derive(Debug, Default)]
pub struct DippedIntoReserve {
    /// Maps address → whether it has dipped into reserve balance.
    inner: RefCell<HashMap<Address, bool>>,
}

impl DippedIntoReserve {
    /// Creates a new empty tracker.
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(HashMap::new()),
        }
    }

    /// Records that an address has dipped into its reserve balance.
    pub fn mark_dipped(&self, address: Address) {
        self.inner.borrow_mut().insert(address, true);
    }

    /// Checks whether an address has dipped into its reserve balance.
    ///
    /// Returns `true` if the address has dipped, `false` if it hasn't
    /// (or hasn't been tracked).
    pub fn has_dipped(&self, address: &Address) -> bool {
        self.inner.borrow().get(address).copied().unwrap_or(false)
    }

    /// Resets the tracker for a new transaction.
    pub fn reset(&self) {
        self.inner.borrow_mut().clear();
    }
}

// Thread-local tracker for the current transaction context.
// In S02, all execution is sequential, so thread-local is safe.
thread_local! {
    static DIPPED_TRACKER: DippedIntoReserve = DippedIntoReserve::new();
}

/// Marks an address as having dipped into its reserve balance.
///
/// Called by the execution pipeline when a balance transfer causes an account's
/// balance to drop below the reserve threshold.
pub fn mark_address_dipped(address: Address) {
    DIPPED_TRACKER.with(|tracker| tracker.mark_dipped(address));
}

/// Queries whether an address has dipped into its reserve balance.
///
/// Returns `true` if dipped, `false` if safe.
pub fn has_address_dipped(address: &Address) -> bool {
    DIPPED_TRACKER.with(|tracker| tracker.has_dipped(address))
}

/// Resets the dipped-into-reserve tracker for a new transaction.
///
/// Must be called before each transaction to ensure clean state.
pub fn reset_dipped_tracker() {
    DIPPED_TRACKER.with(|tracker| tracker.reset());
}

/// Decodes an ABI-encoded address from 32 bytes of input.
///
/// The address occupies the last 20 bytes of the 32-byte word.
/// The first 12 bytes must be zero (standard ABI encoding for `address` type).
fn decode_abi_address(input: &[u8]) -> Result<Address, PrecompileError> {
    if input.len() != MIP4_INPUT_LENGTH {
        return Err(PrecompileError::other(format!(
            "MIP-4: invalid input length {}, expected {}",
            input.len(),
            MIP4_INPUT_LENGTH
        )));
    }

    // First 12 bytes should be zero-padding for ABI-encoded address
    // We don't strictly enforce zero-padding (matching Solidity ABI decoder behavior),
    // but we extract from the canonical position.
    let mut addr_bytes = [0u8; 20];
    addr_bytes.copy_from_slice(&input[12..32]);
    Ok(Address::new(addr_bytes))
}

/// Encodes a boolean result as a 32-byte ABI-encoded value.
///
/// Returns `0x00...01` for `true` and `0x00...00` for `false`.
fn encode_abi_bool(value: bool) -> Bytes {
    let mut output = [0u8; 32];
    if value {
        output[31] = 1;
    }
    Bytes::copy_from_slice(&output)
}

/// The MIP-4 reserve balance precompile function.
///
/// Takes 32 bytes of input (ABI-encoded address), checks whether the address
/// has dipped into its reserve balance, and returns 32 bytes:
/// - `0x01` if the address has NOT dipped (safe)
/// - `0x00` if the address HAS dipped
///
/// Gas cost: 100 (flat).
///
/// # Errors
///
/// Returns `PrecompileError::OutOfGas` if gas_limit < 100.
/// Returns `PrecompileError::Other` if input length != 32.
pub fn reserve_balance_check(input: &[u8], gas_limit: u64) -> PrecompileResult {
    // Check gas
    if gas_limit < MIP4_GAS_COST {
        return Err(PrecompileError::OutOfGas);
    }

    // Decode the target address from ABI-encoded input
    let target_address = decode_abi_address(input)?;

    // Check if the address has dipped into reserve
    let has_dipped = has_address_dipped(&target_address);

    // Return: 0x01 = NOT dipped (safe), 0x00 = HAS dipped
    let result_bool = !has_dipped;
    let output = encode_abi_bool(result_bool);

    Ok(PrecompileOutput::new(MIP4_GAS_COST, output))
}

/// Creates the MIP-4 reserve balance precompile ready for registration.
///
/// Returns a `Precompile` instance with:
/// - ID: `PrecompileId::Custom("mip4_reserve")`
/// - Address: `0x0000...0020`
/// - Function: `reserve_balance_check`
///
/// # Example
///
/// ```ignore
/// use monad_nine_fork::mip4_reserve::create_mip4_precompile;
///
/// let precompile = create_mip4_precompile();
/// assert_eq!(*precompile.address(), MIP4_RESERVE_ADDRESS);
/// ```
pub fn create_mip4_precompile() -> Precompile {
    Precompile::new(
        PrecompileId::custom("mip4_reserve"),
        MIP4_RESERVE_ADDRESS,
        reserve_balance_check,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: ABI-encode an address into 32 bytes (left-padded with zeros).
    fn abi_encode_address(addr: &Address) -> Vec<u8> {
        let mut encoded = vec![0u8; 12]; // 12 zero bytes of padding
        encoded.extend_from_slice(addr.as_slice());
        assert_eq!(encoded.len(), 32);
        encoded
    }

    #[test]
    fn mip4_precompile_address_is_0x20() {
        assert_eq!(
            MIP4_RESERVE_ADDRESS,
            Address::with_last_byte(0x20),
            "MIP-4 precompile should be at address 0x20"
        );
    }

    #[test]
    fn mip4_valid_input_not_dipped_returns_true() {
        // Reset tracker for clean state
        reset_dipped_tracker();

        let addr = Address::with_last_byte(0xAA);
        let input = abi_encode_address(&addr);

        let result = reserve_balance_check(&input, 200).expect("should succeed");

        assert_eq!(result.gas_used, MIP4_GAS_COST, "gas used should be {}", MIP4_GAS_COST);

        // Not dipped → returns 0x01 (true = safe)
        let mut expected = [0u8; 32];
        expected[31] = 1;
        assert_eq!(
            result.bytes.as_ref(),
            &expected,
            "Address that has NOT dipped should return 0x01"
        );
    }

    #[test]
    fn mip4_valid_input_dipped_returns_false() {
        // Reset tracker and mark an address as dipped
        reset_dipped_tracker();

        let addr = Address::with_last_byte(0xBB);
        mark_address_dipped(addr);

        let input = abi_encode_address(&addr);
        let result = reserve_balance_check(&input, 200).expect("should succeed");

        assert_eq!(result.gas_used, MIP4_GAS_COST);

        // Has dipped → returns 0x00 (false = not safe)
        let expected = [0u8; 32];
        assert_eq!(
            result.bytes.as_ref(),
            &expected,
            "Address that HAS dipped should return 0x00"
        );
    }

    #[test]
    fn mip4_invalid_input_length_too_short() {
        let short_input = vec![0u8; 20]; // 20 bytes instead of 32
        let result = reserve_balance_check(&short_input, 200);

        assert!(result.is_err(), "Should error on short input");
        match result.unwrap_err() {
            PrecompileError::Other(msg) => {
                assert!(
                    msg.contains("invalid input length"),
                    "Error should mention invalid input length, got: {}",
                    msg
                );
            }
            other => panic!("Expected Other error, got: {:?}", other),
        }
    }

    #[test]
    fn mip4_invalid_input_length_too_long() {
        let long_input = vec![0u8; 64]; // 64 bytes instead of 32
        let result = reserve_balance_check(&long_input, 200);

        assert!(result.is_err(), "Should error on long input");
    }

    #[test]
    fn mip4_invalid_input_length_empty() {
        let empty_input: Vec<u8> = vec![];
        let result = reserve_balance_check(&empty_input, 200);

        assert!(result.is_err(), "Should error on empty input");
    }

    #[test]
    fn mip4_out_of_gas() {
        reset_dipped_tracker();

        let addr = Address::with_last_byte(0xCC);
        let input = abi_encode_address(&addr);

        // Provide less gas than required
        let result = reserve_balance_check(&input, MIP4_GAS_COST - 1);
        assert!(result.is_err(), "Should error with insufficient gas");
        assert_eq!(
            result.unwrap_err(),
            PrecompileError::OutOfGas,
            "Should be OutOfGas error"
        );
    }

    #[test]
    fn mip4_gas_cost_exactly_sufficient() {
        reset_dipped_tracker();

        let addr = Address::with_last_byte(0xDD);
        let input = abi_encode_address(&addr);

        // Provide exactly the required gas
        let result = reserve_balance_check(&input, MIP4_GAS_COST);
        assert!(result.is_ok(), "Should succeed with exact gas: {:?}", result.err());
        assert_eq!(result.unwrap().gas_used, MIP4_GAS_COST);
    }

    #[test]
    fn mip4_abi_encoding_roundtrip() {
        let original = Address::with_last_byte(0xEE);
        let encoded = abi_encode_address(&original);
        let decoded = decode_abi_address(&encoded).expect("should decode");
        assert_eq!(decoded, original, "ABI encoding/decoding should roundtrip");
    }

    #[test]
    fn mip4_abi_encoding_full_address() {
        // Test with a full non-trivial address
        let addr = Address::new([
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
            0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10,
            0xDE, 0xAD, 0xBE, 0xEF,
        ]);
        let encoded = abi_encode_address(&addr);
        let decoded = decode_abi_address(&encoded).expect("should decode");
        assert_eq!(decoded, addr, "Full address should roundtrip through ABI encoding");
    }

    #[test]
    fn mip4_tracker_reset_clears_state() {
        reset_dipped_tracker();

        let addr = Address::with_last_byte(0xFF);
        mark_address_dipped(addr);
        assert!(has_address_dipped(&addr), "Should be marked as dipped");

        reset_dipped_tracker();
        assert!(!has_address_dipped(&addr), "Should be cleared after reset");
    }

    #[test]
    fn mip4_multiple_addresses_independent() {
        reset_dipped_tracker();

        let addr1 = Address::with_last_byte(0x01);
        let addr2 = Address::with_last_byte(0x02);
        let addr3 = Address::with_last_byte(0x03);

        // Mark only addr2 as dipped
        mark_address_dipped(addr2);

        assert!(!has_address_dipped(&addr1), "addr1 should not be dipped");
        assert!(has_address_dipped(&addr2), "addr2 should be dipped");
        assert!(!has_address_dipped(&addr3), "addr3 should not be dipped");
    }

    #[test]
    fn mip4_create_precompile_has_correct_properties() {
        let precompile = create_mip4_precompile();
        assert_eq!(
            *precompile.address(),
            MIP4_RESERVE_ADDRESS,
            "Precompile should be at address 0x20"
        );
        assert_eq!(
            *precompile.id(),
            PrecompileId::custom("mip4_reserve"),
            "Precompile ID should be 'mip4_reserve'"
        );
    }

    #[test]
    fn mip4_precompile_execute_via_precompile_api() {
        // Test calling through the Precompile::execute API (same as revm would)
        reset_dipped_tracker();

        let precompile = create_mip4_precompile();
        let addr = Address::with_last_byte(0x77);
        let input = abi_encode_address(&addr);

        let result = precompile.execute(&input, 200).expect("should succeed");
        assert_eq!(result.gas_used, MIP4_GAS_COST);

        // Not dipped → should return 0x01
        assert_eq!(result.bytes[31], 1, "Not-dipped should return 0x01");
    }

    #[test]
    fn mip4_config_default() {
        let config = ReserveBalanceConfig::default();
        assert_eq!(
            config.default_reserve_threshold,
            1_000_000_000_000_000_000,
            "Default threshold should be 1 ETH in wei"
        );
    }
}
