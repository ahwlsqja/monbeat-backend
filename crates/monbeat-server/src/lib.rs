//! MonBeat simulation server — Solidity compilation, transaction block construction,
//! parallel execution via monad-core, and game event mapping.
//!
//! # Pipeline
//!
//! 1. `compiler` — Compile Solidity source via solc subprocess
//! 2. `block_builder` — Construct a transaction block from compiled ABI + bytecode
//! 3. `game_events` — Map execution results to musical game events
//! 4. `api` — Axum REST endpoints tying the pipeline together

pub mod api;
pub mod block_builder;
pub mod compiler;
pub mod game_events;
pub mod ws;
