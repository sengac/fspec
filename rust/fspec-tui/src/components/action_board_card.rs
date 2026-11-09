macro_rules! compose_action_board_card {
    ($($collected:tt)*) => {
        compose_action_finish! {
            $($collected)*
            // BOARD-024: board card Enter chooser and complete details.
            /// Board or mux-board Enter on a selected card: open the modal
            /// chooser for the captured card id and origin.
            OpenBoardCardChooser {
                work_unit_id: String,
                origin: crate::components::board_card::BoardCardOrigin,
            },
            /// The card dialog asks the App to read canonical details for a
            /// card; `request_generation` identifies the request so stale
            /// completions are rejected.
            RequestBoardCardDetails {
                request_generation: u64,
                work_unit_id: String,
            },
            /// Canonical details completion: the `{id, workspace, workUnit}`
            /// JSON envelope or a contextual error message.
            BoardCardDetailsLoaded {
                request_generation: u64,
                work_unit_id: String,
                result: Result<String, String>,
            },
            /// A details attachment action failed (path rejected, missing,
            /// directory, viewer unavailable or browser launch failure).
            BoardCardAttachmentFailed { path: String, message: String },

            /// RPC-445: the daemon's workspace list arrived (start first).
            WorkspacesLoaded(Vec<codelet_rpc_types::WorkspaceEntry>),

            /// RPC-445: open / close the board header's workspace dropdown.
            OpenWorkspaceDropdown,

            CloseWorkspaceDropdown,

            /// RPC-445: move the dropdown highlight by the given rows.
            MoveWorkspaceHighlight(isize),

            /// RPC-445: choose dropdown row `Some(i)`, or the highlight.
            ChooseWorkspace(Option<usize>),

            /// RPC-445: work units of a non-start workspace (fetched or pushed).
            WorkspaceWorkUnitsLoaded(codelet_rpc_types::WorkspaceWorkUnits),

            /// RPC-448: open / close / navigate the board's daemon switcher.
            OpenDaemonSwitcher,

            CloseDaemonSwitcher,

            MoveDaemonHighlight(isize),

            /// RPC-448: switch to switcher row `Some(i)`, or the highlight.
            ChooseDaemon(Option<usize>),

            /// RPC-448: a backend connected for the chosen daemon.
            DaemonSwitched(crate::daemons::SwitchedBackend),
            /// TUI-114: open the create dialog in New Bot mode.
            OpenNewBotDialog,
            /// TUI-114: the New Bot dialog was confirmed; create an
            /// orchestrator bot session (isolated or not).
            CreateBotSubmitted {
                isolated: bool,
            },
            /// TUI-114: a session was created with the bot role, so the
            /// agent bar marks its chip as a bot.
            BotSessionCreated(codelet_rpc_types::SessionId),
            /// RIG-016: Alt+Enter while the turn runs — queue this input
            /// to start when the running turn completes. BUG-552: carries
            /// the session whose composer produced it.
            InputQueued(codelet_rpc_types::SessionId, String),
            /// TUI-116: Ctrl+C / Ctrl+D in the stdin answer box — send
            /// SIGINT to the command only, or end its input. App::dispatch
            /// calls `backend.control_exec_stdin` and clears the exec-stdin
            /// slot on success.
            ExecStdinControl {
                agent_session: codelet_rpc_types::SessionId,
                exec_session: String,
                control: codelet_rpc_types::ExecStdinControl,
            },
            /// TUI-117: `backend.write_exec_stdin` failed. App::dispatch puts
            /// `text` back into the session's answer box with a red error
            /// row (or into its history when the request already closed).
            ExecStdinWriteFailed {
                agent_session: codelet_rpc_types::SessionId,
                exec_session: String,
                text: String,
                error: String,
            },
            /// TUI-117: `backend.control_exec_stdin` failed; the answer box
            /// shows a red error row.
            ExecStdinControlFailed {
                agent_session: codelet_rpc_types::SessionId,
                exec_session: String,
                control: codelet_rpc_types::ExecStdinControl,
                error: String,
            },
            /// TUI-117: Ctrl+X on the notice — clear the slot and mute
            /// further requests from this command run (exec session).
            ExecStdinMuted {
                agent_session: codelet_rpc_types::SessionId,
                exec_session: String,
            },
            /// BUG-364: a focus or Paused `get_exec_stdin_request` probe
            /// failed. When `generation` is still current, an open notice
            /// is marked as possibly stale until the next command update.
            ExecStdinProbeFailed {
                agent_session: codelet_rpc_types::SessionId,
                generation: u64,
                error: String,
            },
            /// BUG-394: the Paused probe's `read` (`get_pause_state` or
            /// `get_hitl_request`) failed; App::dispatch logs it and shows
            /// a red notice in the session scrollback.
            PausedReadFailed {
                session_id: codelet_rpc_types::SessionId,
                read: String,
                error: String,
            },
            /// BUG-395: a pause answer, tool-approval choice, pause resume
            /// or question answer (`write`) failed; App::dispatch logs it,
            /// shows a red notice and reads the prompt back.
            PausedWriteFailed {
                session_id: codelet_rpc_types::SessionId,
                write: String,
                error: String,
            },
            /// RPC-454: a workspace-keyed git-state push; applied only while
            /// that workspace is selected.
            WorkspaceGitStateChanged(codelet_rpc_types::WorkspaceGitState),
            /// TUI-122: `a` in the open workspace dropdown — open the
            /// add-workspace dialog.
            OpenAddWorkspaceDialog,
            /// TUI-122: the add-workspace dialog submitted this path.
            AddWorkspace(String),
            /// TUI-122: the daemon refused `path`; the dialog reopens with it.
            WorkspaceAddFailed { path: String, error: String },
            /// TUI-122: the daemon registered this workspace.
            WorkspaceAdded(codelet_rpc_types::WorkspaceEntry),
            /// TUI-122: `d` in the open workspace dropdown — remove the
            /// highlighted workspace.
            RemoveWorkspace,
            /// TUI-122: the daemon removed the workspace at this path.
            WorkspaceRemoved(String),
            /// TUI-121: `a` in the open daemon switcher — the add form.
            OpenAddDaemonForm,
            /// TUI-121: `e` in the open daemon switcher — edit the highlighted
            /// saved daemon.
            OpenEditDaemonForm,
            /// TUI-121: the daemon form submitted `form`, replacing the saved
            /// daemon `previous` when editing (the token Debug is redacted).
            SaveDaemon {
                previous: Option<String>,
                form: crate::daemons::DaemonForm,
            },
            /// TUI-121: `d` in the open daemon switcher — remove the
            /// highlighted saved daemon (TUI-123: after a y/n prompt).
            RemoveSavedDaemon,
            /// TUI-123: the removal prompt was answered `y` for this saved
            /// daemon.
            ConfirmRemoveSavedDaemon(crate::daemons::DaemonChoice),
            /// MODEL-011: a spawned `backend.get_thinking_options(...)` task
            /// resolved; App::dispatch opens the ThinkingLevelDialog with the
            /// levels the session's model offers.
            ThinkingOptionsLoaded(
                codelet_rpc_types::SessionId,
                codelet_rpc_types::ThinkingOptions,
            ),
            /// MODEL-011: a thinking level (or saved default) was refused;
            /// App::dispatch shows the reason in the session's scrollback.
            ThinkingLevelRefused(codelet_rpc_types::SessionId, String),
            /// BUG-601: the daemon answered the blocklist rules turned off
            /// for the session (a read or a Toggle Rule); App::dispatch
            /// shows them in the blocklist view.
            BlocklistDisabledRulesLoaded(codelet_rpc_types::SessionId, Vec<String>),
        }
    };
}
