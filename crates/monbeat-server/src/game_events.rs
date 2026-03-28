//! GameEvent mapping engine — transforms parallel execution results into musical events.
//!
//! Converts monad-core scheduler output (tx results, incarnations, conflict details)
//! into a sequence of `GameEvent` structs that the frontend renders as a rhythm game
//! with audio. Each event carries a MIDI note, lane, timestamp, and type.
//!
//! # Sound Design (PRD v3 Section 7)
//!
//! - **Pentatonic base:** Core 0→C4(60), Core 1→E4(64), Core 2→G4(67), Core 3→A4(69)
//! - **Position modulation:** tx position shifts by pentatonic intervals (+0,+2,+4,+7,+9 cycling)
//! - **Conflict dissonance:** semitone above the base note (C4→Db4=61, E4→F4=65, etc.)
//! - **Re-execution:** ascending 3-note arpeggio from base
//! - **Block complete:** C-E-G chord (60, 64, 67)
//!
//! # Binary Protocol (game-optimize.md Section 5.2)
//!
//! Each GameEvent is 14 bytes: type(u8) + lane(u8) + tx_index(u16) + note(u8) + slot(u8) + timestamp(f64)
//!
//! # Observability
//!
//! - Event count by type reveals execution health at a glance
//! - Zero conflict events = clean parallel execution
//! - Re-execution events proportional to incarnation counts

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Constants — MIDI note mappings per PRD v3 Section 7.1
// ---------------------------------------------------------------------------

/// Base MIDI notes for each execution lane (core).
/// Core 0→C4, Core 1→E4, Core 2→G4, Core 3→A4
const LANE_BASE_NOTES: [u8; 4] = [60, 64, 67, 69];

/// Pentatonic interval offsets (semitones) cycling over tx position within a lane.
/// +0, +2, +4, +7, +9 then repeats.
const PENTATONIC_OFFSETS: [u8; 5] = [0, 2, 4, 7, 9];

/// Number of execution lanes (mapped from tx_index % NUM_LANES).
const NUM_LANES: u16 = 4;

/// Time between successive events (seconds). Creates audible rhythm at ~50 events/sec.
const EVENT_SPACING: f64 = 0.020;

/// Additional delay for conflict events after their associated tx_commit (seconds).
const CONFLICT_DELAY: f64 = 0.005;

/// Additional delay for re-execution events after their associated tx_commit (seconds).
const RE_EXECUTION_DELAY: f64 = 0.010;

// ---------------------------------------------------------------------------
// GameEvent types
// ---------------------------------------------------------------------------

/// Event type discriminant — matches the binary protocol u8 tag.
///
/// Serializes as a plain u8 integer (not a string) to match the binary protocol
/// and keep JSON payloads compact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GameEventType {
    /// Successful transaction commit.
    TxCommit = 1,
    /// Conflict detected between two transactions.
    Conflict = 2,
    /// Transaction is being re-executed (optimistic retry).
    ReExecution = 3,
    /// Re-execution completed successfully.
    ReExecutionResolved = 4,
    /// All transactions in the block have been processed.
    BlockComplete = 5,
}

impl GameEventType {
    /// Deserialize from a u8 tag byte. Returns `None` for unknown tags.
    pub fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(Self::TxCommit),
            2 => Some(Self::Conflict),
            3 => Some(Self::ReExecution),
            4 => Some(Self::ReExecutionResolved),
            5 => Some(Self::BlockComplete),
            _ => None,
        }
    }
}

impl Serialize for GameEventType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> serde::Deserialize<'de> for GameEventType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let byte = u8::deserialize(deserializer)?;
        Self::from_u8(byte)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid GameEventType: {byte}")))
    }
}

