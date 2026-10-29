macro_rules! compose_action_slash_operations {
    ($($collected:tt)*) => {
        compose_action_session_controls! {
            $($collected)*
            // ========================================================================
            // RPC-056: Blocklist view actions. The `/blocklist` slash command
            // dispatches `OpenBlocklistView`; the App spawns
            // `backend.blocklist_list()` whose result lands as
            // `BlocklistRulesLoaded`. The view emits `ToggleBlocklistRule` on
            // Space/Enter and the App folds the toggle into
            // `AgentViewStore.blocklist_disabled_by_session`. Esc dismisses the
            // view via `CloseBlocklistView`.
            // ========================================================================
            /// RPC-056: emitted by the slash command palette to open the
            /// BlocklistView. App::dispatch flips the Navigator's ViewMode to
            /// Blocklist AND spawns `backend.blocklist_list()`.
            OpenBlocklistView,
            /// RPC-056: emitted by the view on Esc. App::dispatch returns the
            /// Navigator to the Agent view.
            CloseBlocklistView,
            /// RPC-056: a spawned `backend.blocklist_list()` task resolved.
            /// App::dispatch folds the result into the open BlocklistView.
            BlocklistRulesLoaded(Vec<codelet_rpc_types::BlocklistRuleInfo>),
            /// RPC-056: emitted by the view on Space/Enter over a rule. The
            /// App folds the toggle into the focused session's entry in
            /// `AgentViewStore.blocklist_disabled_by_session`.
            ToggleBlocklistRule(String),

            // ========================================================================
            // RPC-057: /merge-worktree flow actions. The slash command spawns
            // `backend.inspect_session_changes(session_id)` and lands on
            // `InspectChangesLoaded`; non-empty results push a
            // `MergeConfirmDialog` onto the compositor. Button activation maps
            // to `MergeConfirmed` / `DiscardConfirmed` / `CancelMergeDialog`.
            // ========================================================================
            /// RPC-057: emitted after a successful
            /// `backend.inspect_session_changes()` round-trip. The App pushes
            /// a `MergeConfirmDialog` seeded with the summary onto the
            /// compositor (or emits a [merge] nothing-to-merge notice when
            /// every count is zero).
            OpenMergeConfirmDialog {
                session_id: codelet_rpc_types::SessionId,
                summary: codelet_rpc_types::SessionChangesSummary,
            },
            /// RPC-057: emitted by the slash command path when
            /// `inspect_session_changes` returns zero changes — kept distinct
            /// from `OpenMergeConfirmDialog` so the App can emit the
            /// nothing-to-merge notice without flashing the dialog.
            InspectChangesLoaded {
                session_id: codelet_rpc_types::SessionId,
                summary: codelet_rpc_types::SessionChangesSummary,
            },
            /// RPC-057: user activated the Merge button. App::dispatch pops
            /// the dialog and spawns
            /// `backend.merge_session_worktree(session_id, FastForward)`.
            MergeConfirmed {
                session_id: codelet_rpc_types::SessionId,
            },
            /// RPC-057: user activated the Discard button. App::dispatch pops
            /// the dialog and spawns
            /// `backend.discard_session_worktree(session_id)`.
            DiscardConfirmed {
                session_id: codelet_rpc_types::SessionId,
            },
            /// RPC-057: user pressed Cancel/Esc. App::dispatch pops the
            /// dialog and fires no backend call.
            CancelMergeDialog,
            /// WT-009: emitted by `route_merge_outcome` AFTER the `[merge]
            /// success` notice for a `MergeStatus::Success` outcome. App::dispatch
            /// runs the CloseSession-equivalent teardown for the merged session
            /// (board detach, open_sessions removal, current-work-unit clear,
            /// spawned `backend.destroy_session`) and returns to the board —
            /// mirroring the TS merge → summary → destroySession → onExit flow.
            /// Distinct from `MergeConfirmed` so the success notice lands in the
            /// session's scrollback BEFORE the session context is removed.
            MergeSuccess {
                session_id: codelet_rpc_types::SessionId,
            },
            // ========================================================================
            // WT-005: /worktrees session-worktree listing + prune actions.
            // `/worktrees` dispatches `handle_open_session_worktrees_dialog`
            // which spawns `backend.list_session_worktrees()`; the response
            // lands as `SessionWorktreesLoaded` and pushes the
            // SessionWorktreesDialog onto the Compositor. The dialog itself
            // emits `PruneLeakedWorktrees` (Enter on Prune) or
            // `CancelWorktreesDialog` (Enter on Cancel / Esc). App::dispatch
            // routes Prune into `backend.prune_orphaned_worktrees()` and the
            // outcome into a `[worktrees]` / `[error]` scrollback notice.
            // ========================================================================
            /// WT-005: result of a `backend.list_session_worktrees()` snapshot.
            /// `App::dispatch` pushes the SessionWorktreesDialog (with the
            /// dialog's action_tx wired) onto the Compositor at
            /// Priority::Foreground, idempotently replacing any open layer.
            SessionWorktreesLoaded(Vec<codelet_rpc_types::SessionWorktreeInfo>),
            /// WT-005: user activated the Prune button in the
            /// SessionWorktreesDialog. App::dispatch spawns
            /// `backend.prune_orphaned_worktrees()` and emits
            /// `[worktrees] pruned N leaked worktree(s)` /
            /// `[worktrees] no leaked worktrees to prune` /
            /// `[error] /worktrees prune: {e}` against the focused session.
            PruneLeakedWorktrees,
            /// WT-005: user activated Cancel or pressed Esc in the
            /// SessionWorktreesDialog. App::dispatch pops the dialog with no
            /// backend call.
            CancelWorktreesDialog,
            /// RPC-058: user submitted a `/schedule …` slash command. The App
            /// catch-all dispatcher routes this to
            /// `dispatch_slash_schedule::handle_schedule_subcommand` which fans out to
            /// the matching backend round-trip + notice formatter.
            ScheduleSubcommandParsed(crate::app::schedule_parser::ScheduleSubcommand),
            /// RPC-059: a `/loop …` submit-line input has been parsed into a
            /// [`crate::app::loop_parser::LoopSubcommand`] enum value. The
            /// catch-all dispatcher routes this to
            /// `dispatch_slash_loop::handle_loop_subcommand` which fans out to the
            /// matching backend round-trip + notice formatter.
            LoopSubcommandParsed(crate::app::loop_parser::LoopSubcommand),

        }
    };
}
