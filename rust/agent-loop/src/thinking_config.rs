//! The thinking wire table (MODEL-010), re-exported from codelet-providers.
//! MODEL-011 moved it there so the sessions crate can check a chosen level
//! and list the levels a model offers; the agent loop still builds each
//! turn's thinking parameters from it.
//!
//! Feature: spec/features/thinking-levels-reach-each-provider-through-a-wire-table.feature
//! Feature: spec/features/thinking-dialog-offers-only-the-models-levels.feature

pub use codelet_providers::thinking_wire::*;
