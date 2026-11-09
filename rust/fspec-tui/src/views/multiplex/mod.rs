//! MUX-001 — multiplex (mux) mode: top-level views in a configurable grid.
//!
//! Feature: spec/features/rust-mux-mode.feature +
//! spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature
//!
//! `MultiplexLayout` owns the live mux state (config + cached pane rects +
//! focus + divider drag + the MUX-002 agent window) and routes
//! keyboard/mouse events to exactly ONE pane (the focused one) — the
//! "trap". `App::dispatch` is the single mutation surface; this module is
//! the Navigator-level event + render layer.
//!
//! File map (300-LoC ceiling):
//!   - `mod.rs`    — `MultiplexLayout` mutation API (enable/config/drag)
//!   - `types.rs`  — MuxPaneKind / MuxConfig / state fields
//!   - `layout_tree/` — MUX-010 nested layout: grammar, geometry, fit,
//!     spatial focus, drag re-weighting
//!   - `window.rs` — MUX-002 agent window (clamping, chip rotation, new-agent)
//!   - `focus_moves.rs` — MUX-012 layout-order Shift+Left/Right, MUX-010
//!     spatial Shift+Up/Down
//!   - `rects.rs`  — live visibility + pane-rect recomputation
//!   - `flash.rs`  — MUX-006 pure focus-flash pattern math (bottom-to-top scan)
//!   - `render.rs` — pane dispatch + dividers + mux footer paint
//!   - `keys.rs`   — keyboard routing classification
//!   - `mouse.rs`  — hit-test, click-to-focus, per-divider drag
//!   - `presets.rs`— the default layout
//!   - `menu_bar_rows.rs` — MUX-013 bar buttons and their wrapped rows
//!   - `panes.rs`  — BUG-183 live pane close (Esc on a lazy pane, transient)

pub mod flash;
pub mod focus_moves;
pub mod keys;
pub mod layout_tree;
pub mod menu_bar_rows; // MUX-013
pub mod menu_keys;
pub mod menu_mouse;
pub mod menu_render;
pub mod menu_snapshot;
pub mod menu_state;
pub mod mouse;
pub mod panes;
pub mod presets;
pub mod rects;
pub mod render;
pub mod types;
pub mod window;
pub mod window_focus; // TUI-125

pub use layout_tree::{MuxLayout, MuxLayoutError, MuxNode};
pub use types::{MultiplexLayout, MuxConfig, MuxFocus, MuxPaneKind};

impl Default for MuxConfig {
    fn default() -> Self {
        // Mux is OFF by default; the preset shape (board, agent with
        // agent home focus) is what `/mux` applies on first enable.
        let mut cfg = presets::default_config();
        cfg.enabled = false;
        cfg
    }
}

impl Default for MultiplexLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiplexLayout {
    pub(crate) fn cancel_new_agent_request(&mut self) {
        self.pending_new_agent = false;
    }

    /// MUX-013: the bar's New Agent / New Bot place the session they
    /// start by the MUX-002 rule (`note_session_created`).
    pub(crate) fn request_new_agent(&mut self) {
        self.pending_new_agent = true;
    }

    /// Reset every live (non-persisted) grid field for a fresh layout.
    fn reset_live(&mut self) {
        self.window_start = 0;
        self.pending_new_agent = false;
        self.closed_panes.clear();
        self.pane_rects.clear();
        self.dividers.clear();
        self.clear_drag();
        self.show_all_panes();
    }

    fn clear_drag(&mut self) {
        self.is_dragging = false;
        self.drag_index = None;
        self.drag_divider = None;
        self.drag_layout = None;
    }

    // ── mutation (called from Navigator::apply_action / routing) ────────

    /// Enable mux with the default preset. The pre-mux view is set
    /// separately via [`Self::set_pre_mux_view`] (defaults to Board).
    ///
    /// Fresh entry focuses the BOARD pane (index 0) — the view the user
    /// came from — while `config.focused_pane` (agent, per the default
    /// preset) is the persisted "home" focus restored on reload.
    pub fn enable_default(&mut self) {
        if self.pre_mux_view.is_none() {
            self.pre_mux_view = Some(crate::views::ViewMode::Board);
        }
        self.config = presets::default_config();
        self.focus = 0;
        self.reset_live();
        // MUX-006: fresh mux entry re-arms the flash on the board pane.
        self.rearm_flash(0);
        // MENU-004 R-STATE: a fresh mux entry starts with a clean bar
        // (no stale focus/dropdown/geometry from a prior session).
        self.menu_reset();
    }

