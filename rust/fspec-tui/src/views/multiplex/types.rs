//! MUX-001/MUX-002 — mux types + live layout state.
//!
//! Feature: spec/features/rust-mux-mode.feature

use ratatui::layout::Rect;

use codelet_rpc_types::SessionId;

/// Which top-level view a mux pane hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum MuxPaneKind {
    #[default]
    Board,
    Agent,
    ChangedFiles,
    Checkpoints,
}

/// Mux grid orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum MuxOrientation {
    /// Left | right | …
    #[default]
    Horizontal,
    /// top / bottom / …
    Vertical,
}

/// Where keyboard focus sits inside the mux grid: the index of the
/// focused pane. (The divider is mouse-drag-resizable only — it has
/// no keyboard focus state; Tab pane/divider cycling was removed
/// 2026-08-26 because Tab is reserved for the agent view's turn-select
/// mode.)
pub type MuxFocus = usize;

/// Mux grid configuration (persisted to the shared `fspec-config.json`
/// under `tui.mux`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MuxConfig {
    pub orientation: MuxOrientation,
    /// BUG-166 percentage scale: ONE percent per inter-pane gap — n panes
    /// → n-1 entries, `splits[i]` = pane i's share of the available axis
    /// (after divider subtraction) in percent; the LAST pane always takes
    /// the integer remainder so the scale sums to 100. An empty/short vec
    /// (legacy configs) means the missing entries fall back to the equal
    /// share `available/n`.
    pub splits: Vec<u16>,
    pub panes: Vec<MuxPaneKind>,
    pub focused_pane: usize,
    pub enabled: bool,
}

/// Live layout state (not persisted).
pub struct MultiplexLayout {
    pub config: MuxConfig,
    /// Cached absolute pane rects from the last render (mouse hit-testing).
    pub pane_rects: Vec<Rect>,
    /// BUG-166: cached divider rects — one per inter-pane gap (n-1).
    pub divider_rects: Vec<Rect>,
    /// Live divider drag state (mouse).
    pub is_dragging: bool,
    /// BUG-166: which divider (gap index 0..n-2) is being dragged.
    pub drag_index: Option<usize>,
    /// Live drag override: the dragged pane's width in cells while a
    /// divider drag is in flight (bypasses the clamp so the drag
    /// tracks the cursor live; the clamp re-applies on release).
    pub drag_width: Option<u16>,
    /// BUG-166: last observed (dragged-pane width, available-axis span)
    /// during the drag — used to commit the percent ON RELEASE (the
    /// live drag only sets `drag_width`, so the left panes stay put).
    pub drag_axis: Option<(u16, u16)>,
    /// Focus: a pane or the divider.
    pub(super) focus: MuxFocus,
    /// MUX-002: agent window offset into the ordered open-session list.
    pub(super) window_start: usize,
    /// MUX-002: open-session ids the window is positioned over (synced via
    /// `sync_window` on every render / session change).
    pub(super) sessions: Vec<SessionId>,
    /// MUX-002: rendered pane list (agent slots beyond the open-session
    /// count are dropped — no blank panes).
    pub(super) rendered_panes: Vec<MuxPaneKind>,
    /// MUX-002: a new-agent prompt was requested by Shift+Right at the
    /// right edge; consumed by `note_session_created`.
    pub(super) pending_new_agent: bool,
    /// MUX-002: last observed body area (full area minus the footer row),
    /// so pane rects can be recomputed BEFORE the first render (mouse
    /// hit-testing + the MUX-002 window assertions read `pane_rects()`
    /// right after `/mux`).
    pub(super) body_area: Option<Rect>,
    /// Pre-mux view, restored on `/mux off` / `MuxExit`.
    pub(super) pre_mux_view: Option<crate::views::ViewMode>,
    /// MUX-006/MUX-007: the pane index the focus-flash accent is armed
    /// on (`None` = no accent — mux not entered yet or disabled). The
    /// flash ANIMATION runs for the first 350ms after (re)arming; from
    /// then on the focused pane keeps the settled final frame (the
    /// left-edge strip, MUX-007 R1) until focus moves or mux exits.
    /// Live-only — never persisted (R8).
    pub(super) flash_pane: Option<MuxFocus>,
    /// MUX-006: render-driven flash clock in ms (0 at arm; ≥ 350 the
    /// scan window has elapsed — the accent then settles). Advanced
    /// +16ms per rendered mux frame, unbounded (the pattern fn clamps
    /// at the settle boundary).
    pub(super) flash_clock_ms: u64,
}

