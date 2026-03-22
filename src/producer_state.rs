//! Per-partition producer idempotency state tracking.
//!
//! Implements Kafka-compatible sequence number validation, epoch management,
//! and duplicate detection with bounded LRU eviction.
//!
//! # Design
//! - Bounded map with LRU eviction (configurable max producers per partition)
//! - Validates sequence continuity and epoch freshness
//! - Retains last 5 batch metadata per producer for duplicate detection (Kafka parity)
//! - Duplicate batches return success with original offset (Kafka parity)
//! - Epoch bumps require `base_sequence == 0` for known producers (Kafka parity)
//!
//! This module is a pure state machine with no logging, metrics, or I/O — the
//! caller can inspect [`ValidationResult`] and [`ValidationError`] to drive
//! those concerns externally.

use std::collections::{HashMap, VecDeque};

use crate::ResponseError;

/// Default maximum producers tracked per partition.
pub const DEFAULT_MAX_PRODUCERS: usize = 64;

/// Number of recent batches retained per producer for duplicate detection
/// (matches Kafka's `NUM_BATCHES_TO_RETAIN`).
const NUM_BATCHES_TO_RETAIN: usize = 5;

/// Metadata for a stored batch, used for duplicate detection.
#[derive(Debug, Clone)]
struct BatchMetadata {
    first_seq: i32,
    last_seq: i32,
    base_offset: i64,
}

/// Result of producer state validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Batch accepted — proceed with storage.
    Accepted,
    /// Batch is a duplicate of a previously stored batch — return success
    /// with the original offset (Kafka parity).
    Duplicate {
        /// The base offset that was recorded for the original batch.
        base_offset: i64,
    },
}

/// Why a batch was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Producer sent an epoch older than the one the broker has recorded.
    InvalidProducerEpoch {
        /// The epoch sent by the producer.
        incoming_epoch: i16,
        /// The epoch the broker currently has for this producer.
        current_epoch: i16,
    },
    /// Sequence number is not the expected next value, or an epoch bump
    /// did not start at sequence 0.
    OutOfOrderSequence {
        /// The base sequence sent by the producer.
        base_sequence: i32,
        /// The sequence the broker expected (only meaningful for same-epoch
        /// violations; for epoch-bump violations, this is 0).
        expected_sequence: i32,
    },
}

impl ValidationError {
    /// Convert to the corresponding Kafka [`ResponseError`] code.
    pub fn to_response_error(&self) -> ResponseError {
        match self {
            Self::InvalidProducerEpoch { .. } => ResponseError::InvalidProducerEpoch,
            Self::OutOfOrderSequence { .. } => ResponseError::OutOfOrderSequenceNumber,
        }
    }
}

/// Per-producer state within a single partition.
#[derive(Debug, Clone)]
struct ProducerPartitionState {
    epoch: i16,
    last_sequence: i32,
    last_seen_ms: i64,
    batch_history: VecDeque<BatchMetadata>,
}

/// Per-partition producer state tracker with bounded LRU eviction.
///
/// Thread-safety is the caller's responsibility — this type is not
/// internally synchronized. Wrap in a `Mutex` or use one instance per
/// partition behind a lock as needed.
#[derive(Debug)]
pub struct ProducerStateMap {
    states: HashMap<i64, ProducerPartitionState>,
    max_entries: usize,
}

impl ProducerStateMap {
    /// Create a new producer state map with the given capacity.
    pub fn new(max_entries: usize) -> Self {
        Self {
            states: HashMap::new(),
            max_entries,
        }
    }

    /// Record the `base_offset` for the most recently accepted batch.
    ///
    /// Must be called after a successful store to enable the
    /// duplicate-returns-success behavior.
    pub fn record_offset(
        &mut self,
        producer_id: i64,
        _epoch: i16,
        base_sequence: i32,
        record_count: i32,
        base_offset: i64,
    ) {
        if let Some(state) = self.states.get_mut(&producer_id) {
            let last_seq = base_sequence.wrapping_add(record_count - 1);
            if state.batch_history.len() == NUM_BATCHES_TO_RETAIN {
                state.batch_history.pop_front();
            }
            state.batch_history.push_back(BatchMetadata {
                first_seq: base_sequence,
                last_seq,
                base_offset,
            });
        }
    }