    /// Record the view active before mux was entered (restored on
    /// `/mux off` / `MuxExit`).
    pub fn set_pre_mux_view(&mut self, view: crate::views::ViewMode) {
        self.pre_mux_view = Some(view);
    }

    /// Enable mux with an explicit config (saved / default), remembering
    /// the pre-mux view.
    pub fn enable_with_config(&mut self, config: MuxConfig, pre_mux: crate::views::ViewMode) {
        self.pre_mux_view = Some(pre_mux);
        self.config = config;
        self.reset_live();
        let n = self.rendered_panes.len().max(1);
        self.focus = self.config.focused_pane.min(n - 1);
        // MUX-006: config-driven entry re-arms the flash on the focused
        // pane (the persisted "home" focus).
        self.rearm_flash(self.focus);
        // MENU-004 R-STATE: a fresh mux entry starts with a clean bar.
        self.menu_reset();
    }

    /// Disable mux. Returns the pre-mux view (or `Board`).
    pub fn disable(&mut self) -> crate::views::ViewMode {
        self.config.enabled = false;
        self.clear_drag();
        self.window_start = 0;
        self.pending_new_agent = false;
        self.closed_panes.clear();
        self.rendered_panes = Vec::new();
        self.visible.clear();
        self.transient_board = false;
        self.pane_rects.clear();
        self.dividers.clear();
        // MUX-006: mux exit drops the flash (R7: no flash with mux off).
        self.disarm_flash();
        // MENU-004 R-STATE: mux exit drops the bar state too — no stale
        // focus/dropdown survives on the saved layout.
        self.menu_reset();
        self.pre_mux_view.take().unwrap_or_default()
    }

    /// Move the mux focus to `focus` (clamped to the rendered pane
    /// count) and re-arm the focus flash iff the focus actually moved
    /// (MUX-006 R4: re-arm on a focus CHANGE — no-op re-focuses, window
    /// rotations and same-pane clicks must not re-flash).
    pub fn set_focus(&mut self, focus: MuxFocus) {
        let n = self.rendered_panes.len().max(1);
        self.bump_focus(focus.min(n - 1));
    }

    /// MUX-010: replace the layout (from `/mux <layout>`) and enable mux.
    /// The agent window, closed panes and drag reset; focus clamps to the
    /// new pane count and the flash re-arms (MUX-006 R4: a layout change
    /// re-arms even when the focus index is unchanged).
    pub fn set_layout(&mut self, layout: MuxLayout) {
        self.config.layout = layout;
        self.config.enabled = true;
        self.reset_live();
        let n = self.rendered_panes.len().max(1);
        self.focus = self.focus.min(n - 1);
        self.rearm_flash(self.focus);
        self.recompute_rects();
    }

    /// Begin a divider drag (mouse-down on divider `index`).
    pub fn begin_drag(&mut self, index: usize) {
        let Some(divider) = self.dividers.get(index).cloned() else {
            return;
        };
        self.is_dragging = true;
        self.drag_index = Some(index);
        self.drag_divider = Some(divider);
        self.drag_layout = None;
    }

    /// MUX-010: track a drag to `(col, row)` — the item before the
    /// dragged divider ends at the cursor; the pair is re-weighted live
    /// on a copy of the layout (the stored layout changes on release).
    pub fn drag_to(&mut self, col: u16, row: u16) {
        let Some(divider) = self.drag_divider.clone() else {
            return;
        };
        if !self.is_dragging || self.transient_board {
            return;
        }
        let cursor = if divider.between_columns { col } else { row };
        let before = cursor.saturating_sub(divider.start);
        let mut layout = self.config.layout.clone();
        if layout_tree::drag::reweight(&mut layout, &divider, before) {
            self.drag_layout = Some(layout);
            self.recompute_rects();
        }
    }

    /// End a divider drag: the re-weighted layout becomes the stored
    /// layout (BUG-166: no snap-back — the divider stays where it was
    /// released, within the weight rounding).
    pub fn finish_drag(&mut self) {
        let layout = self.drag_layout.take();
        self.clear_drag();
        if let Some(layout) = layout {
            self.config.layout = layout;
        }
        self.recompute_rects();
    }
}
