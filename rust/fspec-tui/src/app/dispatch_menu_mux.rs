//! MENU-004 — `App::dispatch` arms for the mux top bar (the Mux-view
//! half of the 2-zone menu bar: ring walk, dropdown, chips, view
//! labels, executed rows).
//!
//! Feature: spec/features/mux-surface-single-top-of-mux-menu-bar-per-pane-suppression-spinner-draw-tick.feature
//! Card: MENU-004 (R-STATE, R-KEYS, R-ZONEB, R-CHIPS, R-EXEC),
//! BUG-200 (R-COLUMNS, R-WRAP, R-CHIPS — the board↔bar ring seam
//! edges).
//!
//! Split out of `dispatch_menu.rs` for the 300-LoC ceiling (the
//! `no_codelet_napi_reference_and_300_loc_ceiling` source-shape guard
//! covers every file under `app/`). The Mux `MenuMove` arm is
//! dispatched before the shared `mux` binding because
//! `menu_move_mux` borrows `self.board_store` +
//! `self.navigator.mux` + `self.agent_view_store` together.

use crate::components::Action;

use super::state::App;

impl App {
    /// MENU-004 R-STATE: the Mux-view half of `dispatch_menu` — the bar's
    /// ring/dropdown state lives on `navigator.mux` (RED CARD 1), so the
    /// arms mutate the mux layout instead of the board store. Executed
    /// rows re-dispatch onto the bus (R-EXEC); chip activation focuses the
    /// session WITHOUT flipping out of Mux (R-CHIPS).
    pub(crate) fn dispatch_menu_mux(&mut self, action: &Action) {
        // BUG-200 R-EDGES: the `MenuMove` arm borrows `self.board_store`
        // + `self.navigator.mux` + `self.agent_view_store` together
        // (`menu_move_mux`), so it must NOT run under the `mux` binding
        // below (the borrow-checker would overlap the `&mut
        // self.navigator.mux` with the other `&mut self` paths). The arm
        // is dispatched before the binding; every other arm only needs
        // the mux layout (plus the agent store for chip resolution).
        if let Action::MenuMove(delta) = action {
            // R-ZONEB + MENU-004 R-KEYS + BUG-200 R-EDGES: walk the
            // board↔bar ring with its two seam edges mirrored
            // (body in `menu_move_mux`).
            self.menu_move_mux(*delta);
            return;
        }
        let mux = &mut self.navigator.mux;
        match action {
            Action::MenuFocusToColumns => mux.menu_dismiss(),
            Action::MenuOpenDropdown(category) => mux.menu_open_or_close(*category),
            Action::MenuCloseDropdown => mux.menu_close_dropdown(),
            Action::MenuDismissBar => mux.menu_dismiss(),
            Action::MenuDropdownCursor(delta) => mux.menu_dropdown_cursor(*delta),
            Action::MenuMoveToItem(index) => mux.menu_move_to_item(*index),
            Action::MenuFocusPane(pane) => {
                // R-ZONEB: Enter/click on a pane view label focuses that
                // pane (set_focus) — no view flip, no session change.
                mux.set_focus(*pane);
            }
            Action::MenuChipActivate(index) => {
                // R-CHIPS: resolve against the GLOBAL painted chip list —
                // do NOT flip out of Mux (the agent pane showing it stays
                // put / the window rotates to it).
                let sessions = crate::views::board::menu_snapshot::active_menu_session_ids(
                    &self.agent_view_store,
                );
                let Some(session) = sessions.get(*index) else {
                    return;
                };
                let Some(open_idx) = self
                    .agent_view_store
                    .open_sessions()
                    .iter()
                    .position(|c| &c.id == session)
                else {
                    return;
                };
                // R-CHIPS: "the focused agent pane shows it if it's in the
                // window, otherwise the window rotates." Land the selection
                // VISIBLE: the chip's selected-highlight (MENU-010) paints
                // only on the FOCUSED agent pane's window session, and
                // `sync_mux_focus_to_session` treats that pane as the source
                // of truth — so the activation must also rotate the agent
                // window + move pane focus to the agent pane that renders
                // the clicked session. Without this the click either
                // highlights nothing (Board pane focused) or highlights the
                // wrong chip, and the next event's focus-sync reverts the
                // session switch. No-op when no agent pane is rendered
                // (a Board|Files grid) or the session was closed.
                mux.focus_agent_pane_for_session(session);
                // RPC-024: the full session switch — snapshot the outgoing
                // draft (the live composer hosts the previous current
                // session's draft), focus the slot, restore the incoming
                // session's persisted draft, refresh its supervisor badge.
                // A bare `focus_session_index` would leak: the
                // `sync_mux_focus_to_session` hook early-returns when the
                // focused pane's session already equals the store's
                // current, so its round-trip would never run and the
                // composer would keep showing the previous session's
                // draft on the new session's pane.
                self.switch_to_session_index(open_idx);
                // Stay in Mux — no `active_view` flip, no navigation target.
            }
            Action::MenuExecuteItem { category, row } => {
                let Some(menu_action) = mux.menu_execute_item(*category, *row) else {
                    return;
                };
                // R-EXEC (BUG-203): the registry mapping is payload-free —
                // `NewAgent` ALWAYS mounts the CreateSessionDialog (the
                // shared RPC-060 helper, BUG-199 / BUG-203 parity); the
                // MENU-002/003 R8 current-session substitution is removed.
                self.dispatch(menu_action.to_action());
            }
            // MENU-009 Q1: the mux bar paints NO Zone C — the token is
            // unreachable on this surface (defensive no-op).
            Action::MenuZoneCActivate(_) => {}
            _ => {}
        }
    }

