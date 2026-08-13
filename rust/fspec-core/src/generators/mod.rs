//! Markdown generators — Rust ports of `src/generators/*.ts`.
//!
//! Currently hosts the `FOUNDATION.md` generator (RPC-233). Split across
//! three submodules to stay under the 300-line file standard:
//!   * [`foundation_md`] — the top-level renderer + section builders,
//!   * [`foundation_md_diagrams`] — the two Mermaid diagram builders,
//!   * [`foundation_md_util`] — shared JS-semantics + Mermaid pre-check.

pub mod foundation_md;
mod foundation_md_diagrams;
mod foundation_md_util;
pub mod foundation_schema;
pub mod tags_md;
pub mod tags_schema;

pub use foundation_md::generate_foundation_md;
pub use foundation_md_util::validate_mermaid;
pub use foundation_schema::{format_errors, validate_foundation};
pub use tags_md::generate_tags_md;
pub use tags_schema::{format_tags_errors, validate_tags};
