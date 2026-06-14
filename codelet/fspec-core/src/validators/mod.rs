//! Shared validators — Rust analogue of `src/validators/*.ts`.
//!
//! Hosts the single, spec-compliant JSON Schema engine ([`json_schema`])
//! wrapping the [`jsonschema`](https://docs.rs/jsonschema) crate. Both the
//! foundation and tags schema gates delegate here so there is exactly ONE
//! validation implementation in the codebase (DRY).

pub mod json_schema;

pub use json_schema::{join_errors, validate_against_schema, SchemaError};
