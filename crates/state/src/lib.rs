//! # monad-state
//!
//! State provider trait and in-memory backend for monad-core.
//!
//! This crate defines `StateProvider` — the core abstraction for reading
//! blockchain state (accounts, storage, code, block hashes). The EVM
//! executor consumes this trait, and different backends implement it:
//!
//! - `InMemoryState` (S01) — HashMap-backed mock for testing
//! - MVHashMap adapter (S03) — optimistic concurrency control
//! - MonadDb adapter (future) — persistent storage
//!
//! ## Usage
//!
//! ```rust
//! use monad_state::{StateProvider, InMemoryState};
//! use monad_types::{AccountInfo, Address, U256};
//!
//! let state = InMemoryState::new()
//!     .with_account(
//!         Address::with_last_byte(1),
//!         AccountInfo::new(U256::from(1_000_000u64), 0),
//!     );
//!
//! let acct = state.basic_account(Address::with_last_byte(1)).unwrap();
//! assert!(acct.is_some());
//! ```

pub mod cached_provider;
pub mod in_memory;
pub mod provider;

// Re-export the core trait and default implementation at crate root.
pub use cached_provider::CachedStateProvider;
pub use in_memory::InMemoryState;
pub use provider::StateProvider;
