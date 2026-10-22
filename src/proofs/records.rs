//! The record sequence rule equals the spec's transcription of Java (complete).

use crate::records::record_sequence;
use proto_kafka_spec::records as spec;

/// D6: for every base sequence and offset delta, the crate's `i64` formulation of
/// `DefaultRecord.readFrom`'s sequence equals the spec's literal Java `int` transcription.
#[kani::proof]
fn record_sequence_conforms() {
    let base_sequence: i32 = kani::any();
    let offset_delta: i32 = kani::any();
    assert_eq!(
        record_sequence(base_sequence, offset_delta),
        spec::record_sequence(base_sequence, offset_delta)
    );
}
