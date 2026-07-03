//! Mouse subsystem — hit-testing helper, button/wheel translation, and
//! the TUI-078 native-text-selection toggle scaffolding.
//!
//! Feature files:
//!   - spec/features/boardview-mouse-handling.feature
//!   - spec/features/mouse-tracking-toggle.feature
//!   - spec/features/app-mouse-dispatch.feature
//!   - spec/features/rpc023-source-shape.feature
//!
//! Card: RPC-023 (parent RPC-002).
//!
//! This module replaces the TypeScript `src/tui/utils/mouseProtocol.ts`
//! SGR parser (crossterm does the parsing for us) and the
//! `MOUSE_ENABLE` / `MOUSE_DISABLE` raw-escape writes (the alt-screen
//! lifecycle in `terminal.rs` owns those globally). The two surfaces
//! kept here are exactly what crossterm does not have an opinion about:
//!
//!   * [`rect_contains`] — half-open hit-test helper that components
//!     remember last-rendered Rects against.
//!   * [`MouseTrackingToggle`] — TUI-078 scaffolding for RPC-019's
//!     native text-selection coexistence (button-press → disable mouse
//!     capture so the terminal can begin a selection; 5-second debounce
//!     timer + immediate release-handler re-enable).

pub mod clipboard;
pub mod gesture;
pub mod hit_test;
pub mod selection;
pub mod toggle;

pub use clipboard::Osc52Clipboard;
pub use gesture::{SelectionGesture, SelectionRecognizer};
pub use hit_test::rect_contains;
pub use selection::{Cell, RowSpan, Selection};
pub use toggle::MouseTrackingToggle;
