# COV-073 plan: review Ink TUI features against fspec-tui

Read-only prep, 2026-11-06, against HEAD 768f804b1 (other helpers' uncommitted work in the tree included). No repo files were changed. Feature list computed from every `spec/features/*.feature.coverage` `testMappings[].file` that no longer exists and is not a `rust/` path. Per-feature classification by read-only subagents (raw output, inputs, scripts: /tmp/asst/prep/cov07x-work/, chunks COV-073-c*), spot-checked by the coordinator.

## Totals

- Features still mapping a deleted TypeScript-era test file: **160** (1041 scenarios). Card text said 164 (1066 scenarios).
- 160 features (card said 164). The 4 dropped are orphan coverage files whose feature was deleted long ago: animated-shimmer-on-last-changed-work-unit (99fdc3bb3), checkpoint-viewer-three-pane-layout and fix-three-pane-layout-with-proper-flex-properties (50d4d5932), space-esc-immediate-session-detach-in-agentview (273d6afb3). Action (spec-only, add to any spec-only batch): delete these 4 `.feature.coverage` files and their `.lock` files; nothing to deprecate. BUG-522 (in flight, fspec-validate-reports-orphan-coverage-files) will flag them.
- Partly mapped: duplicate-installation-success-message-in-fspec-init 1/2, duplicate-next-steps-message-in-fspec-init-output 1/3, fix-tui-kanban-column-layout-to-match-table-style 10/12, merge-conflict-markers 5/6 (2 live files), per-model-compaction-threshold 8/10, thinking-level-dialog 9/14, tui-board-column-scrolling 2/7.

| class | features |
|---|---|
| remap | 4 |
| write | 8 |
| merge | 103 |
| retire | 37 |
| human | 8 |
| **total** | **160** |

Most `merge` features also need a few new tests: scenarios the owner feature does not yet prove are moved into the owner feature with a test in the owner's file (see each block's "not covered"). Many features of every class also carry single scenarios to delete or @deprecate.

### (human) questions (8 features)

Recommended answer first in each. Features that hinge on the same product question are grouped under one question; each feature's own wording is in its block below.

**H-RESTORE-PROGRESS. Checkpoint restore progress dialog** (checkpoint-restore-progress-dialog)
> Rust checkpoint restore (RPC-365) shows one "Restoring…" phase then "any key: close", with no per-file n/N progress and no 3s auto-close. (A, recommended) Delete those 3 scenarios and merge the rest into checkpoint-restore + rust-error-notification-status-dialog-wrappers, or (B) keep them and open a card to add per-file progress and auto-close?

**H-AUTO-REATTACH. Session auto re-attach on another card's status change** (session-work-unit-ipc)
> Should the session auto re-attach to another card when the agent runs update-work-unit-status on it? Recommended: (a) No, retire (Rust binds cards explicitly and attributes work_unit_id per tool call); (b) Yes, port as new Rust behaviour plus test.

**H-CORRUPT-WORK-UNITS. Corrupted work-units.json aborts TUI startup** (graceful-initialization-without-spec-directory)
> A corrupted spec/work-units.json at startup makes WorkUnitsWatcher::new fail and aborts embedded TUI startup (rust/fspec/src/common.rs:95); should startup degrade to an empty board and keep watching? Recommended: (a) Yes, bug: file a fix card and keep the scenario; (b) No, delete the scenario.

**H-RESUME-DELETE. Resume view: Delete ALL and auto-exit** (delete-session-from-resume-view-with-confirmation-dialog)
> Rust resume view (RPC-026) deletes only the selected session via a 2-button ConfirmDialog, has no Delete ALL, and shows the empty placeholder after the last delete. (a) @deprecated with behaviour merged into rpc026-resume-session-view, deleting the 4 three-button/delete-all/auto-exit scenarios (recommended), or (b) keep and open a card to port Delete-ALL and auto-exit?

**H-DELETED-FILE-DIFF. Changed Files view errors on a deleted file (BUG-069 regression)** (changed-files-view-doesn-t-show-deleted-files-correctly)
> Selecting a deleted file in the Rust Changed Files view gives an error notice and 'No changes to display' (rust/git/src/diff.rs:27 returns FileNotFound; the bug BUG-069 fixed in TS). (a) Open a bug card so Rust shows 'File was deleted' (or a full removal diff) and move the scenario into rust-changed-files-view with a test (recommended), or (b) delete the scenario?

**H-MERGE-CONFLICT. /merge-worktree conflicts are seeded, not sent** (merge-conflict-llm-context)
> On /merge-worktree conflicts the Rust TUI puts the conflict list into the input box but does not send it to the agent, and the text doesn't mention conflict markers or rerunning /merge-worktree. (A, recommended) accept seeding: merge into rpc057-merge-worktree-dispatch and delete the auto-submit/wording scenarios; (B) bug: auto-submit and add the wording, keeping this feature with a new test.

**H-FILE-SEARCH-WORKTREE. @file search in isolated sessions** (file-search-popup-uses-worktree-path-for-isolated-sessions)
> In an isolated session, should @file search list the files in the session's worktree rather than the board workspace? (Recommended: yes. Add the routing plus a GIT-033 test. Otherwise @deprecate citing RPC-455.)

**H-LOADDATA. Board loadData stability (BUG-119)** (boardview-loaddata-stability)
> Rust has a debounced daemon watcher that keeps the last good snapshot when a re-read fails; the TS loadData path is gone. Should BUG-119 be @deprecated as superseded by work-units-watcher and BUG-379? (Recommended: yes, deprecate. Alternative: rewrite it against rust/core/src/work_units.rs with a new test. Also decide whether a failed re-read, today only logged, should be shown.)

## Rules applied

- One test file maps to one feature (BUG-357, b0141eb56); a feature may map several files; implMappings do not count. Live owners come from /tmp/asst/prep/cov07x-work/test-file-owners.tsv (2,023 owned test files at 768f804b1).
- (merge) is used when the proving Rust test sits in a file another feature owns and that feature describes the same behaviour (mostly the `<cmd>-rust-port` / `<cmd>-cli-subcommand` twins, or a later fspec-tui feature). The helper moves this feature's still-valid, unproven scenarios into the owner feature with a new test in the owner's file, drops the duplicates, then deletes or @deprecates this feature (product owner, 2026-11-05: "Duplicates ... merge into the owning feature").
- (remap) is used only when the proving test is in an unowned file (or one this feature already owns), or its test fns can be split into a new file for this feature.
- (retire) cites the commit or card that removed the behaviour; 103a73072 (TypeScript client removed) is cited only for TypeScript tooling itself (Ink/React internals, vitest, eslint/prettier, npm/vite, Node bundling, execa, winston, isomorphic-git), never as proof a ported behaviour was dropped. Action: @deprecated plus the citation (COV-076 rule [0]).
- Every remapped or new test carries `// @step <exact step text>` comments (BUG-509 exact rule); several (remap) entries note "needs @step comments".
- Scenarios that describe behaviour Rust deliberately changed are marked "delete" or "@deprecated" inside the feature's block (product owner, 2026-11-05: a scenario for behaviour that does not exist may be deleted).
- Use `link-coverage --replace` to swap a deleted TS mapping for the Rust one; gate checks need a binary built from HEAD (~/.local/bin/fspec is rebuilt with ./scripts/build-install.sh after any card that changes fspec code).

## Shared files and cross-card dependencies

Deleted TypeScript test files shared by more than one feature (each feature drops its own mapping; no code conflict, but keep the decisions consistent):

- deleted `src/__tests__/blocklist-napi-integration.test.ts`: blocklist-core-command-tool-filtering (COV-074 remap, COV-074-B4); sensitive-path-prompts (COV-073 merge, COV-073-B4)
- deleted `src/components/__tests__/ConfirmationDialog.test.tsx`: confirmation-dialog (COV-073 merge, COV-073-B4); sensitive-path-prompts (COV-073 merge, COV-073-B4)
- deleted `src/components/__tests__/RoleDialog.test.tsx`: role-dialog (COV-073 merge, COV-073-B5); role-dialog-remove-button (COV-073 retire, COV-073-B5)
- deleted `src/components/__tests__/agent-selector-duplicate-message.test.tsx`: duplicate-installation-success-message-in-fspec-init (COV-073 merge, COV-073-B7); duplicate-next-steps-message-in-fspec-init-output (COV-073 merge, COV-073-B7)
- deleted `src/tui/__tests__/AgentModal.test.tsx`: add-debug-slash-command-to-fspec-tui-agent (COV-073 merge, COV-073-B2); manual-compaction-command (COV-073 merge, COV-073-B2); tui-integration-for-codelet-ai-agent (COV-073 merge, COV-073-B2)
- deleted `src/tui/__tests__/TUI-056-anchor-viewer.test.tsx`: fix-anchor-viewer-integration-with-thinking-dialog-system (COV-073 retire, COV-073-B9); interactive-anchor-point-viewer-with-conversation-navigation (COV-073 retire, COV-073-B9)
- deleted `src/tui/__tests__/context-window-fill-percentage.test.tsx`: context-fill-percentage-realtime-recompute-ui (COV-073 merge, COV-073-B2); context-window-fill-percentage-indicator (COV-073 merge, COV-073-B2)
- deleted `src/tui/components/__tests__/ModelSelectorScreen.integration.test.tsx`: model-screen-stale-profile-sections-from-non-openai-providers-footer-text-unreachable-section-filtering (COV-073 merge, COV-073-B4); model-selector-screen (COV-073 merge, COV-073-B4)
- deleted `src/tui/components/__tests__/SessionHeader.test.tsx`: session-header-isolation-badge (COV-073 merge, COV-073-B5); session-header-realtime-status (COV-073 merge, COV-073-B5)
- deleted `src/tui/components/__tests__/isolated-session-integration.test.tsx`: boardview-createsessiondialog-callback-ignores-isolated-parameter (COV-073 merge, COV-073-B3); createsessiondialog-shows-wrong-text-when-pressing-enter-on-work-unit (COV-073 merge, COV-073-B3); enter-key-on-board-bypasses-create-session-dialog-no-isolated-option (COV-073 merge, COV-073-B3); sessionheader-missing-isisolated-prop-isolated-badge-never-shows (COV-073 merge, COV-073-B3)
- deleted `src/tui/services/__tests__/globalSessionStreamManager.test.ts`: global-session-stream-manager (COV-073 merge, COV-073-B2); isolation-state-stream-sync (COV-073 merge, COV-073-B2)
- deleted `src/tui/services/__tests__/sessionService.test.ts`: fix-tui-068-code-smells-error-handling-console-statements-facade-completion (COV-073 retire, COV-073-B3); refactor-session-work-unit-state-management (COV-073 merge, COV-073-B3)


Rust test files that features in more than one card will edit (all are owned by a `-rust-port`/`-cli-subcommand` twin or a later Rust feature; the features merge into that owner). Run the batches that touch the same file one after another, never in parallel:

- `rust/fspec/tests/cli_init.rs`: double-prompt-and-missing-success-message-in-interactive-init-mode (COV-072-B5); duplicate-installation-success-message-in-fspec-init (COV-073-B7); duplicate-next-steps-message-in-fspec-init-output (COV-073-B7); init-008-and-bug-030-implementation-not-integrated-into-actual-codebase (COV-072-B5)
- `rust/git/tests/changed_files_rpc355.rs`: changed-files-view-doesn-t-show-deleted-files-correctly (COV-073-B7); changed-files-view-missing-unstaged-files (COV-074-B4)

- Found while classifying: rust/fspec-tui/src/store/agent_view/patch_diff.rs:78 still shows "(select turn to /expand)" though /expand was removed (TUI-045); prov114 test fixture labels the Copilot row "Login with GitHub Copilot (device flow)" but projection.rs:56 says "Sign in with device code"; rust/tools/src/blocklist/middleware.rs inline tests carry BLOCK-005 @step comments while BLOCK-007 owns the file (fix when sensitive-path-prompts merges).
- per-model-compaction-threshold remaps to the unowned rust/cli/src/compaction_threshold/configuration_tests.rs; the COV-075 plan floated the same file for CTX-002. Only one feature may map it: settle before COV-073-B1 starts.

## Batches

Batches are connected components over target test files (and shared deleted TS files) inside this card, packed by area, 10-20 features each. No target test file appears in two batches of this card. "Spec-only" batches touch no Rust test file (retire, fully-covered merges, and human features whose fix is spec-only); they can run in parallel with anything.

### COV-073-B1: sessions/core/fspec-tui: HITL, poller, compaction threshold, watcher, startup, markdown, scrollback, restore (18 features; human 3, merge 12, remap 2, write 1)

Blocked in part on the (human) answers for: boardview-loaddata-stability, graceful-initialization-without-spec-directory, checkpoint-restore-progress-dialog.

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| hitl-handler-wiring | write | rust/agent-loop/tests/bug118_hitl_handler_wiring.rs | NAPI scenarios: REFAC-027 (f25b90b2e); TUI dups: inline-hitl-prompt, pause-and-hitl-dialogs, request-user-input-hitl-tool |
| session-footer | merge | rust/agent-loop/tests/wt002_footer_poller_parity/poller_reads_branch_only.rs | rpc018-agent-chrome + footer-cwd-registry + fspec-binary-footer-poller |
| per-model-compaction-threshold | remap | rust/cli/src/compaction_threshold/configuration_tests.rs, rust/agent-loop/tests/ctx007_stream_loop_threshold.rs | - |
| boardview-loaddata-stability | human | rust/core/tests/bug119_work_units_watcher_stability.rs | - |
| graceful-initialization-without-spec-directory | human | rust/core/tests/init016_graceful_missing_spec.rs | - |
| native-fspec-tool-integration-via-napi-rs | merge | rust/fspec-core/tests/dispatcher_test.rs | fspec-tool-rust-dispatcher |
| markdown-table-rendering-in-ai-output | merge | rust/fspec-tui/src/store/agent_view/markdown_tables_tests.rs | markdown-table-box-drawing-rendering-in-rust-chat-view |
| virtuallist-scroll-only-mode-for-agentmodal | remap | rust/fspec-tui/src/views/agent/scrollback_tests.rs | - |
| checkpoint-restore-progress-dialog | human | rust/fspec-tui/src/views/checkpoints/restore_tests.rs, rust/fspec-tui/tests/dialogs_rpc079.rs | checkpoint-restore,rust-error-notification-status-dialog-wrappers |
| container-focus-indication-with-headings | merge | rust/fspec-tui/src/views/checkpoints/tests.rs, rust/fspec-tui/src/views/changed_files/tests.rs | rust-checkpoints-view + rust-changed-files-view |
| model-selector-state-hook | merge | rust/fspec-tui/src/views/model_selector/tests_core.rs | model-selector-view |
| sessionheader-badge-threshold | merge | rust/fspec-tui/tests/tui001_session_header_model_info.rs | session-header-model-info |
| tui-badge-and-fill-display-end-to-end-verification | merge | rust/fspec-tui/tests/tui001_session_header_model_info.rs, rust/cli/src/compaction_threshold/catalog_tests.rs | session-header-model-info (+compaction-threshold-clamped-inputs) |
| persist-last-used-model-selection | merge | rust/sessions/src/startup_model_resolution_tests.rs, rust/sessions/tests/prov120_startup_model_persistence.rs, rust/fspec-tui/tests/bug461_user_config_read_failures_are_reported.rs, rust/sessions/tests/prov122_model_selection_persistence.rs | startup-model-initialization-first-available,persisted-model-store-reconciliation,user-config-read-failures-are-reported,model-selection-persists-last-used-model |
| profile-based-model-selection-not-restored-on-session-startup | merge | rust/sessions/src/startup_model_resolution_tests.rs | startup-model-initialization-first-available |
| model-selection-service | merge | rust/sessions/tests/prov122_model_selection_persistence.rs | model-selection-persists-last-used-model |
| rust-authoritative-context-window | merge | rust/sessions/tests/prov166_wire_model_limits.rs, rust/fspec-tui/tests/tui001_session_header_model_info.rs, rust/agent-loop/tests/cov057_sub_agent_model.rs | session-model-limits-wire (+session-header-model-info, sub-agent-model-inheritance); 4 badge scenarios superseded by CTX-009 |
| agent-modal-model-selection | merge | rust/sessions/tests/resume_restores_manifest_model.rs, rust/fspec-tui/src/views/model_selector/tests_core.rs, rust/fspec-tui/src/views/model_selector/rows_tests.rs, rust/fspec-tui/src/views/model_selector/rows_tests_header_count.rs, rust/providers/tests/model_registry_selection_test.rs, rust/providers/tests/model_selection_test.rs, rust/sessions/src/startup_model_resolution_tests.rs, rust/sessions/src/profile_sections/cloud_section_tests.rs, rust/fspec-tui/tests/tui001_session_header_model_info.rs | model-selector-view,model-selector-rows,model-selector-header-count-label,model-registry-selection-and-filtering,dynamic-model-selection-via-models-dev,startup-model-initialization-first-available,model-selector-cloud-section-assembly,session-header-model-info |

Test files touched: `rust/agent-loop/tests/bug118_hitl_handler_wiring.rs`, `rust/agent-loop/tests/cov057_sub_agent_model.rs`, `rust/agent-loop/tests/ctx007_stream_loop_threshold.rs`, `rust/agent-loop/tests/wt002_footer_poller_parity/poller_reads_branch_only.rs`, `rust/cli/src/compaction_threshold/catalog_tests.rs`, `rust/cli/src/compaction_threshold/configuration_tests.rs`, `rust/core/tests/bug119_work_units_watcher_stability.rs`, `rust/core/tests/init016_graceful_missing_spec.rs`, `rust/fspec-core/tests/dispatcher_test.rs`, `rust/fspec-tui/src/store/agent_view/markdown_tables_tests.rs`, `rust/fspec-tui/src/views/agent/scrollback_tests.rs`, `rust/fspec-tui/src/views/changed_files/tests.rs`, `rust/fspec-tui/src/views/checkpoints/restore_tests.rs`, `rust/fspec-tui/src/views/checkpoints/tests.rs`, `rust/fspec-tui/src/views/model_selector/rows_tests.rs`, `rust/fspec-tui/src/views/model_selector/rows_tests_header_count.rs`, `rust/fspec-tui/src/views/model_selector/tests_core.rs`, `rust/fspec-tui/tests/bug461_user_config_read_failures_are_reported.rs`, `rust/fspec-tui/tests/dialogs_rpc079.rs`, `rust/fspec-tui/tests/tui001_session_header_model_info.rs`, `rust/providers/tests/model_registry_selection_test.rs`, `rust/providers/tests/model_selection_test.rs`, `rust/sessions/src/profile_sections/cloud_section_tests.rs`, `rust/sessions/src/startup_model_resolution_tests.rs`, `rust/sessions/tests/prov120_startup_model_persistence.rs`, `rust/sessions/tests/prov122_model_selection_persistence.rs`, `rust/sessions/tests/prov166_wire_model_limits.rs`, `rust/sessions/tests/resume_restores_manifest_model.rs`

### COV-073-B2: fspec-tui agent view: chunk dispatch, compaction, tokens, tool output (17 features; merge 17)

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| refactor-streamchunk-to-use-proper-discriminated-union-types | merge | rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs, rust/fspec-tui/tests/chunk_rendering_parity_rpc078.rs, rust/fspec-tui/tests/slash_compact_rpc047.rs | agentview-subscribe-broadcasts,agentview-chunk-rendering-parity,slash-command-compact |
| persistent-handler-state-refresh | merge | rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs, rust/fspec-tui/tests/slash_compact_rpc047.rs | agentview-subscribe-broadcasts (+slash-command-compact) |
| persistent-chunk-handler | merge | rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs | agentview-subscribe-broadcasts |
| global-session-stream-manager-chunk-routing | merge | rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs | agentview-subscribe-broadcasts |
| global-session-stream-manager | merge | rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs | agentview-subscribe-broadcasts |
| isolation-state-stream-sync | merge | rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs | agentview-subscribe-broadcasts |
| real-time-tokens-per-second-display-in-agent-modal-header | merge | rust/fspec-tui/tests/agentview_session_header_per_session_tokens_rpc099.rs | agentview-session-header-per-session-tokens |
| reasoning-token-tui-display | merge | rust/fspec-tui/tests/agentview_session_header_per_session_tokens_rpc099.rs, rust/sessions/tests/cmpct062_token_restore.rs | agentview-session-header-per-session-tokens,committed-token-state-restore |
| compaction-indicator-rust-source-of-truth | merge | rust/fspec-tui/tests/automatic_compaction_progress_bug210.rs, rust/fspec-tui/tests/slash_compact_rpc047.rs | automatic-compaction-progress (+slash-command-compact); retry-dialog scenario deleted |
| tui-integration-for-codelet-ai-agent | merge | rust/fspec-tui/tests/board_period_new_agent_rpc395.rs, rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs, rust/fspec-tui/tests/chunkprocessor_parity_rpc091.rs, rust/fspec-tui/tests/bug226_accepted_model.rs, rust/fspec-tui/tests/agentview_session_header_per_session_tokens_rpc099.rs, rust/fspec-tui/tests/prov101_session_creation_decline.rs | board-key-starts-new-agent,agentview-subscribe-broadcasts,agentview-chunkprocessor-parity,accepted-model-selection,agentview-session-header-per-session-tokens,session-creation-decline-surfaced |
| claude-code-style-tool-output-display | merge | rust/fspec-tui/tests/chunkprocessor_parity_rpc091.rs, rust/fspec-tui/tests/tool_call_output_collapse_rpc389.rs | agentview-chunkprocessor-parity,tool-call-output-collapse |
| manual-compaction-command | merge | rust/fspec-tui/tests/slash_compact_rpc047.rs, rust/sessions/tests/rpc418_compact_session/compact_populated_clears_conversation.rs, rust/fspec-tui/tests/compaction_reduction_display_contract_rpc420.rs | slash-command-compact,rust-tui-compact-real-compaction,compaction-reduction-display-contract |
| add-debug-slash-command-to-fspec-tui-agent | merge | rust/fspec-tui/tests/slash_debug_rpc055.rs, rust/cli/tests/debug_capture_prompt_events.rs | rpc055-slash-debug-dispatch |
| debug-badge-session-awareness | merge | rust/fspec-tui/tests/slash_debug_rpc055.rs, rust/fspec-tui/tests/debug_parity_rpc430.rs | rpc055-slash-debug-dispatch (+fix-debug-command-parity-gaps-in-rust-tui) |
| debug-badge-session-cycle | merge | rust/fspec-tui/tests/slash_debug_rpc055.rs, rust/fspec-tui/tests/debug_parity_rpc430.rs | rpc055-slash-debug-dispatch (+fix-debug-command-parity-gaps-in-rust-tui) |
| context-window-fill-percentage-indicator | merge | rust/fspec-tui/tests/agentview_session_header_compaction_percentage_rpc100.rs | agentview-session-header-compaction-percentage, rpc018-agent-chrome |
| context-fill-percentage-realtime-recompute-ui | merge | - | context-fill-percentage-realtime-recompute (2 scenarios @deprecated BUG-459/CMPCT-067) |

Test files touched: `rust/cli/tests/debug_capture_prompt_events.rs`, `rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs`, `rust/fspec-tui/tests/agentview_session_header_compaction_percentage_rpc100.rs`, `rust/fspec-tui/tests/agentview_session_header_per_session_tokens_rpc099.rs`, `rust/fspec-tui/tests/automatic_compaction_progress_bug210.rs`, `rust/fspec-tui/tests/board_period_new_agent_rpc395.rs`, `rust/fspec-tui/tests/bug226_accepted_model.rs`, `rust/fspec-tui/tests/chunk_rendering_parity_rpc078.rs`, `rust/fspec-tui/tests/chunkprocessor_parity_rpc091.rs`, `rust/fspec-tui/tests/compaction_reduction_display_contract_rpc420.rs`, `rust/fspec-tui/tests/debug_parity_rpc430.rs`, `rust/fspec-tui/tests/prov101_session_creation_decline.rs`, `rust/fspec-tui/tests/slash_compact_rpc047.rs`, `rust/fspec-tui/tests/slash_debug_rpc055.rs`, `rust/fspec-tui/tests/tool_call_output_collapse_rpc389.rs`, `rust/sessions/tests/cmpct062_token_restore.rs`, `rust/sessions/tests/rpc418_compact_session/compact_populated_clears_conversation.rs`

### COV-073-B3: fspec-tui isolated sessions, board navigation, merge-worktree, attachments/foundation viewer (17 features; human 1, merge 12, retire 1, write 3)

Blocked in part on the (human) answers for: merge-conflict-llm-context.

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| blocklist-tui-list-form-views | merge | rust/fspec-tui/tests/blocklist_rpc056/rendering.rs, rust/fspec-tui/tests/blocklist_rpc056/interaction.rs | rpc056-blocklist-view-dispatch |
| boardview-createsessiondialog-callback-ignores-isolated-parameter | merge | rust/fspec-tui/tests/board024/chooser.rs | board-card-enter-chooser-and-complete-details |
| enter-key-on-board-bypasses-create-session-dialog-no-isolated-option | merge | rust/fspec-tui/tests/board024/chooser.rs | board-card-enter-chooser-and-complete-details |
| sessionheader-missing-isisolated-prop-isolated-badge-never-shows | merge | rust/fspec-tui/tests/bug178_isolated_badge_chrome_paint.rs | sessionheader-isolated-badge-chrome-paint-wiring |
| create-session-dialog-options | merge | rust/fspec-tui/tests/isolated_session_dialog_rpc060.rs | rpc060-isolated-session-dialog |
| createsessiondialog-shows-wrong-text-when-pressing-enter-on-work-unit | merge | rust/fspec-tui/tests/isolated_session_dialog_rpc060.rs, rust/fspec-tui/tests/shift_right_create_session_dialog/generic.rs | rpc060-isolated-session-dialog (+agentview-shift-right-create-session-dialog) |
| refactor-session-work-unit-state-management | merge | rust/fspec-tui/tests/board_agent_navigation_rpc012.rs, rust/fspec-tui/tests/work_unit_binding_rpc050.rs, rust/fspec-tui/tests/merge_worktree_rpc057.rs | rpc012-board-agent-navigation,work-unit-attach-binding,rpc057-merge-worktree-dispatch |
| merge-worktree-command | merge | rust/fspec-tui/tests/merge_worktree_rpc057.rs | rpc057-merge-worktree-dispatch, isolation-only-opens-the-create-session-dialog-and-merge-worktree-never-closes-the-session-ux-contract-broken-in-the-rust-tui |
| merge-conflict-llm-context | human | rust/fspec-tui/tests/merge_worktree_rpc057.rs | rpc057-merge-worktree-dispatch (if A) |
| merge-worktree-ux | merge | rust/fspec-tui/tests/merge_worktree_rpc057.rs, rust/fspec-tui/tests/wt009_isolation_toggle_merge_teardown.rs | rpc057-merge-worktree-dispatch (+WT-009 feature); action-prompt scenarios superseded by WT-009 |
| fix-tui-068-code-smells-error-handling-console-statements-facade-completion | retire | - | 103a73072 (TS client removed) + RPC-050 work-unit-attach-binding |
| full-screen-tui-layout | write | rust/fspec-tui/tests/board_full_terminal_board013.rs | fspec-tui-terminal (clear-screen scenario) |
| integrate-attachment-server-with-tui-boardview-lifecycle | merge | rust/fspec-tui/tests/board_open_attachment_rpc374.rs, rust/attachment-viewer/tests/viewer_server.rs | rust-board-open-attachment,rust-attachment-viewer-server |
| attachment-selection-dialog-with-keyboard-navigation | merge | rust/fspec-tui/tests/board_open_attachment_rpc374.rs | rust-board-open-attachment |
| view-foundation-md-in-tui-with-attachment-viewer | merge | rust/fspec-tui/tests/board_open_foundation_rpc373.rs, rust/fspec-tui/tests/tui120_foundation_viewer_and_fspec_runner_follow_the_selected_workspace.rs | rust-board-open-foundation (+foundation-viewer-and-fspec-runner-follow-the-selected-workspace); help-line scenarios superseded by MENU-001/MENU-008 |
| stretch-board-content-to-fill-available-viewport-height | write | rust/fspec-tui/tests/board_vertical_stretch_board014.rs | - |
| tui-hangs-when-foundation-json-is-missing | write | rust/fspec-tui/tests/bug072_board_loads_without_spec_dir.rs | - |

Test files touched: `rust/attachment-viewer/tests/viewer_server.rs`, `rust/fspec-tui/tests/blocklist_rpc056/interaction.rs`, `rust/fspec-tui/tests/blocklist_rpc056/rendering.rs`, `rust/fspec-tui/tests/board024/chooser.rs`, `rust/fspec-tui/tests/board_agent_navigation_rpc012.rs`, `rust/fspec-tui/tests/board_full_terminal_board013.rs`, `rust/fspec-tui/tests/board_open_attachment_rpc374.rs`, `rust/fspec-tui/tests/board_open_foundation_rpc373.rs`, `rust/fspec-tui/tests/board_vertical_stretch_board014.rs`, `rust/fspec-tui/tests/bug072_board_loads_without_spec_dir.rs`, `rust/fspec-tui/tests/bug178_isolated_badge_chrome_paint.rs`, `rust/fspec-tui/tests/isolated_session_dialog_rpc060.rs`, `rust/fspec-tui/tests/merge_worktree_rpc057.rs`, `rust/fspec-tui/tests/shift_right_create_session_dialog/generic.rs`, `rust/fspec-tui/tests/tui120_foundation_viewer_and_fspec_runner_follow_the_selected_workspace.rs`, `rust/fspec-tui/tests/work_unit_binding_rpc050.rs`, `rust/fspec-tui/tests/wt009_isolation_toggle_merge_teardown.rs`

### COV-073-B4: fspec-tui confirm/delete dialogs, diffs, file search, keyboard cascade, model selector, blocklist middleware (16 features; human 1, merge 12, write 3)

Blocked in part on the (human) answers for: file-search-popup-uses-worktree-path-for-isolated-sessions.

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| compaction-sessionstatechange-drops-sessionid-wrong-session-shows-compacting-indicator | write | rust/fspec-tui/tests/compacting_status_routed_by_session_cmpct033.rs | - |
| confirmation-dialog | merge | rust/fspec-tui/tests/confirm_dialog_yn_shortcut_rpc164.rs, rust/fspec-tui/src/views/checkpoints/delete_tests.rs | rpc164-provider-settings-confirm-dialog-yn-shortcut,checkpoint-delete |
| sensitive-path-prompts | merge | rust/tools/src/blocklist/middleware.rs | integrate-blocklist-prompt-action-with-tool-pause-system |
| context-fill-percentage-realtime-recompute-restore | write | rust/fspec-tui/tests/context_fill_restore_seeds_threshold_rpc101.rs | - |
| diff-view-for-write-edit-tool-output | merge | rust/fspec-tui/tests/edit_diff_rendering_rpc391.rs, rust/fspec-tui/tests/diff_format_rpc390.rs | agentview-edit-diff-rendering,agentview-edit-diff-generation |
| file-search-popup-uses-worktree-path-for-isolated-sessions | human | rust/fspec-tui/tests/git033_file_search_isolated_worktree.rs | - |
| hitl-freeform-other-option-for-predefined-choices | merge | rust/fspec-tui/tests/inline_hitl_prompt_rpc411/other_mode.rs | inline-hitl-prompt |
| esc-interrupt-during-compaction | merge | rust/fspec-tui/tests/keyboard_cascade_rpc051.rs, rust/fspec-tui/tests/turn_content_modal_rpc382.rs, rust/fspec-tui/tests/agentview_esc_cascade_input_clear_rpc095.rs | keyboard-shortcut-cascade-parity,agentview-turn-content-modal,agentview-esc-cascade-input-clear |
| select-mode-enter-key-opens-full-turn-modal | merge | rust/fspec-tui/tests/turn_content_modal_rpc382.rs | agentview-turn-content-modal (+agentview-turn-content-modal-fullscreen-scroll, agentview-turn-select-mode, agentview-esc-cascade-input-clear, tool-call-output-collapse) |
| loop-shorthand-natural-language-schedule-creation | merge | rust/fspec-tui/tests/loop_dispatch_rpc059.rs | rpc059-loop-dispatch |
| model-selector-screen | merge | rust/fspec-tui/tests/model_selector_filter_row_model007.rs | model-selector-view (+ model-selector-filter/tab/enter/collapse owners) |
| custom-model-registration-and-facade-override-in-model-selector | merge | rust/sessions/src/profile_sections/persistence_tests.rs, rust/sessions/src/profile_sections/tests.rs, rust/sessions/tests/rpc348_mid_session_facade_reresolution.rs, rust/fspec-tui/src/views/model_selector/tests_crud_add.rs | custom-model-persistence, model-selector-profile-server, mid-session-facade-reresolution, model-selector-custom-model-crud (NAPI scenario @deprecated REFAC-027) |
| model-screen-stale-profile-sections-from-non-openai-providers-footer-text-unreachable-section-filtering | merge | rust/sessions/src/profile_sections/tests.rs | model-selector-profile-server (2 scenarios retired per RPC-338/RPC-337) |
| multi-line-text-input-with-cursor-navigation-for-agentview | write | rust/fspec-tui/tests/multiline_input_cursor_editing_tui039.rs | agent-input-multiline-newline-keys,agent-input-soft-wrap-auto-grow,rpc019-multiline-input (3 duplicate scenarios) |
| input-state-not-restored-when-navigating-from-boardview-to-agentview | merge | rust/fspec-tui/tests/pending_input_durability_rpc052.rs | pending-input-draft-persistence |
| input-not-restored-when-switching-between-sessions | merge | rust/fspec-tui/tests/pending_input_durability_rpc052.rs | pending-input-draft-persistence |

