//! Component trait + Priority enum + EventResult + Action types.
//!
//! Feature: spec/features/fspec-tui-trait-surface.feature
//! Rules: [5] Priority, [6] EventResult, [7] Component
//!
//! These types are deliberately small and free of UI logic — they sit
//! beneath the [`crate::compositor::Compositor`] dispatcher and are the
//! only thing the App layer ever sees of a "widget".

use crossterm::event::Event;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::compositor::Compositor;

pub mod attachment_picker_dialog;
pub mod board_exit_confirmation_dialog;
pub mod checkpoint_restore_dialog;
pub mod create_session_dialog;
pub mod dialog_button_hits;
pub mod dialog_theme;
pub mod dialog_theme_paint;
pub mod dialog_theme_rows;
pub mod disconnect_dialog;
pub mod error_dialog;
pub mod exit_confirmation_dialog;
pub mod hello;
pub mod help_content;
pub mod help_dialog;
pub mod help_dialog_scroll;
pub mod list_scrollbar;
pub mod load_state;
pub mod loading_dialog;
pub mod menu_bar;
pub mod menu_bar_help;
pub mod model_selector_dialog_rows;
pub mod mux_config_dialog;
pub mod mux_config_dialog_rows;
pub mod notification_dialog;
pub mod role_dialog;
pub mod scroll_viewport;
pub mod session_storage_dialog;
pub mod spinner;
pub mod status_dialog;
pub mod thinking_level_dialog;
pub mod three_button_dialog;
pub mod work_unit_search_dialog;
pub mod work_unit_search_dialog_accessors;
pub mod work_unit_search_dialog_mouse;
pub mod work_unit_search_filter;
pub mod work_unit_search_rows;

/// Event-handling priority for layered components (RPC-002 doc 09 §A.1).
///
/// `#[repr(u32)]` with discriminants from RPC-008 rule [5]. The exact
/// numeric values matter for cross-binary stability and for the
/// dispatcher's `sort_by_key` ordering.
///
/// RPC-022 added [`Priority::Foreground`] (900) between `High` and
/// `Critical` so the new modal dialogs (ModelSelectorDialog,
/// ThinkingLevelDialog) sit above the always-on background views but
/// still beneath `Critical` dialogs (HelpDialog / DisconnectDialog).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Background = 100,
    Low = 200,
    Medium = 500,
    High = 800,
    Foreground = 900,
    Critical = 1000,
}

/// Deferred mutation of the [`Compositor`] requested by a layer's
/// [`Component::handle_event`] return value (RPC-008 rule [6]).
///
/// The closure receives `&mut Compositor` AFTER dispatch completes so
/// the layer can request its own removal — or push a new layer — without
/// running into borrow conflicts with the iterator dispatch was using.
pub type Callback = Box<dyn FnOnce(&mut Compositor) + Send>;

/// Outcome of a [`Component::handle_event`] call (RPC-008 rule [6]).
///
/// `Ignored` propagates to the next layer in priority order. `Consumed`
/// short-circuits dispatch. Both variants may carry an optional
/// [`Callback`] that the App layer runs against the compositor AFTER
/// dispatch unwinds.
pub enum EventResult {
    Ignored(Option<Callback>),
    Consumed(Option<Callback>),
}

impl EventResult {
    /// Convenience helper: a no-callback Ignored.
    pub fn ignored() -> Self {
        EventResult::Ignored(None)
    }

    /// Convenience helper: a no-callback Consumed.
    pub fn consumed() -> Self {
        EventResult::Consumed(None)
    }

    /// Returns true iff this result short-circuited dispatch.
    pub fn is_consumed(&self) -> bool {
        matches!(self, EventResult::Consumed(_))
    }
}

include!("action_runtime.rs");
include!("action_board.rs");
include!("action_agent_navigation.rs");
include!("action_session_context.rs");
include!("action_prompts.rs");
include!("action_provider_settings.rs");
include!("action_slash_operations.rs");
include!("action_session_controls.rs");
include!("action_project_files.rs");
include!("action_transcript_selection.rs");
include!("action_mux_menu.rs");

macro_rules! compose_action_finish {
    ($($collected:tt)*) => {
        /// Application-level action propagated through every layer's
        /// [`Component::update`] (RPC-008 rule [10]).
        ///
        /// In RPC-008 the only Action that matters end-to-end is `Quit` —
        /// later cards (RPC-009 list view + REPL, RPC-002 Slice 03+ widgets)
        /// will extend this enum.
        ///
        /// RPC-009 extension: seven new variants threaded through the App's
        /// bootstrap subscriber tasks. The `PartialEq, Eq` derives from RPC-008
        /// are dropped because `StreamChunk` (carried by `ChunkReceived`) does
        /// not derive `PartialEq` (it transitively contains struct types like
        /// `TokenTracker` and `ToolCallInfo` that only derive `Debug, Clone,
        /// Serialize, Deserialize`). Tests that need to inspect Action values
        /// use `matches!()` rather than `assert_eq!()`.
        #[derive(Debug, Clone)]
        pub enum Action {
            $($collected)*
        }
    };
}

compose_action_runtime! {}

/// Visible UI element that participates in event dispatch + rendering.
///
/// Deliberately `Send` (no `Sync`) so layers can hold non-Sync state
/// (e.g. once-locks, channels) — the [`Compositor`] is single-threaded
/// from a Component's perspective.
pub trait Component: Send {
    /// Event-dispatch priority (default `Medium`).
    fn priority(&self) -> Priority {
        Priority::Medium
    }

    /// When false the layer is skipped during dispatch + render
    /// (default true).
    fn is_active(&self) -> bool {
        true
    }

    /// Stable identifier — used by `Compositor::remove` and tests.
    /// No default; every Component must provide one.
    fn id(&self) -> &str;

    /// Handle a crossterm `Event`. Default: `Ignored(None)` — the
    /// dispatcher then walks down to the next layer.
    fn handle_event(&mut self, _event: &Event) -> EventResult {
        EventResult::ignored()
    }

    /// Receive an [`Action`] fanned out top-down. Default: `None`.
    fn update(&mut self, _action: Action) -> Option<Action> {
        None
    }

    /// Paint the layer onto the supplied [`Buffer`]. No default —
    /// every Component must implement.
    fn render(&mut self, area: Rect, buf: &mut Buffer);
}
