//! MUX-001 — multiplex (mux) mode: top-level views in a configurable grid.
//!
//! Feature: spec/features/rust-mux-mode.feature
//!
//! `MultiplexLayout` owns the live mux state (config + cached pane rects +
//! focus + divider drag + the MUX-002 agent window) and routes
//! keyboard/mouse events to exactly ONE pane (the focused one) — the
//! "trap". `App::dispatch` is the single mutation surface; this module is
//! the Navigator-level event + render layer.
//!
//! File map (300-LoC ceiling):
//!   - `mod.rs`    — `MultiplexLayout` mutation API (enable/config/drag)
//!   - `types.rs`  — MuxPaneKind / MuxOrientation / MuxConfig / state fields
//!   - `layout.rs` — pure split math (percentage scale, per-gap dividers)
//!   - `splits.rs` — pure percentage-scale math (equal scale, rescale)
//!   - `window.rs` — MUX-002 agent window (rotation, clamping, new-agent)
//!   - `rects.rs`  — live pane-rect recomputation
//!   - `flash.rs`  — MUX-006 pure focus-flash pattern math (right-to-left scan)
//!   - `render.rs` — pane dispatch + dividers + mux footer paint
//!   - `keys.rs`   — keyboard routing classification
//!   - `mouse.rs`  — hit-test, click-to-focus, per-divider drag
//!   - `presets.rs`— default preset + pane-count expansion

pub mod flash;
pub mod keys;
pub mod layout;
pub mod mouse;
pub mod presets;
pub mod rects;
pub mod render;
pub mod splits;
pub mod types;
pub mod window;

pub use layout::{
    calculate_pane_rects, calculate_pane_rects_with_override, divider_rects, DIVIDER_SIZE,
};
pub use presets::DEFAULT_PANES;
pub use splits::{equal_scale, is_equal_scale, normalize_scale, scale_scales, set_drag_pcts};
pub use types::{MultiplexLayout, MuxConfig, MuxFocus, MuxOrientation, MuxPaneKind};

impl Default for MuxConfig {
    fn default() -> Self {
        // Mux is OFF by default; the preset shape (Board | Agent, 50/50,
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
        self.rendered_panes = self.config.panes.clone();
        self.window_start = 0;
        self.pending_new_agent = false;
        self.pane_rects.clear();
        self.divider_rects.clear();
        self.is_dragging = false;
        self.drag_index = None;
        self.drag_width = None;
        self.drag_axis = None;
        // MUX-006: fresh mux entry re-arms the flash on the board pane.
        self.rearm_flash(0);
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
        self.rendered_panes = self.config.panes.clone();
        self.window_start = 0;
        self.pending_new_agent = false;
        let n = self.config.panes.len().max(1);
        self.focus = self.config.focused_pane.min(n - 1);
        self.pane_rects.clear();
        self.divider_rects.clear();
        self.is_dragging = false;
        self.drag_index = None;
        self.drag_width = None;
        self.drag_axis = None;
        // MUX-006: config-driven entry re-arms the flash on the focused
        // pane (the persisted "home" focus).
        self.rearm_flash(self.focus);
    }