Test files touched: `rust/fspec-tui/src/views/checkpoints/delete_tests.rs`, `rust/fspec-tui/src/views/model_selector/tests_crud_add.rs`, `rust/fspec-tui/tests/agentview_esc_cascade_input_clear_rpc095.rs`, `rust/fspec-tui/tests/compacting_status_routed_by_session_cmpct033.rs`, `rust/fspec-tui/tests/confirm_dialog_yn_shortcut_rpc164.rs`, `rust/fspec-tui/tests/context_fill_restore_seeds_threshold_rpc101.rs`, `rust/fspec-tui/tests/diff_format_rpc390.rs`, `rust/fspec-tui/tests/edit_diff_rendering_rpc391.rs`, `rust/fspec-tui/tests/git033_file_search_isolated_worktree.rs`, `rust/fspec-tui/tests/inline_hitl_prompt_rpc411/other_mode.rs`, `rust/fspec-tui/tests/keyboard_cascade_rpc051.rs`, `rust/fspec-tui/tests/loop_dispatch_rpc059.rs`, `rust/fspec-tui/tests/model_selector_filter_row_model007.rs`, `rust/fspec-tui/tests/multiline_input_cursor_editing_tui039.rs`, `rust/fspec-tui/tests/pending_input_durability_rpc052.rs`, `rust/fspec-tui/tests/turn_content_modal_rpc382.rs`, `rust/sessions/src/profile_sections/persistence_tests.rs`, `rust/sessions/src/profile_sections/tests.rs`, `rust/sessions/tests/rpc348_mid_session_facade_reresolution.rs`, `rust/tools/src/blocklist/middleware.rs`

### COV-073-B5: fspec-tui provider settings, credentials, role dialog, dialog theme/compositor (17 features; merge 16, retire 1)

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| anthropic-provider-settings-regression | merge | rust/fspec-tui/tests/prov112/view_keys.rs | provider-settings-oauth-disconnect |
| codex-oauth-integration-bugs | merge | rust/fspec-tui/tests/prov112/view_keys.rs | provider-settings-oauth-disconnect |
| provider-settings-expansion-state | merge | rust/fspec-tui/tests/provider_dispatch_rpc054/credentials.rs | provider-settings-oauth-disconnect (api-key part: rpc054-provider-settings-dispatch) |
| provider-settings-oauth-guards | merge | rust/fspec-tui/tests/provider_settings_list_keybind_parity_rpc157.rs | provider-catalog-canonical-17-provider-ordered-list-with-display-names, provider-settings-rich-tree, provider-settings-parity-and-automatic-model-limits, provider-settings-backend-parity, provider-settings-profile-crud, provider-settings-oauth-login, provider-settings-oauth-disconnect, rpc054-provider-settings-dispatch, rpc106-provider-settings-footer-hints, rpc157-provider-settings-list-mode-keybind-parity |
| github-copilot-tui-provider-integration | merge | rust/fspec-tui/tests/provider_settings_oauth_copilot_device_prov114.rs | provider-settings-oauth-copilot-device |
| oauth-tui-broken-flows | merge | rust/fspec-tui/tests/provider_settings_oauth_login_prov113.rs | provider-settings-oauth-login, provider-settings-rich-tree |
| tui-oauth-login-flow | merge | rust/fspec-tui/tests/provider_settings_oauth_login_prov113.rs | provider-settings-oauth-login (NAPI scenario retired per REFAC-027) |
| tui-anthropic-oauth-login | merge | rust/fspec-tui/tests/provider_settings_oauth_login_prov113.rs | provider-settings-oauth-login |
| compaction-threshold-tui-config | merge | rust/fspec-tui/tests/provider_settings_profile_compaction_threshold_prov115.rs | provider-settings-profile-compaction-threshold |
| profile-form-arrow-key-navigation | merge | rust/fspec-tui/tests/provider_settings_profile_form_prov110.rs | provider-settings-profile-form |
| provider-settings-oauth-logout | merge | rust/fspec-tui/tests/provider_settings_rich_tree_rpc349.rs | provider-settings-rich-tree, provider-settings-oauth-disconnect, rpc106-provider-settings-footer-hints |
| role-banner-display | merge | rust/fspec-tui/tests/role_banner_integration_rpc022.rs | rpc022-role-banner |
| role-dialog | merge | rust/fspec-tui/tests/role_dialog_rpc063.rs, rust/fspec-tui/tests/slash_role_rpc063.rs | role-dialog-component,role-slash-command-end-to-end-ui-dialog |
| role-dialog-remove-button | retire | - | RPC-063 (role-dialog-component: Remove button replaced by Ctrl+D clear) |
| dialog | merge | rust/fspec-tui/tests/rpc027_dialog_theme.rs, rust/fspec-tui/src/compositor_tests.rs, rust/fspec-tui/tests/rpc027_dialog_parity_ef.rs | rpc027-dialog-theme,fspec-tui-compositor,rpc027-model-confirm-dialogs |
| session-header-realtime-status | merge | rust/fspec-tui/tests/rpc029_agent_header_alignment.rs | bug-180-sessionheader-work-unit-status-sync + rpc029-agent-header-alignment |
| session-header-isolation-badge | merge | - | sessionheader-isolated-badge-chrome-paint-wiring |

Test files touched: `rust/fspec-tui/src/compositor_tests.rs`, `rust/fspec-tui/tests/prov112/view_keys.rs`, `rust/fspec-tui/tests/provider_dispatch_rpc054/credentials.rs`, `rust/fspec-tui/tests/provider_settings_list_keybind_parity_rpc157.rs`, `rust/fspec-tui/tests/provider_settings_oauth_copilot_device_prov114.rs`, `rust/fspec-tui/tests/provider_settings_oauth_login_prov113.rs`, `rust/fspec-tui/tests/provider_settings_profile_compaction_threshold_prov115.rs`, `rust/fspec-tui/tests/provider_settings_profile_form_prov110.rs`, `rust/fspec-tui/tests/provider_settings_rich_tree_rpc349.rs`, `rust/fspec-tui/tests/role_banner_integration_rpc022.rs`, `rust/fspec-tui/tests/role_dialog_rpc063.rs`, `rust/fspec-tui/tests/rpc027_dialog_parity_ef.rs`, `rust/fspec-tui/tests/rpc027_dialog_theme.rs`, `rust/fspec-tui/tests/rpc029_agent_header_alignment.rs`, `rust/fspec-tui/tests/slash_role_rpc063.rs`

### COV-073-B6: fspec-tui scheduling, scrollback, resume view, board units, thinking-level dialog (16 features; human 1, merge 14, write 1)

Blocked in part on the (human) answers for: session-work-unit-ipc.

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| schedule-tui-slash-commands | merge | rust/fspec-tui/tests/schedule_dispatch_rpc058.rs | rpc058-schedule-dispatch |
| toggle-line-selection-mode-with-select-command | merge | rust/fspec-tui/tests/scrollback_scroll_rpc094.rs, rust/fspec-tui/tests/view_agent_scrollback_rpc019.rs | rpc094-agentview-scrollback-scroll,rpc019-scrollback |
| attach-to-detached-sessions-from-resume-view | merge | rust/fspec-tui/tests/slash_resume_rpc049.rs, rust/fspec-tui/tests/rpc026_resume_session_view.rs, rust/sessions/tests/tui099_session_list_ordering.rs | slash-command-resume,rpc026-resume-session-view,resume-session-ordering |
| interactive-kanban-board-cli | merge | rust/fspec-tui/tests/store_board_unit_rpc012.rs, rust/fspec-tui/tests/store_board_viewport_rpc016.rs, rust/fspec-tui/tests/view_board_unit_rpc013.rs | rpc012-board-store,rpc016-board-store-viewport,rpc013-board-footer |
| tui-board-column-scrolling | merge | rust/fspec-tui/tests/store_board_viewport_rpc016.rs, rust/fspec-tui/tests/view_board_unit_rpc016.rs | rpc016-board-store-viewport,rpc016-board-viewport |
| fix-tui-kanban-column-layout-to-match-table-style | merge | rust/fspec-tui/tests/view_board_unit_rpc014.rs, rust/fspec-tui/tests/grid_unit_rpc014.rs, rust/fspec-tui/tests/view_board_unit_rpc016.rs, rust/fspec-tui/tests/store_board_unit_rpc012.rs | rpc014-board-grid,rpc014-grid-helpers,rpc016-board-viewport,rpc012-board-store |
| display-attachments-in-work-unit-details-panel | merge | rust/fspec-tui/tests/view_board_unit_rpc014.rs | rpc014-board-grid |
| replace-shimmer-with-fast-forward-emojis-for-active-work-unit | merge | rust/fspec-tui/tests/view_board_unit_rpc016.rs | rpc016-board-viewport |
| purple-watcher-input-display | merge | rust/fspec-tui/tests/supervisor_message_rendering_rpc387.rs | supervisor-message-rendering; Authority scenarios superseded by WATCH-024 |
| thinking-level-dialog | merge | rust/fspec-tui/tests/thinking_level_dialog_rpc022.rs | rpc022-thinking-level-dialog (+ RPC-048/RPC-018/MODEL-011 owners) |
| session-work-unit-ipc | human | rust/fspec-tui/tests/tui060_status_change_reattach.rs (only if kept) | recommended @deprecated 103a73072 + RPC-050 explicit attach |
| board-view-watcher-update | write | rust/fspec-tui/tests/tui079_board_reflects_watcher_snapshots.rs | - |
| select-turns-instead-of-lines-with-select-command | merge | rust/fspec-tui/tests/turn_select_mode_rpc381.rs | agentview-turn-select-mode |
| slash-command-autocomplete-palette | merge | rust/fspec-tui/tests/view_agent_popups_rpc020.rs | rpc020-slash-and-file-popups |
| file-search-popup-modal-for-symbol-input | merge | rust/fspec-tui/tests/view_agent_popups_rpc020.rs, rust/fspec-tui/tests/rpc027_dialog_parity_gh.rs | rpc020-slash-and-file-popups (+rpc027-slash-file-popups) |
| remove-file-watching-from-tui-main-screen-and-lazy-load-changed-files-view | merge | rust/fspec-tui/tests/view_board_unit_rpc015.rs, rust/fspec-tui/tests/file_changes_view_rpc354.rs, rust/fspec-tui/tests/bug181_live_checkpoint_counts.rs | rpc015-board-header,rust-file-changes-view,live-checkpoint-counts-in-the-board-header |

Test files touched: `rust/fspec-tui/tests/bug181_live_checkpoint_counts.rs`, `rust/fspec-tui/tests/file_changes_view_rpc354.rs`, `rust/fspec-tui/tests/grid_unit_rpc014.rs`, `rust/fspec-tui/tests/rpc026_resume_session_view.rs`, `rust/fspec-tui/tests/rpc027_dialog_parity_gh.rs`, `rust/fspec-tui/tests/schedule_dispatch_rpc058.rs`, `rust/fspec-tui/tests/scrollback_scroll_rpc094.rs`, `rust/fspec-tui/tests/slash_resume_rpc049.rs`, `rust/fspec-tui/tests/store_board_unit_rpc012.rs`, `rust/fspec-tui/tests/store_board_viewport_rpc016.rs`, `rust/fspec-tui/tests/supervisor_message_rendering_rpc387.rs`, `rust/fspec-tui/tests/thinking_level_dialog_rpc022.rs`, `rust/fspec-tui/tests/tui060_status_change_reattach.rs`, `rust/fspec-tui/tests/tui079_board_reflects_watcher_snapshots.rs`, `rust/fspec-tui/tests/turn_select_mode_rpc381.rs`, `rust/fspec-tui/tests/view_agent_popups_rpc020.rs`, `rust/fspec-tui/tests/view_agent_scrollback_rpc019.rs`, `rust/fspec-tui/tests/view_board_unit_rpc013.rs`, `rust/fspec-tui/tests/view_board_unit_rpc014.rs`, `rust/fspec-tui/tests/view_board_unit_rpc015.rs`, `rust/fspec-tui/tests/view_board_unit_rpc016.rs`, `rust/sessions/tests/tui099_session_list_ordering.rs`

### COV-073-B7: fspec CLI init, git changed-files/merge, sessions model/credentials, tools facade (12 features; human 1, merge 9, remap 2)

Blocked in part on the (human) answers for: changed-files-view-doesn-t-show-deleted-files-correctly.

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| duplicate-installation-success-message-in-fspec-init | merge | rust/fspec/tests/cli_init.rs | init-cli-subcommand |
| duplicate-next-steps-message-in-fspec-init-output | merge | rust/fspec/tests/cli_init.rs | init-cli-subcommand |
| changed-files-view-doesn-t-show-deleted-files-correctly | human | rust/git/tests/changed_files_rpc355.rs | git-change-type-derivation |
| restore-individual-files-or-all-files-from-checkpoint-in-checkpoint-viewer | merge | rust/git/tests/ghost_commit_tests.rs | checkpoint-restore (parent-dir test into ghost-commit-checkpoints) |
| merge-conflict-markers | remap | rust/git/tests/three_way_merge_integration_tests.rs, rust/git/src/three_way_merge.rs | - |
| tui-custom-provider-section-builder | merge | rust/providers/tests/bug_139_per_model_limits/scenarios.rs | custom-provider-napi-model-limits |
| custom-provider-vision-support | merge | rust/sessions/tests/bug168_model_vision_resolution.rs | session-model-capabilities-registry |
| work-unit-context-service | merge | rust/sessions/tests/cov057_work_unit_context_backend.rs | work-unit-context-backend |
| claude-oauth-model-routing | merge | rust/sessions/tests/prov128_unified_credential_gating.rs | unified-credential-gating |
| codex-model-selector-integration | merge | rust/sessions/tests/prov129_codex_section_synthesis.rs | codex-section-synthesis |
| bridge-session-chunk-filtering | merge | rust/tools/src/bridge.rs | bridge-tool-unit, tool-wrapper-session-association |
| structured-fspectool-results-via-streamchunk-discriminated-union | remap | rust/tools/tests/fspec_facade_wrapper_test.rs | - |

Test files touched: `rust/fspec/tests/cli_init.rs`, `rust/git/src/three_way_merge.rs`, `rust/git/tests/changed_files_rpc355.rs`, `rust/git/tests/ghost_commit_tests.rs`, `rust/git/tests/three_way_merge_integration_tests.rs`, `rust/providers/tests/bug_139_per_model_limits/scenarios.rs`, `rust/sessions/tests/bug168_model_vision_resolution.rs`, `rust/sessions/tests/cov057_work_unit_context_backend.rs`, `rust/sessions/tests/prov128_unified_credential_gating.rs`, `rust/sessions/tests/prov129_codex_section_synthesis.rs`, `rust/tools/src/bridge.rs`, `rust/tools/tests/fspec_facade_wrapper_test.rs`

### COV-073-B8: spec-only: board/details/session merges already proven by owners (16 features; merge 9, retire 7, spec-only)

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| open-attachments-in-browser-from-details-panel | merge | - | rust-board-open-attachment, card-attachments-come-from-the-daemon |
| tui-clear-should-update-react-state-as-side-effect-of-rust-state-change | merge | - | slash-command-clear, session-events-tui-cleared-consumption, bridge-clear-checkpoint-failure, telegram-clear-context-reset |
| fix-work-unit-details-panel-to-be-static-4-lines-high | retire | - | @deprecated TUI-012 + RPC-014 (5-row details strip) |
| tui-016-incomplete-checkpointpanel-using-chokidar-instead-of-ipc-zustand | merge | - | live-checkpoint-counts-watcher-push, rpc015-app-bootstrap |
| screen-component-integration | merge | - | model-selector-view, model-selector-tab-to-providers, model-selector-navigator-wiring, rpc054-provider-settings-dispatch |
| session-attachment-for-work-units-with-tui-resume-integration | merge | - | rpc016-board-viewport, board-card-enter-chooser-and-complete-details (/detach scenario @deprecated RPC-050) |
| tui-session-chunk-filtering | merge | - | agentview-subscribe-broadcasts, rpc024-multi-session-store |
| watcher-session-header-indicator | retire | - | TUI-080 (17d239810) + WATCH-024 |
| anchor-view | retire | - | CMPCT-013 (ab07e96be) |
| pure-flexbox-layout-for-checkpoint-and-changed-files-viewers | retire | - | 103a73072 (Ink flexbox); diff scenario dup of rust-changed-files-view |
| delete-checkpoint-or-all-checkpoints-from-checkpoint-viewer-with-confirmation | merge | - | checkpoint-delete |
| tui-pause-state-display | merge | - | inline-tool-approval-pause-prompt (Continue scenario retired per RPC-406) |
| provider-settings-screen | merge | - | rpc054-provider-settings-view (t/r retired per RPC-154/RPC-157) |
| session-management-panel | retire | - | GIT-036 (7b38c6cb2) |
| session-management-panel-keyboard | retire | - | GIT-036 (7b38c6cb2) |
| default-thinking-level-persistence | retire | - | MODEL-012 |

### COV-073-B9: spec-only: resume/scroll/mouse/selection retire and merges (16 features; human 1, merge 2, retire 13, spec-only)

Blocked in part on the (human) answers for: delete-session-from-resume-view-with-confirmation-dialog.

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| delete-session-from-resume-view-with-confirmation-dialog | human | - | rpc026-resume-session-view |
| multilineinput-should-show-compaction-status-instead-of-conversation-message | retire | - | BUG-213 + TUI-115 (+ CMPCT-013) |
| kanban-tui-keyboard-navigation-page-up-down-and-home-end-key-bindings | merge | - | rpc016-board-viewport |
| virtuallist-height-calculation-ignores-flexbox-container-dimensions | retire | - | 103a73072 (Ink VirtualList measureElement) |
| mouse-scroll-wheel-and-trackpad-support-for-virtuallist | merge | - | rust-changed-files-view |
| native-text-selection-while-preserving-mouse-scroll-wheel-in-virtuallist | retire | - | COPY-006 |
| sgr-mouse-protocol-component-integration | retire | - | 103a73072 (TS SGR parser) + COPY-006 |
| cross-pane-selection-with-correlation-ids | retire | - | TUI-080 (17d239810) + AMGR-008 (a9d161f9b) |
| discuss-selected-feature | retire | - | TUI-080 (17d239810) |
| expand-command-for-turn-based-tool-output-expansion | retire | - | TUI-045 (removed /expand; Enter opens TurnContentModal) |
| copilot-tui-login-dispatch | retire | - | PROV-128 unified-credential-gating + PROV-114 Copilot device login in Provider Settings |
| optimize-context-compaction-performance-and-ux | retire | - | CMPCT-013 (legacy-compaction-cleanup removed the anchor compaction pipeline) |
| fix-anchor-viewer-integration-with-thinking-dialog-system | retire | - | @deprecated CMPCT-013 (legacy-compaction-cleanup, ab07e96be) |
| interactive-anchor-point-viewer-with-conversation-navigation | retire | - | @deprecated CMPCT-013 (legacy-compaction-cleanup, ab07e96be) |
| zustand-state-management-setup-for-fspec-data | retire | - | @deprecated 103a73072 + RPC-012 (BoardStore replaces Zustand store) |
| create-isolated-session-napi | retire | - | @deprecated REFAC-027 (f25b90b2e) |

### COV-073-B10: spec-only: Ink/NAPI/watcher-dialog retire (15 features; retire 15, spec-only)

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| provider-type-consolidation | retire | - | @deprecated 103a73072 |
| isolated-session-napi-bindings | retire | - | @deprecated REFAC-027 (f25b90b2e) |
| scaffold-tui-infrastructure-with-ink-and-cage-components | retire | - | @deprecated 103a73072 |
| compaction-badge-sign-integrity-ink | retire | - | 103a73072 (Rust twin compaction-badge-sign-integrity owns it) |
| refactor-header-and-details-to-component-based-flexbox | retire | - | Ink component/flexbox internals; Ink TUI removed 103a73072; visible behaviour owned by rpc015-board-header/rpc014-board-grid/rpc016-board-viewport |
| remove-dead-split-view | retire | - | TUI-080 done (17d239810); TS-only removal/npm steps; src/ removed 103a73072 |
| auto-inject-toggle-in-watcher-creation-dialog | retire | - | AMGR-008 (a9d161f9b) |
| watcher-creation-dialog-ui | retire | - | AMGR-008 (a9d161f9b) |
| watcher-management-overlay-ui | retire | - | AMGR-008 (a9d161f9b) |
| watcher-split-view-ui | retire | - | TUI-080 (17d239810) |
| watcher-templates | retire | - | AMGR-008 (a9d161f9b); test removed TUI-083 (81c3c239a) |
| worker-module-not-bundled-module-not-found-for-diff-worker-js | retire | - | Node worker bundling; fa07d249a (native Rust diff) + 103a73072 |
| sgr-mouse-protocol | retire | - | RPC-023 (crossterm replaced mouseProtocol.ts) + 103a73072 |
| refactor-watcher-terminology-to-supervisor-subordinate-with-chainofcommand-graph | retire | - | @deprecated 103a73072 + REFAC-027 f25b90b2e (Rust rename done in 8de0ca0c1) |
| codex-subscription-model-filtering | retire | - | dd8da6684 (codex allowlist removed) + PROV-129 codex-section-synthesis (no allowlist filtering) |

## Per-feature decisions

Grouped by batch. Fields: class, area, rust code, rust tests (with current owner), covered, not covered, action, question.

### Batch COV-073-B1

