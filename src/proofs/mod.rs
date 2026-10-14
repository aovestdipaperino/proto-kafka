//! Kani proof harnesses (Phase 2). Built only by `cargo kani --tests`.
//!
//! `primitives`: the pure codecs equal the reference spec (`proto-kafka-spec`, strict mode) on
//! their whole input domain. `wrappers`: the `Buf` adapters in `protocol::types` equal the pure
//! codecs. `framing`: string/bytes encoders equal the spec interpreter (decoders blocked, see
//! the module). `unsafe_sites`: `StrBytes::from_utf8` and the `BytesMut` contract behind
//! `SegmentedBuf::advance_mut`.
mod framing;
mod primitives;
mod unsafe_sites;
mod wrappers;
