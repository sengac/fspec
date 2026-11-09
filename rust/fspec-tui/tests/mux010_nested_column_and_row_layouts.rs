//! MUX-010 — mux layouts nest columns and rows with a bracket layout
//! string, edited in the config dialog with a live preview.
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature
//!
//! Every test drives the real `App` over a `MockBackend` on a paused tokio
//! clock: slash commands go through `Action::InputSubmitted`, keys and the
//! mouse through `App::handle_event`, and frames are drawn on a
//! `TestBackend` of the scenario's terminal size. The scenarios live in
//! `mux010_nested_layouts/` (grid, focus, config, dialog).

mod common;
mod mux010_nested_layouts;