    /// Validate and update producer state for an incoming batch.
    ///
    /// Returns `Ok(ValidationResult::Accepted)` if the batch is new,
    /// `Ok(ValidationResult::Duplicate { base_offset })` if it matches a
    /// previously recorded batch, or `Err(ValidationError)` if rejected.
    ///
    /// The caller should ensure `producer_id >= 0` — non-idempotent
    /// producers should skip validation entirely.
    pub fn validate(
        &mut self,
        producer_id: i64,
        epoch: i16,
        base_sequence: i32,
        record_count: i32,
        now_ms: i64,
    ) -> Result<ValidationResult, ValidationError> {
        let end_sequence = base_sequence.wrapping_add(record_count - 1);

        if let Some(state) = self.states.get_mut(&producer_id) {
            // Stale epoch — zombie producer
            if epoch < state.epoch {
                return Err(ValidationError::InvalidProducerEpoch {
                    incoming_epoch: epoch,
                    current_epoch: state.epoch,
                });
            }

            // Epoch bump — producer reinitialized
            if epoch > state.epoch {
                if base_sequence != 0 {
                    return Err(ValidationError::OutOfOrderSequence {
                        base_sequence,
                        expected_sequence: 0,
                    });
                }
                state.epoch = epoch;
                state.last_sequence = end_sequence;
                state.last_seen_ms = now_ms;
                state.batch_history.clear();
                return Ok(ValidationResult::Accepted);
            }

            // Same epoch — check for duplicate in batch history
            if let Some(dup) = state
                .batch_history
                .iter()
                .find(|b| b.first_seq == base_sequence && b.last_seq == end_sequence)
            {
                return Ok(ValidationResult::Duplicate {
                    base_offset: dup.base_offset,
                });
            }

            // Same epoch — validate sequence continuity
            let expected_sequence = state.last_sequence.wrapping_add(1);
            if base_sequence == expected_sequence {
                state.last_sequence = end_sequence;
                state.last_seen_ms = now_ms;
                return Ok(ValidationResult::Accepted);
            }

            // Out of order
            return Err(ValidationError::OutOfOrderSequence {
                base_sequence,
                expected_sequence,
            });
        }

        // New producer — evict LRU if at capacity
        if self.states.len() >= self.max_entries {
            if let Some((&evict_pid, _)) =
                self.states.iter().min_by_key(|(_, state)| state.last_seen_ms)
            {
                self.states.remove(&evict_pid);
            }
        }

        self.states.insert(
            producer_id,
            ProducerPartitionState {
                epoch,
                last_sequence: end_sequence,
                last_seen_ms: now_ms,
                batch_history: VecDeque::new(),
            },
        );
        Ok(ValidationResult::Accepted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_producer_accepted() {
        let mut map = ProducerStateMap::new(64);
        assert_eq!(
            map.validate(1, 0, 0, 5, 1000),
            Ok(ValidationResult::Accepted)
        );
    }

    #[test]
    fn valid_sequence_continuation() {
        let mut map = ProducerStateMap::new(64);
        assert_eq!(map.validate(1, 0, 0, 5, 1000), Ok(ValidationResult::Accepted));
        assert_eq!(map.validate(1, 0, 5, 5, 2000), Ok(ValidationResult::Accepted));
        assert_eq!(map.validate(1, 0, 10, 1, 3000), Ok(ValidationResult::Accepted));
    }

    #[test]
    fn stale_epoch_rejected() {
        let mut map = ProducerStateMap::new(64);
        assert_eq!(map.validate(1, 5, 0, 1, 1000), Ok(ValidationResult::Accepted));
        assert_eq!(
            map.validate(1, 4, 1, 1, 2000),
            Err(ValidationError::InvalidProducerEpoch {
                incoming_epoch: 4,
                current_epoch: 5,
            })
        );
    }

    #[test]
    fn epoch_bump_resets_sequence() {
        let mut map = ProducerStateMap::new(64);
        assert_eq!(map.validate(1, 0, 0, 5, 1000), Ok(ValidationResult::Accepted));
        assert_eq!(map.validate(1, 1, 0, 3, 2000), Ok(ValidationResult::Accepted));
        assert_eq!(map.validate(1, 1, 3, 1, 3000), Ok(ValidationResult::Accepted));
    }

    #[test]
    fn epoch_bump_rejects_nonzero_sequence() {
        let mut map = ProducerStateMap::new(64);
        assert_eq!(map.validate(1, 0, 0, 5, 1000), Ok(ValidationResult::Accepted));
        assert_eq!(
            map.validate(1, 1, 50, 3, 2000),
            Err(ValidationError::OutOfOrderSequence {
                base_sequence: 50,
                expected_sequence: 0,
            })
        );
    }

    #[test]
    fn out_of_order_rejected() {
        let mut map = ProducerStateMap::new(64);
        assert_eq!(map.validate(1, 0, 0, 5, 1000), Ok(ValidationResult::Accepted));
        assert_eq!(
            map.validate(1, 0, 10, 1, 2000),
            Err(ValidationError::OutOfOrderSequence {
                base_sequence: 10,
                expected_sequence: 5,
            })
        );
    }

    #[test]
    fn duplicate_without_recorded_offset_is_out_of_order() {
        let mut map = ProducerStateMap::new(64);
        assert_eq!(map.validate(1, 0, 0, 5, 1000), Ok(ValidationResult::Accepted));
        assert_eq!(
            map.validate(1, 0, 0, 5, 2000),
            Err(ValidationError::OutOfOrderSequence {
                base_sequence: 0,
                expected_sequence: 5,
            })
        );
    }

    #[test]
    fn duplicate_returns_original_offset() {
        let mut map = ProducerStateMap::new(64);
        assert_eq!(map.validate(1, 0, 0, 5, 1000), Ok(ValidationResult::Accepted));
        map.record_offset(1, 0, 0, 5, 100);
        assert_eq!(
            map.validate(1, 0, 0, 5, 2000),
            Ok(ValidationResult::Duplicate { base_offset: 100 })
        );
    }

    #[test]
    fn duplicate_second_batch_returns_original_offset() {
        let mut map = ProducerStateMap::new(64);
        assert_eq!(map.validate(1, 0, 0, 5, 1000), Ok(ValidationResult::Accepted));
        map.record_offset(1, 0, 0, 5, 100);
        assert_eq!(map.validate(1, 0, 5, 3, 2000), Ok(ValidationResult::Accepted));
        map.record_offset(1, 0, 5, 3, 105);
        assert_eq!(
            map.validate(1, 0, 5, 3, 3000),
            Ok(ValidationResult::Duplicate { base_offset: 105 })
        );
    }

    #[test]
    fn duplicate_detected_across_5_batch_history() {
        let mut map = ProducerStateMap::new(64);
        for i in 0..5 {
            let base = i * 5;
            assert_eq!(
                map.validate(1, 0, base, 5, 1000 + i as i64),
                Ok(ValidationResult::Accepted)
            );
            map.record_offset(1, 0, base, 5, (base * 10) as i64);
        }
        assert_eq!(
            map.validate(1, 0, 0, 5, 9000),
            Ok(ValidationResult::Duplicate { base_offset: 0 })
        );
    }

    #[test]
    fn oldest_batch_evicted_after_6th() {
        let mut map = ProducerStateMap::new(64);
        for i in 0..6 {
            let base = i * 5;
            assert_eq!(
                map.validate(1, 0, base, 5, 1000 + i as i64),
                Ok(ValidationResult::Accepted)
            );
            map.record_offset(1, 0, base, 5, (base * 10) as i64);
        }
        assert_eq!(
            map.validate(1, 0, 0, 5, 9000),
            Err(ValidationError::OutOfOrderSequence {
                base_sequence: 0,
                expected_sequence: 30,
            })
        );
    }

    #[test]
    fn lru_eviction() {
        let mut map = ProducerStateMap::new(3);
        assert_eq!(map.validate(1, 0, 0, 1, 1000), Ok(ValidationResult::Accepted));
        assert_eq!(map.validate(2, 0, 0, 1, 2000), Ok(ValidationResult::Accepted));
        assert_eq!(map.validate(3, 0, 0, 1, 3000), Ok(ValidationResult::Accepted));
        // 4th evicts producer 1
        assert_eq!(map.validate(4, 0, 0, 1, 4000), Ok(ValidationResult::Accepted));
        // Producer 1 was evicted — treated as new
        assert_eq!(map.validate(1, 0, 0, 1, 5000), Ok(ValidationResult::Accepted));
    }

    #[test]
    fn multiple_producers_independent() {
        let mut map = ProducerStateMap::new(64);
        assert_eq!(map.validate(1, 0, 0, 1, 1000), Ok(ValidationResult::Accepted));
        assert_eq!(map.validate(2, 0, 0, 1, 1000), Ok(ValidationResult::Accepted));
        assert_eq!(map.validate(1, 0, 1, 1, 2000), Ok(ValidationResult::Accepted));
        assert_eq!(map.validate(2, 0, 1, 1, 2000), Ok(ValidationResult::Accepted));
    }
}
