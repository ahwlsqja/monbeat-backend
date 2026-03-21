//! Multi-version state management for Block-STM parallel execution.
//!
//! This crate provides the foundational data structures for optimistic
//! concurrency control (OCC) in Block-STM:
//!
//! - [`MVHashMap`] — concurrent multi-version hash map storing versioned values
//! - [`LocationKey`] — granular state location identifiers (storage, balance, nonce, code hash)
//! - [`VersionedValue`] / [`MvReadResult`] — versioned entries and read results with ESTIMATE support
//!
//! # Architecture
//!
//! The MVHashMap is keyed by `(LocationKey, TxIndex, Incarnation)`. When transaction N
//! reads a location, it sees the value written by the highest transaction index < N.
//! If that value is an ESTIMATE marker (the writing transaction is being re-executed),
//! the read returns `MvReadResult::Estimate` — the scheduler interprets this as
//! "suspend this transaction until the dependency resolves."

pub mod mv_database;
pub mod read_write_sets;
pub mod types;
pub mod versioned_state;

// Re-exports for ergonomic use.
pub use mv_database::{MvDatabase, MvDatabaseError};
pub use read_write_sets::{ReadSet, WriteSet};
pub use types::{
    Incarnation, LocationKey, MvReadResult, ReadOrigin, TxIndex, VersionedValue, WriteValue,
};
pub use versioned_state::MVHashMap;