    /// Disable mux. Returns the pre-mux view (or `Board`).
    pub fn disable(&mut self) -> crate::views::ViewMode {
        self.config.enabled = false;
        self.is_dragging = false;
        self.drag_index = None;
        self.drag_width = None;
        self.drag_axis = None;
        self.window_start = 0;
        self.pending_new_agent = false;
        self.rendered_panes = Vec::new();
        self.pane_rects.clear();
        self.divider_rects.clear();
        // MUX-006: mux exit drops the flash (R7: no flash with mux off).
        self.disarm_flash();
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

    /// Set the pane count, expanding/shrinking the pane list with the
    /// default kinds (Board | Agent | ChangedFiles | Checkpoints).
    /// BUG-166: the percentage scale RESCALES to the new count instead
    /// of resetting to equal division (equal splits stay equal).
    pub fn set_pane_count(&mut self, count: usize) {
        let panes: Vec<MuxPaneKind> = DEFAULT_PANES.iter().take(count).copied().collect();
        self.rendered_panes = panes.clone();
        self.config.panes = panes;
        self.config.splits = scale_scales(&self.config.splits, count);
        self.config.enabled = true;
        let n = self.config.panes.len().max(1);
        // Pane-count changes re-home focus to the LAST pane (the newly
        // added / remaining trailing pane). MUX-006: a pane-count
        // (re)layout re-arms the flash on the (re-homed) focused pane —
        // R4 lists pane-count changes as re-arm triggers even when the
        // focus index is unchanged.
        self.focus = n - 1;
        self.rearm_flash(self.focus);
        self.recompute_rects();
    }

    /// Replace the pane list + optional leading split percent.
    /// BUG-166: the scale always ends up with `panes.len() - 1` entries —
    /// an explicit percent seeds entry 0 and the rest split the remainder
    /// equally; with no percent the EXISTING scale rescales to the new
    /// count (equal stays equal, non-equal keeps its ratio).
    pub fn set_pane_list(&mut self, panes: Vec<MuxPaneKind>, split_percent: Option<u16>) {
        let n = panes.len();
        self.config.splits = match split_percent {
            Some(p) if n > 1 => {
                let rest = (100u32.saturating_sub(p as u32).max(1) / (n - 1) as u32).max(1);
                std::iter::once(p)
                    .chain(std::iter::repeat_n(rest as u16, n - 2))
                    .collect()
            }
            _ => scale_scales(&self.config.splits, n),
        };
        self.config.panes = panes;
        self.config.enabled = true;
        // MUX-002: a pane-list change resets the agent window; the
        // rendered list re-derives on the next `sync_window` (or stays
        // as the full list when no sessions are open yet).
        self.window_start = 0;
        self.pending_new_agent = false;
        self.rendered_panes = self.config.panes.clone();
        let n = self.rendered_panes.len().max(1);
        // MUX-006: a pane-list (re)layout re-arms the flash on the
        // (clamped) focused pane — R4 lists pane-list changes as
        // re-arm triggers even when the focus index is unchanged.
        self.focus = self.focus.min(n - 1);
        self.rearm_flash(self.focus);
        self.recompute_rects();
    }

    pub fn set_orientation(&mut self, orientation: MuxOrientation) {
        self.config.orientation = orientation;
        self.recompute_rects();
    }

    /// BUG-166: set scale entry `index` (the pane BEFORE divider
    /// `index`) from the released cursor position, so the divider stays
    /// where the user left it — this ALWAYS writes the entry (even when
    /// the scale was equal), which is the fix for the snap-back bug.
    /// The panes to the right of the dragged divider absorb the change
    /// proportionally; the left panes keep their shares.
    pub fn set_split_index_from_position(&mut self, index: usize, width: u16, total: u16) {
        if total == 0 || index >= self.config.panes.len().saturating_sub(1) {
            return;
        }
        // Grow a short (legacy) scale to full length first, so the
        // release never discards the entries of the other gaps.
        let n = self.config.panes.len();
        if self.config.splits.len() < n - 1 {
            self.config.splits = normalize_scale(&self.config.splits, n);
        }
        // percent = dragged pane width / available axis (the layout-math
        // basis), rounded to the nearest so release keeps the divider
        // within a half cell of where it was dropped. Honoring MUX-003:
        // no 10..=90 clamp on the drag — the position is kept as-is.
        let pct =
            ((width as u32 * 100 + total.max(1) as u32 / 2) / total.max(1) as u32).min(99) as u16;
        self.config.splits = set_drag_pcts(&self.config.splits, index, pct.max(1));
        self.recompute_rects();
    }

    /// End a divider drag: commit the released position as the scale
    /// entry for the dragged divider (BUG-166: no snap-back — the
    /// live drag only tracked `drag_width`, so this is where the
    /// percent is stored, with nearest-integer rounding).
    pub fn finish_drag(&mut self) {
        let index = self.drag_index;
        let axis = self.drag_axis;
        self.is_dragging = false;
        self.drag_index = None;
        self.drag_width = None;
        self.drag_axis = None;
        if let (Some(index), Some((width, total))) = (index, axis) {
            self.set_split_index_from_position(index, width, total);
        }
    }

    /// Begin a divider drag (mouse-down on divider `index`).
    pub fn begin_drag(&mut self, index: usize) {
        self.is_dragging = true;
        self.drag_index = Some(index);
        self.drag_width = None;
        self.drag_axis = None;
    }

    /// Update the live drag width (dragged pane, in cells) and remember
    /// the axis for the release-time commit. The live override (NOT the
    /// scale) drives the recompute, so the panes track the cursor while
    /// the left panes stay put.
    pub fn update_drag(&mut self, index: usize, width: u16, position: u16, total: u16) {
        if !self.is_dragging || self.drag_index != Some(index) {
            return;
        }
        let _ = position;
        self.drag_width = Some(width);
        self.drag_axis = Some((width, total));
        self.recompute_rects();
    }
}
