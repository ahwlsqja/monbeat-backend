//! Bridge between our `StateProvider` trait and revm's `Database` trait.
//!
//! `DbBridge` wraps a reference to any `StateProvider` implementation and adapts
//! it to revm's `Database` interface, handling type conversions between our types
//! and revm's types.

use monad_state::StateProvider;
use monad_types::{Address, EvmError, B256, U256};

use revm::{
    database_interface::{DBErrorMarker, Database},
    state::{AccountInfo as RevmAccountInfo, Bytecode},
};

/// Error type for the database bridge, wrapping our `EvmError`.
#[derive(Debug)]
pub struct DbBridgeError(pub String);

impl std::fmt::Display for DbBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DbBridge error: {}", self.0)
    }
}

impl std::error::Error for DbBridgeError {}
impl DBErrorMarker for DbBridgeError {}

impl From<EvmError> for DbBridgeError {
    fn from(e: EvmError) -> Self {
        DbBridgeError(e.to_string())
    }
}

/// Bridges our `StateProvider` to revm's `Database` trait.
///
/// This is the integration seam between our state abstraction and revm's
/// execution engine. In later slices (S03+), this bridge will be extended
/// to route through MVHashMap for parallel execution, but the interface
/// remains stable.
pub struct DbBridge<'a> {
    state: &'a dyn StateProvider,
}

impl<'a> DbBridge<'a> {
    /// Creates a new `DbBridge` wrapping the given state provider.
    pub fn new(state: &'a dyn StateProvider) -> Self {
        Self { state }
    }
}

impl Database for DbBridge<'_> {
    type Error = DbBridgeError;

    fn basic(&mut self, address: Address) -> Result<Option<RevmAccountInfo>, Self::Error> {
        let maybe_account = self.state.basic_account(address)?;
        Ok(maybe_account.map(|acct| RevmAccountInfo {
            balance: acct.balance,
            nonce: acct.nonce,
            code_hash: acct.code_hash,
            code: acct.code.map(Bytecode::new_raw),
            account_id: None,
        }))
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        let bytes = self.state.code_by_hash(code_hash)?;
        Ok(if bytes.is_empty() {
            Bytecode::default()
        } else {
            Bytecode::new_raw(bytes)
        })
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let value = self.state.storage(address, index)?;
        Ok(value)
    }

    fn block_hash(&mut self, number: u64) -> Result<B256, Self::Error> {
        let hash = self.state.block_hash(number)?;
        Ok(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monad_state::InMemoryState;
    use monad_types::AccountInfo;

    #[test]
    fn test_db_bridge_basic_account() {
        let addr = Address::with_last_byte(0x01);
        let state =
            InMemoryState::new().with_account(addr, AccountInfo::new(U256::from(1000u64), 5));

        let mut bridge = DbBridge::new(&state);
        let acct = bridge.basic(addr).unwrap().unwrap();
        assert_eq!(acct.balance, U256::from(1000u64));
        assert_eq!(acct.nonce, 5);
    }

    #[test]
    fn test_db_bridge_missing_account() {
        let state = InMemoryState::new();
        let mut bridge = DbBridge::new(&state);
        let acct = bridge.basic(Address::with_last_byte(0xFF)).unwrap();
        assert!(acct.is_none());
    }

    #[test]
    fn test_db_bridge_storage() {
        let addr = Address::with_last_byte(0x01);
        let state =
            InMemoryState::new().with_storage(addr, U256::from(0u64), U256::from(42u64));

        let mut bridge = DbBridge::new(&state);
        let val = bridge.storage(addr, U256::from(0u64)).unwrap();
        assert_eq!(val, U256::from(42u64));
    }

    #[test]
    fn test_db_bridge_block_hash() {
        let hash = B256::with_last_byte(0xAA);
        let state = InMemoryState::new().with_block_hash(100, hash);

        let mut bridge = DbBridge::new(&state);
        assert_eq!(bridge.block_hash(100).unwrap(), hash);
        assert_eq!(bridge.block_hash(999).unwrap(), B256::ZERO);
    }
}