/// A single game event for the frontend rhythm-game + audio engine.
///
/// Binary layout: 14 bytes total (type:1 + lane:1 + tx_index:2 + note:1 + slot:1 + timestamp:8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameEvent {
    /// Event type discriminant.
    #[serde(rename = "type")]
    pub event_type: GameEventType,
    /// Execution lane (0–3), mapped from tx_index % 4.
    pub lane: u8,
    /// Transaction index within the block (0-based).
    pub tx_index: u16,
    /// MIDI note number (0–127).
    pub note: u8,
    /// Storage slot identifier for conflict events (0 for non-conflict events).
    pub slot: u8,
    /// Relative timestamp in seconds from simulation start.
    pub timestamp: f64,
}

/// Size of a single GameEvent in the binary wire format.
pub const GAME_EVENT_BYTES: usize = 14;

impl GameEvent {
    /// Serialize to a 14-byte big-endian buffer matching the JS DataView decode layout.
    ///
    /// Layout:
    /// - [0]:    event_type as u8
    /// - [1]:    lane as u8
    /// - [2..4]: tx_index as u16 big-endian
    /// - [4]:    note as u8
    /// - [5]:    slot as u8
    /// - [6..14]: timestamp as f64 big-endian
    pub fn to_bytes(&self) -> [u8; GAME_EVENT_BYTES] {
        let mut buf = [0u8; GAME_EVENT_BYTES];
        buf[0] = self.event_type as u8;
        buf[1] = self.lane;
        let tx_be = self.tx_index.to_be_bytes();
        buf[2] = tx_be[0];
        buf[3] = tx_be[1];
        buf[4] = self.note;
        buf[5] = self.slot;
        let ts_be = self.timestamp.to_be_bytes();
        buf[6..14].copy_from_slice(&ts_be);
        buf
    }

    /// Deserialize from a 14-byte big-endian buffer.
    ///
    /// Returns `None` if the buffer is too short or the event type byte is invalid.
    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < GAME_EVENT_BYTES {
            return None;
        }
        let event_type = GameEventType::from_u8(buf[0])?;
        let lane = buf[1];
        let tx_index = u16::from_be_bytes([buf[2], buf[3]]);
        let note = buf[4];
        let slot = buf[5];
        let timestamp = f64::from_be_bytes([
            buf[6], buf[7], buf[8], buf[9], buf[10], buf[11], buf[12], buf[13],
        ]);
        Some(Self {
            event_type,
            lane,
            tx_index,
            note,
            slot,
            timestamp,
        })
    }
}

// ---------------------------------------------------------------------------
// Input types — decoupled from CLI-specific conflict.rs types
// ---------------------------------------------------------------------------

/// A detected conflict between two transactions, as input to the mapper.
///
/// This is a simplified projection of `crates/cli/src/conflict.rs::ConflictPair`.
/// The API handler converts scheduler output into these before calling the mapper.
#[derive(Debug, Clone)]
pub struct ConflictInput {
    /// Index of the first conflicting transaction.
    pub tx_a: usize,
    /// Index of the second conflicting transaction.
    pub tx_b: usize,
    /// Storage slot involved in the conflict (low byte used as slot identifier).
    pub slot_byte: u8,
}

/// Per-transaction execution summary, as input to the mapper.
#[derive(Debug, Clone)]
pub struct TxResult {
    /// Whether the transaction executed successfully.
    pub success: bool,
    /// Gas consumed.
    pub gas_used: u64,
}

// ---------------------------------------------------------------------------
// Note computation helpers
// ---------------------------------------------------------------------------

/// Compute the pentatonic MIDI note for a transaction based on its lane and position.
///
/// lane = tx_index % 4 → selects base note from LANE_BASE_NOTES
/// position = tx_index / 4 → selects pentatonic offset (cycling through PENTATONIC_OFFSETS)
fn pentatonic_note(tx_index: u16) -> u8 {
    let lane = (tx_index % NUM_LANES) as usize;
    let position = (tx_index / NUM_LANES) as usize;
    let base = LANE_BASE_NOTES[lane];
    let offset = PENTATONIC_OFFSETS[position % PENTATONIC_OFFSETS.len()];
    base.saturating_add(offset)
}