#### hitl-handler-wiring
- class: write
- area: agent-loop / sessions (+ fspec-tui duplicates)
- scenarios: 16 mapped to deleted files
- rust code: rust/agent-loop/src/agent_loop/handlers.rs:23-34 HITL handler: set_hitl_request -> Paused -> wait_for_hitl_response -> clear -> Running; handlers.rs:64-66 clear_input_handlers (called at agent_loop.rs:269); rust/sessions/src/background_session/interactions.rs:105-151 wait (Cancelled fallback on closed channel)/send/set/get; rust/tools/src/request_user_input.rs:182 set_hitl_handler (no handler -> error); rust/rpc-types StreamChunk has no HitlRequest variant
- rust tests: rust/sessions/tests/rpc410_hitl_wire_protocol_parity.rs (owner: hitl-wire-protocol-parity) get_hitl_request / cancel round-trip; rust/sessions/tests/paused_chunk_delivery_rpc409.rs::paused_chunk_reaches_subscribers_while_the_hitl_wait_is_still_pending (owner: paused-chunk-delivery-during-blocking-waits); rust/tools/src/request_user_input.rs headless test (owner: request-user-input-hitl-tool); rust/fspec-tui/tests/inline_hitl_prompt_rpc411/*.rs (owner: inline-hitl-prompt); rust/fspec-tui/tests/pause_hitl_rpc053.rs (owner: pause-and-hitl-dialogs)
- covered: none in a file this feature can own
- not covered: HITL handler stores questions in session state and pauses -> write, HITL handler cleanup on session end -> write, BackgroundSession has HITL request state and response channel pair -> write, Headless mode returns error without blocking -> delete (duplicate of request-user-input-hitl-tool "Request user input in headless mode returns error"), NAPI getter returns HITL request when session is paused -> retire (REFAC-027 f25b90b2e; the RPC replacement is owned by hitl-wire-protocol-parity), NAPI binding converts TypeScript response to Rust HitlResponse -> retire (REFAC-027), NAPI binding converts TypeScript cancellation to Rust HitlResponse -> retire (REFAC-027), useRustSessionState includes hitlRequest in snapshot when paused -> delete (duplicate of pause-and-hitl-dialogs "SessionStateChange{Paused} with hitl_request Some stores a HITL slot"), useRustSessionState returns null hitlRequest when not paused -> delete (duplicate of pause-and-hitl-dialogs "SessionStateChange{Idle} clears the HITL slot"), InputTransition renders HITL question with options inline -> delete (dup inline-hitl-prompt), InputTransition renders freeform-only HITL question -> delete (dup inline-hitl-prompt), Multi-step HITL advances through questions -> delete (dup inline-hitl-prompt), AgentView HITL keyboard handler navigates options -> delete (dup inline-hitl-prompt), AgentView HITL keyboard handler submits all answers -> delete (dup inline-hitl-prompt), User cancels HITL with Escape -> delete (dup inline-hitl-prompt "Esc on an options question cancels the whole request"), HitlRequest StreamChunk variant removed -> delete (TS GlobalSessionStreamManager steps; Rust never had the variant)
- action: Write rust/agent-loop/tests/bug118_hitl_handler_wiring.rs for the 3 Rust handler/state scenarios (register handlers, invoke the HITL closure, check Paused/Running and set/clear state, check the Cancelled fallback after cleanup). Deprecate the 3 NAPI scenarios citing REFAC-027. Delete the 9 TS/TUI duplicates.
- question: -

#### session-footer
- class: merge
- area: fspec-tui / sessions / tools
- scenarios: 11 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/footer.rs:28-29 (#333333 background), :94 (cwd + cyan [⎇ branch]), :156-172 ($HOME to ~); rust/sessions/src/footer_poller.rs:188-199 (reads only get_current_branch)
- rust tests: tests/view_agent_unit_rpc018.rs (owner: rpc018-agent-chrome); tests/view_agent_unit_rpc029.rs (rpc029-agent-structure-alignment); tests/rpc029_agent_header_alignment.rs (rpc029-agent-header-alignment, asserts the background colour); tests/wt002_footer_detached_indicator.rs (tui-footer-detached-indicator); rust/tools/tests/footer_cwd_registry_test.rs (footer-cwd-registry, also TUI-091); rust/agent-loop/tests/wt002_footer_poller_parity/footer_follows_effective_cwd.rs (fspec-binary-footer-poller)
- covered: Display CWD and branch in git repo; Display branch without dirty/untracked indicators; Display only CWD for non-git dir; Dark grey background full width; Detached HEAD; Shorten HOME to tilde; CWD updates on explicit Bash cwd; Branch updates when CWD moves to another repo; Each session tracks CWD independently; CWD returns to session default
- not covered: Footer poller uses near-zero CPU -> write (new rust/agent-loop/tests/wt002_footer_poller_parity/poller_reads_branch_only.rs, owner fspec-binary-footer-poller)
- action: merge into rpc018-agent-chrome (rendering) and footer-cwd-registry / fspec-binary-footer-poller (cwd tracking). Add one branch-only poller test

#### per-model-compaction-threshold
- class: remap
- area: cli (compaction threshold) / agent-loop
- scenarios: 8 mapped to deleted files
- rust code: rust/cli/src/compaction_threshold.rs:60-121 (universal 80% default, Tokens/Percentage override, clamp to usable context); rust/agent-loop/src/turn_compaction.rs:30-38 (stream loop resolves from ProviderManager compaction_threshold_override); rust/cli/src/interactive/context_projection.rs:65 (fill relative to threshold)
- rust tests: rust/cli/src/compaction_threshold/configuration_tests.rs::test_builtin_claude_returns_80_percent, ::test_builtin_gemini_returns_80_percent, ::test_builtin_openai_returns_80_percent, ::test_resolve_openai_gpt4o, ::test_builtin_unknown_returns_80_percent, ::test_resolve_priority_unknown_default, ::test_threshold_config_tokens_resolve, ::test_threshold_config_percentage_60, ::test_resolve_clamps_user_threshold_above_context, ::test_resolve_priority_user_override (owner: none)
- covered: Claude model uses 80% built-in default, Gemini model uses 80% built-in default, OpenAI model uses 80% built-in default, User-configured absolute token threshold, User-configured percentage threshold, Unknown model uses 80% built-in default, User threshold exceeding context window is clamped, Threshold resolution priority chain
- not covered: Context fill percentage uses compaction threshold -> write (add to configuration_tests.rs: fill = tokens / compaction_threshold_for; the similar catalog_tests fn is owned by compaction-threshold-clamped-inputs), Stream loop uses ProviderManager compaction threshold -> write (rust/agent-loop/tests/ctx007_stream_loop_threshold.rs, turn_compaction.rs:30)
- action: Map rust/cli/src/compaction_threshold/configuration_tests.rs (unowned; needs @step comments) and add the 2 new tests. COV-075 plan B7 suggested configuration_tests.rs for CTX-002, but its content matches CTX-007 and CTX-002 is covered by capacity_tests.rs. universal-compaction-default (no mappings) may also want these fns, so don't map the file to both.
- question: -

#### boardview-loaddata-stability
- class: human
- area: core
- scenarios: 8 mapped to deleted files
- rust code: rust/core/src/work_units.rs:249 (notify_debouncer_mini debounced watcher on spec/); :274-284 (a failed re-read logs `warn!` and keeps the prior snapshot, nothing broadcast); TUI has no loadData and only receives pushed snapshots (rust/fspec-tui/src/app/bootstrap.rs:95-108; a lagged channel refetches and shows an error, BUG-379)
- rust tests: rust/core/tests/work_units_watcher_test.rs (owner: work-units-watcher, publish-on-mutation only); rust/fspec-tui/tests/bug379_work_units_refetch_errors.rs (owner: work-units-refetch-errors)
- covered: none directly
- not covered: Lock contention error does not trigger ErrorView -> write or retire; Lock contention does not clear prior successful state -> write (core) or retire; Real errors still set error state -> human (Rust only logs a failed re-read; no error is shown); loadData clears error only on success -> retire; Concurrent loadData calls prevented by in-flight guard -> retire; debounces WorkUnitsUpdate events -> write (core) or retire; Debounce timer resets on each new event -> write (core) or retire; Session context sync preserved after debounced loadData -> delete (dup of bug-180-sessionheader-work-unit-status-sync)
- action: yes → rewrite to the daemon watcher in rust/core/tests/bug119_work_units_watcher_stability.rs (failed re-read keeps the prior snapshot; burst of writes → one snapshot). no → @deprecated citing work-units-watcher + BUG-379.
- question: The TS loadData/globalStreamListener path is gone. Rust has a debounced daemon watcher that keeps the last good snapshot when a re-read fails. Should BUG-119 be @deprecated as superseded by work-units-watcher and BUG-379? (Recommended: yes, deprecate. Alternative: rewrite it against rust/core/src/work_units.rs with a new test. Also decide separately whether a failed re-read, which today is only logged, should be shown to the user.)

#### graceful-initialization-without-spec-directory
- class: human
- area: core
- scenarios: 5 mapped to deleted files
- rust code: rust/core/src/work_units.rs:155-177 (read_snapshot returns empty when work-units.json is missing; parse errors are Err), work_units.rs:290-300 (creates spec/ and watches it), work_units.rs:274-284 (on a failed reload, warns and keeps the last snapshot). WorkUnitsWatcher::new propagates an initial parse error (work_units.rs:235), and rust/fspec/src/common.rs:95-96 fails startup on it.
- rust tests: rust/core/tests/work_units_watcher_test.rs (owner: work-units-watcher; mutation broadcast only)
- covered: none
- not covered: Display empty board when spec directory does not exist -> write, Auto-reload when spec directory is created -> write, Handle temporary file removal during upgrade -> write, Handle corrupted work-units.json gracefully -> human, Clean exit on user termination -> delete (TS file-watcher cleanup and exit code; ratatui teardown is owned elsewhere)
- action: Write rust/core/tests/init016_graceful_missing_spec.rs for the 3 missing-file scenarios. The corrupted-JSON scenario depends on the answer.
- question: Today a corrupted spec/work-units.json at startup makes WorkUnitsWatcher::new fail, which aborts embedded TUI startup (common.rs:95); a corruption during a reload keeps the last good snapshot. Should startup degrade to an empty board and keep watching? Recommended: (a) Yes, treat this as a bug and file a fix card; keep the scenario and test it after the fix. (b) No, delete the scenario and accept the current fail-fast behaviour.

#### native-fspec-tool-integration-via-napi-rs
- class: merge
- area: agent-loop / fspec-core / tools
- scenarios: 3 mapped to deleted files
- rust code: rust/agent-loop/src/fspec_handler.rs:6-60 — without a callback, fspec runs in-process via codelet_fspec_core::dispatch_command and returns data plus system_reminder (no process spawn); rust/tools/src/fspec.rs FspecTool
- rust tests: rust/fspec-core/tests/dispatcher_test.rs ("agent_loop falls back to the Rust dispatcher when the NAPI chunk callback is absent") (owner: fspec-tool-rust-dispatcher)
- covered: AI agent executes multiple commands without spawning delays (in-process dispatch; drop the timing claim)
- not covered: AI agent receives structured data and workflow guidance -> write (into fspec-tool-rust-dispatcher: dispatch_command create-story returns data + system_reminder, in dispatcher_test.rs); AI agent uses Fspec tool alongside other codelet tools -> delete (no distinct testable behaviour)
- action: Merge into fspec-tool-rust-dispatcher (TOOL-019). The NAPI transport itself was removed by REFAC-027 (f25b90b2e) but the in-process Fspec tool remains. Add one system-reminder test to dispatcher_test.rs.

#### markdown-table-rendering-in-ai-output
- class: merge
- area: fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-tui/src/store/agent_view/markdown_table_render.rs:44 parse_alignment + box-drawing grid
- rust tests: rust/fspec-tui/src/store/agent_view/markdown_tables_tests.rs::{simple_two_column_table_renders_as_box_drawing_grid, colon_separators_set_per_column_alignment, table_embedded_in_prose_renders_in_place} (owner: markdown-table-box-drawing-rendering-in-rust-chat-view); rust/fspec-tui/tests/per_message_table_finalization_rpc432.rs (owner: markdown-tables-per-assistant-message-finalization, render on completion)
- covered: Simple table with two columns renders with box borders (bold header not asserted); Table with alignment specifiers renders with correct alignment; Tables mixed with other content preserves surrounding text
- not covered: none
- action: merge into markdown-table-box-drawing-rendering-in-rust-chat-view. Drop all 3 as duplicates.

#### virtuallist-scroll-only-mode-for-agentmodal
- class: remap
- area: fspec-tui
- scenarios: 15 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/scrollback.rs (ScrollbackList scroll and item modes); rust/fspec-tui/src/views/agent/dispatch.rs:206-217 (PageUp/PageDown/End/Home), 266-272 (arrow at the textarea edge scrolls by one line)
- rust tests: rust/fspec-tui/src/views/agent/scrollback_tests.rs (owner: none): scroll_up_disables_stick_and_caps_at_zero, scroll_down_caps_at_max_and_re_enables_stick, push_with_stick_mode_keeps_latest_chunks_visible, rpc381_default_selection_mode_is_scroll_with_no_selection, rpc381_navigate_up_selects_previous_turn / rpc381_navigate_down_clamps_at_last_turn. Duplicates: tests/scrollback_scroll_rpc094.rs (rpc094-…), tests/view_agent_scrollback_rpc019.rs (rpc019-scrollback), views/checkpoints/tests.rs (rust-checkpoints-view)
- covered: Scroll offset clamped at top; Scroll offset clamped at bottom; scrollToEnd auto-scrolls; Arrow keys move selection in item mode; AgentModal uses scroll mode (scrollback_tests.rs); Scroll up with arrow, Home, Mouse scroll with acceleration (RPC-094 duplicates); Page down/up, End (RPC-019 duplicates); CheckpointViewer uses item mode (rust-checkpoints-view "Arrow keys act on the focused pane")
- not covered: Scroll down with arrow key -> write (in scrollback_tests.rs: ScrollbackLineDown from an empty input); isSelected always false -> retire; onFocus never invoked -> retire (React renderItem/onFocus props)
- action: map rust/fspec-tui/src/views/agent/scrollback_tests.rs to this feature (it needs @step comments). Add one Down-arrow line-scroll test. Drop the RPC-094/RPC-019 duplicates. Deprecate the 2 React-prop scenarios (103a73072)

#### checkpoint-restore-progress-dialog
- class: human
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: rust/fspec-tui/src/views/checkpoints/dialog.rs:35-95 — phases Confirm -> Restoring ("Restoring…", no per-file X/Y) -> Complete ("any key: close") or Error ("Error: msg", any key closes); there is no 3s auto-close or countdown. The reusable StatusDialog with a progress counter and 3s auto-close exists in rust/fspec-tui/src/components/status_dialog.rs but checkpoint restore does not use it.
- rust tests: rust/fspec-tui/src/views/checkpoints/restore_tests.rs (owner: checkpoint-restore); rust/fspec-tui/tests/dialogs_rpc079.rs::status_dialog_restoring_shows_progress_counter_and_ignores_esc, ::status_dialog_transitions_restoring_to_complete_with_green_title_and_3s_auto_close, ::status_dialog_transitions_restoring_to_error_with_red_border_and_esc_dismissal (owner: rust-error-notification-status-dialog-wrappers)
- covered: StatusDialog reusability for other operations (RPC-079); CheckpointViewer integration with restore confirmation (t + y -> checkpoint-restore); Error during restore with manual dismissal (checkpoint-restore "A failed restore drives the status dialog to error", but any key dismisses, not only ESC)
- not covered: Multi-file restore with progress tracking -> delete or write; Single file restore with auto-close -> delete or write; Completion notice with early dismissal -> delete or write
- action: Option A: merge scenarios 4-6 into checkpoint-restore / rust-error-notification-status-dialog-wrappers and delete scenarios 1-3. Option B: open a card to send per-file progress into StatusDialog.
- question: Rust checkpoint restore is one daemon call (RPC-365). It shows one "Restoring…" phase and then "any key: close", with no per-file "Restoring X (n/N)" progress and no 3-second auto-close countdown. Should we (A, recommended) delete those 3 scenarios and merge the rest into checkpoint-restore and rust-error-notification-status-dialog-wrappers, or (B) keep them and open a card to add per-file progress and auto-close to checkpoint restore?

#### container-focus-indication-with-headings
- class: merge
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/views/diff_common/pane.rs:40-49 (focused heading: black on green, bold; unfocused: white, not bold); views/checkpoints/mod.rs:62-67 (three panes)
- rust tests: views/checkpoints/tests.rs::tab_moves_focus_from_checkpoints_to_files_and_highlights_heading (owner: rust-checkpoints-view); views/changed_files/tests.rs::tab_moves_focus_to_diff_then_pgdn_scrolls_diff_not_list (owner: rust-changed-files-view)
- covered: Tab navigation to file list in CheckpointViewer
- not covered: CheckpointViewer initial focus on checkpoint list -> write (views/checkpoints/tests.rs); Tab navigation to diff pane in CheckpointViewer -> write (views/checkpoints/tests.rs); ChangedFilesViewer initial focus on file list -> write (views/changed_files/tests.rs); Tab between file list and diff in ChangedFilesViewer -> write (views/changed_files/tests.rs, heading style is not asserted)
- action: merge into rust-checkpoints-view and rust-changed-files-view. Add heading-style tests to their tests.rs files. Unfocused headings are white, not bold (drop "bold")

#### model-selector-state-hook
- class: merge
- area: fspec-tui
- scenarios: 23 mapped to deleted files
- rust code: rust/fspec-tui/src/views/model_selector/state.rs:163 (rows rebuilt from providers + expanded + filter), navigation.rs, dispatch.rs (refresh / select)
- rust tests: rust/fspec-tui/src/views/model_selector/tests_core.rs (owner: model-selector-view); tests_scroll.rs (model-selector-scroll); tests_enter_expand.rs (model-selector-enter-key-behavior); tests_collapse.rs (model-selector-default-expansion); tests_loading_empty.rs (model-view-loading-and-empty-state-feedback); rust/fspec-tui/tests/model_selector_filter_row_model007.rs (model-selector-filter); rust/fspec-tui/tests/model_selector_refresh_forces_catalog_refetch_bug207.rs (the-model-selector-refresh-action-forces-a-catalog-refetch); rust/fspec-tui/src/views/model_selector/rows_tests_profile.rs (model-selector-profile-rendering); rust/sessions/tests/prov131_local_profile_probe_resilience.rs (local-profile-model-probe-resilience)
- covered: Hook initializes with loading state…; Hook loads profile sections…; Hook handles unreachable local servers gracefully; filteredFlatItems filters by provider / model name; Toggle section expansion add/remove; all Navigate up/down scenarios; Auto-scroll down/up; Filter matches provider ID and model name; Refresh models updates cache and reloads; Select cloud provider model returns complete ModelSelection (enter_with_session_emits_model_selected)
- not covered: flatItems is built from providerSections and expandedProviders -> delete (hook internals); getCurrentFlatIndex returns correct position -> delete (hook internals); Scroll and filter reset when model selector opens -> write in model-selector-view or delete; Filter change resets selection to first result -> write in model-selector-filter or delete; Select profile model includes profile configuration -> delete (TS ModelSelection shape; profile-qualified ids are owned by model-selection-persists-last-used-model)
- action: merge into model-selector-view (primary, rust/fspec-tui/src/views/model_selector/tests_core.rs). Drop duplicates against the owners listed above. Delete the hook-internal scenarios.

#### sessionheader-badge-threshold
- class: merge
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/header_build.rs:74,230-237 size_badge_label(compaction_threshold, context_window)
- rust tests: rust/fspec-tui/tests/tui001_session_header_model_info.rs::size_badge_uses_compaction_threshold_when_available ("192k"), ::size_badge_falls_back_to_context_window_when_compaction_threshold_is_unavailable (owner: session-header-model-info); rust/fspec-tui/tests/view_agent_unit_rpc018.rs renders "[192k]" (owner: per its own feature)
- covered: Badge shows compaction threshold for Claude model, Badge falls back to context window when threshold is unavailable, Badge shows compaction threshold for large-context model (same function, 800k is a data variant), Badge shows user-configured custom threshold (same function)
- not covered: Badge shows threshold after session resume -> write (into session-header-model-info, tui001_session_header_model_info.rs, or delete if resume is judged covered by session-model-limits-wire)
- action: Merge into session-header-model-info (tui001_session_header_model_info.rs). Siblings tui-badge-and-fill-display-end-to-end-verification and rust-authoritative-context-window merge into the same owner.
- question: -

#### tui-badge-and-fill-display-end-to-end-verification
- class: merge
- area: fspec-tui / cli
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/header_build.rs:230 badge; rust/cli/src/interactive/context_projection.rs:65 and rust/sessions/src/committed_publication.rs:179 fill = tokens / threshold
- rust tests: rust/fspec-tui/tests/tui001_session_header_model_info.rs::size_badge_uses_compaction_threshold_when_available, ::size_badge_falls_back_to_context_window_when_compaction_threshold_is_unavailable (owner: session-header-model-info); rust/cli/src/compaction_threshold/catalog_tests.rs::test_limits005_context_fill_percentage_accuracy (owner: compaction-threshold-clamped-inputs)
- covered: Badge shows [192k] for Claude Opus 4.6, Badge falls back to context window before model selection, Fill percentage uses compaction threshold as denominator, Badge shows [800k] for Gemini 2.5 Pro (same function), Badge shows [102k] for GPT-4o (same function)
- not covered: none
- action: Merge into session-header-model-info (badge) and compaction-threshold-clamped-inputs (fill denominator); drop all 5 as duplicates. Sibling: sessionheader-badge-threshold.
- question: -

#### persist-last-used-model-selection
- class: merge
- area: sessions
- scenarios: 8 mapped to deleted files
- rust code: rust/sessions/src/last_used_model_persistence.rs:21-40 (reads tui.lastUsedModel; a malformed file is an error, BUG-461); rust/sessions/src/startup_model_resolution.rs:52-97 (restores the persisted model only if its exact model id is in an eligible section, otherwise first-available)
- rust tests: rust/sessions/src/startup_model_resolution_tests.rs::persisted_model_is_restored_exactly, ::persisted_model_lost_credentials_falls_back_to_first_available, ::fresh_launch_selects_first_available_cloud_model (owner: startup-model-initialization-first-available); rust/sessions/tests/prov120_startup_model_persistence.rs (owner: persisted-model-store-reconciliation); rust/fspec-tui/tests/bug461_user_config_read_failures_are_reported.rs (owner: user-config-read-failures-are-reported); rust/sessions/tests/prov122_model_selection_persistence.rs (owner: model-selection-persists-last-used-model)
- covered: Restore persisted model on new session; Restore persisted model from different provider; Fall back when persisted provider has no credentials; Use default selection on fresh install; Handle corrupt config gracefully; Config uses proper nested structure; Config preserves other settings when updating model
- not covered: Fall back when persisted model no longer exists -> write (into startup-model-initialization-first-available, startup_model_resolution_tests.rs; code startup_model_resolution.rs:83)
- action: Merge into startup-model-initialization-first-available (+ persisted-model-store-reconciliation, user-config-read-failures-are-reported, model-selection-persists-last-used-model). Add one "persisted model id missing from the section -> first-available" test.

#### profile-based-model-selection-not-restored-on-session-startup
- class: merge
- area: sessions
- scenarios: 2 mapped to deleted files
- rust code: rust/sessions/src/startup_model_resolution.rs:52 resolve_startup_model (restores the persisted model when a credentialed/reachable section has it)
- rust tests: rust/sessions/src/startup_model_resolution_tests.rs::persisted_model_is_restored_exactly (owner: startup-model-initialization-first-available); rust/sessions/tests/prov122_model_selection_persistence.rs::profile_qualified_selection_round_trips (owner: model-selection-persists-last-used-model)
- covered: Restore persisted cloud model on new session
- not covered: Restore persisted profile-based model on new session -> write in startup-model-initialization-first-available (persisted "openai:work-vllm/Qwen/Qwen3-80B" against a reachable profile section)
- action: merge into startup-model-initialization-first-available (rust/sessions/src/startup_model_resolution_tests.rs). Add a profile-restore test there.

#### model-selection-service
- class: merge
- area: sessions
- scenarios: 6 mapped to deleted files
- rust code: rust/sessions/src/last_used_model_persistence.rs (tui.lastUsedModel); rust/fspec-tui/src/views/model_selector/dispatch.rs (selection with/without session); profiles resolved in Rust, no env vars (rust/providers/src/manager/impl_2.rs:181 notes TS set OPENAI_* env)
- rust tests: rust/sessions/tests/prov122_model_selection_persistence.rs::{active_session_selection_persists_and_round_trips, no_session_selection_writes_both_stores, profile_qualified_selection_round_trips} (owner: model-selection-persists-last-used-model); rust/fspec-tui/src/views/model_selector/tests_core.rs::{enter_with_session_emits_model_selected, enter_without_session_still_emits_selection} (model-selector-view); rust/fspec-tui/tests/model006_no_session_creates_session.rs (no-session-model-selection-creates-session)
- covered: Persist model selection to config file; Select cloud provider model with active session; Select model without active session
- not covered: Delete deprecated handler without breaking build -> retire (TS AgentView, 103a73072); Configure environment variables for profile-based model -> delete (TS env-var approach; Rust resolves profiles directly); Select profile-based model with active session -> delete (dup of profile_qualified_selection_round_trips; no sessionSetModelProfile in Rust)
- action: merge into model-selection-persists-last-used-model (rust/sessions/tests/prov122_model_selection_persistence.rs).

#### rust-authoritative-context-window
- class: merge
- area: sessions / fspec-tui
- scenarios: 10 mapped to deleted files
- rust code: rust/rpc-types/src/session_state.rs:41-50 SessionModel{context_window,max_output_tokens,compaction_threshold: Option}; rust/sessions/src/model_resolution/wire_limits.rs:29-43; rust/fspec-tui/src/views/agent/header_build.rs:230-237 badge = compaction_threshold.or(context_window)
- rust tests: rust/sessions/tests/prov166_wire_model_limits.rs::a_catalogued_session_reports_its_limits, ::an_uncatalogued_codex_session_reports_null_limits (owner: session-model-limits-wire); rust/fspec-tui/tests/tui001_session_header_model_info.rs::size_badge_falls_back_to_context_window_when_compaction_threshold_is_unavailable, ::no_size_badge_when_both_limits_are_unavailable (owner: session-header-model-info); rust/agent-loop/tests/cov057_sub_agent_model.rs (owner: sub-agent-model-inheritance)
- covered: Rust session state exposes model limits via NAPI (via RPC, prov166), Fallback to models.dev before model selection completes, Display context window when model is missing from models.dev (badge from Rust only), Sub-agent inherits Rust-resolved context window
- not covered: Display Rust-resolved context window when models.dev disagrees -> delete (CTX-009 later made the badge show the threshold), Display consistent context window when sources agree -> delete (CTX-009), Display context window for Gemini model -> delete (CTX-009), SessionHeader badge and context fill derive from same source -> delete (CTX-009; fill is owned by tui-badge-and-fill owners), Display context window for custom profile model -> write (into session-model-limits-wire, prov166_wire_model_limits.rs: profile context_window 32000 reported), Restore context window from Rust state on session resume -> write (into session-model-limits-wire: resumed session reports limits)
- action: Merge into session-model-limits-wire (+2 tests in prov166_wire_model_limits.rs), session-header-model-info and sub-agent-model-inheritance. Delete the 4 "[200k]/[1M] from context window" scenarios as superseded by CTX-009 (sessionheader-badge-threshold).
- question: -

#### agent-modal-model-selection
- class: merge
- area: fspec-tui / sessions / providers
- scenarios: 23 mapped to deleted files
- rust code: rust/fspec-tui/src/views/model_selector/* (full-screen /model view: collapsible provider sections, (current) marker rows_render.rs:179, [R]/[V]/[cw] badges rows.rs:118-130, Tab -> provider settings); rust/providers/src/manager/impl_2.rs:55 (registry plus tool_call validation); rust/sessions/src/handle_impl/handle_impl_1.rs:65-71 (resume restores manifest.provider as the model)
- rust tests: model_selector/tests_core.rs (owner: model-selector-view); rows_tests.rs (model-selector-rows); tests_collapse.rs (model-selector-default-expansion); rows_tests_header_count.rs (model-selector-header-count-label); rust/sessions/src/profile_sections/cloud_section_tests.rs (model-selector-cloud-section-assembly); rust/providers/tests/model_registry_selection_test.rs::test_reject_model_without_tool_call_capability, ::test_unknown_model_shows_error_with_fuzzy_suggestions (model-registry-selection-and-filtering); rust/providers/tests/model_selection_test.rs (dynamic-model-selection-via-models-dev); startup_model_resolution_tests.rs (startup-model-initialization-first-available); rust/fspec-tui/tests/tui001_session_header_model_info.rs (session-header-model-info)
- covered: Navigate between provider sections; Expand provider section with Right arrow or Enter; Collapse provider section with Left arrow; Select model with Enter key; Cancel model selection with Escape (model-selector-view); Display reasoning/vision/context-window indicators (model-selector-rows); Header shows model with capability indicators (session-header-model-info); Only show providers with valid credentials (model-selector-cloud-section-assembly); Only show models with tool_call capability; Error message when selected model unavailable (model-registry-selection-and-filtering); New session uses newWithModel factory method (startup-model-initialization-first-available); Graceful fallback when model cache unavailable (dynamic-model-selection-via-models-dev); Provider header shows model count (model-selector-header-count-label)
- not covered: Tab key opens model selector with providers as collapsible sections -> delete (Rust opens it with /model; Tab switches to provider settings, owned by model-selector-tab-to-providers); Show message when provider has no compatible models -> delete; Session stores full model path in persistence -> write; Resumed session restores exact model -> write; Legacy session with provider-only format uses default model -> delete; Fallback when resumed session model no longer exists -> delete; Model list shows consistent format -> delete (cyan band, owned by model-selector-selection-style); Tab hint in header shows model switching available -> delete
- action: Merge into the owners listed above and drop the 15 duplicates. Write rust/sessions/tests/resume_restores_manifest_model.rs (owned by this feature) for the 2 persistence/resume scenarios. Delete the other 6.

### Batch COV-073-B2

#### refactor-streamchunk-to-use-proper-discriminated-union-types
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/store/agent_view/record_chunk.rs — an exhaustive StreamChunk match; SessionStateChange updates status with no scrollback line; UserNotification is rendered verbatim; compaction progress shows in the footer/spinner, not in scrollback
- rust tests: rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs (SessionStateChange) (owner: agentview-subscribe-broadcasts); rust/fspec-tui/tests/chunk_rendering_parity_rpc078.rs (UserNotification verbatim) (owner: agentview-chunk-rendering-parity); rust/fspec-tui/tests/slash_compact_rpc047.rs::session_footer_renders_compaction_progress_when_some (owner: slash-command-compact)
- covered: SessionStateChange chunk updates state without adding to conversation; UserNotification chunk displays message in conversation; Compacting state change does not appear in conversation (progress is in the footer, not the input placeholder)
- not covered: StreamChunk handler uses exhaustive switch without string parsing -> delete (TS handler shape; the Rust enum match is exhaustive at compile time)
- action: Merge into agentview-subscribe-broadcasts, agentview-chunk-rendering-parity and slash-command-compact. Delete the TS switch scenario.

#### persistent-handler-state-refresh
- class: merge
- area: fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_stream_chunks.rs:60-97 SessionStateChange -> set_session_status for every state (no separate persistent handler); compaction progress is cleared only by the CompactionProgress/CompactionComplete paths (dispatch_stream_chunks.rs:50-58), never by SessionStateChange
- rust tests: rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs::session_state_change_chunk_updates_per_session_status (Running) (owner: agentview-subscribe-broadcasts); rust/fspec-tui/tests/slash_compact_rpc047.rs CompactionComplete clears progress (owner: slash-command-compact)
- covered: SessionStateChange(Running) via persistent handler keeps isLoading true
- not covered: SessionStateChange(Idle) via persistent handler transitions isLoading to false -> write (into agentview-subscribe-broadcasts, agent_view_chunk_dispatch_rpc045.rs: Idle chunk -> status Idle), endCompaction guard preserved for non-CompactionComplete state changes -> write (into slash-command-compact, slash_compact_rpc047.rs: Idle chunk leaves compaction progress set)
- action: Merge into agentview-subscribe-broadcasts (+1 test) and slash-command-compact (+1 test). Drop the refreshRustState wording (React).
- question: -

#### persistent-chunk-handler
- class: merge
- area: fspec-tui
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-tui/src/app/bootstrap.rs:117 — one global `chunks_rx()` subscriber forwards every `(SessionId, StreamChunk)` as `Action::ChunkReceived`, whatever the input source and whether the TUI is idle; rust/fspec-tui/src/store/agent_view/record_chunk.rs:48 (UserInput) / :131 (IncomingMessage) render injected input
- rust tests: rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs::background_session_chunk_appends_to_correct_session_context (owner: agentview-subscribe-broadcasts); rust/fspec-tui/tests/chunk_rendering_parity_rpc078.rs::{text_assistant_chunk_renders_as_white_circle_bullet_prefixed_line, done_chunk_produces_no_scrollback_line, user_input_chunk_renders_as_green_you_prefixed_line} (owner: agentview-chunk-rendering-parity); rust/fspec-tui/tests/chunk_rendering_parity_rpc071.rs::incoming_message_chunk_renders_as_supervisor_prefix_line (owner: agentview-chunk-to-lines-parity-...-rpc-045-state-chunks-consumed-silently)
- covered: Display TextChunks from bridge input while TUI is idle; Switching sessions does not show chunks from previous session (chunks land in their own session's scrollback); Done chunk updates conversation state and re-enables input (Done is silent; input follows SessionStateChange); WatcherInput chunk shows injected input before LLM response (now UserInput/IncomingMessage)
- not covered: Display ToolCall chunks from bridge input while TUI is idle -> delete (dup of chunk-rendering features); User input via TUI flows through persistent handler -> delete (React handler internals); Handler not registered when session is null -> delete (no handler registry in Rust); Chunks flow from Rust NAPI through GlobalSessionStreamManager to React state -> retire (NAPI removed, REFAC-027 f25b90b2e)
- action: merge into agentview-subscribe-broadcasts (rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs); drop the covered scenarios as duplicates, delete the rest. Sibling: global-session-stream-manager-chunk-routing (BRIDGE-012) gets the same decision.

#### global-session-stream-manager-chunk-routing
- class: merge
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: rust/fspec-tui/src/app/bootstrap.rs:117 (single global chunks_rx subscription at bootstrap); rust/fspec-tui/src/app/dispatch.rs:27 → handle_chunk_received routes by SessionId; rust/fspec-tui/src/store/agent_view.rs:152 session_context_mut_for returns None for unknown ids
- rust tests: rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs::background_session_chunk_appends_to_correct_session_context (owner: agentview-subscribe-broadcasts); rust/fspec-tui/tests/store_agent_view_multisession_rpc024.rs (owner: rpc024-multi-session-store: "session_context_mut_for routes chunks by id" / "returns None for unknown ids")
- covered: Route chunk to correct session handler; Ignore chunks for sessions without handlers
- not covered: Register global callback once at initialization -> delete (NAPI callback; Rust subscribes once in bootstrap); Multiple handlers for same session all receive chunk -> delete (no handler registry); Global handlers receive all chunks with session_id -> delete (no handler registry); No sessionAttach or sessionDetach calls -> retire (TS source check, 103a73072)
- action: merge into agentview-subscribe-broadcasts (rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs). Same decision as persistent-chunk-handler and global-session-stream-manager.

#### global-session-stream-manager
- class: merge
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: rust/fspec-tui/src/app/bootstrap.rs:117 (one subscriber for all sessions, focus-independent); rust/fspec-tui/src/app/dispatch_stream_chunks.rs (FspecCommandRequest answered via send_fspec_result whatever the view)
- rust tests: rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs::{fspec_command_request_list_work_units_round_trips_via_send_fspec_result, fspec_command_request_unknown_command_returns_error_result, background_session_chunk_appends_to_correct_session_context} (owner: agentview-subscribe-broadcasts)
- covered: Fspec command completes successfully after user navigates away (request answered without focus); AgentView receives UI chunks but not FspecCommandRequest (handled by the dispatcher, silent in scrollback; see also chunk_rendering_parity_rpc071::all_state_only_and_tool_variants_are_suppressed_from_scrollback)
- not covered: Multiple detached sessions can invoke fspec tools concurrently -> write in owner or delete; GlobalSessionStreamManager subscribes to new sessions automatically -> delete (single global broadcast, no per-session subscribe); unsubscribes when session is destroyed -> delete (same); Tests use real NAPI bindings without mocks -> retire (NAPI removed, REFAC-027 f25b90b2e)
- action: merge into agentview-subscribe-broadcasts. It shares globalSessionStreamManager.test.ts with isolation-state-stream-sync; same decision for both.

#### isolation-state-stream-sync
- class: merge
- area: fspec-tui
- scenarios: 2 mapped to deleted files
- rust code: rust/fspec-tui/src/store/agent_view/isolation_state.rs:87 set_isolation_state (per-session map, written from the IsolationStateChange chunk arm in dispatch_stream_chunks.rs)
- rust tests: rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs::isolation_state_change_chunk_updates_per_session_isolation_state (owner: agentview-subscribe-broadcasts)
- covered: IsolationStateChange StreamChunk updates sessionStore
- not covered: IsolationStateChange StreamChunk ignored for non-active sessions -> write in owner, reworded (Rust stores s-B's state in s-B's slot and leaves s-A unchanged; it is not ignored)
- action: merge into agentview-subscribe-broadcasts. Add a background-session isolation test in agent_view_chunk_dispatch_rpc045.rs. Sibling: global-session-stream-manager.

#### real-time-tokens-per-second-display-in-agent-modal-header
- class: merge
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/header_build.rs:131-146 (magenta "N.N tok/s" before the token block, only when is_loading and tokens_per_second is Some); rate computed in the backend by rust/core/src/streaming_display/tok_per_sec_calculator.rs (windowed and EMA-smoothed, not a mean of all samples)
- rust tests: rust/fspec-tui/tests/agentview_session_header_per_session_tokens_rpc099.rs ("8.5 tok/s", "12.0 tok/s"; owner: agentview-session-header-per-session-tokens)
- covered: Display tokens per second after multiple token updates (left of the token count), Update tokens per second in real-time during streaming (per-TokenUpdate value)
- not covered: Suppress tokens per second before rate samples available -> write (tokens_per_second None), Hide tokens per second when streaming ends -> write (is_loading gate, header_build.rs:140), Display proper header layout during streaming -> delete (Rust header has no "Agent: claude (streaming...)" layout), Calculate tokens per second for slow provider -> delete (backend calculator concern, not header)
- action: Merge into agentview-session-header-per-session-tokens. Add the 2 negative tests to rust/fspec-tui/tests/agentview_session_header_per_session_tokens_rpc099.rs.

#### reasoning-token-tui-display
- class: merge
- area: fspec-tui
- scenarios: 2 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/header_build.rs:148 (` {n}🧠` only when reasoning_tokens > 0); rust/fspec-tui/src/store/agent_view/token_state.rs:123; restore of reasoning tokens from the committed manifest in rust/sessions
- rust tests: rust/fspec-tui/tests/agentview_session_header_per_session_tokens_rpc099.rs ("Reasoning brain suffix toggles…", owner: agentview-session-header-per-session-tokens); rust/sessions/tests/cmpct062_token_restore.rs::saved_context_spend_cache_and_reasoning_become_live_state (owner: committed-token-state-restore)
- covered: SessionHeader displays reasoning tokens when present; Token persistence saves and restores reasoning tokens
- not covered: none
- action: both scenarios duplicate later owners (RPC-099, CMPCT-062). Merge = drop them, which in practice means @deprecated citing RPC-099 and CMPCT-062. This matches the product owner's RIG-012 decision for sibling reasoning-token-napi-bridge.

#### compaction-indicator-rust-source-of-truth
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: per-session compaction progress in AgentViewStore (rust/fspec-tui/src/app/dispatch_stream_chunks.rs:50-58 set/clear_compaction_progress by session_id); /compact Err -> error notice (no retry dialog anywhere in rust/fspec-tui)
- rust tests: rust/fspec-tui/tests/automatic_compaction_progress_bug210.rs (owner: automatic-compaction-progress); rust/fspec-tui/tests/slash_compact_rpc047.rs (owner: slash-command-compact)
- covered: Switching away from a compacting session clears the indicator, Switching back to a compacting session shows the indicator (both via "Background compaction progress stays attached to its original session"), Manual /compact command shows the indicator via Rust state (via slash-command-compact progress-segment scenarios)
- not covered: Compaction retry dialog works independently of display state -> delete (Rust has no retry/continue/cancel dialog; /compact Err emits an error notice, slash-command-compact "emits an error notice ... on Err")
- action: Merge into automatic-compaction-progress and slash-command-compact; drop 3 as duplicates and delete the retry-dialog scenario.
- question: -

#### tui-integration-for-codelet-ai-agent
- class: merge
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: AgentView replaced the AgentModal overlay: board '.' / Shift+Right open the agent (rust/fspec-tui/src/views/board/keys.rs); chunks are streamed through record_chunk.rs; tools run with no approval except blocklist/RLCD pauses; sessions persist across view switches
- rust tests: rust/fspec-tui/tests/board_period_new_agent_rpc395.rs (owner: board-key-starts-new-agent); agent_view_chunk_dispatch_rpc045.rs (agentview-subscribe-broadcasts); chunkprocessor_parity_rpc091.rs (agentview-chunkprocessor-parity); bug226_accepted_model.rs (accepted-model-selection); agentview_session_header_per_session_tokens_rpc099.rs (agentview-session-header-per-session-tokens); prov101_session_creation_decline.rs (session-creation-decline-surfaced)
- covered: Open agent modal and send prompt with streaming response; Auto-execute tool calls without approval; Switch provider mid-conversation; Display token usage and provider status; Handle missing credentials gracefully (decline shown as an error; no setup-instructions text)
- not covered: Fresh session on modal reopen -> delete (Rust keeps sessions open across Board/Agent switches, e.g. bug164 mux retained / rpc012 back-to-board preserves)
- action: Merge into board-key-starts-new-agent, agentview-subscribe-broadcasts, agentview-chunkprocessor-parity, accepted-model-selection, agentview-session-header-per-session-tokens and session-creation-decline-surfaced. Delete "Fresh session on modal reopen".

#### claude-code-style-tool-output-display
- class: merge
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: rust/fspec-tui/src/store/agent_view/chunk_tool_result.rs:23-50 — tool card header "{name}({args})" with a "● " prefix (rendered_chunk.rs:27); collapse of settled and streaming tool bodies (RPC-389)
- rust tests: rust/fspec-tui/tests/chunkprocessor_parity_rpc091.rs::tool_call_flushes_in_flight_and_pushes_tool_call_card, ::tool_result_attaches_to_matching_header_and_pushes_fresh_placeholder (owner: agentview-chunkprocessor-parity); rust/fspec-tui/tests/tool_call_output_collapse_rpc389.rs::streaming_body_shows_last_10, ::settled_long_body_collapses_to_last_8 (owner: tool-call-output-collapse)
- covered: Tool header displays in Claude Code format; Tool header remains visible after completion; All tool types use the same display format (agentview-chunkprocessor-parity); Streaming output uses fixed-height scrolling window (tool-call-output-collapse)
- not covered: Long output is collapsed with expand indicator -> delete (Rust collapses to the last 8 lines with the full body in a modal; no "+N lines (ctrl+o to expand)"); System-reminder content displays as nested tree node -> delete (none found: grepped system-reminder/system_reminder in rust/fspec-tui/src)
- action: Merge into agentview-chunkprocessor-parity and tool-call-output-collapse. Delete the 2 scenarios Rust does not implement.

#### manual-compaction-command
- class: merge
- area: fspec-tui + sessions
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_slash_commands.rs:84-97 (/compact -> backend.compact_session; on Err the notice is "[error] /compact failed: {e}"); rust/sessions/src/handle_impl/handle_impl_2.rs:84-87 ("Nothing to compact - no messages yet")
- rust tests: rust/fspec-tui/tests/slash_compact_rpc047.rs::compaction_complete_chunk_clears_progress_and_emits_notice, ::compact_emits_error_notice_into_originating_session_scrollback (owner: slash-command-compact); rust/sessions/tests/rpc418_compact_session/compact_populated_clears_conversation.rs (empty-session test) (owner: rust-tui-compact-real-compaction); rust/fspec-tui/tests/compaction_reduction_display_contract_rpc420.rs (owner: compaction-reduction-display-contract)
- covered: all (Successful manual compaction with compression feedback -> slash-command-compact + compaction-reduction-display-contract; Empty session shows nothing to compact -> rust-tui-compact-real-compaction; Compaction failure preserves context -> slash-command-compact)
- not covered: none
- action: Merge into slash-command-compact (RPC-047) and rust-tui-compact-real-compaction. Drop all 3 as duplicates (the error text in Rust is "[error] /compact failed:", not "Compaction failed:").

#### add-debug-slash-command-to-fspec-tui-agent
- class: merge
- area: fspec-tui + cli (debug capture)
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_slash_debug.rs (toggle_debug, notice with the path, [DEBUG] badge); event capture in rust/cli/src/interactive/stream_loop/telemetry.rs:21 (compaction.check), :33 (api.request), :61 (api.response.start), usage.rs:101 (token.update), recovery_compaction.rs:471 (compaction.triggered)
- rust tests: rust/fspec-tui/tests/slash_debug_rpc055.rs::debug_emits_success_notice_with_resolved_file_path_on_ok, ::session_header_debug_badge_appears_when_debug_enabled_is_true, ::session_header_debug_badge_absent_when_debug_enabled_is_false (owner: rpc055-slash-debug-dispatch)
- covered: Enable debug capture mode; Disable debug capture mode (the step paths say ~/.rust/debug but Rust uses ~/.fspec, per fix-debug-command-parity-gaps-in-rust-tui)
- not covered: Debug events captured during prompt -> write; Compaction triggered event captured -> write (the events are emitted but no test checks them during a prompt)
- action: Merge the 2 toggle scenarios into rpc055-slash-debug-dispatch. Write rust/cli/tests/debug_capture_prompt_events.rs (owned by this feature) for the 2 event scenarios. Siblings manual-compaction-command and tui-integration-for-codelet-ai-agent share AgentModal.test.tsx; both are merged.

#### debug-badge-session-awareness
- class: merge
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/store/agent_view/isolation_state.rs:95-101 per-session debug_enabled_by_session map; rust/fspec-tui/src/app/dispatch_stream_chunks.rs:113 DebugStateChange -> store; rust/fspec-tui/src/views/agent/header_build.rs:83 " [DEBUG]" badge; rust/fspec-tui/src/app/dispatch_session_chrome.rs:49-62 attach-time get_debug_enabled hydration; rust/sessions/src/handle_impl/handle_impl_3.rs:101-134 toggle_debug emits DebugStateChange
- rust tests: rust/fspec-tui/tests/slash_debug_rpc055.rs::session_header_debug_badge_appears_when_debug_enabled_is_true, ::session_header_debug_badge_absent_when_debug_enabled_is_false, ::debug_state_change_chunk_updates_badge_state_for_focused_session (owner: rpc055-slash-debug-dispatch); rust/fspec-tui/tests/debug_parity_rpc430.rs::session_attach_hydrates_debug_state_from_backend (owner: fix-debug-command-parity-gaps-in-rust-tui)
- covered: Toggling debug fires a Rust stream event that updates Zustand store (TUI half), Disabling debug fires a stream event that removes the badge (TUI half), Debug state is hydrated from Rust when attaching to an existing session
- not covered: DEBUG badge reflects only the active session's debug state when switching sessions -> write (into rpc055-slash-debug-dispatch, slash_debug_rpc055.rs: two sessions, render each), Rust-side "toggle_debug emits DebugStateChange" step -> write (into rpc055-slash-debug-dispatch; new fn in slash_debug_rpc055.rs or a sessions test it owns), Local React debug state and duplicate handler are removed -> delete (React internals)
- action: Merge into rpc055-slash-debug-dispatch (slash_debug_rpc055.rs); the hydration scenario is already owned by fix-debug-command-parity-gaps-in-rust-tui (debug_parity_rpc430.rs). Keep the outcome consistent with sibling debug-badge-session-cycle.
- question: -

#### debug-badge-session-cycle
- class: merge
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: same as debug-badge-session-awareness: per-session HashMap rust/fspec-tui/src/store/agent_view.rs:96 (no reset on focus change), DebugStateChange handled for any session_id at rust/fspec-tui/src/app/dispatch_stream_chunks.rs:113
- rust tests: rust/fspec-tui/tests/slash_debug_rpc055.rs::debug_only_affects_focused_session_background_untouched, ::debug_state_change_chunk_updates_badge_state_for_focused_session (owner: rpc055-slash-debug-dispatch); rust/fspec-tui/tests/debug_parity_rpc430.rs::session_attach_hydrates_debug_state_from_backend (owner: fix-debug-command-parity-gaps-in-rust-tui)
- covered: DEBUG badge appears when attaching to a session that already has debug enabled in Rust, Toggling debug on one session does not affect other sessions' state (partly, via debug_only_affects_focused_session_background_untouched)
- not covered: DEBUG badge reappears when cycling back to a session that had debug enabled -> write (into rpc055-slash-debug-dispatch: Shift+Right/Left then render), Each session retains its own debug state across multiple switches -> write (same new fn, A/B/C cycle), Debug stream event for an inactive session updates its stored state -> write (into rpc055-slash-debug-dispatch: DebugStateChange for an unfocused session then focus it)
- action: Merge into rpc055-slash-debug-dispatch, adding one or two cycling tests to rust/fspec-tui/tests/slash_debug_rpc055.rs. Sibling: debug-badge-session-awareness.
- question: -

#### context-window-fill-percentage-indicator
- class: merge
- area: fspec-tui
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/header_build.rs:131-180 (`tokens: in↓ out↑ ` followed by `[N%]`), header_build.rs:201-211 (context_fill_color: <50 green, <70 yellow, <85 magenta, else red)
- rust tests: rust/fspec-tui/tests/view_agent_unit_rpc018.rs (owner: rpc018-agent-chrome; "[0%]" on an empty store, "tokens: 1234↓ 567↑ [45%]" after ContextFillUpdate); rust/fspec-tui/tests/agentview_session_header_compaction_percentage_rpc100.rs (owner: agentview-session-header-compaction-percentage; >=85 is Red, raw >100)
- covered: Display shows 0% at start of fresh conversation (text only), Display shows percentage in red zone (85%+), Percentage resets after compaction (verbatim ContextFillUpdate), Percentage indicator is positioned correctly in header (after the token count; Rust has no Tab Switch component), Percentage displays the backend's physical-occupancy calculation verbatim
- not covered: Display shows percentage in green zone (0-49%) -> write, Display shows percentage in yellow zone (50-69%) -> write, Display shows percentage in magenta zone (70-84%) -> write (colour bands at header_build.rs:201)
- action: Merge into agentview-session-header-compaction-percentage. Add green/yellow/magenta band assertions to rust/fspec-tui/tests/agentview_session_header_compaction_percentage_rpc100.rs. Drop the "before the Tab Switch" clause.

#### context-fill-percentage-realtime-recompute-ui
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/store/agent_view/token_state.rs (apply_token_tracker / apply_context_fill); rust/fspec-tui/src/views/agent/header_build.rs:158-180
- rust tests: rust/fspec-tui/tests/token_state_realtime_recompute_rpc101.rs::{token_update_without_prior_context_fill_leaves_pct_unchanged, cache_heavy_token_update_does_not_collapse_badge} (owner: context-fill-percentage-realtime-recompute)
- covered: TokenUpdate without prior ContextFillUpdate leaves the badge unchanged, Badge shows physical occupancy including output and reasoning during streaming (owner's cache-heavy/no-discount scenario)
- not covered: TokenUpdate after cached threshold recomputes the badge locally without a new ContextFillUpdate -> retire (BUG-459/CMPCT-067 removed the local recompute), Authoritative ContextFillUpdate overrides any locally-recomputed value -> retire (BUG-459/CMPCT-067)
- action: Merge into context-fill-percentage-realtime-recompute (Rust twin). Deprecate the 2 recompute scenarios citing BUG-459 (coherent-request-context-fill). Sibling context-window-fill-percentage-indicator shares the deleted TS file and is also merged.

### Batch COV-073-B3

#### blocklist-tui-list-form-views
- class: merge
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: rust/fspec-tui/src/views/blocklist/panes.rs:103, 153-200 (list with category, Rule Details with Pattern/Reason/Guidance); views/blocklist/mod.rs (Space/Enter toggles the session-disabled set)
- rust tests: tests/blocklist_rpc056/dispatch.rs, interaction.rs, rendering.rs (owner: rpc056-blocklist-view-dispatch); views/blocklist/keybindings_tests.rs (owner: blocklist-view-keybindings)
- covered: View blocklist rules via /blocklist; User disables rule for session; User re-enables rule (TUI-side toggle and "(disabled)" suffix); Navigate with keyboard (arrows and Esc; BLOCK-010 removed j/k, so delete that step)
- not covered: View rule details -> write (in tests/blocklist_rpc056/rendering.rs; panes.rs:153-200 is unasserted); Session toggles cleared on TUI restart -> write (in tests/blocklist_rpc056/interaction.rs)
- action: merge into rpc056-blocklist-view-dispatch. Add 2 tests. Drop the j/k step, citing BLOCK-010

#### boardview-createsessiondialog-callback-ignores-isolated-parameter
- class: merge
- area: fspec-tui
- scenarios: 2 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_create_session_dialog.rs (CreateSessionSubmitted{isolated} -> create_isolated_session / create_session); board path goes through the BOARD-024 chooser
- rust tests: rust/fspec-tui/tests/board024/chooser.rs::unassociated_agent_choice_retains_normal_isolated_and_cancel_behavior (owner: board-card-enter-chooser-and-complete-details); rust/fspec-tui/tests/isolated_session_dialog_rpc060.rs::create_session_submitted_isolated_spawns_create_isolated_session, ::create_session_submitted_non_isolated_spawns_create_session (owner: rpc060-isolated-session-dialog)
- covered: all
- not covered: none
- action: Merge into board-card-enter-chooser-and-complete-details (board024/chooser.rs) and drop both scenarios as duplicates. Same decision for the other three features sharing isolated-session-integration.test.tsx: createsessiondialog-shows-wrong-text-..., enter-key-on-board-bypasses-..., sessionheader-missing-isisolated-prop-....
- question: -

#### enter-key-on-board-bypasses-create-session-dialog-no-isolated-option
- class: merge
- area: fspec-tui
- scenarios: 2 mapped to deleted files
- rust code: Board Enter -> BOARD-024 chooser (rust/fspec-tui/src/components/board_card/chooser.rs); "Open in agent" on an unattached card -> CreateSessionDialog (Yes / Yes - Isolated / Cancel); an attached card -> the earliest associated agent, with no create dialog
- rust tests: rust/fspec-tui/tests/board024/chooser.rs::unassociated_agent_choice_retains_normal_isolated_and_cancel_behavior and the default-agent-choice test (owner: board-card-enter-chooser-and-complete-details)
- covered: all (through the chooser hop)
- not covered: none
- action: Merge into board-card-enter-chooser-and-complete-details (board024/chooser.rs) and drop both scenarios as duplicates. BOARD-024 replaced the old "dialog appears directly on Enter" wording, so nothing needs moving. Sibling decision: boardview-createsessiondialog-....
- question: -

#### sessionheader-missing-isisolated-prop-isolated-badge-never-shows
- class: merge
- area: fspec-tui
- scenarios: 2 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/header.rs (green [ISOLATED] from store isolation_state)
- rust tests: rust/fspec-tui/tests/bug178_isolated_badge_chrome_paint.rs::scenario_paint_header_and_role_paints_isolated_when_store_says_session_is_isolated (checks Color::Green), ::scenario_paint_header_and_role_does_not_paint_isolated_when_store_says_not_isolated (owner: sessionheader-isolated-badge-chrome-paint-wiring)
- covered: all
- not covered: none
- action: Merge into sessionheader-isolated-badge-chrome-paint-wiring and drop both scenarios as duplicates. Sibling decision: boardview-createsessiondialog-....
- question: -

#### create-session-dialog-options
- class: merge
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/components/create_session_dialog.rs — 3 options (Yes / Yes - Isolated / Cancel), defaults to Yes, title includes the work unit
- rust tests: rust/fspec-tui/tests/isolated_session_dialog_rpc060.rs::create_session_dialog_defaults_to_yes_with_unattached_title, ::create_session_dialog_enter_on_isolated_emits_submitted_isolated, ::create_session_dialog_enter_on_cancel_emits_cancelled, ::create_session_dialog_esc_emits_cancelled, ::create_session_dialog_renders_work_unit_aware_title, ::create_session_dialog_renders_three_canonical_labels (owner: rpc060-isolated-session-dialog)
- covered: all
- not covered: none
- action: Merge into rpc060-isolated-session-dialog (RPC-060). All 5 scenarios are duplicates.

#### createsessiondialog-shows-wrong-text-when-pressing-enter-on-work-unit
- class: merge
- area: fspec-tui
- scenarios: 2 mapped to deleted files
- rust code: rust/fspec-tui/src/components/create_session_dialog.rs:90 "Work on {id}?" and :270-272 "Start an AI session for this task" / "Begin a fresh AI conversation, not linked to any task."
- rust tests: rust/fspec-tui/tests/isolated_session_dialog_rpc060.rs::create_session_dialog_renders_work_unit_aware_title (owner: rpc060-isolated-session-dialog); rust/fspec-tui/tests/board024/chooser.rs asserts "Work on AUTH-001?" (owner: board-card-enter-chooser-and-complete-details); rust/fspec-tui/tests/shift_right_create_session_dialog/generic.rs::shift_right_with_work_unit_binding_renders_generic_strings (owner: agentview-shift-right-create-session-dialog)
- covered: Show generic unattached text when using Shift+Right navigation, Show work-unit-aware text when pressing Enter on a story (title only)
- not covered: "description should be 'Start an AI session for this task'" step -> write (add the assertion to rpc060-isolated-session-dialog's create_session_dialog_renders_work_unit_aware_title)
- action: Merge scenario 1 into rpc060-isolated-session-dialog (isolated_session_dialog_rpc060.rs, add the description assertion) and drop scenario 2 as a duplicate of agentview-shift-right-create-session-dialog. Board Enter now opens the BOARD-024 chooser first. Sibling decision: see boardview-createsessiondialog-....
- question: -

#### refactor-session-work-unit-state-management
- class: merge
- area: fspec-tui
- scenarios: 14 mapped to deleted files
- rust code: board Enter/Shift+Right creates or attaches a session (rpc012 navigation); Action::AttachWorkUnitToSession (RPC-050); `SessionCreated` ignores ambient board selection; rust/fspec-tui/src/app/dispatch_merge_worktree.rs:141-188 (merge / discard via /merge-worktree dialog, not on close)
- rust tests: rust/fspec-tui/tests/board_agent_navigation_rpc012.rs + board_agent_navigation/creation.rs (owner: rpc012-board-agent-navigation); rust/fspec-tui/tests/work_unit_binding_rpc050.rs (work-unit-attach-binding); rust/fspec-tui/tests/merge_worktree_rpc057.rs (rpc057-merge-worktree-dispatch); rust/fspec-tui/tests/wt009_isolation_toggle_merge_teardown.rs (isolation-only-opens-the-create-session-dialog-…)
- covered: Session attaches to selected work unit when entering agent mode ("Enter on an unassociated work unit offers explicit creation"); New session does not auto-attach after closing / after detaching / when no selection ("Action::SessionCreated ignores ambient work unit selection"); Work unit context updates via IPC and attachToWorkUnit orchestrates (RPC-050 attach binding); Isolated session merge (WT-009 "Successful merge closes the session")
- not covered: Duplicate state removed from fspecStore -> retire (TS store); destroySession orchestrates all cleanup atomically -> delete (TS facade); detachFromWorkUnit clears all state atomically -> delete (/detach removed, remove-detach-slash-command-with-attach-path-intact); Isolated session discard removes worktree without applying changes -> delete (discard is the /merge-worktree dialog, owned by rpc057-merge-worktree-dispatch); AgentView uses sessionService facade… -> retire (TS); BoardView IPC handler uses sessionService… -> retire (TS); globalStreamListener uses sessionService… -> retire (TS)
- action: merge into rpc012-board-agent-navigation (primary). Attach scenarios go to work-unit-attach-binding, merge scenarios to rpc057-merge-worktree-dispatch. TS-facade scenarios are deleted citing 103a73072. Sibling TUI-069 is retired.

#### merge-worktree-command
- class: merge
- area: fspec-tui
- scenarios: 7 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_merge_worktree.rs:39-63 (inspect, then "nothing to merge" or MergeConfirmDialog; an inspect Err becomes "[error] /merge-worktree: …"); rust/sessions/src/handle_impl/handle_impl_9.rs:101 ("session … is not isolated"); rust/fspec-tui/src/views/agent/slash_commands.rs:155 ("Merge worktree changes and close session"; no "sessions" entry)
- rust tests: rust/fspec-tui/tests/merge_worktree_rpc057.rs (owner: rpc057-merge-worktree-dispatch); rust/fspec-tui/tests/wt009_isolation_toggle_merge_teardown.rs (owner: isolation-only-opens-the-create-session-dialog-and-merge-worktree-never-closes-the-session-ux-contract-broken-in-the-rust-tui)
- covered: Successful merge closes session and returns to board (wt009), Merge worktree with no changes shows nothing to merge (rpc057), Merge worktree with conflicts keeps session open (wt009 conflict seeds input, session stays)
- not covered: Merge worktree in non-isolated session shows error -> write (Rust surfaces the backend's "[error] /merge-worktree: session … is not isolated"; reword the message step), /merge-worktree command is registered in slash command registry -> write (registry assertion), /sessions command is removed from slash command registry -> delete, SessionManagementPanel component and tests are removed -> delete (TS files)
- action: Merge into rpc057-merge-worktree-dispatch. Add the non-isolated and registry tests to rust/fspec-tui/tests/merge_worktree_rpc057.rs.

#### merge-conflict-llm-context
- class: human
- area: fspec-tui
- scenarios: 7 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_merge_worktree.rs:234-240 on Conflict sends Action::SeedPendingInput (fills the input draft only; nothing is submitted to the agent); build_conflict_context :253-269 = "Merge produced conflicts in the following files:" + bullets + "Effective worktree: <path>" (no mention of conflict markers, no "run /merge-worktree again" instruction, no system-reminder); Success/NoChanges/Err send no context
- rust tests: rust/fspec-tui/tests/merge_worktree_rpc057.rs (Conflict seeds input; Success/NoChanges/Err notices) (owner: rpc057-merge-worktree-dispatch); rust/fspec-tui/tests/wt009_isolation_toggle_merge_teardown.rs (conflict seeds worktree path) (owner: isolation-only-opens-the-create-session-dialog-and-merge-worktree-never-closes-the-session-ux-contract-broken-in-the-rust-tui)
- covered: Multiple file conflicts send context listing all files (seeded, not sent), Successful merge does not send context to LLM (no context; but no "Press Enter to close" prompt, see merge-worktree-ux), Non-conflict error does not send context to LLM, Context message does not contain system-reminder tags (true but not asserted)
- not covered: Single file conflict sends context to LLM -> write or delete (depends on answer), Conflict context triggers auto-submit to Rust session -> delete if seeding is accepted, No context sent when session ID is missing -> delete (the Rust merge needs a session id; no-session is a silent no-op, covered by rpc057)
- action: If the recommended answer is accepted: merge into rpc057-merge-worktree-dispatch (merge_worktree_rpc057.rs), drop the duplicates, delete the auto-submit, marker-wording and missing-session scenarios, and optionally add a no-system-reminder assertion. RPC-057 says it has "TS parity with ... pendingAutoSubmitRef" but only seeds the draft.
- question: When /merge-worktree hits conflicts, the Rust TUI puts the conflict list into the input box but does not send it to the agent, and the text doesn't mention conflict markers or tell the user to rerun /merge-worktree. Pick one: (A, recommended) accept seeding as the intended Rust behaviour, merge this feature into rpc057-merge-worktree-dispatch and delete the auto-submit/wording scenarios; (B) treat it as a bug: auto-submit the context and add the marker/rerun wording, keeping this feature with a new test.

#### merge-worktree-ux
- class: merge
- area: fspec-tui
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/merge_confirm_dialog.rs (per-file change summary shown BEFORE merging); rust/fspec-tui/src/app/dispatch_merge_worktree.rs:210-240 Success -> "[merge] success" then immediate teardown + BackToBoard (WT-009), NoChanges -> "[merge] nothing to merge", Conflict -> seeded conflict context, Err -> "[error] /merge-worktree: ..."; there is no press-Enter action prompt
- rust tests: rust/fspec-tui/tests/merge_worktree_rpc057.rs (summary dialog, NoChanges, Conflict, Err) (owner: rpc057-merge-worktree-dispatch); rust/fspec-tui/tests/wt009_isolation_toggle_merge_teardown.rs (success closes session, NoChanges keeps it open) (owner: isolation-only-opens-the-create-session-dialog-and-merge-worktree-never-closes-the-session-ux-contract-broken-in-the-rust-tui); rust/fspec-tui/tests/wt006_merge_worktree_ignored_only.rs (owner: merge-worktree-ignored-only-nothing-to-merge)
- covered: Successful merge shows file-by-file summary and action prompt (summary half, via MergeConfirmDialog), Nothing to merge keeps session open without action prompt, Non-conflict error keeps session open without action prompt, Merge conflicts show detailed file list and guidance (file list only, seeded into input)
- not covered: Escape in action prompt closes session same as Enter -> delete (WT-009 rule 3 closes the session right away on success), Character input is blocked during action prompt -> delete (WT-009), Action prompt is a generic reusable mechanism -> delete (WT-009 / Ink InputTransition), Action prompt guards against double invocation -> delete (WT-009)
- action: Merge into rpc057-merge-worktree-dispatch and the WT-009 feature; drop 4 as duplicates and delete the 4 action-prompt scenarios as superseded by WT-009 (immediate teardown + BackToBoard). Sibling: merge-conflict-llm-context (same mergeWorktreeHandler).
- question: -

#### fix-tui-068-code-smells-error-handling-console-statements-facade-completion
- class: retire
- area: none
- scenarios: 3 mapped to deleted files
- rust code: none found: sessionService.ts facade (getAttachedWorkUnit, attachToWorkUnit title default) is TS-only; Rust attaches via Action::AttachWorkUnitToSession with the board's WorkUnitContext (RPC-050)
- rust tests: none (closest: rust/fspec-tui/tests/work_unit_binding_rpc050.rs, owner: work-unit-attach-binding)
- covered: none
- not covered: sessionService exports getAttachedWorkUnit -> retire; attachToWorkUnit accepts optional title parameter -> retire; attachToWorkUnit defaults title to workUnitId -> retire
- action: @deprecated citing 103a73072 (TS client removed) and RPC-050 work-unit-attach-binding. It shares sessionService.test.ts with refactor-session-work-unit-state-management.

#### full-screen-tui-layout
- class: write
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/views/board/render.rs:60-80 (the board lays out over the whole frame area); rust/fspec-tui/src/terminal/mod.rs:46-102 (EnterAlternateScreen gives a clean screen)
- rust tests: rust/fspec-tui/tests/panic_hook.rs::terminal_guard_init_enables_alt_screen_raw_mode_mouse_and_bracketed_paste (owner: fspec-tui-terminal); nothing proves the board fills the terminal or follows a resize
- covered: Screen is cleared before rendering to eliminate artifacts (fspec-tui-terminal, alt screen)
- not covered: Board fills entire terminal in standard 80x24 terminal -> write; Board fills entire terminal in larger 120x40 terminal -> write; Board automatically resizes when terminal dimensions change -> write; BoardView uses useStdout hook for terminal dimension detection -> retire (Ink internals)
- action: Write rust/fspec-tui/tests/board_full_terminal_board013.rs (owned by this feature) for the 3 size scenarios (Rust uses the full height, not rows-1, so the steps need updating). Merge the clear-screen scenario into fspec-tui-terminal and delete the useStdout scenario. The related stretch-board feature (BOARD-014) gets its own file.

#### integrate-attachment-server-with-tui-boardview-lifecycle
- class: merge
- area: fspec-tui + attachment-viewer
- scenarios: 5 mapped to deleted files
- rust code: the attachment viewer is served by the daemon (rust/attachment-viewer); the TUI opens attachments through the daemon (RPC-374, BUG-431); the TUI owns no attachment server (grepped localhost/AttachmentServer in rust/fspec-tui/src)
- rust tests: rust/fspec-tui/tests/board_open_attachment_rpc374.rs (owner: rust-board-open-attachment); rust/attachment-viewer/tests/viewer_server.rs::render_mermaid_code_blocks_for_client_side_rendering, ::serve_a_binary_image_attachment_raw_with_the_correct_content_type (owner: rust-attachment-viewer-server)
- covered: Open attachment from TUI and browser shows HTTP URL; Markdown attachment renders with mermaid diagrams; Image attachment displays directly in browser
- not covered: Server stops when TUI exits -> delete; TUI continues working when server fails to start -> delete (the server lifecycle belongs to the daemon, not the TUI)
- action: Merge into rust-board-open-attachment and rust-attachment-viewer-server. Delete the 2 TUI-owned server lifecycle scenarios.

#### attachment-selection-dialog-with-keyboard-navigation
- class: merge
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/components/attachment_picker_dialog.rs:133-160 (Esc pops, Up/Down clamp, Enter opens and pops); render_dialog has no scroll viewport for row lists
- rust tests: rust/fspec-tui/tests/board_open_attachment_rpc374.rs::{pressing_a_on_a_card_with_attachments_opens_the_picker, pressing_a_on_a_card_with_no_attachments_is_a_silent_no_op, the_picker_lists_the_selected_work_units_attachments, selecting_an_attachment_opens_it_from_the_selected_workspace_on_the_daemon} (owner: rust-board-open-attachment); attachment_picker_dialog.rs::tests::down_clamps_at_the_last_row_and_up_clamps_at_the_first (owner: none)
- covered: Open dialog showing all attachments, Navigate and open selected attachment, No dialog when work unit has no attachments
- not covered: Close dialog with Esc key -> write (attachment_picker_dialog.rs:136), Scrollable list for many attachments -> delete (the picker does not scroll and has no scroll indicators)
- action: Merge into rust-board-open-attachment. Add an Esc test to rust/fspec-tui/tests/board_open_attachment_rpc374.rs. Delete the scroll scenario.

#### view-foundation-md-in-tui-with-attachment-viewer
- class: merge
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/views/board.rs D/d -> Action::OpenFoundation; rust/fspec-tui/src/app/dispatch_viewer.rs:19-52 reads spec/FOUNDATION.md from the daemon workspace and opens it in a viewer, with error notices "FOUNDATION.md could not be opened: ..."; the menu bar Foundation item is at rust/fspec-tui/src/components/menu_bar/items.rs:103
- rust tests: rust/fspec-tui/tests/board_open_foundation_rpc373.rs::pressing_uppercase_d_opens_the_foundation_document (owner: rust-board-open-foundation); rust/fspec-tui/tests/tui120_foundation_viewer_and_fspec_runner_follow_the_selected_workspace.rs::pressing_d_opens_the_selected_workspaces_foundation_from_the_daemon (owner: foundation-viewer-and-fspec-runner-follow-the-selected-workspace)
- covered: Press D key to open FOUNDATION.md in browser, Reuse attachment server infrastructure (daemon-served viewer)
- not covered: Error message when FOUNDATION.md does not exist -> write (into foundation-viewer-and-fspec-runner-follow-the-selected-workspace: missing file -> "could not be opened" notice), Keybinding help line displays D View FOUNDATION.md -> delete (MENU-001/MENU-008 menu bar replaced the help line; the board footer has no D hint), Diamond separator appears in correct position -> delete (same)
- action: Merge into rust-board-open-foundation / foundation-viewer-and-fspec-runner-follow-the-selected-workspace (+1 missing-file test in the tui120 file). Delete the 2 help-line scenarios.
- question: -

#### stretch-board-content-to-fill-available-viewport-height
- class: write
- area: fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-tui/src/views/board/render.rs:66-76 — the content row is Constraint::Min(0) between 18 fixed rows, so it fills the remaining height; :154 records last_viewport_height
- rust tests: none (no test asserts the content height at given terminal sizes)
- covered: none
- not covered: Columns fill vertical space in standard 80x24 terminal -> write; Columns fill vertical space in larger 120x40 terminal -> write; Column height adjusts when terminal is resized -> write
- action: Write rust/fspec-tui/tests/board_vertical_stretch_board014.rs (owned by this feature). The scenario row counts must change to the Rust layout (80x24 -> 6 content rows, 120x40 -> 22), not 20/36.

#### tui-hangs-when-foundation-json-is-missing
- class: write
- area: core / fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: rust/core/src/work_units.rs:166-174 (read_snapshot returns an empty snapshot when spec/work-units.json is missing); the board never reads foundation.json at startup
- rust tests: none for a missing spec dir or work-units.json (rust/core/tests/work_units_watcher_test.rs, owner work-units-watcher, only tests existing files). Esc exit: tests/boardview_esc_exit_confirmation_rpc102.rs (owner: boardview-esc-key-exit-confirmation)
- covered: none
- not covered: TUI loads with no spec directory -> write; TUI loads with spec dir but no foundation.json -> write; User can exit TUI with ESC -> delete (Rust Esc opens an exit confirmation first; that behaviour is owned by boardview-esc-key-exit-confirmation)
- action: write a new rust/fspec-tui/tests/bug072_board_loads_without_spec_dir.rs (empty board, responsive keys). Delete the ESC scenario as superseded by RPC-102. Possible overlap with graceful-initialization-without-spec-directory (INIT-016, also on deleted TS)

### Batch COV-073-B4

#### compaction-sessionstatechange-drops-sessionid-wrong-session-shows-compacting-indicator
- class: write
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_stream_chunks.rs:60 — `ChunkReceived(id, SessionStateChange{state})` writes status into the routed session's slot via `set_session_status` (rust/fspec-tui/src/store/agent_view/isolation_state.rs:69, keyed `HashMap<SessionId, SessionStatus>`); chips render `↻` for Compacting per session (rust/fspec-tui/src/components/menu_bar/chips.rs:98)
- rust tests: rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs::session_state_change_chunk_updates_per_session_status (owner: agentview-subscribe-broadcasts — only Running on one session, no background Compacting case)
- covered: none
- not covered: Background session auto-compacts while a different session is viewed -> write; Two sessions open and the background one triggers auto-compaction at its context limit -> write; Manual /compact on a non-active session -> delete (Rust /compact only targets the current session, rust/fspec-tui/src/app/dispatch_slash_commands.rs:66); SessionChunkHandler propagates routed sessionId -> write (reword steps: drop NAPI/SessionChunkHandler, use Action::ChunkReceived(s-B, Compacting) while s-A focused)
- action: write rust/fspec-tui/tests/compacting_status_routed_by_session_cmpct033.rs (dispatch Compacting for background s-B, assert session_status_for(s-B)=Compacting, s-A unchanged, chip on s-B only). Related: RPC-045 owns the general routing.

#### confirmation-dialog
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/confirm_dialog.rs:196-222 — y/Y returns Primary, n/N and Esc return Cancel; :134 the accent is always Accent::Yellow (no riskLevel); rust/fspec-tui/src/views/checkpoints/delete.rs:34-80 — typed DELETE ALL confirmation
- rust tests: rust/fspec-tui/tests/confirm_dialog_yn_shortcut_rpc164.rs::pressing_y_on_two_button_dialog_emits_primary (owner: rpc164-provider-settings-confirm-dialog-yn-shortcut); rust/fspec-tui/src/views/checkpoints/delete_tests.rs::confirming_the_typed_delete_all_dispatches_delete_all_checkpoints (owner: checkpoint-delete); rust/fspec-tui/tests/rpc027_dialog_parity_ef.rs::confirm_dialog_renders_with_yellow_accent_and_caller_supplied_title (owner: rpc027-model-confirm-dialogs)
- covered: Y/N mode calls onConfirm when Y pressed (rpc164); Typed mode requires exact phrase match (checkpoint-delete)
- not covered: Risk level maps to Dialog border color -> delete; No risk level defaults to neutral styling -> delete (Rust has no risk level; the ConfirmDialog accent is fixed yellow, and rpc027-model-confirm-dialogs owns that)
- action: Merge into rpc164-provider-settings-confirm-dialog-yn-shortcut (Y/N) and checkpoint-delete (typed phrase). Delete the 2 riskLevel scenarios. Sibling sensitive-path-prompts (same TS file) is also merged.

#### sensitive-path-prompts
- class: merge
- area: tools (blocklist) + fspec-tui (inline triple pause)
- scenarios: 6 mapped to deleted files
- rust code: rust/tools/src/blocklist/middleware.rs:291-350 — check_file_path pauses with PauseKind::Triple; AllowOnce -> Ok, AllowSession -> allow_for_session(pattern), Denied -> "User denied access"; :387-413 session allowances (allow/is_allowed/clear)
- rust tests: rust/tools/src/blocklist/middleware.rs::test_prompt_rule_allow_once_response, ::test_prompt_rule_allow_session_response, ::test_session_allowances_pattern_matching, ::test_prompt_rule_deny_response, ::test_session_allowances_cleared_on_restart, ::test_session_allowances_add_and_check (owner: integrate-blocklist-prompt-action-with-tool-pause-system)
- covered: Prompt for SSH config access - user allows once; ... - user allows for session; ... - user denies; Session allowances cleared on TUI restart (BLOCK-007 owns the same 5 behaviours)
- not covered: Prompt for environment file access -> delete; Prompt for fspec config access -> delete (Rust shows the rule's reason or "Sensitive file access", middleware.rs:328-331; no "AI wants to read ..." text; the .env prompt itself is already in BLOCK-007's scenarios)
- action: Merge into integrate-blocklist-prompt-action-with-tool-pause-system (BLOCK-007). Drop the 4 duplicates, delete the 2 message-text scenarios. middleware.rs tests still carry BLOCK-005 @step comments (lines 537-570); remove or re-point them. Sibling confirmation-dialog shares ConfirmationDialog.test.tsx and is also merged.

#### context-fill-percentage-realtime-recompute-restore
- class: write
- area: fspec-tui
- scenarios: 1 mapped to deleted files
- rust code: rust/fspec-tui/src/store/agent_view/token_state.rs:107-137 — the ContextFillUpdate arm caches `context_threshold_tokens` (non-finite or 0 threshold leaves it unset) and later TokenUpdates recompute against it; restored transcripts replay through the same reducer (no extractTokenStateFromChunks)
- rust tests: rust/fspec-tui/tests/token_state_realtime_recompute_rpc101.rs::cache_heavy_token_update_does_not_collapse_badge (owner: context-fill-percentage-realtime-recompute — live path only, no restore)
- covered: none
- not covered: Session restore seeds the threshold cache so live TokenUpdates immediately move the badge -> write (reword: drop extractTokenStateFromChunks / cachedContextThresholdRef)
- action: write rust/fspec-tui/tests/context_fill_restore_seeds_threshold_rpc101.rs: replay a restored transcript whose last ContextFillUpdate has threshold=100000, then one live TokenUpdate moves the badge; a fill without a threshold leaves it unset.

#### diff-view-for-write-edit-tool-output
- class: merge
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/store/agent_view/pending_tool_diff.rs + chunk_tool_result.rs:32 (capture_pending_diff) — Edit/Write diff cards
- rust tests: rust/fspec-tui/tests/edit_diff_rendering_rpc391.rs::edit_replacing_one_line_shows_old_on_red_and_new_on_green, ::write_of_a_new_three_line_file_shows_three_green_lines, ::edit_over_collapse_limit_shows_25_inline_while_modal_shows_full (owner: agentview-edit-diff-rendering); rust/fspec-tui/tests/diff_format_rpc390.rs::tree_connectors_prefix_the_first_line_and_indent_the_rest_while_empty_content_yields_empty (owner: agentview-edit-diff-generation)
- covered: Edit tool shows single line replacement with diff colors; Write tool shows new file content as additions; Diff output uses tree connector pattern
- not covered: Edit tool shows multi-line replacement with grouped changes -> write (into agentview-edit-diff-generation, diff_format_rpc390.rs); Long diff output is collapsed with expand indicator -> delete (RPC-391 shows 25 lines inline plus a full modal, not 4 lines + "ctrl+o to expand")
- action: Merge into agentview-edit-diff-rendering (RPC-391) and agentview-edit-diff-generation (RPC-390). Add one grouped-replacement test to rust/fspec-tui/tests/diff_format_rpc390.rs.

#### file-search-popup-uses-worktree-path-for-isolated-sessions
- class: human
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_slash_commands.rs:178 → rust/fspec-tui/src/app/workspace_scoped_calls.rs:47 — @file search uses the board's selected workspace (RPC-455), or the daemon cwd (rust/rpc/src/service_implementation/workspace_history.rs:89); it never uses the session's worktree path, even though the TUI holds it (`isolation_state_for(session).worktree_path`, rust/fspec-tui/src/store/agent_view/isolation_state.rs:80)
- rust tests: rust/fspec-tui/tests/rpc455_file_search_history_and_session_lists_follow_the_selected_workspace.rs (owner: file-search-history-and-session-lists-follow-the-selected-workspace) — workspace scoping only
- covered: File search in non-isolated session searches project root; File search before session creation uses project root fallback (both via RPC-455 workspace scoping)
- not covered: File search in isolated session finds files AI created in worktree -> write or retire; does NOT show files AI deleted -> write or retire; reflects worktree filesystem state -> write or retire; falls back to project root when sessionGetEffectiveCwd returns null -> write or retire
- action: if yes, route `handle_search_files` through the focused session's worktree_path when isolated and write rust/fspec-tui/tests/git033_file_search_isolated_worktree.rs; merge the 2 covered scenarios into RPC-455. If no, @deprecated citing RPC-455.
- question: In an isolated session, should @file search list the files in the session's worktree rather than the board workspace? (Recommended: yes. The Rust TUI already knows the worktree path. Add the routing plus a GIT-033 test. Otherwise @deprecate GIT-033 citing RPC-455.)

#### hitl-freeform-other-option-for-predefined-choices
- class: merge
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/store/agent_view/hitl_state.rs:52 (`other_active` sub-mode); rust/fspec-tui/src/views/agent/hitl_prompt.rs:35 (EMPTY_HINT "Please type a response or press Esc to go back")
- rust tests: rust/fspec-tui/tests/inline_hitl_prompt_rpc411/other_mode.rs::{enter_on_other_activates_freeform_mode_with_the_shared_input_empty_submit_hint_and_submit, Esc-in-Other-mode test}; rust/fspec-tui/tests/inline_hitl_prompt_rpc411/options_lifecycle.rs::answers_accumulate_across_questions_and_submit_together_after_the_last_question; rust/fspec-tui/tests/inline_hitl_prompt_rpc411/freeform_draft_paste.rs::freeform question test (owner of all: inline-hitl-prompt)
- covered: Select Other and submit freeform response; Escape from Other freeform returns to option list; Empty freeform submission is rejected
- not covered: Mixed Other and predefined across multi-question flow -> write in owner (accumulation test uses only predefined answers); Freeform-only question does not show Other -> write in owner (freeform test checks the hint but not that "Other..." is absent)
- action: merge into inline-hitl-prompt (rust/fspec-tui/tests/inline_hitl_prompt_rpc411/*). Add the 2 missing cases as owner scenarios and tests in other_mode.rs.

#### esc-interrupt-during-compaction
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_esc_cascade.rs — level 4: a Compacting or Running session interrupts; modal/popup levels come first; level 6: an idle session clears the input
- rust tests: rust/fspec-tui/tests/keyboard_cascade_rpc051.rs::esc_level_4_compacting_session_interrupts_without_navigating, ::esc_level_4_running_session_interrupts_without_navigating_back (owner: keyboard-shortcut-cascade-parity); rust/fspec-tui/tests/turn_content_modal_rpc382.rs::esc_closes_the_modal_but_stays_in_turn_selection_mode (owner: agentview-turn-content-modal); rust/fspec-tui/tests/agentview_esc_cascade_input_clear_rpc095.rs::esc_level_6_idle_session_with_nonempty_input_clears_buffer (owner: agentview-esc-cascade-input-clear)
- covered: all (Esc interrupts compaction when only compaction is active; Esc interrupts when both compaction and loading are active, where Rust has a single Compacting status; Esc closes modal before interrupting compaction; Esc clears input when neither is active)
- not covered: none
- action: Merge into keyboard-shortcut-cascade-parity, agentview-turn-content-modal and agentview-esc-cascade-input-clear. Drop all 4 as duplicates.

#### select-mode-enter-key-opens-full-turn-modal
- class: merge
- area: fspec-tui
- scenarios: 12 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/turn_modal.rs:1-37 (Enter in SELECT opens the modal titled by role; footer "↑↓ Scroll | Esc Close"); rust/fspec-tui/src/store/agent_view/chunk_wrap.rs:26 "(Enter to view full)" hint
- rust tests: rust/fspec-tui/tests/turn_content_modal_rpc382.rs::enter_on_the_selected_turn_opens_the_turn_content_modal, ::the_modal_shows_the_full_content_of_a_collapsed_turn, ::esc_closes_the_modal_but_stays_in_turn_selection_mode, ::a_second_esc_after_closing_the_modal_exits_turn_selection_mode (owner: agentview-turn-content-modal); rust/fspec-tui/tests/turn_content_modal_fullscreen_scroll_parity_rpc383.rs footer/scroll (owner: agentview-turn-content-modal-fullscreen-scroll); rust/fspec-tui/tests/turn_select_mode_rpc381.rs Esc exits select (owner: agentview-turn-select-mode); rust/fspec-tui/tests/agentview_esc_cascade_input_clear_rpc095.rs (owner: agentview-esc-cascade-input-clear); rust/fspec-tui/tests/tool_call_output_collapse_rpc389.rs (owner: tool-call-output-collapse)
- covered: Open truncated tool output in modal via Enter key, Close modal with Esc returns to select mode, Exit select mode with Esc when modal is not open, Esc behavior when not in select mode clears input, Modal displays navigation hints in footer, Collapsed output shows updated hint text, Esc key closes modal before disabling select mode, Open short message with no fullContent falls back to content
- not covered: Open user message in modal -> write (into agentview-turn-content-modal: user turn title), Modal displays role-specific title for <role> messages -> write (same new fn in turn_content_modal_rpc382.rs), Diff coloring preserved in modal for Edit tool output -> write (into agentview-turn-content-modal, or delete if the modal renders plain text), The /expand command is no longer recognized -> delete (there is no /expand anywhere in Rust; see expand-command retire)
- action: Merge into agentview-turn-content-modal (+1-2 tests in turn_content_modal_rpc382.rs); the other duplicates belong to the RPC-381/383/095/389 owners.
- question: -

#### loop-shorthand-natural-language-schedule-creation
- class: merge
- area: fspec-tui
- scenarios: 11 mapped to deleted files
- rust code: rust/fspec-tui/src/app/loop_parser.rs:76 parse_loop_command; rust/core/src/loops/mod.rs:181-245 (in-memory LoopStore, min 1s interval, idle-check skip)
- rust tests: rust/fspec-tui/tests/loop_dispatch_rpc059.rs (owner: rpc059-loop-dispatch); rust/core/tests/loop_task_test/*.rs (owner: per-entry-spawned-task-for-sub-minute-loop-intervals)
- covered: Create loop with leading interval token; Create loop with default interval when none specified; Create loop with trailing interval clause; Cancel an active loop by job ID; List all active loops; Show usage help when no arguments provided; Overlap policy defaults to skip (SCHED-013 "Skip firing when session is busy")
- not covered: Round sub-minute intervals up to one minute -> delete (superseded by SCHED-013: minimum interval is 1 second); Chain slash commands as loop prompts -> write in rpc059-loop-dispatch or delete; Session-scoped schedules are lost on restart -> delete (LoopStore is in-memory by construction; no test value); Loop schedules count toward session limit -> delete (no session limit in Rust loops: rust/core/src/loops/mod.rs)
- action: merge into rpc059-loop-dispatch (rust/fspec-tui/tests/loop_dispatch_rpc059.rs). The sub-minute scenario is superseded by SCHED-013.

#### model-selector-screen
- class: merge
- area: fspec-tui
- scenarios: 18 mapped to deleted files
- rust code: rust/fspec-tui/src/views/model_selector/dispatch.rs:33-80 (Esc close, / filter, arrows, a/e/d); rust/fspec-tui/src/views/model_selector/crud.rs:168-190 (filter mode: Esc clears and exits, Enter commits, Backspace, Char)
- rust tests: views/model_selector/tests_core.rs (owner: model-selector-view); tests_tab.rs (owner: model-selector-tab-to-providers); tests_enter_expand.rs (owner: model-selector-enter-key-behavior); tests_collapse.rs (owner: model-selector-default-expansion); tests_first_press_nav.rs (owner: model-selector-first-press-navigation); tests/model_selector_filter_row_model007.rs (owner: model-selector-filter)
- covered: Navigate down/up; Collapse with Left; Expand with Right; Close with Escape when no filter; Switch to provider settings with Tab; Select a model with Enter; Toggle section with Enter on header; Enter filter mode with slash; Type characters in filter mode; Delete characters with backspace; Exit filter mode with Enter (model007 commit_filter); Refresh with r; Auto-expand section containing current model
- not covered: Clear filter with Escape when filter is active -> write; Clear filter and exit filter mode with Escape -> write (both in tests/model_selector_filter_row_model007.rs, owner model-selector-filter; crud.rs:170-172 is untested); uses useModelSelectorState hook -> retire; ModelSelectorView is purely presentational -> retire
- action: merge into model-selector-view and its sibling owners. Add one filter-mode Esc test to model_selector_filter_row_model007.rs. Deprecate the 2 React-structure scenarios (103a73072)

#### custom-model-registration-and-facade-override-in-model-selector
- class: merge
- area: sessions
- scenarios: 19 mapped to deleted files
- rust code: rust/sessions/src/profile_sections/config.rs:20-21,99-113 (customModels with serde default, facade); rust/sessions/src/profile_sections.rs:95-129 (merge_profile_models: a custom model replaces a discovered model with the same id, is_custom=true); rust/sessions/src/profile_sections/custom_models.rs:22-48; model selector CRUD in rust/fspec-tui/src/views/model_selector
- rust tests: rust/fspec-tui/src/views/model_selector/tests_crud_add.rs + tests_crud_delete.rs (owner: model-selector-custom-model-crud); rust/sessions/src/profile_sections/persistence_tests.rs (owner: custom-model-persistence); rust/sessions/src/profile_sections/tests.rs::failed_probe_with_custom_models_is_not_unreachable (owner: model-selector-profile-server); rust/sessions/tests/rpc348_mid_session_facade_reresolution.rs (owner: mid-session-facade-reresolution); rust/fspec-tui/src/views/model_selector/rows_tests.rs ([C] badge; owner: model-selector-rows)
- covered: Add a custom model to a profile, Custom model persists in fspec-config.json, Custom model with all optional metadata fields (full_definition_round_trips), Custom models appear when /v1/models endpoint is unreachable, Facade override to Codex changes tool schemas / Facade override to Gemini changes tool schemas / No facade override uses default OpenAI tool schemas (RPC-348 facade resolution), Add custom model via 'a' keybind on profile section, Edit custom model via 'e' keybind, Delete custom model via 'd' keybind with confirmation, Add keybind is ignored on cloud provider sections, Cancel custom model form with Escape
- not covered: Existing config without customModels field loads normally -> write (serde default; persistence_tests.rs), Custom model overrides an auto-discovered model with matching ID -> write (merge_profile_models), Deleting a custom model that overrides an auto-discovered model -> write (merge_profile_models), Empty /v1/models with custom models shows profile section normally -> write (profile_sections/tests.rs), Facade override does not activate provider-specific features -> write (rpc348 file), Facade override propagates through NAPI boundary -> retire (NAPI removed, REFAC-027 f25b90b2e), Form field navigation with arrow keys -> write (tests_crud_add.rs)
- action: Merge, split by layer: config into custom-model-persistence (persistence_tests.rs), sections into model-selector-profile-server (profile_sections/tests.rs), facade into mid-session-facade-reresolution (rpc348 file), keys into model-selector-custom-model-crud (tests_crud_add.rs). Deprecate the NAPI scenario citing REFAC-027.

#### model-screen-stale-profile-sections-from-non-openai-providers-footer-text-unreachable-section-filtering
- class: merge
- area: sessions / fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/sessions/src/profile_sections/config_io.rs:50-53 (reads only providers.openai.profiles); rust/sessions/src/profile_sections.rs:141-155; rust/fspec-tui/src/views/model_selector/rows.rs:26-27 (FOOTER has no Tab hint, RPC-337 rule [12]); rust/fspec-tui/src/views/model_selector/header.rs:39-49 (unreachable profile header kept, with a marker)
- rust tests: rust/sessions/src/profile_sections/tests.rs (owner: model-selector-profile-server); views/model_selector/rows_tests_profile.rs (owner: model-selector-profile-rendering); tests/model_selector_title_two_span_model008.rs (owner: model-selector-title)
- covered: Reachable OpenAI profile with models appears (model-selector-profile-server); Header counts only model items (model-selector-title "(12 models)")
- not covered: Stale non-OpenAI provider profiles are never loaded -> write (in rust/sessions/src/profile_sections/tests.rs, owner model-selector-profile-server); Unreachable OpenAI profile is filtered -> retire (RPC-338: "An unreachable profile header renders a red marker and is never hidden"); Footer shows symmetric Tab hint -> retire (RPC-337 footer is "Enter Select | ←→ Expand/Collapse | / Filter | r Refresh | Esc Close")
- action: merge into model-selector-profile-server. Add one openai-only loader test there. Delete the 2 scenarios superseded by RPC-338 and RPC-337. Same deleted TS file as model-selector-screen (also merge)

#### multi-line-text-input-with-cursor-navigation-for-agentview
- class: write
- area: fspec-tui
- scenarios: 13 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/multiline_input_key.rs:81-84 — every key not handled above is passed to tui_textarea (cursor movement, Backspace merge, Alt+Backspace, Home, word moves, line wrap at edges)
- rust tests: rust/fspec-tui/tests/agent_input_multiline_newline_rpc402.rs (Shift+Enter mid-word split) (owner: agent-input-multiline-newline-keys); rust/fspec-tui/tests/agent_input_soft_wrap_rpc405.rs (6-row window scroll) (owner: agent-input-soft-wrap-auto-grow); rust/fspec-tui/tests/view_agent_multiline_input_rpc019.rs (Shift+Up -> HistoryPrev) (owner: rpc019-multiline-input)
- covered: Shift+Enter inserts newline and splits line; Scroll viewport when content exceeds 5 lines (Rust cap is 6); Shift+Up triggers history navigation callback
- not covered: Move cursor left within a line; Move cursor right and stop at end; Move cursor down to shorter line clamps column; Backspace at line start merges lines; Alt+Backspace deletes word before cursor; Insert character at cursor position; Home key moves cursor to line start; Alt+Right moves cursor to end of word; Right arrow at end of line wraps to next line; Left arrow at start of line wraps to previous line -> write (all 10)
- action: Write rust/fspec-tui/tests/multiline_input_cursor_editing_tui039.rs (owned by this feature) for the 10 editing scenarios. Check the Alt+Right word-end semantics against tui_textarea when writing. Drop the 3 duplicates into their owners.

#### input-state-not-restored-when-navigating-from-boardview-to-agentview
- class: merge
- area: fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_pending_input.rs (debounced PendingInputChanged save, hydration); AgentView is an always-on container (rust/fspec-tui/src/views/agent.rs:1)
- rust tests: tests/pending_input_durability_rpc052.rs (owner: pending-input-draft-persistence)
- covered: Input state is synced to Rust on every keypress (RPC-052 fires PendingInputChanged on each text change and debounces the save); Pending input preserved across Shift+Right/Left session switches (RPC-052 "Shift+Left/Right cycling uses SessionContext.input_draft")
- not covered: Pending input restored when returning from BoardView to AgentView -> write (in tests/pending_input_durability_rpc052.rs)
- action: merge into pending-input-draft-persistence (RPC-052). Add one Board-to-Agent round-trip test

#### input-not-restored-when-switching-between-sessions
- class: merge
- area: fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_pending_input.rs + SessionContext.input_draft (store per session); Shift+Left/Right restores the draft
- rust tests: rust/fspec-tui/tests/pending_input_durability_rpc052.rs::shift_arrow_cycling_uses_session_context_input_draft (owner: pending-input-draft-persistence)
- covered: Pending input is preserved when switching between sessions with Shift+Right/Left, Empty input is correctly set when switching to session with no pending input
- not covered: Different pending inputs are preserved for each session -> write (extend the owner test with a distinct s-2 draft "Goodbye")
- action: Merge into pending-input-draft-persistence (pending_input_durability_rpc052.rs). Drop 2 as duplicates and add a two-draft assertion for the third.
- question: -

### Batch COV-073-B5

#### anthropic-provider-settings-regression
- class: merge
- area: fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-tui/src/views/provider_settings/list.rs:124 — `d` on an oauth-status row opens the DisconnectOAuth confirm (list_actions::delete_on_nav_item); Enter on a login row starts OAuth (oauth_login.rs)
- rust tests: rust/fspec-tui/tests/prov112/view_keys.rs (owner: provider-settings-oauth-disconnect); rust/fspec-tui/tests/prov_099_anthropic_apikey_projection_test.rs (owner: anthropic-oauth-vs-api-key-classification)
- covered: Delete action on Claude OAuth provider disconnects OAuth; Claude provider with OAuth tokens shows connected status
- not covered: Edit action on Claude OAuth provider starts OAuth flow -> delete (no edit action; login is the Enter-on-login-row path, PROV-102/PROV-113)
- action: merge into provider-settings-oauth-disconnect (rust/fspec-tui/tests/prov112/). The status scenario duplicates anthropic-oauth-vs-api-key-classification. Same decision as sibling tui-anthropic-oauth-login.

#### codex-oauth-integration-bugs
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/views/provider_settings/list.rs:92 (Enter dispatched by nav-row kind; api-key row opens the key editor, login row starts OAuth), :124 (`d` on oauth-status row → disconnect)
- rust tests: rust/fspec-tui/tests/prov112/view_keys.rs + dispatch.rs (owner: provider-settings-oauth-disconnect, includes the codex clear); rust/fspec-tui/tests/prov_098_nav_tree_credential_status_test.rs (owner: provider-settings-nav-tree-credential-status: "OAuth-logged-in codex header matches the screenshot case"); rust/fspec-tui/tests/provider_settings_oauth_login_prov113.rs (provider-settings-oauth-login, codex login)
- covered: Codex provider with OAuth tokens shows connected status; Delete on Codex provider disconnects OAuth
- not covered: Edit key on Codex provider starts OAuth flow -> delete (no edit key, PROV-102/PROV-113); Edit key on non-OAuth provider shows API key editor -> delete (Enter on api-key row; owned by the provider-settings dispatch features)
- action: merge into provider-settings-oauth-disconnect (rust/fspec-tui/tests/prov112/). Status scenario duplicates provider-settings-nav-tree-credential-status.

#### provider-settings-expansion-state
- class: merge
- area: fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-tui/src/views/provider_settings/nav_tree_ops.rs:93-99 (the expanded set survives reload), nav_tree_ops.rs:146-169 (set_navigate_target/apply_pending_navigate, PROV-036 parity); rust/fspec-tui/src/app/oauth_disconnect.rs:9. API-key delete sets no navigate target (rust/fspec-tui/src/app/dispatch_provider_settings.rs:191).
- rust tests: rust/fspec-tui/tests/prov112/dispatch.rs (cursor returns to the provider row after disconnect) and rust/fspec-tui/tests/prov112/view_keys.rs (Esc cancel keeps the row) (owner: provider-settings-oauth-disconnect)
- covered: Expansion state preserved after OAuth disconnect confirmation, Cancel disconnect keeps cursor on logout row
- not covered: Expansion state preserved after API key deletion -> write (expansion survives reload; drop the "selected index points to the provider row" clause because Rust does not re-focus on API-key delete)
- action: Merge into provider-settings-oauth-disconnect. The API-key scenario needs a new test in rust/fspec-tui/tests/provider_dispatch_rpc054/credentials.rs (owner rpc054-provider-settings-dispatch).

#### provider-settings-oauth-guards
- class: merge
- area: fspec-tui
- scenarios: 28 mapped to deleted files
- rust code: rust/fspec-tui/src/views/provider_settings/nav_item.rs:80-160 (tree), footer_hints.rs:70-82, oauth_confirm.rs, list_actions.rs; rust/sessions profile CRUD (PROV-161)
- rust tests: provider_settings_canonical_order_rpc107.rs (provider-catalog-canonical-17-…); provider_settings_rich_tree_rpc349.rs (provider-settings-rich-tree); provider_settings_parity_prov161.rs (provider-settings-parity-and-automatic-model-limits); rust/sessions/tests/prov161_provider_settings_backend.rs (provider-settings-backend-parity); provider_settings_profile_crud_prov111.rs (provider-settings-profile-crud); provider_settings_oauth_login_prov113.rs (provider-settings-oauth-login); prov112/* (provider-settings-oauth-disconnect); provider_dispatch_rpc054/* (rpc054-provider-settings-dispatch); provider_settings_footer_hints_rpc106.rs (rpc106-provider-settings-footer-hints); provider_settings_arrow_expand_collapse_prov134.rs (provider-settings-arrow-expand-collapse); rpc154_summary_t_keybind_removal_shape.rs (provider-settings-detail-summary-t-keybind-removal-shape)
- covered: Provider list contains only providers with tool calling support (17 canonical), all 5 "Expanding …" scenarios (rich-tree + PROV-161), the 3 saveProfile scenarios (backend-parity), Enter on a provider row toggles expansion, Enter on a login item starts the OAuth flow, Enter on an API key item opens the key editor, Enter on a profile item opens the profile editor, Enter on create new profile starts profile creation, Pressing 'd' on an API key item shows delete confirmation, Declining API key delete confirmation preserves the key, Confirming API key delete removes the key, Pressing 'd' on an OAuth status item shows disconnect confirmation, Confirming OAuth disconnect clears tokens, Pressing 'd' on a profile item shows delete confirmation, Pressing 't' does nothing on any item, Footer updates based on selected item type, Tab hint says "Switch to models" on provider settings panel
- not covered: Pressing 'e' does nothing on any item -> write, Pressing 'n' does nothing on any item -> write (both in provider_settings_list_keybind_parity_rpc157.rs), PROVIDER_ENV_VARS includes codex entry -> delete (TS constant), Dead code files are removed -> delete (TS files), Dead types in provider.ts are cleaned up -> delete (TS types)
- action: Merge into the listed owners. The only new tests are 'e'/'n' no-op tests in rust/fspec-tui/tests/provider_settings_list_keybind_parity_rpc157.rs (owner rpc157-provider-settings-list-mode-keybind-parity).

#### github-copilot-tui-provider-integration
- class: merge
- area: fspec-tui
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-tui/src/views/provider_settings/projection.rs:56-60 (github-copilot has a single device-code login method); rust/fspec-tui/src/views/provider_settings/oauth_copilot.rs (deployment-type select, enterprise URL entry, normalisation); nav_item.rs:113 (Logout row)
- rust tests: rust/fspec-tui/tests/provider_settings_oauth_copilot_device_prov114.rs (owner: provider-settings-oauth-copilot-device); rust/fspec-tui/tests/prov112/dispatch.rs (github-copilot clear route; owner: provider-settings-oauth-disconnect); rust/fspec-tui/tests/provider_settings_canonical_order_rpc107.rs (owner: provider-catalog-canonical-17-provider-ordered-list-with-display-names)
- covered: GitHub Copilot appears in the TUI providers list after provider registration (rpc107), Starting login transitions the TUI into deployment-type selection mode, Selecting github.com launches device-code flow without prompting for URL, Selecting enterprise prompts for the enterprise URL before the device flow, Submitting a valid enterprise URL normalizes it and launches the device flow (prov114), Disconnecting OAuth deletes the credential file via the NAPI bridge (prov112 copilot route; NAPI wording stale)
- not covered: Expanding GitHub Copilot row reveals the device-flow login option only -> write (projection.rs:56; the real label is "Sign in with device code", not the fixture label), After successful authorization the GitHub Copilot row shows OAuth status -> write (Logout row appears after reload)
- action: Merge into provider-settings-oauth-copilot-device. Add the 2 tests to rust/fspec-tui/tests/provider_settings_oauth_copilot_device_prov114.rs.

#### oauth-tui-broken-flows
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/views/provider_settings/nav_item.rs:113-133 (OAuth status row plus login rows when expanded); rust/fspec-tui/src/views/provider_settings/oauth_login.rs:164 ('o' opens the URL while input is empty); oauth_login_render.rs:105 ("c: copy URL · o: open URL"); row_render.rs:18 (status text on the provider row)
- rust tests: rust/fspec-tui/tests/provider_settings_rich_tree_rpc349.rs (owner: provider-settings-rich-tree); rust/fspec-tui/tests/provider_settings_oauth_login_prov113.rs (owner: provider-settings-oauth-login); rust/fspec-tui/tests/prov_098_nav_tree_credential_status_test.rs (owner: provider-settings-nav-tree-credential-status)
- covered: Expanded OAuth provider with tokens shows status and re-login options (rich-tree "configured OAuth provider reveals logout and login rows"; the label is "Logout from OAuth [..]" since PROV-035), Provider status indicator remains visible when expanded (prov_098)
- not covered: Headless code input is width-constrained for long OAuth codes -> delete (Ink Box layout), Headless mode provides keybinds to copy and open the authorize URL -> write ('o' half and hint text; prov113 already covers 'c')
- action: Merge into provider-settings-oauth-login. Add the 'o' open-URL and hint test to rust/fspec-tui/tests/provider_settings_oauth_login_prov113.rs.

#### tui-oauth-login-flow
- class: merge
- area: fspec-tui
- scenarios: 11 mapped to deleted files
- rust code: rust/fspec-tui/src/views/provider_settings/oauth_login.rs:55-87 (Esc cancels the waiting modes, Enter retries on error, Esc back on error), :146; rust/providers/src/oauth/callback_server.rs:100-104 ("OAuth login timed out after N seconds.")
- rust tests: tests/provider_settings_oauth_login_prov113.rs (owner: provider-settings-oauth-login); tests/provider_settings_rich_tree_rpc349.rs (provider-settings-rich-tree); tests/prov_098_nav_tree_credential_status_test.rs (provider-settings-nav-tree-credential-status)
- covered: Codex shows OAuth login options when no tokens; Non-codex providers show no ChatGPT login; Successful browser login; Successful device login; Escape cancels device waiting; Retry after error
- not covered: Browser login times out after 5 minutes -> write; Escape cancels browser waiting -> write; Go back to provider list after OAuth error -> write (all 3 in tests/provider_settings_oauth_login_prov113.rs); Codex shows as configured when tokens exist -> delete the "no login options" step (RPC-349: a configured OAuth provider shows logout and login rows; the checkmark is covered by prov_098); NAPI codex OAuth bindings importable -> retire (NAPI removed, REFAC-027 f25b90b2e)
- action: merge into provider-settings-oauth-login (PROV-113). Add 3 key-path tests there. Deprecate the NAPI scenario citing REFAC-027

#### tui-anthropic-oauth-login
- class: merge
- area: fspec-tui
- scenarios: 15 mapped to deleted files
- rust code: rust/fspec-tui/src/views/provider_settings/oauth_login.rs (browser/headless rows), oauth_login_render.rs:89 (headless code entry), oauth_confirm.rs (disconnect); rust/providers/src/oauth/callback_server.rs:101 (login timeout); rust/providers/src/claude_headless_login.rs:126 (CSRF state mismatch); list.rs:92/124 (Enter / d dispatch by nav row, no edit key)
- rust tests: rust/fspec-tui/tests/provider_settings_oauth_login_prov113.rs (owner: provider-settings-oauth-login); rust/fspec-tui/tests/prov112/*.rs (provider-settings-oauth-disconnect); rust/fspec-tui/tests/prov_099_anthropic_apikey_projection_test.rs (anthropic-oauth-vs-api-key-classification); rust/fspec-tui/tests/prov_098_nav_tree_credential_status_test.rs (provider-settings-nav-tree-credential-status); rust/fspec-tui/tests/prov159_oauth_logout.rs (prov159-oauth-logout)
- covered: Anthropic provider shows OAuth login options when no tokens exist; Successful browser OAuth login flow; Successful headless OAuth login flow; Escape cancels browser OAuth waiting state; Escape cancels headless code entry state; Anthropic provider shows connected status when OAuth tokens exist; OAuth status takes precedence over API key status; Disconnect OAuth clears tokens and reverts status; Disconnect OAuth with existing API key reverts to API key status; Retry browser OAuth after error; Go back to provider list after OAuth error
- not covered: Non-OAuth providers do not show OAuth login options -> write in provider-settings-nav-tree-credential-status or delete; Browser OAuth login times out after 5 minutes -> write in provider-settings-oauth-login (callback_server.rs:101) or delete; Edit key on Anthropic provider with OAuth tokens starts OAuth flow -> delete (no edit key; login rows replaced it, PROV-102/PROV-113); Headless code entry with invalid CSRF state shows error -> write in provider-settings-oauth-login
- action: merge into provider-settings-oauth-login (primary, rust/fspec-tui/tests/provider_settings_oauth_login_prov113.rs). Disconnect scenarios go to provider-settings-oauth-disconnect, status scenarios to anthropic-oauth-vs-api-key-classification. Siblings PROV-027 and PROV-019 get the same treatment.

#### compaction-threshold-tui-config
- class: merge
- area: fspec-tui
- scenarios: 13 mapped to deleted files
- rust code: rust/fspec-tui/src/views/provider_settings/profile_form_parse.rs:33 profile_compaction_trigger (percentage 1..=100, tokens >=1000, else unset); rust/fspec-tui/src/views/model_selector/form.rs:225 parse_compaction_trigger; model_selector/form_fields.rs:68 "Compaction Trigger" field; rust/sessions/src/model_resolution.rs:231 + model_resolution/wire_limits.rs:29 (threshold override resolved in Rust, no NAPI)
- rust tests: rust/fspec-tui/tests/provider_settings_profile_compaction_threshold_prov115.rs::{below_minimum_percentage_is_unset, percentage_boundaries_one_and_hundred_are_accepted, above_maximum_percentage_is_omitted_but_profile_saves, below_minimum_token_count_is_unset, empty_and_non_numeric_input_remain_unset} (owner: provider-settings-profile-compaction-threshold); rust/fspec-tui/tests/provider_settings_profile_form_prov110.rs::compaction_threshold_percentage_is_parsed_on_save (provider-settings-profile-form)
- covered: Parse percentage compaction threshold input; Empty compaction threshold uses built-in default; Reject invalid percentage values; Reject token count below minimum threshold
- not covered: Parse token count compaction threshold input -> write in owner ("200000" → tokens); Provider Settings Panel includes compaction threshold field -> write in owner or delete; Custom Model Form includes compaction threshold field -> delete (owned by model-selector-custom-model-crud form); ModelSelection type includes compactionThreshold -> retire (TS type); 3x "Model selection service passes/omits compaction threshold to NAPI" -> retire (NAPI, REFAC-027 f25b90b2e); Profile compaction threshold flows through when model has none -> delete (TS/NAPI plumbing; Rust resolution owned by mid-session-model-reresolution); NAPI type declarations include compaction threshold parameters -> retire (REFAC-027)
- action: merge into provider-settings-profile-compaction-threshold (rust/fspec-tui/tests/provider_settings_profile_compaction_threshold_prov115.rs) with a new token-count test there. Delete the NAPI scenarios citing REFAC-027.

#### profile-form-arrow-key-navigation
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/views/provider_settings/profile_form_submit.rs:25-27 (Tab ignored, Down/Up move field); profile_form_render.rs:18 FORM_FOOTER "↑/↓: switch field · Enter: save · Esc: cancel" (used by footer_hints.rs:35)
- rust tests: rust/fspec-tui/tests/provider_settings_profile_form_prov110.rs::{down_arrow_from_name_editing_focuses_first_connection_field, up_arrow_on_first_field_re_enters_name_editing_in_create_mode, tab_is_ignored_and_leaves_focused_field_unchanged} (owner: provider-settings-profile-form)
- covered: Navigate to next field with Down arrow; Navigate to previous field with Up arrow; Tab key does not navigate form fields
- not covered: Footer shows arrow key navigation hints -> write in owner (no test asserts FORM_FOOTER)
- action: merge into provider-settings-profile-form (rust/fspec-tui/tests/provider_settings_profile_form_prov110.rs). Add one footer-hint test there.

#### provider-settings-oauth-logout
- class: merge
- area: fspec-tui
- scenarios: 7 mapped to deleted files
- rust code: rust/fspec-tui/src/views/provider_settings/projection.rs:99-103 ("Logout from OAuth [{name}]"), nav_item.rs:113-133, footer_hints.rs:82 ("Enter: logout"), oauth_confirm.rs:60 ("Disconnect OAuth for {provider_id}?")
- rust tests: provider_settings_rich_tree_rpc349.rs (owner: provider-settings-rich-tree); prov112/view_keys.rs (owner: provider-settings-oauth-disconnect); provider_settings_footer_hints_rpc106.rs (owner: rpc106-provider-settings-footer-hints); prov_099_anthropic_apikey_projection_test.rs (owner: anthropic-oauth-vs-api-key-classification)
- covered: Expanding Anthropic with OAuth shows logout line, Expanding Codex with OAuth shows logout line (prov112 per-provider Outline), Enter on logout line triggers disconnect confirmation, Pressing 'd' on logout line triggers disconnect confirmation, Footer shows Enter logout hint when logout line is selected, OAuth provider without tokens has no logout line (rich-tree uncredentialed OAuth)
- not covered: Non-OAuth provider has no logout line when expanded -> write (rust/fspec-tui/tests/provider_settings_rich_tree_rpc349.rs). Note: Rust labels say "[Anthropic]"/provider name and the prompt says "Disconnect OAuth for <id>?", not "[Claude]" / "Disconnect Claude OAuth? (y/n)"; reword on merge.
- action: Merge into provider-settings-rich-tree (tree rows) and provider-settings-oauth-disconnect (confirm). Add one negative test to rust/fspec-tui/tests/provider_settings_rich_tree_rpc349.rs.

#### role-banner-display
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/role_banner.rs:34-50 (Role: prefix, collapse, truncate); rust/fspec-tui/src/views/agent/pane_render.rs:60 (zero height without a role)
- rust tests: tests/role_banner_integration_rpc022.rs (owner: rpc022-role-banner); tests/slash_role_rpc063.rs (owner: role-slash-command-end-to-end-ui-dialog)
- covered: RoleBanner shows active role text; RoleBanner hidden when no role set; Long role text is truncated
- not covered: RoleBanner appears after setting role via /role dialog -> write (in tests/role_banner_integration_rpc022.rs, owner rpc022-role-banner)
- action: merge into rpc022-role-banner. Drop 3 duplicates; add one test that submits the role dialog and then renders the banner

#### role-dialog
- class: merge
- area: fspec-tui
- scenarios: 9 mapped to deleted files
- rust code: rust/fspec-tui/src/components/role_dialog.rs:147-152 — single-line TextArea: Enter saves, an empty draft or Ctrl+D clears, Esc cancels; :198 footer "Enter Save | Ctrl+D Clear | Esc Cancel"; :215 Accent::Cyan. No OK/Cancel button row and no Tab focus cycle.
- rust tests: rust/fspec-tui/tests/role_dialog_rpc063.rs::role_dialog_seeded_with_no_role_opens_with_empty_editable_buffer, ::role_dialog_seeded_with_existing_role_pre_fills_editable_buffer, ::enter_saves_non_empty_draft_and_removes_dialog_from_compositor, ::enter_on_empty_draft_clears_the_role, ::esc_cancels_the_dialog_without_emitting_an_action (owner: role-dialog-component); rust/fspec-tui/tests/slash_role_rpc063.rs (owner: role-slash-command-end-to-end-ui-dialog)
- covered: Set role via /role dialog on session with no existing role; Edit existing role via /role dialog; Clear role by submitting empty text area; Cancel role dialog with ESC (the button/Tab steps in these become Enter)
- not covered: Tab cycles focus between text area and buttons -> delete; Cancel button dismisses dialog without changes -> delete; Left/right arrows navigate between OK and Cancel buttons -> delete; Multi-line text editing in role dialog -> delete (RPC-063 redesigned the dialog as single-line with no buttons); /role requires an active session -> delete (Rust does nothing silently; role-slash-command-end-to-end-ui-dialog owns "Palette pick of /role with no active session is a silent no-op")
- action: Merge into role-dialog-component (RPC-063). Drop the 4 duplicates and delete the 5 button/multi-line/status-message scenarios as superseded by RPC-063. Sibling role-dialog-remove-button (same TS file) is retired for the same reason.

#### role-dialog-remove-button
- class: retire
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/components/role_dialog.rs:147 — Ctrl+D clears the role; there is no Remove button and no button row
- rust tests: rust/fspec-tui/tests/role_dialog_rpc063.rs::ctrl_d_clears_the_role_and_removes_dialog (owner: role-dialog-component)
- covered: none (the "Pressing Remove clears the role" outcome is already proved through Ctrl+D by role-dialog-component)
- not covered: all 5 -> retire
- action: @deprecated citing RPC-063 (role-dialog-component replaced the OK/Remove/Cancel buttons with the Enter Save / Ctrl+D Clear / Esc Cancel keys). Consistent with sibling role-dialog.

#### dialog
- class: merge
- area: fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-tui/src/components/dialog_theme.rs — render_dialog/dialog_rect (centred, rounded border in the accent colour); rust/fspec-tui/src/compositor.rs — a Critical/Foreground modal takes input first
- rust tests: rust/fspec-tui/tests/rpc027_dialog_theme.rs::dialog_rect_centers_the_dialog_within_the_available_area (+ render_dialog border-accent test) (owner: rpc027-dialog-theme); rust/fspec-tui/src/compositor_tests.rs::critical_priority_modal_pushed_on_top_intercepts_subsequent_keystrokes (owner: fspec-tui-compositor); rust/fspec-tui/tests/rpc027_dialog_parity_ef.rs::confirm_dialog_esc_returns_cancel_from_any_focused_index (owner: rpc027-model-confirm-dialogs)
- covered: all (Render centered modal with custom border color -> rpc027-dialog-theme; Dialog captures all input when active -> fspec-tui-compositor; Handle ESC key to call onClose -> rpc027-model-confirm-dialogs)
- not covered: none
- action: Merge into rpc027-dialog-theme, fspec-tui-compositor and rpc027-model-confirm-dialogs. Drop all 3 as duplicates.

#### session-header-realtime-status
- class: merge
- area: fspec-tui
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/header_build.rs:33-36 ("(ID: status)" / "(ID)"); the WorkUnitsLoaded status sync (BUG-180)
- rust tests: tests/bug180_wu_status_sync.rs (owner: bug-180-sessionheader-work-unit-status-sync); tests/rpc029_agent_header_alignment.rs (owner: rpc029-agent-header-alignment)
- covered: Status change via fspec command updates header in realtime (BUG-180 "SessionHeader repaints with the fresh status after WorkUnitsLoaded"); Opening AgentView shows work unit info (rpc029 "Header inserts work-unit prefix…")
- not covered: Header displays work unit ID without status -> write (in tests/rpc029_agent_header_alignment.rs; header_build.rs:35 is untested); SessionHeader subscribes to sessionStore -> retire; sessionStore provides fields -> retire; AgentView syncs snapshot to sessionStore -> retire; BoardView singleton file watcher -> retire; AgentView does NOT create its own watcher -> retire (all five are zustand/React hook wiring)
- action: merge into bug-180-sessionheader-work-unit-status-sync and rpc029-agent-header-alignment. Add one "(LEGACY-001)" test. Deprecate the 5 TS-store scenarios citing 103a73072

#### session-header-isolation-badge
- class: merge
- area: fspec-tui
- scenarios: 2 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/header.rs:11 ([ISOLATED] badge); store/agent_view/isolation_state.rs
- rust tests: tests/bug178_isolated_badge_chrome_paint.rs (owner: sessionheader-isolated-badge-chrome-paint-wiring)
- covered: all
- not covered: none
- action: merge into sessionheader-isolated-badge-chrome-paint-wiring (BUG-178) and drop both scenarios. Same deleted TS file as session-header-realtime-status (also merge)

### Batch COV-073-B6

#### schedule-tui-slash-commands
- class: merge
- area: fspec-tui
- scenarios: 12 mapped to deleted files
- rust code: rust/fspec-tui/src/app/schedule_parser.rs:92 parse_schedule_command; rust/fspec-tui/src/app/dispatch_slash_schedule.rs:60-104 (add/list/pause/resume/remove via backend, error notice on failure)
- rust tests: rust/fspec-tui/tests/schedule_dispatch_rpc058.rs (owner: rpc058-schedule-dispatch); rust/fspec-core/tests/add_schedule.rs::{scenario_cron_with_fewer_than_five_fields_is_rejected_without_writing, scenario_duplicate_schedule_name_is_rejected_and_existing_entry_preserved, scenario_agent_schedule_missing_role_and_prompt_is_rejected, scenario_invalid_timezone_is_rejected_without_writing} (owner: add-schedule-rust-port); rust/fspec-core/tests/remove_schedule.rs::scenario_removing_nonexistent_schedule_errors_and_leaves_file_unchanged (owner: remove-schedule-rust-port)
- covered: Add an agent-type schedule; Add a shell-type schedule; List all schedules; Pause; Resume; Remove; Show usage help when no subcommand provided; Reject invalid cron expression / duplicate name / nonexistent removal / missing fields / invalid timezone (validation in the rust-port twins, TUI error-notice path in rpc058 "add error emits an error notice")
- not covered: none
- action: merge into rpc058-schedule-dispatch (rust/fspec-tui/tests/schedule_dispatch_rpc058.rs). Drop all 12 as duplicates. The validation rules stay with add-schedule-rust-port and remove-schedule-rust-port.

#### toggle-line-selection-mode-with-select-command
- class: merge
- area: fspec-tui
- scenarios: 8 mapped to deleted files
- rust code: there is no /select command and no line-selection mode (grepped slash_commands.rs, slash_parser.rs). SELECT is the Tab-toggled turn mode (rust/fspec-tui/src/views/agent/scrollback_select.rs, RPC-381). Scrolling and stick-to-bottom are in scrollback.rs.
- rust tests: rust/fspec-tui/tests/scrollback_scroll_rpc094.rs::rpc094_up_arrow_with_empty_input_scrolls_scrollback_one_line, ::rpc094_scroll_down_to_tail_re_engages_stick (owner: rpc094-agentview-scrollback-scroll); rust/fspec-tui/tests/view_agent_scrollback_rpc019.rs (PageUp disables stick) (owner: rpc019-scrollback); rust/fspec-tui/tests/turn_select_mode_rpc381.rs (owner: agentview-turn-select-mode)
- covered: Arrow keys scroll viewport in scroll mode; Sticky scroll when user scrolls away in scroll mode; Re-enable auto-scroll when scrolling back to bottom in scroll mode; AgentView starts in scroll mode by default
- not covered: Enable line selection mode with /select command -> delete; Disable line selection mode with /select command -> delete; Arrow keys select lines in line selection mode -> delete; Auto-scroll to new messages in line selection mode -> delete (line selection was replaced by turn selection in TUI-042 and then by Tab in RPC-381)
- action: Merge the 4 scroll-mode scenarios into rpc094-agentview-scrollback-scroll / rpc019-scrollback. Delete the 4 line-selection scenarios, citing TUI-042 / RPC-381.

#### attach-to-detached-sessions-from-resume-view
- class: merge
- area: fspec-tui + sessions
- scenarios: 7 mapped to deleted files
- rust code: rust/sessions/src/session_manager/manager_impl_2.rs:20-62 (merges in-memory and persisted sessions, sorted by updated_at_ms descending); rust/fspec-tui/src/app/dispatch_attach.rs:26 (attach hydrates the transcript); rust/fspec-tui/src/views/agent/resume_session_view.rs:225 (D deletes); no 🔄/💾 status icons (grepped resume_session_view*.rs)
- rust tests: rust/fspec-tui/tests/slash_resume_rpc049.rs (owner: slash-command-resume); rust/fspec-tui/tests/rpc026_resume_session_view.rs (owner: rpc026-resume-session-view); rust/sessions/tests/tui099_session_list_ordering.rs (owner: resume-session-ordering)
- covered: Attach to running session shows buffered output then live stream; Idle session loads from persistence as before; Attach to idle background session shows complete output (slash-command-resume); Delete running session destroys background session (rpc026-resume-session-view); Sessions sorted by most recent first (resume-session-ordering)
- not covered: Resume view shows running and idle sessions with status icons -> delete; Backward compatible when no background sessions exist -> delete (Rust shows name and detail rows with no icons; resume-view-rich-session-display owns the row format)
- action: Merge into slash-command-resume, rpc026-resume-session-view and resume-session-ordering. Delete the 2 icon scenarios.

#### interactive-kanban-board-cli
- class: merge
- area: fspec-tui
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-tui/src/store/board.rs (7 columns, focus next/prev); rust/fspec-tui/src/views/board/columns.rs:38 (header is the uppercase status only, no count or points); viewport.rs:152 (cell "ID [pts]", coloured by type, no emoji icons); footer.rs:30 (UnifiedBoardLayout footer); Enter opens the agent/details chooser, not a detail view
- rust tests: rust/fspec-tui/tests/store_board_unit_rpc012.rs::focus_next_and_prev_wrap_at_boundaries, ::boardstore_seeds_work_units_grouped_into_7_columns_with_focus_at_backlog (owner: rpc012-board-store); store_board_viewport_rpc016.rs (rpc016-board-store-viewport); view_board_unit_rpc013.rs (rpc013-board-footer); view_board_unit_rpc014.rs (rpc014-board-grid)
- covered: Navigate to next column with right arrow; Navigate down within column; Load board data on mount; Display footer with keyboard shortcuts (Rust footer string differs; rpc013-board-footer owns it)
- not covered: Open detail view with Enter key -> delete (Enter opens the chooser; details live in the always-visible details strip, RPC-014/BOARD-024); Display column header with count and points -> delete; Display work unit card with type icon and estimate -> delete (RPC-014 uses type colour + [estimate]); Display empty column message -> delete (none found: "No work units" in rust/fspec-tui/src)
- action: Merge into rpc012-board-store, rpc016-board-store-viewport and rpc013-board-footer. The column-nav scenario needs @step text on store_board_unit_rpc012.rs::focus_next_and_prev_wrap_at_boundaries. Delete the 4 layout scenarios (the TS layout was itself superseded by UnifiedBoardLayout).

#### tui-board-column-scrolling
- class: merge
- area: fspec-tui
- scenarios: 2 mapped to deleted files
- rust code: rust/fspec-tui/src/store/board_viewport.rs:170-185 — the visible-row calculation subtracts the up/down arrow rows; move_selection scrolls the focused column
- rust tests: rust/fspec-tui/tests/store_board_viewport_rpc016.rs::move_selection_beyond_bottom_of_viewport_scrolls_focused_column_down (owner: rpc016-board-store-viewport); rust/fspec-tui/tests/view_board_unit_rpc016.rs::column_with_mid_range_scroll_renders_both_up_and_down_arrows (owner: rpc016-board-viewport)
- covered: Automatic scroll when navigating beyond visible viewport; Account for scroll indicators when calculating visible items
- not covered: none (of the 2 deleted-mapped scenarios). The other 5 scenarios have no mappings at all; rpc016 owners already cover scroll up, arrows, wrap-around and per-column offsets.
- action: Merge into rpc016-board-store-viewport and rpc016-board-viewport (RPC-016). Drop the 2 duplicates; the 5 unmapped scenarios merge into the same owners.

#### fix-tui-kanban-column-layout-to-match-table-style
- class: merge
- area: fspec-tui
- scenarios: 10 mapped to deleted files
- rust code: rust/fspec-tui/src/views/board/grid.rs + borders.rs (box-drawing table, width split); render.rs:66-76 layout; columns.rs:38 (header is the status only); viewport.rs:152 (no emoji)
- rust tests: rust/fspec-tui/tests/view_board_unit_rpc014.rs (box-drawing borders, focused header cyan, focused selected cell) (owner: rpc014-board-grid); grid_unit_rpc014.rs (column widths, build_border_row) (owner: rpc014-grid-helpers); view_board_unit_rpc016.rs (PageDown/PageUp, arrows) (owner: rpc016-board-viewport); store_board_unit_rpc012.rs::focus_next_and_prev_wrap_at_boundaries (owner: rpc012-board-store)
- covered: Display unified table layout with box-drawing characters (no Changed Files row, per TUI-014); Navigate between columns with arrow keys; Navigate within column with arrow down (the Rust highlight is bg=green); Scroll by page with Page Down key; Scroll indicator when scrolled past start; Adapt column width to terminal size; Table borders stay aligned after terminal resize
- not covered: No action when Page Up pressed at top -> write (into rpc016-board-viewport, view_board_unit_rpc016.rs); Display work unit with type icon and priority -> delete; Column header shows status, count, and points -> delete (RPC-014/RPC-016 layout)
- action: Merge into rpc014-board-grid, rpc014-grid-helpers, rpc016-board-viewport and rpc012-board-store. Add one PageUp-at-top test to view_board_unit_rpc016.rs and delete the 2 emoji/count scenarios. The 2 Ink-only scenarios (useStdout, useMemo) are not deleted-mapped but should be retired.

#### display-attachments-in-work-unit-details-panel
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/views/board/details_strip.rs:241-270 (build_attachments_line: "Attachments: none" when empty, else `Attachments (use the "A" key to view): ` + comma-joined basenames, truncated with "…")
- rust tests: rust/fspec-tui/tests/view_board_unit_rpc014.rs::attachments_row_renders_comma_joined_basenames_with_a_key_hint (owner: rpc014-board-grid)
- covered: Display single attachment filename, Display multiple attachment filenames (owner checks "notes.md, ref.md")
- not covered: Display 'No attachments' when work unit has no attachments -> write (Rust and final TS both paint "Attachments: none"; reword the step), Truncate attachment list when too many to fit -> delete (Rust truncates with "…" and shows no "...N more" count; the TS overflow suffix was not ported)
- action: Merge into rpc014-board-grid. Add an empty-attachments render test to rust/fspec-tui/tests/view_board_unit_rpc014.rs. Delete the "...N more" scenario.

#### replace-shimmer-with-fast-forward-emojis-for-active-work-unit
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/views/board/viewport.rs:157 (`⏩ {prefix}{id}{points} ⏩` for the last-changed unit); rust/fspec-tui/src/store/board.rs:252
- rust tests: rust/fspec-tui/tests/view_board_unit_rpc016.rs::most_recently_changed_work_unit_renders_the_marker_prefix_and_suffix (owner: rpc016-board-viewport)
- covered: Display fast-forward emojis around active work unit, Display non-active work unit without emojis (the owner asserts AUTH-001 is not marked)
- not covered: Emojis move when active work unit changes -> write (render twice with a newer timestamp on the other unit), Shimmer animation is removed from active work unit -> delete (Ink animation internals; ratatui never had a shimmer)
- action: Merge into rpc016-board-viewport. Add the "marker moves" test to rust/fspec-tui/tests/view_board_unit_rpc016.rs.

#### purple-watcher-input-display
- class: merge
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: parse_supervisor_envelope in rust/fspec-tui/src/store/agent_view/session_context.rs; IncomingMessage renders "[W] <role>> <body>" in magenta
- rust tests: rust/fspec-tui/tests/supervisor_message_rendering_rpc387.rs::incoming_message_space_separated_backend_envelope_renders_magenta_bracket_w_line, ::parse_newline_separated_supervisor_envelope_yields_role_and_body, ::parse_supervisor_envelope_without_header_falls_back_to_supervisor_role (owner: supervisor-message-rendering)
- covered: Display watcher input in magenta color, Process WatcherInput chunk to conversation message (as IncomingMessage), Parse multiline watcher message (newline-separated body, mostly), Parse regular message without watcher prefix (Rust falls back to role "supervisor" instead of returning null)
- not covered: Parse watcher prefix with supervisor authority -> delete (WATCH-024 removed Authority; the prefix is now [SUPERVISOR: role | Session: id]), Parse watcher prefix with peer authority -> delete (WATCH-024)
- action: Merge into supervisor-message-rendering (supervisor_message_rendering_rpc387.rs). Drop the 4 duplicates and delete the 2 Authority scenarios as superseded by WATCH-024.
- question: -

#### thinking-level-dialog
- class: merge
- area: fspec-tui
- scenarios: 9 of 14 mapped to deleted files (the other 5 have no mapping at all)
- rust code: rust/fspec-tui/src/components/thinking_level_dialog.rs:176-183 (wrap-around navigation), :210-227 (keys); rust/fspec-tui/src/views/agent/header_build.rs:93-97, 186-192 ([T:<level>], hidden for Off); rust/fspec-tui/src/app/dispatch_slash_commands.rs:63 (no session: silent no-op)
- rust tests: tests/thinking_level_dialog_rpc022.rs (owner: rpc022-thinking-level-dialog, includes arrow_keys_navigate_the_5_levels_with_wrap_around); tests/app_dispatch_rpc022.rs (rpc022-app-dispatch); tests/slash_thinking_rpc048.rs (slash-command-thinking-inline); tests/view_agent_unit_rpc018.rs (rpc018-agent-chrome, [T:] badge present/absent); tests/model011_thinking_dialog_offers_only_the_models_levels.rs (thinking-dialog-offers-only-the-models-levels)
- covered: Open dialog with slash command; Navigate and select; Navigation wraps; Cancel with Escape; Base level persists and applies to every request (MODEL-011); SessionHeader shows base level when idle; Dialog opens with current level highlighted; Set level directly with argument (unmapped); Invalid argument shows error (unmapped; RPC-048 gibberish, different text); SessionHeader hides badge when Off (unmapped)
- not covered: Slash command palette closes when thinking dialog opens -> write (in tests/thinking_level_dialog_rpc022.rs); Dialog displays labels with token budgets -> retire (MODEL-011: options come from model capabilities; no "~4K tokens" text); Arguments are case insensitive (unmapped) -> delete (MODEL-011 replaced the fixed off/low/med/high vocabulary); Command requires active session (unmapped) -> delete (Rust is a silent no-op, dispatch_slash_commands.rs:63)
- action: merge into rpc022-thinking-level-dialog (with RPC-048, RPC-018, MODEL-011 owners). Add one palette-pick test. Delete or deprecate the 3 scenarios that differ, citing MODEL-011

#### session-work-unit-ipc
- class: human
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: none found: grepped `work-unit-changed`, `WorkUnitChanged`, auto-attach on update-work-unit-status in rust/tools, rust/agent-loop, rust/sessions, rust/fspec-tui. Rust binds a card only through Action::AttachWorkUnitToSession (rust/fspec-tui/src/app/dispatch_work_unit_binding.rs:34) and when a session is created from a card (dispatch_focus.rs:6). Each tool call now carries an explicit work_unit_id attribution (rust/tools/src/tool_admission.rs:20).
- rust tests: none
- covered: none
- not covered: all 5 -> retire if the answer is "retire", else write
- action: Recommended: @deprecated citing 103a73072 (the TS IPC socket is gone) and the explicit attach model (RPC-050 work-unit-attach-binding). If kept: write rust/fspec-tui/tests/tui060_status_change_reattach.rs.
- question: When the agent runs `update-work-unit-status` on a different card, should the session automatically re-attach to that card? Recommended: (a) No, retire the feature. Rust binds cards explicitly (board chooser / AttachWorkUnitToSession) and attributes each tool call's work_unit_id. (b) Yes, port it as new Rust behaviour plus a test.

#### board-view-watcher-update
- class: write
- area: fspec-tui
- scenarios: 10 mapped to deleted files
- rust code: rust/core/src/work_units.rs:182 read_snapshot emits records in `states[<column>]` order and the watcher pushes full snapshots; rust/fspec-tui/src/app/dispatch.rs:62 WorkUnitsLoaded → handle_start_work_units_loaded (rust/fspec-tui/src/app/dispatch_workspaces.rs:41) regroups the board; last-changed indicator in rust/fspec-tui/src/store/board.rs
- rust tests: rust/core/src/work_units.rs::{read_snapshot_preserves_states_array_order_per_column, read_snapshot_appends_orphan_records_sorted_by_id} (owner: none — unowned, no @step); rust/core/tests/work_units_watcher_test.rs (work-units-watcher); rust/fspec-tui/tests/bug180_wu_status_sync.rs (bug-180-sessionheader-work-unit-status-sync); rust/fspec-tui/tests/store_board_unit_rpc012.rs (rpc012-board-store, regrouping)
- covered: Session header status syncs from store data after watcher reload (dup of BUG-180 "WorkUnitsLoaded syncs per-session WorkUnitContext status")
- not covered: Work unit status change preserves correct column priority order -> write; External priority reordering is reflected on the board -> write; Last-changed indicator updates when status changes externally -> write; Attachment added externally appears in details panel -> write; Deleted work unit disappears from the board -> write; New work unit created externally appears on the board -> write; globalStreamListener calls loadData on WorkUnitsUpdate event -> retire (TS internals); Watcher event chunk data is not used for store updates -> delete (Rust deliberately uses the pushed snapshot); Session context cleared when attached work unit is deleted externally -> delete (contradicted by BUG-180 "WorkUnitsLoaded does not destroy a binding whose unit was deleted")
- action: write rust/fspec-tui/tests/tui079_board_reflects_watcher_snapshots.rs (dispatch successive WorkUnitsLoaded snapshots, assert column order, last-changed, attachments, add/delete). The unowned order tests in rust/core/src/work_units.rs could also be remapped here for the 2 ordering scenarios (they need @step comments).

#### select-turns-instead-of-lines-with-select-command
- class: merge
- area: fspec-tui
- scenarios: 9 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/scrollback_select.rs:46-80 — Tab enters SELECT and selects the last turn; Up/Down clamp; navigate_turn re-anchors the scroll offset; one turn = one RenderedChunk, so a tool card is its own turn
- rust tests: rust/fspec-tui/tests/turn_select_mode_rpc381.rs::tab_enters_turn_selection_mode_and_selects_last_turn, ::pressing_tab_again_exits_turn_selection_mode, ::up_arrow_selects_the_previous_turn, ::up_arrow_on_first_turn_clamps_the_selection, ::down_arrow_on_last_turn_clamps_the_selection (owner: agentview-turn-select-mode)
- covered: Enable turn selection mode; Navigate between turns with arrow keys; Disable turn selection mode; Navigation stays at first turn when pressing up; Navigation stays at last turn when pressing down; AgentView starts in scroll mode by default (all via Tab, not /select)
- not covered: Multi-line turn highlighting -> write; Viewport scrolls to show selected turn -> write; Tool messages are selectable as separate turns -> write
- action: Merge into agentview-turn-select-mode (RPC-381). Add 3 tests to rust/fspec-tui/tests/turn_select_mode_rpc381.rs. The "/select" steps become Tab.

#### slash-command-autocomplete-palette
- class: merge
- area: fspec-tui
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/popups.rs:37-60 (classify_buffer: a leading '/' opens the palette, an empty buffer closes it); slash_command_popup.rs; slash_command_popup_rows.rs:24-28 ("(no matching commands)"); slash_commands.rs (registry)
- rust tests: rust/fspec-tui/tests/view_agent_popups_rpc020.rs (owner: rpc020-slash-and-file-popups); rust/fspec-tui/src/views/agent/popups.rs::tests::empty_buffer_closes_popups (owner: none)
- covered: Show palette when typing slash at start of input, Filter commands by prefix matching (TS-era names mode/mcp were removed by TUI-077; reword the list), Accept selected command with Tab key, Close palette with Escape key, Execute command immediately with Enter key (/clear)
- not covered: Navigate through command list with arrow keys -> write, Show no matching commands message -> write (slash_command_popup_rows.rs:28), Close palette when deleting slash character -> write (or map popups.rs::empty_buffer_closes_popups to the owner)
- action: Merge into rpc020-slash-and-file-popups. Add the 3 tests to rust/fspec-tui/tests/view_agent_popups_rpc020.rs.

#### file-search-popup-modal-for-symbol-input
- class: merge
- area: fspec-tui
- scenarios: 7 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/file_search_popup.rs (opens at the '@' anchor); file_search_popup_keys.rs:22-53 (Esc, Up/Down, PgUp/PgDn, Home/End, Enter/Tab splice); rust/core/src/file_search.rs:36-40 `**/*<prefix>*` case-insensitive glob, mtime-sorted; empty prefix is skipped
- rust tests: rust/fspec-tui/tests/view_agent_popups_rpc020.rs::typing_at_opens_file_search_popup_at_anchor, ::pressing_enter_on_file_match_splices_path_with_trailing_space, ::pressing_esc_on_file_search_popup_dismisses_without_modifying_input (owner: rpc020-slash-and-file-popups); rust/fspec-tui/tests/rpc027_dialog_parity_gh.rs no-match/empty-state (owner: rpc027-slash-file-popups); rust/core/src/file_search.rs inline tests (owner: none)
- covered: File selection and @filepath insertion, No results found with escape handling (no-match text via rpc027, Esc via rpc020), Immediate popup trigger with real-time file filtering (trigger only)
- not covered: Keyboard navigation identical to slash command palette -> write (Up/Down highlight into rpc020-slash-and-file-popups, view_agent_popups_rpc020.rs), Backend file search integration using Glob tool -> write (into rpc020-slash-and-file-popups or rpc020-source-shape: test rust/core/src/file_search.rs `**/*but*` pattern), Fuzzy file matching with partial queries -> delete (Rust matches a substring glob, not non-contiguous fuzzy), Center screen popup appearance like slash command palette -> delete (the Rust popup sits at the '@' anchor), "shows recent files on bare @" step -> delete (empty prefix is skipped)
- action: Merge into rpc020-slash-and-file-popups (view_agent_popups_rpc020.rs). The no-match rendering is already owned by rpc027-slash-file-popups.
- question: -

#### remove-file-watching-from-tui-main-screen-and-lazy-load-changed-files-view
- class: merge
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/views/board/header.rs + checkpoint_status.rs (header shows only Checkpoints Manual/Auto; no changed-files counter); F opens the Changed Files view (views/changed_files); live checkpoint-count push (BUG-181)
- rust tests: rust/fspec-tui/tests/view_board_unit_rpc015.rs (owner: rpc015-board-header); rust/fspec-tui/tests/file_changes_view_rpc354.rs (owner: rust-file-changes-view); rust/fspec-tui/tests/bug181_live_checkpoint_counts.rs (owner: live-checkpoint-counts-in-the-board-header)
- covered: Main board header shows checkpoints but not changed files counter; F key opens changed files viewer with lazy-loaded git status; Main board does not auto-update when files are staged externally (the board has no counter; F loads fresh); Checkpoint stash watcher remains active and updates header
- not covered: Remove changed files watching tests but keep checkpoint tests -> delete (describes TS test-file housekeeping)
- action: Merge into rpc015-board-header, rust-file-changes-view and live-checkpoint-counts-in-the-board-header. Delete the TS test-housekeeping scenario.

### Batch COV-073-B7

#### duplicate-installation-success-message-in-fspec-init
- class: merge
- area: fspec CLI
- scenarios: 1 mapped to deleted files
- rust code: rust/fspec/src/init.rs:81-92 — prints "✓ Installed fspec for {agent}", the file list and "Next steps:" exactly once after init; the interactive selector rust/fspec/src/init_selector.rs only renders the list and prints no success line
- rust tests: rust/fspec/tests/cli_init.rs (owner: init-cli-subcommand) checks that "✓ Installed fspec for claude", "Next steps:" and the activation text are present, but not that each appears exactly once
- covered: none (presence is proved; "no duplicate" is not)
- not covered: Initialize fspec for Claude Code without duplicate message -> write (into owner); Initialize fspec for other agents without duplicate message -> write (into owner)
- action: Merge into init-cli-subcommand. Add occurrence-count assertions (each success line appears exactly once, for claude and for one other agent) in rust/fspec/tests/cli_init.rs. Sibling duplicate-next-steps-message-in-fspec-init-output (same TS file) is merged the same way.

#### duplicate-next-steps-message-in-fspec-init-output
- class: merge
- area: fspec CLI
- scenarios: 1 mapped to deleted files
- rust code: rust/fspec/src/init.rs:92 — "Next steps:" is printed once by the run handler; init_selector.rs prints nothing, so the interactive path cannot duplicate it
- rust tests: rust/fspec/tests/cli_init.rs (owner: init-cli-subcommand) checks presence only
- covered: none
- not covered: Interactive mode shows single 'Next steps' message (AFTER FIX) -> write (into owner); CLI mode shows single 'Next steps' message (CURRENT BEHAVIOR) -> write (into owner); Interactive mode shows duplicate 'Next steps' message (BEFORE FIX) -> delete (describes the bug)
- action: Merge into init-cli-subcommand with a "Next steps:" count==1 assertion in rust/fspec/tests/cli_init.rs (the interactive path needs a TTY, so assert the single print site or the selector's lack of output). Consistent with duplicate-installation-success-message-in-fspec-init.

#### changed-files-view-doesn-t-show-deleted-files-correctly
- class: human
- area: git / fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: rust/git/src/status.rs:30-31 (deleted_from_index); rust/fspec-tui/src/views/diff_common/row.rs:25 ("D" is red). rust/git/src/diff.rs:27-28 returns FileNotFound for a working-tree-deleted file. The TUI then posts "The diff of X could not be read…" and shows "No changes to display" (rust/fspec-tui/src/app/dispatch_changed_files.rs:117-124, views/changed_files/mod.rs:286)
- rust tests: rust/git/tests/changed_files_rpc355.rs (owner: git-change-type-derivation, "Unstaged file deleted … change type D"); views/diff_common/row.rs (owner: sanitize-file-paths-and-labels-in-changed-files-and-checkpoint-views, status_color D is red)
- covered: View unstaged deleted file shows D in red
- not covered: View staged deleted file -> write (in rust/git/tests/changed_files_rpc355.rs); View diff panel for deleted file -> human
- action: merge the 2 list scenarios into git-change-type-derivation (add a staged-delete test). The diff-panel scenario depends on the answer below
- question: Selecting a deleted file in the Rust Changed Files view yields an error notice and "No changes to display", which is exactly what BUG-069 fixed in TS. Should we (a) open a bug card so Rust shows "File was deleted" (or a full removal diff) and move this scenario into rust-changed-files-view with a test (recommended), or (b) delete the scenario?

#### restore-individual-files-or-all-files-from-checkpoint-in-checkpoint-viewer
- class: merge
- area: fspec-tui
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-tui/src/views/checkpoints/keys.rs:59-60 (r/R, t/T); rust/fspec-tui/src/views/checkpoints/restore.rs:21-24 (r only in the Files pane), :39 (t has no pane guard); rust/git/src/ghost_commit.rs:665-666 (restore creates parent dirs)
- rust tests: views/checkpoints/restore_tests.rs (owner: checkpoint-restore); tests/checkpoint_restore_dispatch_rpc365.rs (owner: checkpoint-restore-dispatch); rust/git/tests/ghost_commit_tests.rs (owner: ghost-commit-checkpoints)
- covered: Restore single file with R key; Restore all files with T key; Cancel single file restore with N; Cancel restore all with N; R key only works when files pane is focused
- not covered: Restore deleted file creates file and parent directories -> write (in rust/git/tests/ghost_commit_tests.rs, owner ghost-commit-checkpoints); Restore file with uncommitted changes shows warning -> delete (no "Current changes will be LOST" path in Rust); T key only works when checkpoints or files pane is focused -> delete (Rust opens restore-all from the Diff pane too, restore.rs:39)
- action: merge into checkpoint-restore (RPC-365). Write a parent-dir restore test in rust/git/tests/ghost_commit_tests.rs. Delete the 2 scenarios Rust does not implement

#### merge-conflict-markers
- class: remap
- area: git
- scenarios: 5 mapped to deleted files
- rust code: rust/git/src/three_way_merge.rs:143-521 (markers, clean merge, add/add, binary, identical, re-merge)
- rust tests: rust/git/tests/three_way_merge_integration_tests.rs::test_apply_writes_conflict_markers_to_worktree_file, ::test_apply_auto_merges_non_overlapping_changes, ::test_apply_new_file_conflict_writes_markers, ::test_apply_binary_conflict_no_markers (line 264), ::test_apply_identical_changes_no_conflict, ::test_apply_re_merge_after_resolution_succeeds (line 388) (owner: merge-conflict-markers; all have @step comments)
- covered: all 6
- not covered: none
- action: Remove the src/tui/handlers/__tests__/mergeConflictMarkers.test.ts mappings. Add integration-test mappings for "Binary file conflict ..." (three_way_merge_integration_tests.rs:256-322) and "Re-running merge ..." (:381-) which today map only source lines.
- question: -

#### tui-custom-provider-section-builder
- class: merge
- area: providers
- scenarios: 5 mapped to deleted files
- rust code: rust/providers/src/custom/management.rs:220 (list_providers_info carries per-model context_window/max_output/supports_*); rust/sessions/src/handle_impl/handle_impl_5.rs:52-64 (maps custom models into wire ModelEntry with context_window/supports_vision/supports_reasoning)
- rust tests: rust/providers/tests/bug_139_per_model_limits/scenarios.rs::napi_list_providers_returns_per_model_limits_for_custom_providers, ::json_omits_context_window_list_providers_reports_it_unavailable; rust/providers/tests/bug_139_per_model_limits/unavailable_limits.rs::a_model_without_max_output_tokens_uses_the_declared_defaults_max_output_tokens (owner of all: custom-provider-napi-model-limits)
- covered: customProviderSectionBuilder sources contextWindow from NAPI entry; 1M JSON context_window round-trips; 200k JSON context_window round-trips verbatim; Max output tokens propagates verbatim (no 8192 fallback); supports_* flags propagate through the builder
- not covered: none
- action: merge into custom-provider-napi-model-limits (rust/providers/tests/bug_139_per_model_limits/*). Drop all 5 as duplicates. Sibling custom-provider-vision-support covers the vision flag.

#### custom-provider-vision-support
- class: merge
- area: sessions
- scenarios: 4 mapped to deleted files
- rust code: rust/providers/src/custom/config.rs:79 (`#[serde(default)] supports_vision`); rust/sessions/src/handle_impl/handle_impl_5.rs:63 (list_providers carries supports_vision); rust/sessions/src/model_resolution.rs:52-63 (custom model vision resolved from config, absent → false); rust/fspec-tui/src/views/agent/header_build.rs ([V] badge)
- rust tests: rust/sessions/tests/bug168_model_vision_resolution.rs::{registry_entry_set_at_session_creation_for_a_vision_custom_model, registry_entry_set_at_session_creation_for_a_non_vision_custom_model} (owner: session-model-capabilities-registry); rust/fspec-tui/tests/tui001_session_header_model_info.rs (owner: session-header-model-info; vision badge from resolved supports_vision)
- covered: Custom provider JSON with supports_vision true propagates; Custom provider model without supports_vision defaults to false; SessionHeader renders [V] badge for vision-enabled model (generic, not custom-specific)
- not covered: Legacy config without supports_vision loads cleanly without regression -> delete (dup of the default-false scenario, serde default config.rs:79)
- action: merge into session-model-capabilities-registry (rust/sessions/tests/bug168_model_vision_resolution.rs). The badge scenario duplicates session-header-model-info.

#### work-unit-context-service
- class: merge
- area: sessions
- scenarios: 5 mapped to deleted files
- rust code: session work-unit focus updated on update-work-unit-status, with environment reminder "Current work unit: <ID>" (rust/sessions/src/focus_reconciliation.rs, handle_impl_2.rs)
- rust tests: rust/sessions/tests/cov057_work_unit_context_backend.rs::{scenario_work_unit_id_appears_in_environment_information_when_entering_agentview, scenario_llm_receives_notification_when_updating_a_different_work_unit, scenario_no_notification_when_updating_the_same_work_unit, scenario_no_notification_when_no_active_session_exists} (owner: work-unit-context-backend)
- covered: Work unit context stored when entering AgentView; LLM receives notification when updating a different work unit (the Rust reminder names only the current unit); No notification when updating the same work unit; No notification when no active session exists
- not covered: System reminder formatted with previous and current work unit -> delete (Rust reminder carries only "Current work unit: <ID>"; the owner feature dropped the previous unit)
- action: merge into work-unit-context-backend (rust/sessions/tests/cov057_work_unit_context_backend.rs).

#### claude-oauth-model-routing
- class: merge
- area: sessions
- scenarios: 6 mapped to deleted files
- rust code: rust/sessions/src/handle_impl/handle_impl_5.rs:40-120 (list_providers: credential-gated cloud catalog); provider_has_credentials gate (Claude OAuth satisfies anthropic); rust/sessions/src/startup_model_resolution.rs:52 (persisted model restored)
- rust tests: rust/sessions/tests/prov128_unified_credential_gating.rs::{claude_oauth_continues_to_satisfy_the_anthropic_gate, no_credentials_gate_and_header_agree_uncredentialed} (owner: unified-credential-gating); rust/sessions/tests/rpc073_cloud_model_catalog.rs::{credentialed_provider_lists_tool_call_models_newest_first_dropping_deprecated, uncredentialed_provider_yields_no_models} (cloud-model-catalog-population); rust/sessions/src/startup_model_resolution_tests.rs::persisted_model_is_restored_exactly (startup-model-initialization-first-available)
- covered: Anthropic models appear in model selector when OAuth tokens exist; No Anthropic section when no OAuth tokens and no API key; API key only shows Anthropic section without OAuth; Persisted Anthropic model restored on startup with OAuth tokens (cloud restore, no OAuth-specific variant)
- not covered: Both API key and OAuth tokens show Anthropic section -> delete (dup: either credential satisfies the gate); Non-OAuth providers unaffected by Claude OAuth changes -> delete (dup of prov128 per-provider gate)
- action: merge into unified-credential-gating (rust/sessions/tests/prov128_unified_credential_gating.rs). Restore duplicates startup-model-initialization-first-available.

#### codex-model-selector-integration
- class: merge
- area: sessions
- scenarios: 6 mapped to deleted files
- rust code: rust/sessions/src/cloud_models.rs:206 synthesize_codex_section (whole OpenAI catalog moved under codex, standalone openai section removed); handle_impl_5.rs:105-120; PROV-146 excludes the OpenAI cloud catalog from the `openai` section
- rust tests: rust/sessions/tests/prov129_codex_section_synthesis.rs::{codex_oauth_yields_populated_codex_section_and_no_openai_section, codex_section_lists_all_openai_models_from_models_dev, without_codex_credentials_openai_section_is_suppressed} (owner: codex-section-synthesis); rust/sessions/tests/prov146_openai_cloud_excluded.rs::scenario_codex_section_still_lists_all_openai_models_when_openai_key_set (owner: openai-cloud-catalog-excluded-from-openai-api-provider-section); rust/sessions/tests/codex_catalog_limit_resolution.rs (codex-catalog-limit-resolution, codex/<model> sessions)
- covered: All OpenAI cloud models appear in Codex section when OAuth tokens exist; No Codex section when OAuth tokens absent; Codex OAuth active with OpenAI API key shows only Codex section for cloud models; Selecting a model from Codex section creates session with codex provider
- not covered: Persisted Codex model restored on startup -> write in startup-model-initialization-first-available (resolve_startup_model with "codex/gpt-5.2") or delete; OpenAI API key without Codex OAuth shows OpenAI section -> delete (contradicted by PROV-146: a raw OpenAI key yields no OpenAI cloud section)
- action: merge into codex-section-synthesis (rust/sessions/tests/prov129_codex_section_synthesis.rs).

#### bridge-session-chunk-filtering
- class: merge
- area: tools
- scenarios: 3 mapped to deleted files
- rust code: rust/tools/src/bridge.rs:122-178 (BridgeManager keyed by session_id; get_or_create_bridge_manager), bridge.rs:223-237 (Bridge tool stores session_id at construction)
- rust tests: rust/tools/src/bridge.rs::tests::{test_relay_stream_chunks_to_connected_endpoint, test_receive_input_from_endpoint_and_inject} (owner: bridge-tool-unit); rust/tools/tests/tool_wrapper_session_association_test.rs (owner: tool-wrapper-session-association)
- covered: Bridge receives session_id from tool call context (tool-wrapper-session-association "Bridge tool wrapper stores session_id at construction"), Bridge input and response flow (bridge-tool-unit "Receive input from endpoint and inject into session"; the WatcherInput chunk was renamed SupervisorInput by WATCH-024)
- not covered: Bridge relays only bridged session chunks -> write (two sessions' managers; a chunk relayed for A never reaches B's endpoint)
- action: Merge into bridge-tool-unit. Add the cross-session relay test to the rust/tools/src/bridge.rs test module. Sibling tui-session-chunk-filtering (also BRIDGE-012) is merged too.

#### structured-fspectool-results-via-streamchunk-discriminated-union
- class: remap
- area: tools
- scenarios: 5 mapped to deleted files
- rust code: rust/tools/src/fspec_handler.rs:30-130 (FspecRequest/FspecResult with success/data/error/system_reminder); rust/rpc-types/src/stream_chunk_constructors.rs:167 (StreamChunk::FspecCommandRequest); FSPEC_INTERCEPT is absent from wrapper.rs
- rust tests: rust/tools/tests/fspec_facade_wrapper_test.rs::test_fspec_tool_wrapper_emits_structured_json_marker, ::test_fspec_tool_wrapper_with_gemini_facade, ::test_fspec_request_json_has_all_required_fields_for_typescript, ::test_fspec_wrapper_does_not_emit_fspec_intercept_string (owner: none; the file header already names this feature)
- covered: FspecTool emits structured command request and receives typed result; TypeScript handles FspecCommandRequest with type-safe field access; FSPEC_INTERCEPT string pattern is removed after migration (needs @step comments: several Then-steps in the file differ from the feature text, and the "TypeScript ..." steps should be reworded for the Rust consumer)
- not covered: Failed fspec command returns structured error in FspecCommandResult -> write (fspec_handler.rs:105-130); System reminder is included in FspecCommandResult for workflow guidance -> write (fspec_handler.rs:53)
- action: Map rust/tools/tests/fspec_facade_wrapper_test.rs to this feature. Add 2 test fns there (error result, system_reminder) and fix the @step text.

### Batch COV-073-B8

#### open-attachments-in-browser-from-details-panel
- class: merge
- area: fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-tui/src/components/attachment_picker_dialog.rs:148-152 (Enter emits Action::OpenAttachment); the daemon/attachment-viewer serves the file, and its refusals are shown (BUG-431)
- rust tests: rust/fspec-tui/tests/board_open_attachment_rpc374.rs::selecting_an_attachment_opens_it_from_the_selected_workspace_on_the_daemon (owner: rust-board-open-attachment); rust/fspec-tui/tests/bug431_card_attachments_come_from_the_daemon.rs (owner: card-attachments-come-from-the-daemon)
- covered: Open image attachment in browser, Open PDF attachment in default viewer (same open path; duplicate), Handle missing attachment file gracefully (BUG-431 "The daemon's refusal is shown for an attachment it cannot serve")
- not covered: none
- action: Merge into rust-board-open-attachment (open) and card-attachments-come-from-the-daemon (missing file). Opening the first attachment directly on 'a' was already replaced by the TUI-019 picker. No new tests.

#### tui-clear-should-update-react-state-as-side-effect-of-rust-state-change
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/sessions/src/background_session/history.rs:4-28 (clear_history resets messages and tokens, re-injects context reminders, emits SessionStateChange Cleared and the session-cleared event); rust/agent-loop/src/bridge_control.rs:4 (the bridge uses the same clear_history); rust/fspec-tui/src/app/dispatch_slash_clear.rs:41-62 (optimistic local reset, then backend.clear_history; errors go to tracing only)
- rust tests: rust/fspec-tui/tests/slash_clear_rpc046.rs (owner: slash-command-clear); rust/fspec-tui/tests/rpc437_session_cleared_tui_consumption.rs (owner: session-events-tui-cleared-consumption; resets tokens and fill); rust/sessions/tests/rpc436_session_cleared_broadcast.rs (owner: session-events-daemon-cleared-broadcast); rust/cli/tests/telegram_clear_context_test/{context,state}.rs (owner: telegram-clear-context-reset); rust/agent-loop/tests/cmpct054_bridge_clear_failure.rs (owner: bridge-clear-checkpoint-failure); rust/fspec-tui/tests/slash_clear_ts_parity_rpc074.rs (owner: rpc074-clear-ts-parity)
- covered: TUI /clear triggers Rust state change which emits chunk to update React, Bridge /clear uses same Rust code path as TUI, System reminders preserved after clear (telegram-clear-context-reset), Clear failure does not corrupt React state (session level: cmpct054 "no cleared event; messages and counters unchanged")
- not covered: none. Note: the TUI scrollback is reset optimistically before the backend call (RPC-074 TS parity), so the TUI-level clause "conversation should remain unchanged" no longer holds; drop it when merging.
- action: Merge into slash-command-clear / session-events-tui-cleared-consumption, with the bridge and failure parts going to bridge-clear-checkpoint-failure and telegram-clear-context-reset. No new tests.

#### fix-work-unit-details-panel-to-be-static-4-lines-high
- class: retire
- area: fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-tui/src/views/board/details_strip.rs:1-16,32-40 (fixed 5-row strip: title, 2 description rows, attachments, metadata; centered "No work unit selected" placeholder)
- rust tests: rust/fspec-tui/tests/view_board_unit_rpc014.rs::{no_work_unit_selected_paints_centered_placeholder, selected_work_unit_paints_title_and_metadata_rows_of_details_strip} (owner: rpc014-board-grid)
- covered: No work unit selected shows empty 4-line panel (placeholder part only, already owned by rpc014-board-grid)
- not covered: Work unit with description displays all 4 content lines -> retire, Work unit without description maintains 4-line height -> retire
- action: @deprecated citing TUI-012 (the attachments row made it a 5-row strip) and RPC-014 (the Rust 5-row details strip in details_strip.rs). Rust has no 4-line layout with metadata on line 3.

#### tui-016-incomplete-checkpointpanel-using-chokidar-instead-of-ipc-zustand
- class: merge
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: rust/fspec-tui/src/app/bootstrap.rs:27 (fires checkpoint_counts at startup); rust/fspec-tui/src/app/checkpoint_calls.rs:38-41; the checkpoint-changed push from the git-state watcher
- rust tests: rust/fspec-tui/tests/bug181_checkpoint_watcher_push.rs (owner: live-checkpoint-counts-watcher-push); rust/fspec-tui/tests/bug181_live_checkpoint_counts.rs (owner: live-checkpoint-counts-in-the-board-header); rust/fspec-tui/tests/app_bootstrap_rpc015.rs (owner: rpc015-app-bootstrap)
- covered: Manual checkpoint triggers IPC update to TUI, Auto checkpoint triggers IPC update to TUI, Cleanup checkpoints triggers IPC update to TUI (owner: "Checkpoint delete publishes decremented counts", which covers ref deletion by cleanup-checkpoints), TUI loads initial checkpoint counts on startup (rpc015-app-bootstrap)
- not covered: CheckpointPanel does not import chokidar -> delete (TS source check), Zustand store exposes checkpoint counts state -> delete (Zustand internals)
- action: Merge into live-checkpoint-counts-watcher-push (push) and rpc015-app-bootstrap (initial load). No new tests.

#### screen-component-integration
- class: merge
- area: fspec-tui
- scenarios: 10 mapped to deleted files
- rust code: rust/fspec-tui/src/views/model_selector (full-screen selector); rust/fspec-tui/src/app/dispatch_model_selector.rs; rust/fspec-tui/src/app/dispatch_provider_settings.rs; navigator Tab switching (rpc337)
- rust tests: rust/fspec-tui/src/views/model_selector/tests_core.rs (owner: model-selector-view); rust/fspec-tui/src/views/model_selector/tests_tab.rs (owner: model-selector-tab-to-providers); rust/fspec-tui/tests/rpc337_navigator_model_selector.rs (owner: model-selector-navigator-wiring); rust/fspec-tui/tests/provider_dispatch_rpc054/lifecycle.rs (owner: rpc054-provider-settings-dispatch); rust/fspec-tui/tests/slash_command_wiring_rpc022.rs (owner: rpc022-slash-command-wiring)
- covered: Open model selector screen via /model command, Open provider settings screen via /provider command, Switch from model selector to provider settings via Tab, Switch from provider settings to model selector via Tab, Model selection updates session, Close model selector screen via Escape, Close provider settings screen via Escape, Model data is loaded lazily when model selector opens (rpc022 "/model … spawns list_providers")
- not covered: AgentView does not have duplicate model state -> delete (TS source grep), Shared store provides model data to both AgentView and ModelSelectorScreen -> delete (Zustand store)
- action: Merge into model-selector-view / model-selector-tab-to-providers / model-selector-navigator-wiring / rpc054-provider-settings-dispatch. No new tests.

#### session-attachment-for-work-units-with-tui-resume-integration
- class: merge
- area: fspec-tui
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_focus.rs:6-22 (binds the card when a session is created from it); rust/fspec-tui/src/views/board/viewport.rs:157 (🟢 prefix); board Enter chooser (BOARD-024); /resume never rebinds a card (rust/fspec-tui/src/app/dispatch_resume_search_views.rs)
- rust tests: rust/fspec-tui/tests/view_board_unit_rpc016.rs::work_unit_with_an_attached_session_renders_the_green_circle_prefix (owner: rpc016-board-viewport); rust/fspec-tui/tests/board024/chooser.rs (owner: board-card-enter-chooser-and-complete-details); rust/fspec-tui/tests/slash_detach_rpc050.rs (owner: remove-detach-slash-command-with-attach-path-intact)
- covered: Session auto-attaches to work unit on first message (now bound at creation: board chooser "Default agent choice uses the captured card"), Work unit with attached session displays green indicator, Work unit without attached session displays normally (rpc016 suite), Pressing Enter on work unit with attached session resumes conversation (chooser "earliest associated agent")
- not covered: Detach command removes session attachment and clears conversation -> retire (/detach removed; product owner 2026-11-05 deprecates RPC-050's /detach), Resume command attaches selected session to current work unit -> delete, Resume command replaces existing session attachment -> delete (Rust /resume restores a session without binding a card), Session attachments are cleared when app closes -> delete (daemon-held sessions and bindings survive client restart; TUI-113 auto-restore)
- action: Merge into rpc016-board-viewport (indicator) and board-card-enter-chooser-and-complete-details (Enter/auto-bind). Deprecate the /detach scenario citing RPC-050 (remove-detach-slash-command-with-attach-path-intact). No new tests.

#### tui-session-chunk-filtering
- class: merge
- area: fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_stream_chunks.rs (chunks routed by SessionId); AgentViewStore::session_context_mut_for (per-session scrollback)
- rust tests: rust/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs (owner: agentview-subscribe-broadcasts; "Background-session chunks are routed by SessionId, not by focus"); rust/fspec-tui/tests/store_agent_view_multisession_rpc024.rs (owner: rpc024-multi-session-store; "SessionContext owns its own scrollback", "session_context_mut_for routes chunks by id, not focus")
- covered: TUI displays only current session chunks, Session switch changes which chunks are displayed, Bridge input displays in TUI with correct session (bridge chunks carry the session id; same routing, duplicate)
- not covered: none
- action: Merge into agentview-subscribe-broadcasts (with rpc024-multi-session-store). No new tests. Sibling bridge-session-chunk-filtering is merged into bridge-tool-unit.

#### watcher-session-header-indicator
- class: retire
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: none for a watcher header. The watcher header lived in SplitSessionView, which TUI-080 deleted (17d239810, "remove dead split view"). WATCH-024 renamed watcher to supervisor/subordinate. The regular Rust header has [R]/[V]/[SELECT] and " [Subordinate of: …]" (rust/fspec-tui/src/views/agent/header_build.rs:64-102)
- rust tests: none for watcher sessions. The regular-session header is owned by session-header-model-info (tests/tui001_session_header_model_info.rs) and rpc029-agent-structure-alignment
- covered: none (watcher-specific)
- not covered: Full header displays for watcher session -> retire; Reasoning indicator -> retire; Vision indicator -> retire; Context fill 80 percent -> retire; Regular session header unchanged -> retire (duplicate of session-header-model-info); SELECT indicator in watcher header -> retire
- action: @deprecated citing TUI-080 (17d239810, split view removed) and WATCH-024 (watcher became supervisor/subordinate)

#### anchor-view
- class: retire
- area: fspec-tui
- scenarios: 8 mapped to deleted files
- rust code: none found. Grepped "anchor", "/anchors", AnchorView in rust/fspec-tui/src; none of the hits are an anchor view or a /anchors command. ab07e96be deleted AnchorView.tsx, AnchorListPane, AnchorPreviewPane and the anchor detector
- rust tests: none
- covered: none
- not covered: all 8 -> retire
- action: @deprecated citing CMPCT-013 (legacy-compaction-cleanup, ab07e96be: anchor infrastructure and AnchorView removed). Siblings with the same treatment: TUI-056 anchor-viewer features (fix-anchor-viewer-integration-with-thinking-dialog-system, interactive-anchor-point-viewer-with-conversation-navigation)

#### pure-flexbox-layout-for-checkpoint-and-changed-files-viewers
- class: retire
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: ratatui computes the layout from the frame Rect on every draw (rust/fspec-tui/src/views/diff_common/pane.rs:32). The Changed Files view loads real diffs through backend.file_diff (rust/fspec-tui/src/app/dispatch_changed_files.rs:114)
- rust tests: views/changed_files/tests.rs (owner: rust-changed-files-view); views/changed_files/loading_tests.rs (owner: changed-files-view-f-shows-staged-animated-loading-dialog…)
- covered: ChangedFilesViewer loads real git diffs (duplicate of rust-changed-files-view "Moving the selection down reloads the diff…" and "The diff pane renders colored add, remove and hunk-header lines")
- not covered: VirtualList uses flexbox without hardcoded heights -> retire; CheckpointViewer uses flexbox proportional sizing -> retire; Terminal resize causes automatic layout adjustment -> retire
- action: @deprecated citing 103a73072 (Ink flexbox/VirtualList internals; ratatui layout replaces them). Drop the diff scenario as a duplicate of rust-changed-files-view. Keep consistent with the VirtualList siblings TUI-008 and TUI-001

#### delete-checkpoint-or-all-checkpoints-from-checkpoint-viewer-with-confirmation
- class: merge
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/views/checkpoints/keys.rs:62-63 (d/D single delete, a/A delete all); rust/fspec-tui/src/views/checkpoints/delete.rs:44 (typed DELETE ALL), :135 (close the view when empty)
- rust tests: views/checkpoints/delete_tests.rs (owner: checkpoint-delete); tests/checkpoint_delete_dispatch_rpc366.rs (owner: checkpoint-delete-dispatch)
- covered: Delete single checkpoint with D key; Cancel single deletion with N; Delete last remaining checkpoint exits to board; Cancel delete ALL with ESC; Delete ALL with typed confirmation (Rust binds delete-all to "a", not Shift+D; Shift+D maps to single delete)
- not covered: none
- action: merge into checkpoint-delete (RPC-366). Drop all 5 as duplicates. Rust differs on the key: delete-all is "a", not Shift+D

#### tui-pause-state-display
- class: merge
- area: fspec-tui
- scenarios: 2 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/pause_prompt.rs (inline Confirm/Triple prompt); rust/rpc-types/src/session_state.rs:82-85 (wire PauseKind has only Confirm and Triple, no Continue)
- rust tests: tests/inline_pause_prompt_rpc406/rendering.rs, keys.rs, lifecycle.rs (owner: inline-tool-approval-pause-prompt)
- covered: TUI shows confirm dialog for dangerous operations (duplicate of RPC-406 "Confirm pause prompt renders yellow header, dim details line, and Y/N row")
- not covered: TUI transitions from Thinking to Paused and back -> retire (Continue pause and Enter-to-resume removed; RPC-406 "The pause modal is deleted and resume is unreachable from the prompt")
- action: merge into inline-tool-approval-pause-prompt (RPC-406). Delete the Continue/resume scenario, citing RPC-406

#### provider-settings-screen
- class: merge
- area: fspec-tui
- scenarios: 18 mapped to deleted files
- rust code: rust/fspec-tui/src/views/provider_settings/list.rs:31-70 (filter mode, two-step Esc cascade, Tab, Enter, d); rust/fspec-tui/src/views/provider_settings/list_actions.rs:55-57 (Enter on a provider row toggles expansion); detail.rs:57-66 (t keybind removed, RPC-154)
- rust tests: tests/provider_settings_view_rpc054.rs (owner: rpc054-provider-settings-view); provider_settings_tab_switch_to_models_rpc160.rs (rpc160-…); provider_settings_flat_tree_rpc103.rs::enter_on_provider_toggles_expansion_without_mutating_selected_index (rpc103-…); provider_settings_edit_silent_cancel_rpc162.rs (rpc162-…); confirm_dialog_yn_shortcut_rpc164.rs (rpc164-…); provider_settings_profile_form_prov110.rs (provider-settings-profile-form); provider_settings_list_keybind_parity_rpc157.rs (rpc157-…)
- covered: Switch to model selector with Tab; Close with Escape when no filter; Clear filter with Escape when active; Navigate down; Enter filter mode with slash; Expand provider section with Enter; Confirm profile deletion with y; Cancel profile deletion with n; Save API key with Enter; Cancel API key edit with Escape; Navigate to next field with Down; Cancel profile form with Escape; Exit filter mode keeping filter with Enter; Clear filter and exit filter mode with Escape
- not covered: Test connection with t key -> retire (RPC-154 removed the t keybind; no view emits TestProviderConnection); Refresh providers with r key -> retire (RPC-157: r/R silently ignored in list mode); uses useProviderSettingsState hook -> retire; renders ProviderSettingsPanel -> retire
- action: merge into rpc054-provider-settings-view (plus the sibling owners above). Drop 14 as duplicates. Deprecate t/r citing RPC-154/RPC-157, and the 2 React-structure scenarios citing 103a73072

#### session-management-panel
- class: retire
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: no Session Management Panel. 7b38c6cb2 (GIT-036, "add merge worktree command handler and remove SessionManagementPanel") deleted it. Isolated creation is rust/fspec-tui/src/app/dispatch_create_session_dialog.rs; worktree listing and pruning is /worktrees (WT-005)
- rust tests: tests/isolated_session_dialog_rpc060.rs (owner: rpc060-isolated-session-dialog) covers isolated vs normal creation
- covered: Create session with isolated toggle disabled; Create session with isolated toggle enabled (both duplicates of rpc060)
- not covered: View panel with pending sessions -> retire; Merge a pending_merge session -> retire; Discard -> retire; Prune orphaned -> retire
- action: @deprecated citing GIT-036 (7b38c6cb2, panel removed in favour of /merge-worktree; /worktrees covers listing and pruning). Sibling session-management-panel-keyboard gets the same treatment

#### session-management-panel-keyboard
- class: retire
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: none (panel removed in 7b38c6cb2)
- rust tests: none
- covered: none
- not covered: all 4 -> retire
- action: @deprecated citing GIT-036 (7b38c6cb2). Same treatment as session-management-panel

#### default-thinking-level-persistence
- class: retire
- area: fspec-tui / sessions
- scenarios: 9 mapped to deleted files
- rust code: rust/fspec-tui/src/components/thinking_level_dialog.rs:37 (footer "↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close"), :226 (D sets the default). Saved levels are per model under tui.thinkingLevels (MODEL-012); the global tui.defaultThinkingLevel is ignored
- rust tests: tests/tui095_live_default_marker.rs (owner: thinking-dialog-live-default-marker); tests/tui094_default_indicator_parity.rs (thinking-dialog-default-indicator); tests/rpc027_dialog_parity_bcd/thinking_level.rs (rpc027-help-disconnect-thinking-dialogs); tests/model012_thinking_level_remembered_per_model.rs (thinking-level-remembered-per-model)
- covered: Set default via D key; Dialog footer shows D; Default indicator shown; No indicator when none; Indicator moves on D; Current selection independent of default (all duplicates in TUI-094/TUI-095/RPC-027/MODEL-012)
- not covered: Restore default thinking level on new session -> retire; Use Off when no default is set -> retire; Handle corrupt config gracefully -> retire (MODEL-012: global default ignored, no saved level means "not set" rather than Off, and an unreadable level is reported rather than loaded as Off)
- action: @deprecated citing MODEL-012 (per-model saved level replaced the global tui.defaultThinkingLevel). The UI scenarios are already owned by TUI-094/TUI-095/RPC-027

### Batch COV-073-B9

#### delete-session-from-resume-view-with-confirmation-dialog
- class: human
- area: fspec-tui
- scenarios: 9 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/resume_session_view.rs:55-57, 168-175, 228-232 (D opens a 2-button ConfirmDialog "Delete session?" for the selected session only), :253 (footer "… D Delete …"). rust/core/src/persistence/sessions.rs:25 delete_session. There is no Delete-ALL path, and the view does not close itself once the list is empty
- rust tests: tests/rpc026_resume_session_view.rs (owner: rpc026-resume-session-view); rust/core/tests/session_delete_cleanup_liveness.rs (owner: session-delete-cleanup-liveness)
- covered: Cancel option closes dialog; ESC cancels dialog; Footer shows D Delete; Delete This Session removes and refreshes (rpc026 "D opens the ConfirmDialog and Enter on Primary deletes the session"); Session deletion triggers orphaned message cleanup (session-delete-cleanup-liveness)
- not covered: D key opens three-button dialog -> human; Delete ALL Sessions removes all and exits -> human; Arrow keys navigate between the three buttons -> human; Deleting last session exits resume mode -> human (Rust shows the empty placeholder instead)
- action: drop the 5 covered scenarios as duplicates of rpc026-resume-session-view / session-delete-cleanup-liveness. Then follow the answer below
- question: The Rust resume view (RPC-026) deletes only the selected session through a 2-button ConfirmDialog, has no "Delete ALL Sessions", and shows the empty placeholder after the last delete. Should this feature be (a) marked @deprecated with its behaviour merged into rpc026-resume-session-view, deleting the 4 three-button/delete-all/auto-exit scenarios (recommended), or (b) kept, with a card to port Delete-ALL and auto-exit to Rust?

#### multilineinput-should-show-compaction-status-instead-of-conversation-message
- class: retire
- area: fspec-tui
- scenarios: 7 mapped to deleted files
- rust code: rust/fspec-tui/src/views/agent/input_transition.rs:20-24 (Compacting state on the input row); rust/fspec-tui/src/views/agent/busy_composer.rs:37,46 (composer stays editable while Compacting); rust/fspec-tui/src/views/agent/dispatch.rs:238
- rust tests: tests/automatic_compaction_progress_bug210.rs (owner: automatic-compaction-progress); tests/truthful_compaction_activity_bug213.rs (truthful-compaction-activity); tests/single_sourced_compaction_notice_rpc421.rs (single-sourced-compaction-notice)
- covered: Conversation history remains clean (RPC-421: exactly one notice); Input returns to normal after compaction (automatic-compaction-progress)
- not covered: Placeholder shows "analyzing anchors… 15/32 turns" -> retire; Detailed compaction phases -> retire; Hook-triggered compaction progress -> retire; Emergency compaction progress -> retire (BUG-213: activity instead of message-count progress, and CMPCT-013 removed anchors); Input blocks typing during compaction -> retire (TUI-115: composer stays editable and queues while Compacting)
- action: @deprecated citing BUG-213 and TUI-115 (also CMPCT-013 for anchors). The 2 covered scenarios are already owned by RPC-421 and BUG-210

#### kanban-tui-keyboard-navigation-page-up-down-and-home-end-key-bindings
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/store/board_viewport.rs:53 (PageUp/PageDown by viewport height; select_first/last_in_focused)
- rust tests: tests/view_board_unit_rpc016.rs (owner: rpc016-board-viewport); tests/store_board_viewport_rpc016.rs (owner: rpc016-board-store-viewport)
- covered: all
- not covered: none
- action: merge into rpc016-board-viewport (its "PageDown/PageUp/Home/End" scenarios) and drop all 4

#### virtuallist-height-calculation-ignores-flexbox-container-dimensions
- class: retire
- area: fspec-tui
- scenarios: 3 mapped to deleted files
- rust code: none needed. Ratatui panes get their size from the frame Layout on every draw (rust/fspec-tui/src/views/diff_common/pane.rs:32-39); there is no measureElement step
- rust tests: none
- covered: none
- not covered: all 3 -> retire
- action: @deprecated citing 103a73072 (Ink VirtualList measureElement/flexbox internals). Same treatment as the GIT-006 and TUI-032 siblings

#### mouse-scroll-wheel-and-trackpad-support-for-virtuallist
- class: merge
- area: fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-tui/src/views/changed_files/mouse.rs (wheel over the file list and diff pane); rust/fspec-tui/src/components/scroll_viewport.rs:81-110 (WheelVelocity acceleration with a 150 ms gap, not a 100 ms throttle)
- rust tests: views/changed_files/tests.rs (owner: rust-changed-files-view, "Mouse wheel selection over the file list…", "A mouse wheel scroll over the focused diff pane…")
- covered: Scroll down with mouse wheel in file list; Scroll up with mouse wheel; Scroll with trackpad gesture (terminals report trackpad scrolling as wheel events)
- not covered: Rapid scroll with throttling -> delete (Rust uses WheelVelocity acceleration rather than a 100 ms throttle)
- action: merge into rust-changed-files-view and drop 3 duplicates. Delete the throttling scenario

#### native-text-selection-while-preserving-mouse-scroll-wheel-in-virtuallist
- class: retire
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: in-app selection and copy keep mouse capture on (rust/fspec-tui/src/mouse/selection.rs, clipboard.rs). MouseTrackingToggle (rust/fspec-tui/src/mouse/toggle.rs) is RPC-023 scaffolding that no production path uses
- rust tests: tests/agentview_text_selection_copy_copy006.rs (owner: agentview-text-selection-copy); tests/scrollback_scroll_rpc094.rs (rpc094-agentview-scrollback-scroll)
- covered: User scrolls with mouse wheel in conversation view (RPC-094 / COPY-006 "Wheel scrolling still works")
- not covered: Click-drag disables tracking for native selection -> retire; Re-enable after 5 s timeout -> retire; Button release re-enables -> retire; Rapid clicks extend debounce -> retire; Timer cleaned up on navigate away -> retire
- action: @deprecated citing COPY-006 ("Mouse capture remains enabled throughout selection and copy" replaced native-selection toggling). Same treatment as the BUG-131 sibling

#### sgr-mouse-protocol-component-integration
- class: retire
- area: fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: crossterm decodes mouse events (no hand-written SGR parser); wheel handling is in rust/fspec-tui/src/views/agent/mouse_dispatch.rs and rust/fspec-tui/src/views/board/mouse.rs
- rust tests: tests/scrollback_scroll_rpc094.rs (owner: rpc094-agentview-scrollback-scroll); tests/board_mouse_rpc023.rs (owner: boardview-mouse-handling)
- covered: VirtualList scroll-up via SGR; scroll-down via SGR (RPC-094 wheel); Board view scroll (boardview-mouse-handling wheel scenarios)
- not covered: Left-click press disables mouse tracking -> retire; Release re-enables tracking -> retire (COPY-006 keeps capture on)
- action: @deprecated citing 103a73072 (TS stdin SGR parser) and COPY-006. The wheel scenarios are already owned by RPC-094 and boardview-mouse-handling. Same treatment as the TUI-078 sibling

#### cross-pane-selection-with-correlation-ids
- class: retire
- area: fspec-tui (split view) / sessions
- scenarios: 6 mapped to deleted files
- rust code: rust/sessions/src/background_session/output.rs:59-69 still stamps `{session_id}-{counter}` correlation_id and attaches pending observed ids (left over, nothing reads it); rust/sessions/src/background_session/progress.rs:35 set_pending_observed_correlation_ids has no callers (the supervisor evaluation loop was removed). The ratatui TUI has no split view and no cross-pane highlight.
- rust tests: none
- covered: none
- not covered: StreamChunk receives correlation ID in handle_output -> retire (leftover code; optional write rust/sessions/tests/watch011_chunk_correlation_ids.rs if the PO wants to keep it), Parent turn selection highlights correlated watcher turns -> retire, Watcher turn selection highlights observed parent turns -> retire, Direct user message to watcher has no correlation -> retire, Cross-pane highlight updates as selection moves -> retire, Watcher evaluation captures observed correlation IDs -> retire
- action: @deprecated citing TUI-080 (17d239810 removed SplitSessionView, correlationMapping and cross-pane-correlation.test.tsx) and AMGR-008 (a9d161f9b removed the supervisor evaluation loop). Sibling features: discuss-selected-feature, watcher-split-view-ui, remove-dead-split-view.
- question: -

#### discuss-selected-feature
- class: retire
- area: fspec-tui (split view)
- scenarios: 5 mapped to deleted files
- rust code: none found: grepped rust/fspec-tui for "Regarding turn", "parent session", split/watcher pane. There is no split view and no discuss pre-fill.
- rust tests: none
- covered: none
- not covered: all 5 -> retire
- action: @deprecated citing TUI-080 (17d239810 deleted SplitSessionView.tsx and discuss-selected.test.tsx). Sibling: cross-pane-selection-with-correlation-ids, watcher-split-view-ui.
- question: -

#### expand-command-for-turn-based-tool-output-expansion
- class: retire
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: none found: there is no /expand in rust/fspec-tui/src/views/agent/slash_commands.rs. Collapsed output uses "... +N lines (Enter to view full)" (rust/fspec-tui/src/store/agent_view/chunk_wrap.rs:26). NOTE: an old "(select turn to /expand)" hint is still in rust/fspec-tui/src/store/agent_view/patch_diff.rs:78 (diff_codec.rs:45 comment).
- rust tests: none
- covered: none
- not covered: all 6 -> retire
- action: @deprecated citing TUI-045 (select-mode-enter-key-opens-full-turn-modal removed /expand and replaced it with Enter -> TurnContentModal; Rust parity in RPC-382/RPC-389). Optionally file a bug for the old /expand hint in patch_diff.rs:78.
- question: -

#### copilot-tui-login-dispatch
- class: retire
- area: fspec-tui
- scenarios: 1 mapped to deleted files
- rust code: none found: no model-selection→Copilot-login dispatch (grepped copilot/login/device in rust/fspec-tui/src/app). The model selector only lists models of credentialed providers (PROV-128 gate, PROV-127 drops empty sections). Copilot login happens in Provider Settings (rust/fspec-tui/src/app/dispatch_provider_settings_copilot.rs:44, PROV-114)
- rust tests: rust/sessions/tests/prov128_unified_credential_gating.rs::github_copilot_oauth_alone_satisfies_the_gate (owner: unified-credential-gating); rust/fspec-tui/tests/provider_settings_oauth_copilot_device_prov114.rs (owner: provider-settings-oauth-copilot-device)
- covered: none
- not covered: TUI launches Copilot OAuth login when user picks Copilot model with no credentials -> retire (an uncredentialed Copilot model cannot be picked)
- action: @deprecated citing PROV-128 (unified-credential-gating) and PROV-114 (Copilot device login in Provider Settings).

#### optimize-context-compaction-performance-and-ux
- class: retire
- area: none (legacy compaction)
- scenarios: 6 mapped to deleted files
- rust code: none found: grepped "Analyzing anchors", "Generating summary", "Retrying", "continue without compacting" in rust/; CMPCT-013 deleted the anchor/selector/compactor pipeline
- rust tests: none (input blocking during compaction is already owned elsewhere: rust/fspec-tui/tests/multilineinput_parity_rpc095.rs, owner agentview-multilineinput-parity)
- covered: none
- not covered: all 6 -> retire
- action: @deprecated citing CMPCT-013 (legacy-compaction-cleanup removed the anchor-analysis compaction, its progress and its retry flow; compaction is now agent-driven via inject_summary/SessionSearch).

#### fix-anchor-viewer-integration-with-thinking-dialog-system
- class: retire
- area: none
- scenarios: 2 mapped to deleted files
- rust code: none found: grepped `/anchors`, `AnchorViewer`, `anchor_viewer` in rust/ (only the compaction annotation_detector matches)
- rust tests: none
- covered: none
- not covered: Anchor viewer opens without disrupting existing UI dialogs -> retire, Anchor viewer displays session data without causing crashes -> retire
- action: @deprecated citing CMPCT-013 (legacy-compaction-cleanup), commit ab07e96be. That commit deleted the anchor model and the TS `/anchors` handler. Sibling feature interactive-anchor-point-viewer-with-conversation-navigation shares TUI-056-anchor-viewer.test.tsx and gets the same decision.

#### interactive-anchor-point-viewer-with-conversation-navigation
- class: retire
- area: none
- scenarios: 4 mapped to deleted files
- rust code: none found: grepped `/anchors`, `ErrorResolution`, `AnchorPoint` (gone from the Rust TUI and sessions; slash registry rust/fspec-tui/src/views/agent/slash_commands.rs has no anchors)
- rust tests: none
- covered: none
- not covered: all 4 -> retire
- action: @deprecated citing CMPCT-013, commit ab07e96be (anchor detection and the `/anchors` viewer removed). Same decision as fix-anchor-viewer-integration-with-thinking-dialog-system.

#### zustand-state-management-setup-for-fspec-data
- class: retire
- area: none
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-tui/src/store/board.rs (BoardStore groups daemon-pushed work units into 7 columns; no epics list, no Immer)
- rust tests: rust/fspec-tui/tests/store_board_unit_rpc012.rs (owner: rpc012-board-store)
- covered: none (Rust loading and per-status grouping are already owned by rpc012-board-store)
- not covered: all 5 -> retire (Zustand/Immer store, epics.json loading, and useWorkUnitsByEpic selector are TS-only)
- action: @deprecated citing 103a73072 (TS client removed) and RPC-012 (the Rust BoardStore replaces the Zustand store).

#### create-isolated-session-napi
- class: retire
- area: none
- scenarios: 5 mapped to deleted files
- rust code: the NAPI binding is gone (rust/napi/src/git.rs deleted in f25b90b2e). Worktree creation lives in rust/git (worktree/isolated-session code).
- rust tests: rust/git/tests/isolated_session_tests.rs (owner: isolated-session-creation); rust/git/tests/worktree_tests.rs (owner: git-worktree-creation-for-backgroundsession-isolation)
- covered: none (the git-level behaviour is already owned by isolated-session-creation / session-list-inspect)
- not covered: all 5 -> retire
- action: @deprecated citing REFAC-027 (f25b90b2e, codelet-napi crate removed). Sibling isolated-session-napi-bindings gets the same decision.

### Batch COV-073-B10

#### provider-type-consolidation
- class: retire
- area: none
- scenarios: 4 mapped to deleted files
- rust code: none found: TS-only refactor (src/tui/types/provider.ts, AgentView.tsx imports, npm build)
- rust tests: none
- covered: none
- not covered: all 4 -> retire
- action: @deprecated citing 103a73072 (TS client and build infrastructure removed).

#### isolated-session-napi-bindings
- class: retire
- area: none
- scenarios: 6 mapped to deleted files
- rust code: the NAPI bindings were deleted with rust/napi/src/git.rs (f25b90b2e). The underlying list/inspect/merge/discard/prune logic lives in rust/git and is exposed over RPC (RPC-057).
- rust tests: rust/git/tests/session_list_inspect_tests.rs (owner: session-list-inspect); session_merge_tests.rs (owner: session-merge-operations); session_discard_tests.rs (owner: session-discard-operations); session_orphan_pruning_tests.rs (owner: session-orphan-pruning)
- covered: none (the git behaviour is already owned by the 4 features above)
- not covered: all 6 -> retire
- action: @deprecated citing REFAC-027 (f25b90b2e). Same decision as create-isolated-session-napi.

#### scaffold-tui-infrastructure-with-ink-and-cage-components
- class: retire
- area: none
- scenarios: 5 mapped to deleted files
- rust code: none found: Ink FullScreenLayout, ViewManager, Zustand+Immer, ink-testing-library, InputModeContext/useSafeInput are TS-only
- rust tests: none
- covered: none
- not covered: all 5 -> retire
- action: @deprecated citing 103a73072 (the Ink/React TUI was replaced by the ratatui TUI in rust/fspec-tui).

#### compaction-badge-sign-integrity-ink
- class: retire
- area: none
- scenarios: 3 mapped to deleted files
- rust code: the TS Ink SessionHeader is gone. The Rust twin behaviour (writer clamp, verbatim render) is owned by compaction-badge-sign-integrity
- rust tests: tests/compaction_badge_sign_integrity_cmpct040.rs (owner: compaction-badge-sign-integrity)
- covered: none (all scenarios are about TS writers and the Ink header)
- not covered: all 3 -> retire
- action: @deprecated citing 103a73072 (TS Ink client removed). The Rust twin compaction-badge-sign-integrity (CMPCT-040) owns the equivalent behaviour

#### refactor-header-and-details-to-component-based-flexbox
- class: retire
- area: none (Ink layout internals)
- scenarios: 14 mapped to deleted files
- rust code: the visible behaviour lives in rust/fspec-tui/src/views/board/checkpoint_status.rs:41-48 ("Checkpoints: None" / "N Manual, M Auto"), details_strip.rs:9 ("No work unit selected"), header.rs, borders.rs, viewport.rs
- rust tests: rust/fspec-tui/tests/view_board_unit_rpc015.rs (owner: rpc015-board-header); rust/fspec-tui/tests/view_board_unit_rpc014.rs (owner: rpc014-board-grid); rust/fspec-tui/tests/view_board_unit_rpc016.rs (owner: rpc016-board-viewport)
- covered: none in this feature's terms (Box borderStyle / flexGrow / rows.push / calculateViewportHeight are Ink-only)
- not covered: all 14 -> retire
- action: @deprecated: this was an Ink component/flexbox refactor (inherently TypeScript; the Ink TUI was removed in 103a73072). The visible text, borders and viewport behaviour are already owned by rpc015-board-header, rpc014-board-grid and rpc016-board-viewport.
- question: -

#### remove-dead-split-view
- class: retire
- area: none (TypeScript cleanup)
- scenarios: 3 mapped to deleted files
- rust code: none (the scenarios grep src/, delete .tsx test files and run npm build/test)
- rust tests: none
- covered: none
- not covered: all 3 -> retire
- action: @deprecated: the TypeScript-only removal card is done (TUI-080, 17d239810; its own test was deleted in TUI-083, 81c3c239a) and src/ is gone (103a73072). It describes npm build/test steps. Siblings: cross-pane-selection-with-correlation-ids, discuss-selected-feature, watcher-split-view-ui.
- question: -

#### auto-inject-toggle-in-watcher-creation-dialog
- class: retire
- area: none (supervisor UI removed)
- scenarios: 5 mapped to deleted files
- rust code: none found: there is no creation dialog or auto-inject field in rust/fspec-tui. Only the leftover StreamChunk::SupervisorPendingInjection is at rust/rpc-types/src/stream_chunk_constructors.rs:151.
- rust tests: none
- covered: none
- not covered: all 5 -> retire
- action: @deprecated citing AMGR-008 (a9d161f9b removed SupervisorCreateView/TemplateForm/TemplateList and the supervisor loop; the AgentManager tool replaces them). The TS test survived only as a NAPI-mock test until 103a73072. Siblings: watcher-creation-dialog-ui, watcher-management-overlay-ui, watcher-templates.
- question: -

#### watcher-creation-dialog-ui
- class: retire
- area: none (supervisor UI removed)
- scenarios: 6 mapped to deleted files
- rust code: none found: there is no supervisor/watcher creation dialog or N key in rust/fspec-tui (grepped supervisor/template/watcher)
- rust tests: none
- covered: none
- not covered: all 6 -> retire
- action: @deprecated citing AMGR-008 (a9d161f9b; RoleAuthority/authority toggle was already removed by WATCH-024). Siblings as above.
- question: -

#### watcher-management-overlay-ui
- class: retire
- area: none (supervisor UI removed)
- scenarios: 10 mapped to deleted files
- rust code: none found: there is no supervisor management overlay in rust/fspec-tui. Supervisor links are display-only (rust/fspec-tui/src/app/dispatch_supervisor_links.rs, RPC-061).
- rust tests: none
- covered: none
- not covered: all 10 -> retire
- action: @deprecated citing AMGR-008 (a9d161f9b removed the supervisor management UI; no supervisor components remained at 103a73072^, and the TS test only mocked NAPI). Siblings as above.
- question: -

#### watcher-split-view-ui
- class: retire
- area: none (split view removed)
- scenarios: 8 mapped to deleted files
- rust code: none found: there is no split-pane session view (subordinates open as their own AgentView)
- rust tests: none
- covered: none
- not covered: all 8 -> retire
- action: @deprecated citing TUI-080 (17d239810 deleted SplitSessionView.tsx and watcher-split-view.test.tsx: "subordinates use their own session view now"). Siblings: cross-pane-selection-with-correlation-ids, discuss-selected-feature, remove-dead-split-view.
- question: -

#### watcher-templates
- class: retire
- area: none (supervisor UI removed)
- scenarios: 24 mapped to deleted files
- rust code: none found: there are no supervisor templates, template storage or quick-spawn slash command in rust/
- rust tests: none
- covered: none
- not covered: all 24 -> retire
- action: @deprecated citing AMGR-008 (a9d161f9b deleted SupervisorTemplateList/Form and supervisorTemplateStorage; the test was deleted in TUI-083, 81c3c239a). Siblings as above.
- question: -

#### worker-module-not-bundled-module-not-found-for-diff-worker-js
- class: retire
- area: none (Node bundling)
- scenarios: 3 mapped to deleted files
- rust code: none (Node worker_threads path resolution; Rust diffs run natively)
- rust tests: none
- covered: none
- not covered: all 3 -> retire
- action: @deprecated: inherently Node bundling. fa07d249a ("migrate to node SEA build with native Rust diff operations") and 103a73072 removed the TS client.
- question: -

#### sgr-mouse-protocol
- class: retire
- area: none
- scenarios: 7 mapped to deleted files
- rust code: rust/fspec-tui/src/mouse/mod.rs:12-15 — "replaces the TypeScript src/tui/utils/mouseProtocol.ts SGR parser (crossterm does the parsing for us)"; capture is crossterm EnableMouseCapture/DisableMouseCapture (rust/fspec-tui/src/mouse/toggle.rs:27)
- rust tests: none for SGR parsing (crossterm's job); mouse dispatch is owned by app-mouse-dispatch (rust/fspec-tui/tests/app_mouse_dispatch_rpc023.rs)
- covered: none
- not covered: all 7 -> retire (Ink 6.8.0 parser workaround)
- action: @deprecated citing RPC-023 (crossterm mouse handling replaced mouseProtocol.ts) and 103a73072.

#### refactor-watcher-terminology-to-supervisor-subordinate-with-chainofcommand-graph
- class: retire
- area: sessions
- scenarios: 16 mapped to deleted files
- rust code: the Rust rename landed in 8de0ca0c1: codelet_sessions::ChainOfCommand, SupervisorPendingInjection, and no RoleAuthority remain. The TS components, NAPI bindings, template form/storage and the /supervisor slash command do not exist in Rust (slash registry rust/fspec-tui/src/views/agent/slash_commands.rs has no supervisor/watcher).
- rust tests: rust/sessions/tests/cov057_watchgraph_relationships.rs (owner: watchgraph-and-session-relationship-tracking); rust/sessions/tests/cov057_watcher_injection_format.rs (owner: watcher-injection-message-format)
- covered: none as an owner; ChainOfCommand behaviour and the injection prefix are already proved by the two COV-057 features above
- not covered: all 16 -> retire (rename/code-shape scenarios; TS and NAPI parts deleted)
- action: @deprecated citing 103a73072 (TS) and REFAC-027 f25b90b2e (NAPI). The Rust rename is complete (8de0ca0c1), and its behaviour is owned by watchgraph-and-session-relationship-tracking and watcher-injection-message-format.

#### codex-subscription-model-filtering
- class: retire
- area: sessions
- scenarios: 6 mapped to deleted files
- rust code: none found: the Codex allowlist (rust/sessions/src/codex_allowlist.rs, rust/sessions/data/codex-models.json) was deleted in dd8da6684 ("chore: fix openai model list"); rust/sessions/src/handle_impl/handle_impl_5.rs:111 now says "no allowlist filtering", and so does codex-section-synthesis.feature:9
- rust tests: rust/sessions/tests/prov146_openai_cloud_excluded.rs:307-310 asserts the full catalog under Codex with no allowlist filter (owner: openai-cloud-catalog-excluded-from-openai-api-provider-section)
- covered: none
- not covered: all 6 -> retire (allowlist removed); "No Codex OAuth shows full unfiltered catalog" is also contradicted by PROV-146
- action: @deprecated citing dd8da6684 (codex allowlist removed) and PROV-129 codex-section-synthesis ("no allowlist filtering").

