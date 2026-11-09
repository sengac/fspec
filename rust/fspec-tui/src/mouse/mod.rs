//! Mouse subsystem — hit-testing helper, button/wheel translation,
//! selection and clipboard.
//!
//! Feature files:
//!   - spec/features/boardview-mouse-handling.feature
//!   - spec/features/app-mouse-dispatch.feature
//!   - spec/features/rpc023-source-shape.feature
//!
//! Card: RPC-023 (parent RPC-002).
//!
//! This module replaces the TypeScript `src/tui/utils/mouseProtocol.ts`
//! SGR parser (crossterm does the parsing for us) and the
//! `MOUSE_ENABLE` / `MOUSE_DISABLE` raw-escape writes (the alt-screen
//! lifecycle in `terminal.rs` owns those globally). [`rect_contains`] is
//! the half-open hit-test helper that components remember last-rendered
//! Rects against. TUI-132: mouse capture stays on for the whole session;
//! text is copied with the TUI's own copy keys.

pub mod clipboard;
pub mod gesture;
pub mod hit_test;
pub mod scrollbar_drag;
pub mod selection;

pub use clipboard::Osc52Clipboard;
pub use gesture::{SelectionGesture, SelectionRecognizer};
pub use hit_test::rect_contains;
pub use scrollbar_drag::{ScrollbarDrag, ScrollbarGeometry};
pub use selection::{Cell, RowSpan, Selection};
