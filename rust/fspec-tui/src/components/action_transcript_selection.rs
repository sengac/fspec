macro_rules! compose_action_transcript_selection {
    ($($collected:tt)*) => {
        compose_action_mux_menu! {
            $($collected)*
            /// RPC-381 SELECT mode (Tab toggle + ↑/↓ turn nav).
            ToggleTurnSelectMode,
            TurnNavUp,
            TurnNavDown,
            /// RPC-382: open/close the turn content modal (Enter on the selected
            /// turn opens it; Esc closes it). Parameterless — the App reducer
            /// resolves the turn `seq` from the focused scrollback's selection.
            OpenTurnModal,
            CloseTurnModal,
            /// RPC-383: scroll the OPEN turn content modal's body (offset clamped;
            /// selection untouched). Up/Down 1 row, Page* a page, Home/End ends.
            TurnModalScrollUp,
            TurnModalScrollDown,
            TurnModalPageUp,
            TurnModalPageDown,
            TurnModalHome,
            TurnModalEnd,
            /// TUI-103: jump the turn content modal to a specific scroll offset
            /// via scrollbar click-and-drag.
            TurnModalJumpToOffset(usize),

            /// COPY-006: emitted by `views/agent/mouse_dispatch.rs` when a
            /// left-button press lands inside the scrollback rect (drag start /
            /// long-press anchor). App::dispatch begins a live selection on the
            /// focused SessionContext's ScrollbackList at the supplied cell.
            /// Mouse capture stays ON — selection is drawn by the TUI itself.
            SelectionBegin(crate::mouse::selection::Cell),
            /// COPY-010: emitted by a long-press (BeginLine) to select the WHOLE
            /// line under the press. App::dispatch routes it to
            /// `ScrollbackList::selection_begin_line`.
            SelectionBeginLine(crate::mouse::selection::Cell),
            /// COPY-006: emitted as the pointer drags (button-held move) inside
            /// the scrollback rect. App::dispatch extends the live selection's
            /// cursor to the supplied cell and refreshes the highlight spans.
            SelectionExtend(crate::mouse::selection::Cell),
            /// COPY-006: emitted on left-button release (or long-press fire)
            /// when a live selection exists. App::dispatch reconstructs the
            /// selected text and copies it to the terminal clipboard via OSC52.
            /// Per rule [2] the selection is NOT cleared on commit — it stays
            /// highlighted so the user sees what was copied.
            SelectionCommit,
            /// COPY-006: clears any live selection + highlight. Emitted on Esc
            /// (first cascade level) and on any scrollback scroll (rule [7]).
            SelectionClear,
            /// COPY-007: emitted by the AgentView composer on a Commit gesture
            /// carrying the prompt-free selected text. App::dispatch copies it
            /// via OSC 52 (`self.clipboard.copy(text)`), reusing the COPY-006 writer.
            CopyToClipboard(String),

        }
    };
}