/// Compute the dissonant (conflict) note — one semitone above the base note for the lane.
fn conflict_note(tx_index: u16) -> u8 {
    let lane = (tx_index % NUM_LANES) as usize;
    LANE_BASE_NOTES[lane].saturating_add(1)
}

/// Compute the 3-note ascending arpeggio for re-execution events.
/// Returns (note1, note2, note3) — base, +4 semitones, +7 semitones.
fn re_execution_arpeggio(tx_index: u16) -> (u8, u8, u8) {
    let lane = (tx_index % NUM_LANES) as usize;
    let base = LANE_BASE_NOTES[lane];
    (base, base.saturating_add(4), base.saturating_add(7))
}

// ---------------------------------------------------------------------------
// GameEventMapper
// ---------------------------------------------------------------------------

/// Maps parallel execution results into a sequence of musical game events.
///
/// The mapper is stateless — all context is passed via `map_to_events`.
pub struct GameEventMapper;

impl GameEventMapper {
    /// Transform execution results into an ordered sequence of game events.
    ///
    /// # Arguments
    ///
    /// * `tx_results` — per-transaction execution outcome (success/gas_used)
    /// * `incarnations` — per-transaction incarnation count (0 = executed once, >0 = re-executed)
    /// * `conflicts` — detected conflict pairs between transactions
    ///
    /// # Returns
    ///
    /// Events ordered by timestamp. The sequence:
    /// 1. TxCommit events for each successful transaction
    /// 2. Conflict events interleaved after relevant tx commits
    /// 3. ReExecution + ReExecutionResolved for re-executed transactions
    /// 4. BlockComplete chord at the end
    pub fn map_to_events(
        tx_results: &[TxResult],
        incarnations: &[u32],
        conflicts: &[ConflictInput],
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut timestamp = 0.0_f64;

        // Build a lookup: tx_index → list of conflicts involving that tx
        let mut tx_conflicts: HashMap<usize, Vec<&ConflictInput>> = HashMap::new();
        for c in conflicts {
            tx_conflicts.entry(c.tx_a).or_default().push(c);
            tx_conflicts.entry(c.tx_b).or_default().push(c);
        }

        // Track which conflict pairs we've already emitted (avoid duplicates)
        let mut emitted_conflicts: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::new();

        // Phase 1: TxCommit events + interleaved Conflict events
        for (i, tx) in tx_results.iter().enumerate() {
            let tx_index = i as u16;
            let lane = (tx_index % NUM_LANES) as u8;

            if tx.success {
                // TxCommit event with pentatonic note
                events.push(GameEvent {
                    event_type: GameEventType::TxCommit,
                    lane,
                    tx_index,
                    note: pentatonic_note(tx_index),
                    slot: 0,
                    timestamp,
                });
                timestamp += EVENT_SPACING;
            }

            // Conflict events associated with this tx (emitted with slight delay)
            // Limit: at most one conflict event per tx to prevent event explosion
            // (C++ engine reports all N*(N-1)/2 conflict pairs — 300 TXs → 45,000+ pairs)
            if let Some(conflict_list) = tx_conflicts.get(&i) {
                let mut emitted_for_this_tx = false;
                for c in conflict_list {
                    let pair_key = (c.tx_a.min(c.tx_b), c.tx_a.max(c.tx_b));
                    if emitted_conflicts.insert(pair_key) && !emitted_for_this_tx {
                        emitted_for_this_tx = true;
                        let conflict_tx = tx_index;
                        events.push(GameEvent {
                            event_type: GameEventType::Conflict,
                            lane: (conflict_tx % NUM_LANES) as u8,
                            tx_index: conflict_tx,
                            note: conflict_note(conflict_tx),
                            slot: c.slot_byte,
                            timestamp: timestamp - EVENT_SPACING + CONFLICT_DELAY,
                        });
                        timestamp += EVENT_SPACING;
                    }
                }
            }
        }

        // Phase 2: ReExecution events for txs with incarnation > 0
        for (i, &inc) in incarnations.iter().enumerate() {
            if inc > 0 {
                let tx_index = i as u16;
                let lane = (tx_index % NUM_LANES) as u8;
                let (n1, n2, n3) = re_execution_arpeggio(tx_index);

                // Ascending 3-note arpeggio (ReExecution events)
                for (j, note) in [n1, n2, n3].iter().enumerate() {
                    events.push(GameEvent {
                        event_type: GameEventType::ReExecution,
                        lane,
                        tx_index,
                        note: *note,
                        slot: 0,
                        timestamp: timestamp + (j as f64) * RE_EXECUTION_DELAY,
                    });
                }
                timestamp += 3.0 * RE_EXECUTION_DELAY;

                // ReExecutionResolved — returns to pentatonic note
                events.push(GameEvent {
                    event_type: GameEventType::ReExecutionResolved,
                    lane,
                    tx_index,
                    note: pentatonic_note(tx_index),
                    slot: 0,
                    timestamp,
                });
                timestamp += EVENT_SPACING;
            }
        }

        // Phase 3: BlockComplete chord — C-E-G (MIDI 60, 64, 67)
        let block_complete_notes: [u8; 3] = [60, 64, 67];
        for (j, &note) in block_complete_notes.iter().enumerate() {
            events.push(GameEvent {
                event_type: GameEventType::BlockComplete,
                lane: j as u8,
                tx_index: 0,
                note,
                slot: 0,
                timestamp: timestamp + (j as f64) * 0.002, // slight spread for chord
            });
        }

        events
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Note computation tests ---

    #[test]
    fn test_pentatonic_note_lane_base_notes() {
        // tx_index 0..3 → lanes 0..3, position=0 → base notes with +0 offset
        assert_eq!(pentatonic_note(0), 60); // Core 0 → C4
        assert_eq!(pentatonic_note(1), 64); // Core 1 → E4
        assert_eq!(pentatonic_note(2), 67); // Core 2 → G4
        assert_eq!(pentatonic_note(3), 69); // Core 3 → A4
    }

    #[test]
    fn test_pentatonic_note_position_modulation() {
        // tx_index 4..7 → lanes 0..3, position=1 → base + 2 semitones
        assert_eq!(pentatonic_note(4), 62); // C4 + 2 = D4
        assert_eq!(pentatonic_note(5), 66); // E4 + 2 = F#4

        // tx_index 8..11 → position=2 → base + 4
        assert_eq!(pentatonic_note(8), 64); // C4 + 4 = E4

        // tx_index 12..15 → position=3 → base + 7
        assert_eq!(pentatonic_note(12), 67); // C4 + 7 = G4

        // tx_index 16..19 → position=4 → base + 9
        assert_eq!(pentatonic_note(16), 69); // C4 + 9 = A4
    }

    #[test]
    fn test_pentatonic_note_cycling() {
        // tx_index 20 → position=5 → wraps back to offset[0] = +0
        assert_eq!(pentatonic_note(20), 60); // C4 + 0 = C4 again
    }

    #[test]
    fn test_conflict_note_semitone_above() {
        assert_eq!(conflict_note(0), 61); // C4 + 1 = Db4
        assert_eq!(conflict_note(1), 65); // E4 + 1 = F4
        assert_eq!(conflict_note(2), 68); // G4 + 1 = Ab4
        assert_eq!(conflict_note(3), 70); // A4 + 1 = Bb4
    }

    #[test]
    fn test_re_execution_arpeggio() {
        let (n1, n2, n3) = re_execution_arpeggio(0); // Lane 0, base C4
        assert_eq!(n1, 60); // C4
        assert_eq!(n2, 64); // E4 (C4+4)
        assert_eq!(n3, 67); // G4 (C4+7)

        let (n1, n2, n3) = re_execution_arpeggio(1); // Lane 1, base E4
        assert_eq!(n1, 64); // E4
        assert_eq!(n2, 68); // G#4 (E4+4)
        assert_eq!(n3, 71); // B4 (E4+7)
    }

    // --- Mapper integration tests ---

    #[test]
    fn test_map_no_conflicts_counter_contract() {
        // Counter contract: deploy + 2 function calls, no conflicts, no re-execution
        let tx_results = vec![
            TxResult { success: true, gas_used: 100_000 },  // deploy
            TxResult { success: true, gas_used: 50_000 },   // increment
            TxResult { success: true, gas_used: 50_000 },   // decrement
        ];
        let incarnations = vec![0, 0, 0];
        let conflicts = vec![];

        let events = GameEventMapper::map_to_events(&tx_results, &incarnations, &conflicts);

        // Should have: 3 TxCommit + 3 BlockComplete = 6 events
        let tx_commits: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::TxCommit)
            .collect();
        assert_eq!(tx_commits.len(), 3, "3 successful txs → 3 TxCommit events");

        let block_completes: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::BlockComplete)
            .collect();
        assert_eq!(block_completes.len(), 3, "block complete is C-E-G chord (3 notes)");

