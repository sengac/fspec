//! TUI-129 — advertised keys do what their hints say.
//!
//! Feature: spec/features/advertised-keys-do-what-their-hints-say.feature
//!
//! Every test drives the real `App` over a `MockBackend` on a paused tokio
//! clock: keys go through `App::handle_event`, and every task and action
//! they start is run until nothing moves (`fixture::drain`), so a check
//! that something did not happen is made after everything the key caused
//! has run. The scenarios live in `tui129_advertised_keys/` (board, menus,
//! agent view).

mod common;
mod tui129_advertised_keys;