impl MultiplexLayout {
    pub fn new() -> Self {
        let config = MuxConfig::default();
        Self {
            focus: config.focused_pane,
            config,
            pane_rects: Vec::new(),
            divider_rects: Vec::new(),
            is_dragging: false,
            drag_index: None,
            drag_width: None,
            drag_axis: None,
            window_start: 0,
            sessions: Vec::new(),
            rendered_panes: Vec::new(),
            pending_new_agent: false,
            // MUX-002: seed a 120x23 body (24 rows minus the footer row)
            // so `recompute_rects` produces rects BEFORE the first render
            // (mouse hit-testing + window assertions read `pane_rects()`
            // right after `/mux`). The first real render overwrites it
            // with the live frame area.
            body_area: Some(ratatui::layout::Rect::new(0, 0, 120, 23)),
            pre_mux_view: None,
            flash_pane: None,
            flash_clock_ms: 0,
        }
    }

    // ── accessors ────────────────────────────────────────────────────────

    pub fn config(&self) -> &MuxConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut MuxConfig {
        &mut self.config
    }

    pub fn focus(&self) -> MuxFocus {
        self.focus
    }

    pub fn pane_rects(&self) -> &[Rect] {
        &self.pane_rects
    }

    /// BUG-166: the cached divider rects — one per inter-pane gap.
    pub fn divider_rects(&self) -> &[Rect] {
        &self.divider_rects
    }

    pub fn is_dragging(&self) -> bool {
        self.is_dragging
    }

    /// The view that was active before mux was entered (restored on exit).
    pub fn pre_mux_view(&self) -> Option<crate::views::ViewMode> {
        self.pre_mux_view
    }

    // ── MUX-006: focus flash (live-only, never persisted — R8) ──────────

    /// MUX-006: the pane index the focus flash is armed on (`None` = no
    /// flash in flight).
    pub fn flash_pane(&self) -> Option<MuxFocus> {
        self.flash_pane
    }

    /// MUX-006: the flash clock in ms since arming (0 at arm; the render
    /// path advances it [`super::flash::FLASH_FRAME_MS`] per frame).
    pub fn flash_clock_ms(&self) -> u64 {
        self.flash_clock_ms
    }

    /// MUX-006: true iff a focus flash is in flight (mux enabled, armed,
    /// window not yet elapsed). Feeds the run-loop draw gate as the 5th
    /// `tick_should_draw` operand so the 16ms tick keeps redrawing
    /// during the 350ms window (R6). MUX-007 R4: this stays 350ms-
    /// bounded — the SETTLED accent does not keep the gate open.
    pub fn is_flash_active(&self) -> bool {
        self.config.enabled
            && self.flash_pane.is_some()
            && self.flash_clock_ms < super::flash::FLASH_MS
    }

    /// MUX-007: true iff the focused pane owns the flash accent — the
    /// scan is in flight OR the window has elapsed and the accent has
    /// settled (R1: the final left-edge frame stays painted on every
    /// subsequent frame of the focused pane). Paint gate only; NOT fed
    /// to the tick gate (the settled strip is repaint content, not an
    /// animation — R4).
    pub fn has_settled_flash(&self) -> bool {
        self.config.enabled && self.flash_pane.is_some()
    }

    /// MUX-006: (re)arm the focus flash on `pane` — restarts the
    /// 350ms window at the right edge of the scan. No-op when mux is
    /// disabled or `pane` is not a rendered pane index (R4/R7).
    pub(super) fn rearm_flash(&mut self, pane: MuxFocus) {
        if !self.config.enabled || pane >= self.rendered_panes.len().max(1) {
            return;
        }
        self.flash_pane = Some(pane);
        self.flash_clock_ms = 0;
    }

    /// MUX-006: move focus to `next` and re-arm the flash iff the focus
    /// actually moved (R4: re-arm on a focus CHANGE, not on every call
    /// — window rotations that keep the focus must not re-flash).
    pub(super) fn bump_focus(&mut self, next: MuxFocus) {
        if next == self.focus {
            return;
        }
        self.focus = next;
        self.rearm_flash(next);
    }

    /// MUX-006: drop the flash (mux exit — R7: no flash with mux off).
    pub(super) fn disarm_flash(&mut self) {
        self.flash_pane = None;
        self.flash_clock_ms = 0;
    }

    /// MUX-006: advance the render-driven flash clock by one frame
    /// (called once per rendered mux frame — the run loop owns the
    /// clock, the view only reports state). The 350ms scan ANIMATION
    /// expires at [`super::flash::FLASH_MS`] (the tick gate closes,
    /// R6); MUX-007: the accent itself is retained — the clock keeps
    /// advancing (unbounded) and the pattern fn clamps at the settle
    /// boundary, so the focused pane keeps the left-edge strip until
    /// focus moves or mux exits (R1/R2; `disarm_flash` on exit).
    pub fn advance_flash_clock(&mut self) {
        if self.flash_pane.is_none() {
            return;
        }
        self.flash_clock_ms = self
            .flash_clock_ms
            .saturating_add(super::flash::FLASH_FRAME_MS);
    }
}
