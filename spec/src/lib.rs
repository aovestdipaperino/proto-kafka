//! Executable reference specification of the Kafka wire protocol.
//!
//! Independence rule: this crate MUST NOT depend on `proto-kafka`, and its logic
//! (`wire`, `interp`, `records`) is written from the Kafka protocol guide and
//! `docs/FINDINGS.md`, never ported from `proto-kafka` or `protocol_codegen`.
//! The only generated file is `generated_schemas.rs`, which contains data only.

pub mod error;
pub mod header;
pub mod interp;
pub mod schema;
pub mod value;
pub mod wire;
