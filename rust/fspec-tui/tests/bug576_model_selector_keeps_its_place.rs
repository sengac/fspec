//! BUG-576 — the model selector keeps its place across loads, refreshes
//! and edits, and its footer shows the keys that work (AK-9).
//!
//! Feature: spec/features/model-selector-keeps-its-place-across-loads.feature
//!
//! Every test drives the real `App` over a `MockBackend` on a paused tokio
//! clock. MockBackend's call gates (`tests/common/call_gates.rs`) hold the
//! provider list, the session list and session creation, so a test decides
//! when, and in which order, each call answers; a gated call answers with
//! the backend state at the moment it was called. The scenarios live in
//! `bug576_model_selector/` (reloads, opening, current model, selections,
//! footer).

mod bug576_model_selector;
mod common;
