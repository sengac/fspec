macro_rules! compose_action_mux_menu {
    ($($collected:tt)*) => {
        compose_action_finish! {
            $($collected)*
            // MUX-001: mux mode (multiplexed top-level views with /mux config).
            /// Enter a work unit from the board pane while in mux mode — focus
            /// the agent pane instead of flipping the whole view (R8). All
            /// other mux transitions are /mux-driven (dispatch_mux.rs).
            MuxEnterWorkUnit(String),
            // MUX-004: mux configuration dialog (bare /mux opens it).
            /// The MuxConfigDialog committed its draft (Enter) — apply the
            /// draft layout to the live mux (orientation/panes/scale re-derived
            /// for the new pane count) and flip the enabled state per the
            /// draft (R7). Emitted by `MuxConfigDialog` on Enter.
            MuxConfigApplied(crate::views::multiplex::MuxConfig),
            /// The MuxConfigDialog committed its draft AND persisted it to the
            /// shared `fspec-config.json` `tui.mux` key ('s' keybinding). Emitted
            /// by `MuxConfigDialog` on 's'.
            MuxConfigAppliedAndSaved(crate::views::multiplex::MuxConfig),
            /// MUX-009: the Board view's modifier-free 'm'/'M' keybinding opens
            /// the same MuxConfigDialog `/mux` opens (MUX-004) — the Board pane
            /// inside the mux grid reuses the binding (R3). Emitted by
            /// `BoardView::handle_event`; App::dispatch routes it to
            /// `handle_open_mux_config_dialog` (idempotent, one instance).
            OpenMuxConfigDialog,
            /// BOARD-023: the Board view's modifier-free 'u'/'U' keybinding
            /// (case-insensitive) opens the actions popup dialog
            /// (Priority::Foreground, Yellow accent, stable id
            /// 'board-actions-dialog'). Serves the single Board view AND the
            /// focused Board pane in mux mode (the binding lives in
            /// BoardView::handle_event, which mux routes through). App::dispatch
            /// routes it to `handle_open_board_keybinding_dialog` (idempotent,
            /// one instance).
            OpenBoardKeybindingDialog,
            /// BOARD-023: the actions dialog's '? Help' row — App::dispatch
            /// routes this to the SAME push helper the stage-4 '?' shortcut
            /// uses (HelpDialog::for_board), so both entry points share one
            /// code path (R6, DRY).
            OpenBoardHelp,
            /// BOARD-023: the actions dialog's 'Esc Exit' row — App::dispatch
            /// routes this to the SAME push helper the stage-4 Esc shortcut
            /// uses (BoardExitConfirmationDialog). This is the ONLY path from
            /// the actions dialog to the exit confirmation — the dialog's own
            /// Esc key always closes the dialog instead (R8).
            OpenBoardExitConfirmation,

            // MENU-002: the board's 2-zone menu bar + column⇄menu⇄chip ring.
            /// Left/Right (h/l) + wheel over the bar: walk the ring by `delta`
            /// (+1 forward, -1 backward) — owned by `BoardStore`
            /// (`menu_focus_next/prev`); the store's ring math decides the
            /// column⇄item⇄chip wrap.
            MenuMove(i32),
            /// Up/Down from the bar: drop focus back into the focused column
            /// (R3) — the bar highlight clears, the column keeps its selection.
            MenuFocusToColumns,
            /// Enter / click on the category item at `idx`: open its dropdown
            /// with the cursor on row 0 (a second open-tick on the ALREADY-open
            /// item closes it instead — GUI parity, R4/R7).
            MenuOpenDropdown(usize),
            /// Close the open dropdown; the focused item (if any) stays
            /// highlighted (R9). No-op when nothing is open.
            MenuCloseDropdown,
            /// BUG-196 R3: a left click OUTSIDE the open dropdown — 'close +
            /// de-select' as one intent: the dropdown closes AND the bar's ring
            /// focus clears (the parent item is de-selected), so the surface's
            /// key bindings are live again immediately. The click STILL lands
            /// on the surface it hit (the outside-click caller decides).
            /// `MenuCloseDropdown` keeps its 'item stays focused' semantics for
            /// the keyboard path (R4: Esc is a 'stay on the bar' gesture; the
            /// click-away is a 'leave the bar' gesture).
            MenuDismissBar,
            /// Up/Down / wheel over an OPEN dropdown: move its cursor by
            /// `delta`, wrapping at both ends (R6).
            MenuDropdownCursor(i32),
            /// Enter with the category's dropdown open: execute the
            /// `row`-th entry (resolved through the MenuCategories registry)
            /// and close the dropdown (R4).
            MenuExecuteItem {
                category: usize,
                row: usize,
            },
            /// Enter / click on a session chip: jump to that session's Agent
            /// view (R5) — dispatch resolves the chip's `SessionId`.
            MenuChipActivate(usize),
            /// Mouse click on a menu item: focus it (clearing any column
            /// bar-highlight) — paired with `MenuOpenDropdown` by the caller.
            MenuMoveToItem(usize),
            // MENU-004: the mux bar's Zone B pane view label.
            /// Enter / click on a Zone B pane view label in the MUX bar: focus
            /// that pane (`set_focus`). New variant (MENU-004) — chips and
            /// panes need different resolution (chips activate a session,
            /// panes move the mux focus).
            MenuFocusPane(usize),
            /// MENU-009: Enter / left click on a Zone C button (index into the
            /// surface's `zone_c` slice — board: `[New Agent]`, agent:
            /// `[New Agent, Close Agent [esc]]`, mux: none). The `App`
            /// dispatch resolves it per surface: the board's `New Agent`
            /// reuses the `.`-key target (selected unit's session, R8
            /// substitution — BUG-203 supersedes it with the payload-free
            /// CreateSessionDialog mapping); the agent's buttons (BUG-199) —
            /// `New Agent` mounts the CreateSessionDialog (start a new agent)
            /// and `Close Agent` ALWAYS mounts the 'Exit Session?'
            /// ExitConfirmationDialog (BUG-204, superseding BUG-199 R2 — the
            /// AgentEscPressed cascade is the physical Esc key's job).
            /// A single token keeps the dispatch arm readable and avoids
            /// overloading the Zone A item index (the design doc's Q-free
            /// choice).
            MenuZoneCActivate(usize),
        }
    };
}
