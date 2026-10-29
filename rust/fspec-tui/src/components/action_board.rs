macro_rules! compose_action_board {
    ($($collected:tt)*) => {
        compose_action_agent_navigation! {
            $($collected)*
            /// RPC-012: BoardView emits this on Enter on a selected work unit.
            /// App::dispatch sets AgentViewStore.current_work_unit_id + status
            /// and switches the Navigator's active_view to Agent.
            EnterWorkUnit(String),
            /// RPC-012: BoardView emits this on Shift+Right. `Some(sid)` = navigate
            /// to the attached session; `None` = create-session dialog.
            OpenAgentView(Option<codelet_rpc_types::SessionId>),
            /// RPC-012: AgentView emits this on ESC — Navigator flips back to Board.
            BackToBoard,
            /// RPC-012: programmatic navigation-target set/clear (used by
            /// future slices to script navigation without a keystroke).
            NavigationTargetSet(Option<codelet_rpc_types::SessionId>),
            /// RPC-012: explicit session-attachment action. App::dispatch
            /// emits this on Action::SessionCreated when
            /// AgentViewStore.current_work_unit_id is Some(_).
            AttachSession(String, codelet_rpc_types::SessionId),
            /// RPC-012: BoardView navigation — focus previous column.
            FocusPrevColumn,
            /// RPC-012: BoardView navigation — focus next column.
            FocusNextColumn,
            /// RPC-012: BoardView navigation — move selection down in focused
            /// column.
            SelectNext,
            /// RPC-012: BoardView navigation — move selection up in focused
            /// column.
            SelectPrev,
            /// RPC-012: placeholder — priority reorder up. Backend persistence
            /// is out of scope for this slice (no-op in App::dispatch).
            ReorderUp,
            /// RPC-012: placeholder — priority reorder down.
            ReorderDown,
            /// RPC-015: bootstrap's `backend.checkpoint_counts()` has returned;
            /// the BoardStore's `checkpoint_counts` field is updated so the
            /// BoardView header paints the live counts on the next render.
            CheckpointCountsLoaded(codelet_rpc_types::CheckpointCounts),
            /// BUG-182: fresh `GitState` frame — folded onto the EXISTING
            /// single-writer paths + visible lazy-view re-fetch (R3/R4, body in
            /// `app/dispatch_git_state.rs`).
            GitStateChanged(codelet_rpc_types::GitState),
            /// BUG-182: git-state changed-files REFRESH (unlike `ChangedFilesLoaded`,
            /// which resets): re-apply keeping the selection by path (R6).
            GitChangedFilesLoaded(Vec<codelet_rpc_types::ChangedFile>),
            /// BUG-182: git-state checkpoint REFRESH (unlike `CheckpointsLoaded`,
            /// which resets): re-apply keeping the selection by work-unit + name (R6).
            GitCheckpointsLoaded(Vec<codelet_rpc_types::CheckpointInfo>),
            /// RPC-016: PageUp pressed while BoardView is focused. The payload
            /// is the most recent viewport_height observed by BoardView so
            /// App::dispatch can scroll the focused column by exactly that
            /// many rows (matching the TS UnifiedBoardLayout behaviour).
            ScrollFocusedColumnUp(usize),
            /// RPC-016: PageDown pressed while BoardView is focused. Same
            /// payload semantics as `ScrollFocusedColumnUp`.
            ScrollFocusedColumnDown(usize),
            /// RPC-016: Home pressed while BoardView is focused. App::dispatch
            /// resets the focused column's selected_index + scroll_offset to 0.
            SelectFirstInFocused,
            /// RPC-016: End pressed while BoardView is focused. App::dispatch
            /// sets the focused column's selected_index to units.len()-1 and
            /// adjusts the scroll_offset so the last unit stays visible.
            SelectLastInFocused,
            /// RPC-023: BoardView emits this on a left-click hit-test against
            /// a column header (or content row). `App::dispatch` maps the index
            /// through `COLUMN_ORDER` and calls `BoardStore::set_focused_column`.
            SetFocusedColumn(usize),
            /// RPC-023: BoardView emits this on a left-click hit-test against
            /// a content row. The payload is the WORK-UNIT index in the focused
            /// column (visible row + `scroll_offset`). App::dispatch routes
            /// through `BoardStore::select_index_in_focused` so the click both
            /// updates the selection AND adjusts the viewport when crossing the
            /// scroll boundary.
            SelectIndexInFocused(usize),
            /// RPC-023: scaffolding for the TUI-078 native-text-selection
            /// toggle (RPC-019). Emitted by `MouseTrackingToggle`'s tokio timer
            /// 5 seconds after the most recent `temporarily_disable` call. The
            /// payload is the toggle's `owner_id` so multiple toggle instances
            /// can coexist (a future App::dispatch routing layer will look up
            /// the owner and call `re_enable()`). For RPC-023 this variant
            /// exists but `App::dispatch` does not yet route it — the BoardView
            /// slice intentionally does not opt into TUI-078 button-press.
            ReEnableMouseTracking(String),
        }
    };
}
