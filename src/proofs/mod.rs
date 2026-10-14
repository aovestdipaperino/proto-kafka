//! Kani proof harnesses (Phase 2). Built only by `cargo kani --tests`.
//!
//! `primitives`: the pure codecs equal the reference spec (`proto-kafka-spec`, strict mode) on
//! their whole input domain. `wrappers`: the `Buf` adapters in `protocol::types` equal the pure
//! codecs. `framing` and `unsafe_sites` are added in Task 6.
mod primitives;
mod wrappers;