    /// BUG-200 R-EDGES + BUG-201: the Mux `MenuMove` arm body — the
    /// board↔bar ring with its two seam edges mirrored, gated on the
    /// Board pane being the FOCUSED mux pane. When it is, one
    /// continuous ring (user directive): the 7 board columns
    /// (col0 = backlog … col6 = blocked) ⇄ the mux bar's items
    /// (Item0 = Kanban …) ⇄ the Zone B cells (pane view labels + the
    /// global chips), ring order
    ///
    /// ```text
    /// col0 … col6 → Item0 … ItemN → ZoneB0 … ZoneB_last → col0 …
    /// ```
    ///
    /// so each seam crossing is a single step, exactly like the single
    /// Board view's column⇄item⇄chip⇄button ring (MENU-002/009):
    ///
    /// - **Seam 1 (columns' right edge ⇄ the bar's left edge):**
    ///   `Right` off col6 (the last column) lands on `Item(0)` (the
    ///   key path uses the `MenuMoveToItem(0)` edge rule in
    ///   `classify_bar_key`; the dispatch mirror covers the wheel
    ///   path, which emits `MenuMove` directly); `Left` off `Item(0)`
    ///   lands on col6.
    /// - **Seam 2 (the bar's right edge ⇄ the columns' left edge):**
    ///   `Right` off the LAST Zone B cell (last chip / last view
    ///   label) lands on col0 (backlog); `Left` off col0 lands on the
    ///   last Zone B cell. The board store's own wrap into its
    ///   (suppressed, invisible) bar segment is mirrored onto the MUX
    ///   bar's ring so the columns stay a continuous wrap-around.
    ///
    /// BUG-201: when the Board pane is NOT the focused mux pane the
    /// seams DO NOT fire — the bar is a closed items⇄Zone B loop
    /// (no column stops; `MenuSnapshot::advance` wraps both edges
    /// onto the bar itself) and the board store's `focused_column`
    /// stays untouched. Interior landings walk each segment's own
    /// ring (the board store `menu_move` for the columns;
    /// `MenuSnapshot::advance` for the bar). `menu_chips` agrees with
    /// the painted bar because the same dispatch tick's
    /// `feed_menu_ring_size` refreshed it (R8 lockstep); the Zone B
    /// cells are `view_label_count()` + chips.
    fn menu_move_mux(&mut self, delta: i32) {
        use crate::components::menu_bar::MenuFocus;
        use crate::store::COLUMN_ORDER;
        // The board-pane flag is captured BEFORE the `mux` mutable
        // borrow (it reads the same layout — stable for the walk: no
        // arm below mutates the pane list or the pane focus). It scopes
        // the COLUMN walk (R-KEYS) AND — BUG-201 — the two seam
        // crossings: with the Board pane NOT focused the bar is a
        // closed items⇄Zone B loop with NO column stops (the
        // `advance` walk below wraps at both edges) and the
        // board store's `focused_column` must never be touched.
        let board_pane = self.mux_board_pane_focused();
        let mux = &mut self.navigator.mux;
        if mux.menu_ring_active() {
            let focus = mux.menu_focus();
            let open = mux.open_menu();
            let last_cell = mux
                .view_label_count()
                .saturating_add(mux.menu_chips())
                .saturating_sub(1);
            // Seam 2 (the bar's right edge): the last Zone B cell (last
            // chip / last view label) — Right wraps back to the FIRST
            // column (backlog), the columns' left edge (col0 ⇄
            // lastZoneB). This is the "go right off the end of the menu
            // bar → back to the board" crossing. BUG-201: only while the
            // Board pane is focused — off the board pane the closed
            // loop's `advance` wrap (last cell → Item(0)) owns this step
            // instead and the board columns stay untouched.
            if board_pane
                && delta > 0
                && matches!(focus, Some(MenuFocus::ZoneB(c)) if c == last_cell)
            {
                self.board_store.set_focused_column(COLUMN_ORDER[0]);
                mux.menu_dismiss();
                return;
            }
            // Seam 1 (the bar's left edge): the first item (Kanban) —
            // Left wraps to the LAST column (blocked), the columns'
            // right edge (col6 ⇄ Item0). BUG-201: board-pane-gated,
            // same as Seam 2 (off the board pane `advance` wraps
            // Item(0) → the last Zone B cell).
            if board_pane && delta < 0 && matches!(focus, Some(MenuFocus::Item(0))) {
                self.board_store
                    .set_focused_column(COLUMN_ORDER[COLUMN_ORDER.len() - 1]);
                mux.menu_dismiss();
                return;
            }
            // Interior bar walk: items ⇄ Zone B cells (the per-frame
            // snapshot, the board store's R8 `menu_move` mirror).
            let snapshot = crate::views::multiplex::menu_snapshot::build_snapshot(
                mux,
                &self.agent_view_store,
                focus,
                open,
                crate::views::multiplex::menu_render::now_ms(),
            );
            let next = snapshot.advance(focus, delta);
            mux.menu_move_to_ring_position(next);
            mux.menu_reanchor_or_close();
            return;
        }
        // Bar ring inactive: a column owns the focus. Only the board
        // pane feeds the columns (R-KEYS).
        if !board_pane {
            return;
        }
        self.board_store.menu_move(delta);
        // BUG-200 R-EDGES: the board store's own ring edge wraps into
        // its (suppressed) bar segment, which is invisible in Mux —
        // mirror the crossing onto the MUX bar's ring so the columns
        // stay a continuous wrap-around ring:
        //
        // - `delta > 0` (Right off the last column, col6): the store
        //   wraps to its first item (Kanban) — seam 1, land on the MUX
        //   bar's first item. (The KEY path uses the `MenuMoveToItem(0)`
        //   intercept instead; this covers the wheel path, which emits
        //   `MenuMove` directly.)
        // - `delta < 0` (Left off the first column, col0): the store
        //   wraps to its last bar stop — seam 2, land on the MUX bar's
        //   LAST Zone B cell (last chip when chips paint; the last view
        //   label when they don't).
        //
        // Either way the store's stale (hidden) bar focus clears so the
        // next `MenuMove` continues on the MUX ring, not the store's
        // hidden one.
        if delta > 0 && self.board_store.menu_focus() == Some(MenuFocus::Item(0)) {
            self.board_store.menu_clear_focus();
            mux.menu_move_to_ring_position(MenuFocus::Item(0));
        } else if delta < 0 && self.board_store.menu_focus().is_some() {
            self.board_store.menu_clear_focus();
            let last_cell = mux
                .view_label_count()
                .saturating_add(mux.menu_chips())
                .saturating_sub(1);
            mux.menu_move_to_ring_position(MenuFocus::ZoneB(last_cell));
        }
    }
}