        // No conflict or re-execution events
        assert!(
            events
                .iter()
                .all(|e| e.event_type != GameEventType::Conflict),
            "no conflicts expected"
        );
        assert!(
            events
                .iter()
                .all(|e| e.event_type != GameEventType::ReExecution),
            "no re-executions expected"
        );

        // Verify pentatonic notes for tx 0, 1, 2
        assert_eq!(tx_commits[0].note, 60); // lane 0, pos 0 → C4
        assert_eq!(tx_commits[1].note, 64); // lane 1, pos 0 → E4
        assert_eq!(tx_commits[2].note, 67); // lane 2, pos 0 → G4

        // Block complete chord: C-E-G
        assert_eq!(block_completes[0].note, 60);
        assert_eq!(block_completes[1].note, 64);
        assert_eq!(block_completes[2].note, 67);
    }

    #[test]
    fn test_map_with_conflicts() {
        // Two txs that conflict on a storage slot
        let tx_results = vec![
            TxResult { success: true, gas_used: 50_000 },
            TxResult { success: true, gas_used: 50_000 },
        ];
        let incarnations = vec![0, 0];
        let conflicts = vec![ConflictInput {
            tx_a: 0,
            tx_b: 1,
            slot_byte: 7,
        }];

        let events = GameEventMapper::map_to_events(&tx_results, &incarnations, &conflicts);

        let conflict_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::Conflict)
            .collect();
        assert_eq!(conflict_events.len(), 1, "one conflict pair → one conflict event");
        assert_eq!(conflict_events[0].slot, 7, "slot byte should be preserved");

        // Conflict note should be semitone above base
        let conflict_note_val = conflict_events[0].note;
        let lane = conflict_events[0].lane as usize;
        assert_eq!(
            conflict_note_val,
            LANE_BASE_NOTES[lane] + 1,
            "conflict note should be semitone above lane base"
        );
    }

    #[test]
    fn test_map_with_re_execution() {
        // One tx re-executed (incarnation=2 means it was re-run twice)
        let tx_results = vec![
            TxResult { success: true, gas_used: 50_000 },
            TxResult { success: true, gas_used: 60_000 },
        ];
        let incarnations = vec![0, 2]; // tx1 re-executed
        let conflicts = vec![];

        let events = GameEventMapper::map_to_events(&tx_results, &incarnations, &conflicts);

        let re_exec_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::ReExecution)
            .collect();
        assert_eq!(re_exec_events.len(), 3, "re-execution produces 3-note arpeggio");

        let resolved_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::ReExecutionResolved)
            .collect();
        assert_eq!(resolved_events.len(), 1, "one resolved event per re-executed tx");

        // Arpeggio notes should be ascending from base
        let base = LANE_BASE_NOTES[1]; // tx1 → lane 1 → E4=64
        assert_eq!(re_exec_events[0].note, base);       // E4
        assert_eq!(re_exec_events[1].note, base + 4);   // G#4
        assert_eq!(re_exec_events[2].note, base + 7);   // B4

        // Resolved note returns to pentatonic
        assert_eq!(resolved_events[0].note, pentatonic_note(1));
    }

    #[test]
    fn test_map_mixed_sender_contract() {
        // Simulates a mixed-sender contract with conflicts and re-executions
        let tx_results = vec![
            TxResult { success: true, gas_used: 100_000 }, // deploy
            TxResult { success: true, gas_used: 50_000 },  // fn1
            TxResult { success: true, gas_used: 50_000 },  // fn2
            TxResult { success: true, gas_used: 50_000 },  // fn3
            TxResult { success: false, gas_used: 30_000 }, // fn4 — reverted, no TxCommit
        ];
        let incarnations = vec![0, 0, 1, 0, 0]; // tx2 re-executed once
        let conflicts = vec![
            ConflictInput { tx_a: 1, tx_b: 2, slot_byte: 0 },
        ];

        let events = GameEventMapper::map_to_events(&tx_results, &incarnations, &conflicts);

        // 4 successful TxCommit (tx0,1,2,3 — tx4 reverted so no commit)
        let tx_commits: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::TxCommit)
            .collect();
        assert_eq!(tx_commits.len(), 4, "4 successful txs produce 4 commits");

        // 1 conflict event
        let conflict_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::Conflict)
            .collect();
        assert_eq!(conflict_events.len(), 1);

        // 3 re-execution notes + 1 resolved for tx2
        let re_exec: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::ReExecution)
            .collect();
        assert_eq!(re_exec.len(), 3);

        let resolved: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::ReExecutionResolved)
            .collect();
        assert_eq!(resolved.len(), 1);

        // 3 BlockComplete notes
        let bc: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::BlockComplete)
            .collect();
        assert_eq!(bc.len(), 3);
    }

    #[test]
    fn test_map_empty_block() {
        let events = GameEventMapper::map_to_events(&[], &[], &[]);
        // Only BlockComplete chord
        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|e| e.event_type == GameEventType::BlockComplete));
    }

    #[test]
    fn test_timestamps_are_monotonically_ordered() {
        let tx_results = vec![
            TxResult { success: true, gas_used: 50_000 },
            TxResult { success: true, gas_used: 50_000 },
            TxResult { success: true, gas_used: 50_000 },
        ];
        let incarnations = vec![0, 1, 0];
        let conflicts = vec![ConflictInput { tx_a: 0, tx_b: 1, slot_byte: 0 }];

        let events = GameEventMapper::map_to_events(&tx_results, &incarnations, &conflicts);

        // All timestamps should be >= 0
        for event in &events {
            assert!(event.timestamp >= 0.0, "timestamp should be non-negative");
        }

        // BlockComplete events should be at the end (highest timestamps)
        let max_non_bc_ts = events
            .iter()
            .filter(|e| e.event_type != GameEventType::BlockComplete)
            .map(|e| e.timestamp)
            .fold(0.0_f64, f64::max);

        let min_bc_ts = events
            .iter()
            .filter(|e| e.event_type == GameEventType::BlockComplete)
            .map(|e| e.timestamp)
            .fold(f64::INFINITY, f64::min);

        assert!(
            min_bc_ts >= max_non_bc_ts,
            "BlockComplete events should come after all other events"
        );
    }

    #[test]
    fn test_lane_assignment() {
        let tx_results: Vec<TxResult> = (0..8)
            .map(|_| TxResult { success: true, gas_used: 50_000 })
            .collect();
        let incarnations = vec![0; 8];
        let conflicts = vec![];

        let events = GameEventMapper::map_to_events(&tx_results, &incarnations, &conflicts);

        let tx_commits: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::TxCommit)
            .collect();

        // Each tx should map to lane = tx_index % 4
        for commit in &tx_commits {
            assert_eq!(
                commit.lane,
                (commit.tx_index % 4) as u8,
                "lane should be tx_index % 4"
            );
        }
    }

    #[test]
    fn test_conflict_dedup() {
        // Same conflict pair should only produce one event even if both txs are iterated
        let tx_results = vec![
            TxResult { success: true, gas_used: 50_000 },
            TxResult { success: true, gas_used: 50_000 },
        ];
        let incarnations = vec![0, 0];
        let conflicts = vec![ConflictInput { tx_a: 0, tx_b: 1, slot_byte: 3 }];

        let events = GameEventMapper::map_to_events(&tx_results, &incarnations, &conflicts);
        let conflict_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::Conflict)
            .collect();
        assert_eq!(
            conflict_events.len(),
            1,
            "duplicate conflict pair should produce exactly one event"
        );
    }

    #[test]
    fn test_game_event_serialization() {
        let event = GameEvent {
            event_type: GameEventType::TxCommit,
            lane: 0,
            tx_index: 5,
            note: 60,
            slot: 0,
            timestamp: 0.100,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":1"), "type should serialize as u8 discriminant");
        assert!(json.contains("\"lane\":0"));
        assert!(json.contains("\"tx_index\":5"));
        assert!(json.contains("\"note\":60"));
    }

    #[test]
    fn test_multiple_conflicts_same_tx() {
        // tx1 conflicts with both tx0 and tx2
        let tx_results = vec![
            TxResult { success: true, gas_used: 50_000 },
            TxResult { success: true, gas_used: 50_000 },
            TxResult { success: true, gas_used: 50_000 },
        ];
        let incarnations = vec![0, 0, 0];
        let conflicts = vec![
            ConflictInput { tx_a: 0, tx_b: 1, slot_byte: 1 },
            ConflictInput { tx_a: 1, tx_b: 2, slot_byte: 2 },
        ];

        let events = GameEventMapper::map_to_events(&tx_results, &incarnations, &conflicts);
        let conflict_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == GameEventType::Conflict)
            .collect();
        assert_eq!(conflict_events.len(), 2, "two distinct conflict pairs → two events");
    }

    // --- Binary serialization tests ---

    #[test]
    fn test_binary_round_trip_all_event_types() {
        // Round-trip through to_bytes → from_bytes for every event type
        let types = [
            GameEventType::TxCommit,
            GameEventType::Conflict,
            GameEventType::ReExecution,
            GameEventType::ReExecutionResolved,
            GameEventType::BlockComplete,
        ];
        for (i, &etype) in types.iter().enumerate() {
            let event = GameEvent {
                event_type: etype,
                lane: (i as u8) % 4,
                tx_index: (i as u16) * 100 + 7,
                note: 60 + i as u8,
                slot: i as u8,
                timestamp: 0.020 * (i as f64),
            };
            let bytes = event.to_bytes();
            assert_eq!(bytes.len(), GAME_EVENT_BYTES);
            let decoded = GameEvent::from_bytes(&bytes).expect("round-trip decode failed");
            assert_eq!(decoded, event, "round-trip mismatch for {:?}", etype);
        }
    }

    #[test]
    fn test_binary_byte_level_js_compatibility() {
        // Known event → to_bytes → verify exact byte positions match JS DataView reads:
        //   view.getUint8(0)   = type
        //   view.getUint8(1)   = lane
        //   view.getUint16(2)  = txIndex (big-endian)
        //   view.getUint8(4)   = note
        //   view.getUint8(5)   = slot
        //   view.getFloat64(6) = timestamp (big-endian)
        let event = GameEvent {
            event_type: GameEventType::Conflict, // type = 2
            lane: 3,
            tx_index: 258, // 0x0102 → big-endian [0x01, 0x02]
            note: 65,
            slot: 7,
            timestamp: 0.040,
        };
        let bytes = event.to_bytes();

        // offset 0: type
        assert_eq!(bytes[0], 2, "type byte");
        // offset 1: lane
        assert_eq!(bytes[1], 3, "lane byte");
        // offset 2-3: tx_index big-endian
        assert_eq!(bytes[2], 0x01, "tx_index high byte");
        assert_eq!(bytes[3], 0x02, "tx_index low byte");
        // offset 4: note
        assert_eq!(bytes[4], 65, "note byte");
        // offset 5: slot
        assert_eq!(bytes[5], 7, "slot byte");
        // offset 6-13: timestamp as f64 big-endian
        let ts_bytes = (0.040_f64).to_be_bytes();
        assert_eq!(&bytes[6..14], &ts_bytes, "timestamp big-endian f64");
    }

    #[test]
    fn test_binary_edge_cases() {
        // Max u16 tx_index
        let max_tx = GameEvent {
            event_type: GameEventType::TxCommit,
            lane: 0,
            tx_index: u16::MAX, // 65535
            note: 127,          // max MIDI note
            slot: 255,
            timestamp: 0.0,     // zero timestamp
        };
        let bytes = max_tx.to_bytes();
        let decoded = GameEvent::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.tx_index, 65535);
        assert_eq!(decoded.note, 127);
        assert_eq!(decoded.slot, 255);
        assert_eq!(decoded.timestamp, 0.0);

        // tx_index high-byte check: 65535 = 0xFF 0xFF
        assert_eq!(bytes[2], 0xFF);
        assert_eq!(bytes[3], 0xFF);

        // Very large timestamp
        let large_ts = GameEvent {
            event_type: GameEventType::BlockComplete,
            lane: 2,
            tx_index: 0,
            note: 67,
            slot: 0,
            timestamp: 999_999.999_999,
        };
        let decoded2 = GameEvent::from_bytes(&large_ts.to_bytes()).unwrap();
        assert_eq!(decoded2.timestamp, 999_999.999_999);
    }

    #[test]
    fn test_binary_invalid_type_byte() {
        let mut bytes = [0u8; GAME_EVENT_BYTES];
        // type byte 0 is not a valid GameEventType
        bytes[0] = 0;
        assert!(GameEvent::from_bytes(&bytes).is_none(), "type 0 should fail");

        // type byte 6 is out of range
        bytes[0] = 6;
        assert!(GameEvent::from_bytes(&bytes).is_none(), "type 6 should fail");

        // type byte 255
        bytes[0] = 255;
        assert!(GameEvent::from_bytes(&bytes).is_none(), "type 255 should fail");
    }

    #[test]
    fn test_binary_buffer_too_short() {
        // 13 bytes = one byte short
        let short_buf = [1u8; 13];
        assert!(GameEvent::from_bytes(&short_buf).is_none(), "13 bytes should fail");

        // empty
        assert!(GameEvent::from_bytes(&[]).is_none(), "empty buffer should fail");
    }

    #[test]
    fn test_binary_game_event_bytes_constant() {
        assert_eq!(GAME_EVENT_BYTES, 14, "wire format is exactly 14 bytes");
    }

    #[test]
    fn test_binary_from_u8_all_variants() {
        assert_eq!(GameEventType::from_u8(1), Some(GameEventType::TxCommit));
        assert_eq!(GameEventType::from_u8(2), Some(GameEventType::Conflict));
        assert_eq!(GameEventType::from_u8(3), Some(GameEventType::ReExecution));
        assert_eq!(GameEventType::from_u8(4), Some(GameEventType::ReExecutionResolved));
        assert_eq!(GameEventType::from_u8(5), Some(GameEventType::BlockComplete));
        assert_eq!(GameEventType::from_u8(0), None);
        assert_eq!(GameEventType::from_u8(6), None);
    }
}
