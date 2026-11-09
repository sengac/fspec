# COV-074 plan: review utils, hooks, git, research and server features against the Rust code

Read-only prep, 2026-11-06, against HEAD 768f804b1 (other helpers' uncommitted work in the tree included). No repo files were changed. Feature list computed from every `spec/features/*.feature.coverage` `testMappings[].file` that no longer exists and is not a `rust/` path. Per-feature classification by read-only subagents (raw output, inputs, scripts: /tmp/asst/prep/cov07x-work/, chunks COV-074-c*), spot-checked by the coordinator.

## Totals

- Features still mapping a deleted TypeScript-era test file: **123** (733 scenarios). Card text said 124 (736 scenarios).
- 123 features (card said 124). The 1 dropped is an orphan coverage file: add-kotlin-and-dart-language-support-to-ast-research-tool.feature.coverage (added in 3873a4e60 with no feature file). Action: delete it (spec-only). Its behaviour (ast research tool) retires with CLI-015 anyway.
- Partly mapped: ansi-escape-codes-and-tui-content-leaking-into-fspec-tool-call-results 4/5, block-notifications 1/2 (1 live file), blocklist-core-command-tool-filtering 3/3 (1 live file), implement-file-locking-system-lockedfilemanager-refactoring 1/21, isolated-session-worktree-initialization 3/4 (1 live file), provider-configuration-and-credentials-management 19/20, support-multiple-ai-agents-beyond-claude 3/16, wire-lockedfilemanager-errors-to-winston-logger 2/5.
- Non-src paths: 12 features map extension/src/** (removed 14cb69eb6); 3 map e2e/** (removed 103a73072); demo-scripts-incorrectly-try-to-import-non-existent-testcontroller-class maps ../fspec.videos/src/** (a separate repo, not on this machine).

| class | features |
|---|---|
| remap | 9 |
| write | 31 |
| merge | 18 |
| retire | 46 |
| human | 19 |
| **total** | **123** |

Most `merge` features also need a few new tests: scenarios the owner feature does not yet prove are moved into the owner feature with a test in the owner's file (see each block's "not covered"). Many features of every class also carry single scenarios to delete or @deprecate.

### (human) questions (19 features)

Recommended answer first in each. Features that hinge on the same product question are grouped under one question; each feature's own wording is in its block below.

**H-RESEARCH. Research tool execution was never ported (RPC-286 list-only)** (confluence-research-tool-integration, jira-research-tool-integration, perplexity-research-tool-integration, interactive-research-command-with-multiple-backend-support, stakeholder-communication-research-script-for-unanswered-questions, fix-fspec-research-tool-ast-help-command-should-display-tool-specific-help-instead-of-failing, jira-research-tool-outputs-object-object-in-markdown-format-for-description-field, configuration-management-with-tui-integration, enhanced-research-tool-guidance-in-system-reminders)
> Rust `fspec research` only lists tools: running a known tool returns 'not yet ported' (rust/fspec-core/src/commands/research.rs:7,203; RPC-286 ported LIST mode only) and CLI-015 removed the `ast` research tool in favour of `fspec astgrep`. No card exists to port execution. (a, recommended) Deprecate research execution: mark the execution/auto-attach/per-tool-help/research-reminder features and scenarios @deprecated citing RPC-286 (plus CLI-015 where they push the ast tool), and merge their list-mode/config duplicates into research-rust-port / research-cli-subcommand. (b) Keep them and open a card to port research tool execution. Sub-question for tool-discovery-and-status-display (RES-010): Rust lists tools with ✓/✗ and dropped the CONFIGURED/NOT CONFIGURED labels and the ENV/USER/PROJECT config source; (a, recommended) rewrite RES-010 to the Rust output and add ENV/user-config resolution tests, (b) restore the labels, (c) deprecate RES-010.

**H-HOOKS. Global command hooks in spec/fspec-hooks.json never run** (hook-condition-evaluation, hook-configuration-schema-and-validation, hook-discovery-and-event-naming, hook-execution-engine, system-reminder-formatting-for-ai, integrate-hooks-into-all-commands, hook-system-documentation-and-examples)
> Rust never executes the pre-/post-<command> hooks configured in spec/fspec-hooks.json (rust/core/src/lifecycle_hooks/config.rs:15: fspec CLI events are "ignored by Rust engine"), although add-hook, list-hooks, remove-hook and validate-hooks were ported and still manage that file; only work-unit virtual pre-<status> hooks run (update_work_unit_status/board_state.rs:70). (a, recommended) Open a card to port command-hook execution and keep HOOK-002/003/004/005/006/008 for it. (b) @deprecate the command-hook features citing 103a73072 and fix the hook help text. For HOOK-009 (hook docs, moved to fspec.dev in 051c9db83, examples target hooks Rust never runs): (a, recommended) @deprecated citing 051c9db83, or (b) keep with a Rust test over examples/hooks/.

**H-VIRTUAL-HOOK-CONTEXT. Virtual hooks never receive git context (and post-status virtual hooks never run)** (work-unit-scoped-hooks-for-dynamic-validation)
> Rust stores `gitContext: true` and generates the script, but never passes staged/unstaged files to virtual hooks (virtual_hooks_exec.rs:65 "todo: gitContext support"). Should the two git-context execution scenarios be (a) kept, with a card to implement gitContext execution [recommended, since add-virtual-hook advertises --git-context], or (b) deleted, with --git-context documented as script-generation only? Coordinator note for the same card: virtual_hooks_exec.rs:34 run_for_transition (post-<status> virtual hooks) has no caller anywhere, so post-* virtual hooks never run either; only pre-* hooks run (board_state.rs:70).

**H-MIGRATE. No migration system in Rust** (build-migration-system)
> Rust has no migration system or `fspec migrate`: (a) @deprecated citing 103a73072 [recommended], or (b) keep it and open a card to port migrations?

**H-AGENT-RUNTIME. Agent-aware status/validate output (INIT-008)** (agent-runtime-detection-for-context-aware-cli-output)
> Rust formats status/validate output the same for every agent (always <system-reminder>); per-agent formatting only exists in review/update-foundation: (a) rewrite INIT-008 (merge the init scenario into init-rust-port, retarget env-override at review, delete the Cursor/Aider/plain-default scenarios) [recommended], or (b) keep it and open a card to port agent-aware status/validate output?

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
- deleted `src/commands/__tests__/init-multi-agent.test.ts`: spec-agent-md-files-contain-stub-instead-of-comprehensive-workflow (COV-074 write, COV-074-B1); support-multiple-ai-agents-beyond-claude (COV-074 remap, COV-074-B1)
- deleted `src/commands/__tests__/research-help.test.ts`: fix-fspec-research-tool-ast-help-command-should-display-tool-specific-help-instead-of-failing (COV-074 human, COV-074-B5); tool-specific-help-system (COV-072 human, COV-072-B2)
- deleted `src/generators/__tests__/foundation-md.test.ts`: bounded-context-map-ui-duplicates-each-bounded-context-name-in-visualization (COV-074 write, COV-074-B2); enhance-foundation-md-event-storm-visualization (COV-074 write, COV-074-B2)
- deleted `src/generators/__tests__/generate-event-storm-section-in-foundation-md.test.ts`: generate-event-storm-section-in-foundation-md-with-validated-mermaid-diagram (COV-074 write, COV-074-B2); mermaid-validation-test-uses-invalid-test-data (COV-074 write, COV-074-B2)
- deleted `src/git/__tests__/status.test.ts`: changed-files-view-missing-unstaged-files (COV-074 merge, COV-074-B4); replace-git-cli-usage-with-isomorphic-git-library (COV-074 retire, COV-074-B4)
- deleted `src/research-tools/__tests__/multi-language-support.test.ts`: add-java-php-ruby-bash-support-to-ast-research-tool (COV-074 retire, COV-074-B6); add-multi-language-support-to-ast-research-tool-kotlin-dart-python-go-rust-swift-c-c-c (COV-074 retire, COV-074-B6)
- deleted `src/utils/__tests__/activationMessage.test.ts`: agent-specific-activation-message-not-customized-in-fspec-init-success-output (COV-074 write, COV-074-B1); codex-init-writes-prompt-to-home-directory (COV-074 write, COV-074-B1)
- deleted `src/utils/__tests__/ensure-files.test.ts`: automatic-json-file-initialization (COV-071 merge, COV-071-B1); work-units-json-not-stamped-with-current-version-on-initial-creation (COV-074 write, COV-074-B3)
- deleted `src/utils/__tests__/foundation-check.test.ts`: foundation-existence-check-in-commands (COV-072 write, COV-072-B4); foundation-missing-error-message-is-not-imperative-enough (COV-074 write, COV-074-B2)
- deleted `src/utils/__tests__/templateTransformation.test.ts`: fix-multi-agent-support-critical-issues (COV-072 merge, COV-072-B5); support-multiple-ai-agents-beyond-claude (COV-074 remap, COV-074-B1)


Rust test files that features in more than one card will edit (all are owned by a `-rust-port`/`-cli-subcommand` twin or a later Rust feature; the features merge into that owner). Run the batches that touch the same file one after another, never in parallel:

- `rust/fspec-core/tests/add_attachment.rs`: attachment-support-for-discovery-process (COV-072-B1); mermaid-validation-fails-with-dompurify-error (COV-074-B1); validate-mermaid-diagrams-in-attachments (COV-072-B1); work-unit-attachments (COV-072-B1)
- `rust/fspec-core/tests/update_work_unit_status.rs`: clarify-backward-movement-in-workflow-documentation (COV-072-B3); coverage-file-synchronization (COV-074-B3); example-mapping-integration (COV-072-B3); feature-file-prefill-detection (COV-072-B3); kanban-workflow-state-management (COV-072-B3); parent-work-units-shouldn-t-require-scenarios (COV-072-B3); prevent-retroactive-state-walking-enforce-temporal-ordering (COV-072-B3); virtual-hooks-don-t-execute-during-status-transitions (COV-072-B3); work-item-types-for-stories-tasks-and-bugs (COV-072-B3); work-unit-dependency-management (COV-071-B1)
- `rust/git/tests/changed_files_rpc355.rs`: changed-files-view-doesn-t-show-deleted-files-correctly (COV-073-B7); changed-files-view-missing-unstaged-files (COV-074-B4)

- Coordinator-verified: virtual_hooks_exec.rs:34 run_for_transition has no caller (rg finds none), so post-<status> virtual hooks never run; fold into H-VIRTUAL-HOOK-CONTEXT.
- Found while classifying (worth bug cards): bootstrap_doc.txt:325 tells agents to run `fspec derive-tags-from-foundation`, which never existed; Codex init reads only HOME and falls back to "." (init/files.rs:45), so on Windows the prompt lands in the project; FspecTool::call is a stub that only returns an error (the working path is FspecToolFacadeWrapper).

## Batches

Batches are connected components over target test files (and shared deleted TS files) inside this card, packed by area, 10-20 features each. No target test file appears in two batches of this card. "Spec-only" batches touch no Rust test file (retire, fully-covered merges, and human features whose fix is spec-only); they can run in parallel with anything.

### COV-074-B1: fspec-core utils (schema, project root, step, similarity, mermaid), attachment-viewer (15 features; human 1, merge 4, remap 5, write 5)

Blocked in part on the (human) answers for: agent-runtime-detection-for-context-aware-cli-output.

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| fullscreen-mermaid-diagram-viewer-with-zoom-and-pan-controls | merge | rust/attachment-viewer/tests/viewer_mermaid_fullscreen.rs | rust-viewer-mermaid-fullscreen |
| mermaid-diagram-zoom-not-centered-on-mouse-and-trackpad-causes-jittering | merge | rust/attachment-viewer/tests/viewer_mermaid_fullscreen.rs | rust-viewer-mermaid-fullscreen |
| font-size-controls-for-attachment-viewer | merge | rust/attachment-viewer/tests/viewer_prism_theme_fonts.rs | rust-viewer-prism-theme-fonts |
| improve-scenario-similarity-matching-accuracy | write | rust/fspec-core/src/commands/generate_scenarios/similarity_tests.rs | - |
| support-multiple-ai-agents-beyond-claude | remap | rust/fspec-core/src/commands/init/tests.rs | - |
| spec-agent-md-files-contain-stub-instead-of-comprehensive-workflow | write | rust/fspec-core/tests/bug028_agent_doc_comprehensive.rs | root-stub scenario @deprecated: 8fcf476b7 |
| step-validation-hardcoded-to-javascript-comment-syntax-should-be-language-agnostic | write | rust/fspec-core/src/commands/link_coverage/step.rs | S8 @deprecated BUG-499 |
| design-generic-foundation-schema | remap | rust/fspec-core/src/generators/foundation_schema.rs, rust/fspec-core/tests/design_generic_foundation_schema.rs | - |
| project-root-detection | remap | rust/fspec-core/src/io/project_root.rs | - |
| generate-foundation-md-fails-with-mermaid-jsdom-parentrule-error-when-bounded-contexts-exist | remap | rust/fspec-core/src/utils/mermaid_validation.rs | - |
| validate-json-schema | remap | rust/fspec-core/src/validators/json_schema.rs, rust/fspec-core/src/generators/tags_schema.rs | validate-foundation-schema-rust-port (S5/S6 duplicates) |
| mermaid-validation-fails-with-dompurify-error | merge | rust/fspec-core/tests/add_attachment.rs | add-attachment-rust-port |
| codex-init-writes-prompt-to-home-directory | write | rust/fspec-core/tests/agent018_codex_home_prompt.rs | - |
| agent-specific-activation-message-not-customized-in-fspec-init-success-output | write | rust/fspec-core/tests/agent_specific_activation_message.rs, rust/fspec-core/src/commands/init/activation_message_tests.rs | - |
| agent-runtime-detection-for-context-aware-cli-output | human | rust/fspec-core/tests/agent_runtime_detection.rs | init-rust-port (init-writes-config dup) |

Test files touched: `rust/attachment-viewer/tests/viewer_mermaid_fullscreen.rs`, `rust/attachment-viewer/tests/viewer_prism_theme_fonts.rs`, `rust/fspec-core/src/commands/generate_scenarios/similarity_tests.rs`, `rust/fspec-core/src/commands/init/activation_message_tests.rs`, `rust/fspec-core/src/commands/init/tests.rs`, `rust/fspec-core/src/commands/link_coverage/step.rs`, `rust/fspec-core/src/generators/foundation_schema.rs`, `rust/fspec-core/src/generators/tags_schema.rs`, `rust/fspec-core/src/io/project_root.rs`, `rust/fspec-core/src/utils/mermaid_validation.rs`, `rust/fspec-core/src/validators/json_schema.rs`, `rust/fspec-core/tests/add_attachment.rs`, `rust/fspec-core/tests/agent018_codex_home_prompt.rs`, `rust/fspec-core/tests/agent_runtime_detection.rs`, `rust/fspec-core/tests/agent_specific_activation_message.rs`, `rust/fspec-core/tests/bug028_agent_doc_comprehensive.rs`, `rust/fspec-core/tests/design_generic_foundation_schema.rs`

### COV-074-B2: fspec-core FOUNDATION.md event storm, checkpoints, prefill, hook config (H-HOOKS) (15 features; human 5, write 10)

Blocked in part on the (human) answers for: hook-configuration-schema-and-validation, hook-discovery-and-event-naming, hook-condition-evaluation, hook-system-documentation-and-examples, hook-execution-engine.

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| update-agents-md-with-big-picture-event-storming-workflow-documentation | write | rust/fspec-core/tests/big_picture_event_storm_docs.rs | - |
| prefill-detection-incorrectly-flags-code-syntax-as-placeholders | write | rust/fspec-core/tests/bug079_prefill_false_positives.rs | - |
| bounded-context-map-ui-duplicates-each-bounded-context-name-in-visualization | write | rust/fspec-core/tests/bug085_bounded_context_map_labels.rs | - |
| enhance-foundation-md-event-storm-visualization | write | rust/fspec-core/tests/found042_foundation_md_event_storm_sections.rs | - |
| checkpoint-creation-fails-when-deleted-files-exist | write | rust/fspec-core/tests/checkpoint_deleted_files.rs | - |
| documentation-clarity-improvements | write | rust/fspec-core/tests/documentation_clarity_improvements.rs | - |
| generate-event-storm-section-in-foundation-md-with-validated-mermaid-diagram | write | rust/fspec-core/tests/found017_event_storm_section_mermaid.rs | - |
| mermaid-validation-test-uses-invalid-test-data | write | rust/fspec-core/tests/test009_mermaid_injection_rejected.rs | - |
| complete-big-picture-event-storm-schema-integration | write | rust/fspec-core/tests/foundation_event_storm_schema.rs | - |
| foundation-missing-error-message-is-not-imperative-enough | write | rust/fspec-core/tests/foundation_missing_error_message.rs | - |
| hook-configuration-schema-and-validation | human | rust/fspec-core/tests/hook002_command_hook_config.rs | scenario 5 -> validate-hooks-rust-port |
| hook-discovery-and-event-naming | human | rust/fspec-core/tests/hook004_discovery_event_naming.rs | - |
| hook-condition-evaluation | human | rust/fspec-core/tests/hook005_condition_evaluation.rs | - |
| hook-system-documentation-and-examples | human | rust/fspec-core/tests/hook009_hook_examples.rs | @deprecated: 051c9db83 (if a) |
| hook-execution-engine | human | rust/fspec-core/tests/hook_execution_engine.rs | - |

Test files touched: `rust/fspec-core/tests/big_picture_event_storm_docs.rs`, `rust/fspec-core/tests/bug079_prefill_false_positives.rs`, `rust/fspec-core/tests/bug085_bounded_context_map_labels.rs`, `rust/fspec-core/tests/checkpoint_deleted_files.rs`, `rust/fspec-core/tests/documentation_clarity_improvements.rs`, `rust/fspec-core/tests/found017_event_storm_section_mermaid.rs`, `rust/fspec-core/tests/found042_foundation_md_event_storm_sections.rs`, `rust/fspec-core/tests/foundation_event_storm_schema.rs`, `rust/fspec-core/tests/foundation_missing_error_message.rs`, `rust/fspec-core/tests/hook002_command_hook_config.rs`, `rust/fspec-core/tests/hook004_discovery_event_naming.rs`, `rust/fspec-core/tests/hook005_condition_evaluation.rs`, `rust/fspec-core/tests/hook009_hook_examples.rs`, `rust/fspec-core/tests/hook_execution_engine.rs`, `rust/fspec-core/tests/test009_mermaid_injection_rejected.rs`

### COV-074-B3: fspec-core reminders, hooks formatting, research, search, version stamp, help interceptor (15 features; human 4, merge 3, write 8)

Blocked in part on the (human) answers for: system-reminder-formatting-for-ai, configuration-management-with-tui-integration, work-unit-scoped-hooks-for-dynamic-validation, integrate-hooks-into-all-commands.

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| system-reminder-formatting-for-ai | human | rust/fspec-core/tests/hook_system_reminder_formatting.rs | - |
| mermaid-validation-using-parse-instead-of-render-misses-semantic-errors | write | rust/fspec-core/tests/mermaid_attachment_semantic_validation.rs | - |
| improve-step-workflow-clarity-to-prevent-wrong-test-file-and-missing-comments | write | rust/fspec-core/tests/refac003_step_workflow_clarity.rs | - |
| configuration-management-with-tui-integration | human | rust/fspec-core/tests/research.rs | research-rust-port |
| search-scenarios-returns-no-results-when-it-should-find-matching-scenarios | merge | rust/fspec-core/tests/search_scenarios.rs | search-scenarios-rust-port |
| duplicate-work-unit-ids-in-state-arrays-when-moving-backward-or-to-same-state | write | rust/fspec-core/tests/state_arrays_no_duplicates.rs | - |
| step-validation-system-reminder-doesn-t-explain-proper-comment-placement-and-deduplication | write | rust/fspec-core/tests/step_validation_reminder_guidance.rs | - |
| system-reminder-anti-drift-pattern | write | rust/fspec-core/tests/system_reminder_anti_drift.rs | - |
| coverage-file-synchronization | merge | rust/fspec-core/tests/update_work_unit_status.rs | delete-scenario-rust-port,coverage-stats-after-scenario-deletion,generate-coverage-rust-port,update-scenario-rust-port,update-work-unit-status-rust-port |
| work-unit-scoped-hooks-for-dynamic-validation | human | rust/fspec-core/tests/virtual_hooks_reminders_and_display.rs | scenario 2 -> virtual-hooks-don-t-execute-during-status-transitions; 5 -> copy-virtual-hooks-rust-port; 10 -> add-/remove-virtual-hook-rust-port |
| work-units-json-not-stamped-with-current-version-on-initial-creation | write | rust/fspec-core/tests/work_units_version_stamp.rs | - |
| unified-shift-arrow-navigation-across-boardview-agentview-and-splitpaneview | merge | rust/fspec-tui/tests/shift_arrow_end_of_list_rpc096.rs | agentview-shift-arrow-end-of-list-parity + agentview-shift-right-create-session-dialog; 7 watcher/split/parent scenarios @deprecated: BUG-124 |
| command-help-system | write | rust/fspec/tests/cli_help_interceptor.rs | remove-tag-from-scenario-cli-subcommand,tag-stats-cli-subcommand (S1/S4 duplicates) |
| integrate-hooks-into-all-commands | human | rust/fspec/tests/cli_integrate_hooks_into_commands.rs | - |
| model-selector-keyboard-navigation-e2e | write | rust/fspec/tests/model_nav_pty_prov104.rs | - |

Test files touched: `rust/fspec-core/tests/hook_system_reminder_formatting.rs`, `rust/fspec-core/tests/mermaid_attachment_semantic_validation.rs`, `rust/fspec-core/tests/refac003_step_workflow_clarity.rs`, `rust/fspec-core/tests/research.rs`, `rust/fspec-core/tests/search_scenarios.rs`, `rust/fspec-core/tests/state_arrays_no_duplicates.rs`, `rust/fspec-core/tests/step_validation_reminder_guidance.rs`, `rust/fspec-core/tests/system_reminder_anti_drift.rs`, `rust/fspec-core/tests/update_work_unit_status.rs`, `rust/fspec-core/tests/virtual_hooks_reminders_and_display.rs`, `rust/fspec-core/tests/work_units_version_stamp.rs`, `rust/fspec-tui/tests/shift_arrow_end_of_list_rpc096.rs`, `rust/fspec/tests/cli_help_interceptor.rs`, `rust/fspec/tests/cli_integrate_hooks_into_commands.rs`, `rust/fspec/tests/model_nav_pty_prov104.rs`

### COV-074-B4: tools/git/fspec CLI: PTY e2e, changed files, checkpoints, worktree, unicode paths, blocklist (15 features; merge 2, remap 4, retire 1, write 8)

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| enter-key-model-selection-end-to-end-in-the-model-view | write | rust/fspec/tests/model_select_pty_prov117.rs | - |
| rpc-074-e2e-clear-no-notice | write | rust/fspec/tests/slash_clear_pty_rpc074.rs | - |
| changed-files-view-missing-unstaged-files | merge | rust/git/tests/changed_files_rpc355.rs | git-change-type-derivation (+ rust-changed-files-view for colours) |
| replace-git-cli-usage-with-isomorphic-git-library | retire | - | @deprecated: 103a73072 + GIT-039 (isomorphic-git removed) |
| checkpoint-restore-shows-file-not-found-but-doesn-t-delete-files-added-after-checkpoint | write | rust/git/tests/checkpoint_restore_deletes_new_files.rs | ghost-commit-checkpoints (S1 duplicate) |
| git-repository-operations | write | rust/git/tests/git013_repository_operations.rs | TS API scenario @deprecated: REFAC-027 + 103a73072 |
| handle-binary-files-and-large-files-in-diff-display | write | rust/git/tests/tui030_diff_binary_and_truncation.rs | - |
| isolated-session-worktree-initialization | remap | rust/git/tests/worktree_index_initialization_tests.rs | - |
| native-rust-diff-operations | merge | rust/rpc/tests/checkpoint_transport_rpc362.rs | git-changed-files-transport + checkpoint-transport; 3 NAPI/worker/esbuild scenarios @deprecated: REFAC-027 + 103a73072 |
| blocklist-core-command-tool-filtering | remap | rust/tools/src/blocklist/mod.rs | - |
| unicode-path-normalization | remap | rust/tools/src/unicode_path.rs | unicode-path-normalization-rust (2 dups); NAPI scenarios @deprecated REFAC-027 |
| block-notifications | remap | rust/tools/tests/block_notifications_integration_test.rs | - |
| blocklist-system-integration-tests | write | rust/tools/tests/blocklist_system_integration.rs, rust/fspec-tui/tests/blocklist_system_integration_view.rs | - |
| core-fspectool-implementation | write | rust/tools/tests/core_fspectool_implementation.rs | - |
| system-reminder-preservation | write | rust/tools/tests/system_reminder_preservation.rs | delete 3 console.error scenarios citing 103a73072/TOOL-019 |

Test files touched: `rust/fspec-tui/tests/blocklist_system_integration_view.rs`, `rust/fspec/tests/model_select_pty_prov117.rs`, `rust/fspec/tests/slash_clear_pty_rpc074.rs`, `rust/git/tests/changed_files_rpc355.rs`, `rust/git/tests/checkpoint_restore_deletes_new_files.rs`, `rust/git/tests/git013_repository_operations.rs`, `rust/git/tests/tui030_diff_binary_and_truncation.rs`, `rust/git/tests/worktree_index_initialization_tests.rs`, `rust/rpc/tests/checkpoint_transport_rpc362.rs`, `rust/tools/src/blocklist/mod.rs`, `rust/tools/src/unicode_path.rs`, `rust/tools/tests/block_notifications_integration_test.rs`, `rust/tools/tests/blocklist_system_integration.rs`, `rust/tools/tests/core_fspectool_implementation.rs`, `rust/tools/tests/system_reminder_preservation.rs`

### COV-074-B5: spec-only: thinking keywords, viewer, config, help merges, research human questions (16 features; human 7, merge 5, retire 4, spec-only)

Blocked in part on the (human) answers for: fix-fspec-research-tool-ast-help-command-should-display-tool-specific-help-instead-of-failing, confluence-research-tool-integration, jira-research-tool-integration, perplexity-research-tool-integration, interactive-research-command-with-multiple-backend-support, stakeholder-communication-research-script-for-unanswered-questions, build-migration-system.

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| dynamic-thinking-level-detection-via-keywords | retire | - | @deprecated MODEL-013 (supersedes keyword detection) |
| local-web-server-for-attachment-viewing-with-markdown-and-mermaid-rendering | merge | - | rust-attachment-viewer-server, rust-viewer-prism-theme-fonts |
| anchor-links-not-working-in-markdown-attachment-viewer | merge | - | markdown-heading-anchors |
| shared-configuration-management-utilities-for-user-and-project-config-files | merge | - | shared-config-merge |
| fspec-help-displays-hardcoded-version-0-0-1-instead-of-package-json-version | merge | - | fspec-binary-cargo-shape-rpc010 |
| commonpatterns-help-formatting | merge | - | add-virtual-hook-cli-subcommand,add-command-cli-subcommand (help-fixture tests) |
| fix-fspec-research-tool-ast-help-command-should-display-tool-specific-help-instead-of-failing | human | - | @deprecated: CLI-015 + RPC-286 (if a) |
| confluence-research-tool-integration | human | - | - |
| jira-research-tool-integration | human | - | - |
| perplexity-research-tool-integration | human | - | - |
| interactive-research-command-with-multiple-backend-support | human | - | research-rust-port / research-cli-subcommand (list + invalid-tool scenarios) |
| stakeholder-communication-research-script-for-unanswered-questions | human | - | @deprecated: RPC-286 (if a) |
| build-migration-system | human | - | - |
| ast-research-tool-fails-with-file-is-required-error-even-when-file-flag-is-provided | retire | - | @deprecated citing CLI-015 (research registry drops the ast tool) |
| refactor-ast-tool-to-use-deterministic-tree-sitter-queries-instead-of-semantic-natural-language-parsing | retire | - | @deprecated citing CLI-015 (research registry drops the ast tool) |
| ast-tool-returns-stub-instead-of-actual-parsing | retire | - | @deprecated citing CLI-015 (research registry drops the ast tool) |

### COV-074-B6: spec-only: ast research tool retire (CLI-015), event storm, locking, research guidance (16 features; human 2, merge 1, retire 13, spec-only)

Blocked in part on the (human) answers for: jira-research-tool-outputs-object-object-in-markdown-format-for-description-field, enhanced-research-tool-guidance-in-system-reminders.

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| ast-research-tool-fails-with-relative-file-paths | retire | - | @deprecated citing CLI-015 (research registry drops the ast tool) |
| jira-research-tool-outputs-object-object-in-markdown-format-for-description-field | human | - | - |
| add-kotlin-language-support-to-ast-research-tool | retire | - | @deprecated citing CLI-015 (research registry drops the ast tool) |
| add-java-php-ruby-bash-support-to-ast-research-tool | retire | - | @deprecated citing CLI-015 (research registry drops the ast tool) |
| add-multi-language-support-to-ast-research-tool-kotlin-dart-python-go-rust-swift-c-c-c | retire | - | @deprecated citing CLI-015 (research registry drops the ast tool) |
| event-storm-data-model-for-work-units | merge | - | add-domain-event-rust-port (scenario 1 dup; delete 3 type-only scenarios) |
| enhanced-research-tool-guidance-in-system-reminders | human | - | @deprecated CLI-015 + RPC-286 (if retired) |
| implement-file-locking-system-lockedfilemanager-refactoring | retire | - | @deprecated (scenario) BUG-477 + 103a73072 |
| shift-arrow-session-switching | retire | - | @deprecated: VIEWNV-001 + RPC-096 (wrap-around replaced by Board/CreateDialog ends) |
| refactor-checkpoint-counts-to-be-command-triggered-instead-of-file-watching | retire | - | @deprecated BUG-181 (watcher push replaced IPC) |
| demo-scripts-incorrectly-try-to-import-non-existent-testcontroller-class | retire | - | 103a73072 (TS tui-test tooling, external fspec.videos repo) |
| browser-tab-creation | retire | - | 14cb69eb6 (Chrome extension removed) |
| browser-script-execution | retire | - | 14cb69eb6 (Chrome extension removed) |
| screenshot-native-messaging-resilience | retire | - | 14cb69eb6 (Chrome extension removed) |
| dom-scanning | retire | - | 14cb69eb6 (Chrome extension removed) |
| element-targeted-screenshot | retire | - | 14cb69eb6 (Chrome extension removed) |

### COV-074-B7: spec-only: Chrome extension retire (14cb69eb6) (16 features; retire 16, spec-only)

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| iframe-aware-dom-scanning | retire | - | 14cb69eb6 (Chrome extension removed) |
| advanced-interactivity-heuristics | retire | - | 14cb69eb6 (Chrome extension removed) |
| page-diff | retire | - | 14cb69eb6 (Chrome extension removed) |
| ref-resolution-click-fill | retire | - | 14cb69eb6 (Chrome extension removed) |
| ref-state-management | retire | - | 14cb69eb6 (Chrome extension removed) |
| browser-scan-diff-documentation | retire | - | 14cb69eb6 (Chrome extension removed) |
| webmcp-dynamic-tool-discovery | retire | - | 14cb69eb6 (Chrome extension removed) |
| ast-code-researcher-for-pattern-detection-and-deep-analysis | retire | - | CLI-015 (ast research tool replaced by AstGrep / fspec astgrep) |
| codelet-napi-rs-native-module-bindings | retire | - | REFAC-027 f25b90b2e (codelet-napi crate removed) |
| github-actions-ci-cd-for-codelet-napi-cross-platform-builds | retire | - | REFAC-027 f25b90b2e + 5969cfb58 (workflow removed) |
| jsonl-format-investigation-for-work-units-json-scalability | retire | - | TECH-001 (investigation only, JSONL never adopted; test deleted 67b5947d8) |
| napi-session-co-listener-parity | retire | - | REFAC-027 f25b90b2e (NAPI removed) |
| napi-work-units-watcher-callback | retire | - | REFAC-027 f25b90b2e (NAPI removed) |
| parent-command-for-quick-return | retire | - | WATCH-024 rule 14 (/parent removed) |
| rpc-068-final-ts-regression-and-boundary-audit | retire | - | @deprecated: REFAC-027 (f25b90b2e) + 103a73072 |
| fix-all-failing-tests-15-failures-across-3-test-suites | retire | - | @deprecated: 103a73072 (vitest suite removed) |

### COV-074-B8: spec-only: NAPI / Node tooling / TS-only retire (15 features; merge 3, retire 12, spec-only)

| feature | class | target test files | merge into / retire citation |
|---|---|---|---|
| eliminate-tree-sitter-legacy-peer-deps-requirement | retire | - | @deprecated: 103a73072 (npm build removed) |
| remove-isomorphic-git-dependency | retire | - | @deprecated: 103a73072 + REFAC-027 |
| implement-automated-discovery-code-analysis | retire | - | @deprecated: FOUND-010 (d6d3169ba removed automated detection) |
| migrate-hook-execution-to-use-execa-library | retire | - | @deprecated: 103a73072 (execa/Node removed); executor behaviour owned by agent-lifecycle-hooks |
| napi-binding-setup-for-fspectool | retire | - | @deprecated citing REFAC-027 (f25b90b2e) + TOOL-019 |
| typescript-function-call-integration | retire | - | @deprecated citing REFAC-027 (f25b90b2e) + TOOL-019 |
| ansi-escape-codes-and-tui-content-leaking-into-fspec-tool-call-results | retire | - | @deprecated citing 103a73072 + TOOL-019 |
| capture-all-console-methods-and-redirect-to-winston-logger | retire | - | @deprecated citing 103a73072 (winston/console capture is TS tooling) |
| wire-lockedfilemanager-errors-to-winston-logger | retire | - | @deprecated 103a73072 + 828fe3306 (winston removed) |
| add-winston-universal-logger-for-fspec | retire | - | @deprecated 103a73072 + 828fe3306 (winston removed; Rust uses tracing) |
| sibling-watcher-navigation | retire | - | @deprecated: BUG-124 (navigation flattened; chain_of_command ignored) |
| provider-configuration-display-and-profiles | merge | - | model-selector-profile-server,provider-settings-profile-crud,openai-profile-credential-injection,local-profile-model-probe-resilience,provider-config-profile-persistence,model-selector-section-ordering |
| provider-configuration-and-credentials-management | merge | - | provider-credential-persistence,move-credential-management-to-rust,provider-credential-deletion,provider-settings-credential-masking-with-prefix-aware-mask-helper-and-source-tag,provider-settings-nav-tree-credential-status,rpc054-provider-settings-dispatch,rpc160-provider-settings-tab-switch-to-models,openai-compatible-local-model-support-vllm-ollama; NAPI scenarios @deprecated REFAC-027 |
| background-session-management-with-attach-detach | retire | - | REFAC-027 f25b90b2e (NAPI removed) + BUG-205 (Detach removed) |
| migrate-fspec-research-ast-tool-from-tree-sitter-to-codelet-ast-grep | merge | - | fspec-astgrep-cli-subcommand, ast-code-refactor-tool-for-codelet (dups); NAPI x2 retire REFAC-027 f25b90b2e; package.json retire 103a73072; then @deprecated citing CLI-015 |

## Per-feature decisions

Grouped by batch. Fields: class, area, rust code, rust tests (with current owner), covered, not covered, action, question.

### Batch COV-074-B1

#### fullscreen-mermaid-diagram-viewer-with-zoom-and-pan-controls
- class: merge
- area: attachment-viewer
- scenarios: 15 mapped to deleted files
- rust code: `rust/attachment-viewer/src/markdown/template/mermaid_modal.rs:48,158-159` (mode indicator text "Zoom Mode (hold Space for Pan)" / "Pan Mode", `active` class, opacity 0.5/1). mermaid_modal.rs:174,214 (download name `'mermaid-diagram-' + Date.now() + '.svg'`). `rust/attachment-viewer/src/markdown/template/mermaid_wheel.rs:21-60` (cursor-locked wheel zoom clamped to 0.5..5).
- rust tests: rust/attachment-viewer/tests/viewer_mermaid_fullscreen.rs::each_diagram_gets_wrapper_with_fullscreen_and_download_buttons, ::modal_opens_and_closes_on_esc_and_backdrop, ::zoom_clamped_with_reset_and_live_percentage, ::space_enters_pan_mode_and_svg_downloads_as_blob, ::cursor_centered_wheel_zoom_is_clamped, ::horizontal_scroll_pans_in_zoom_mode, ::mode_indicator_fades_after_inactivity, ::space_toggles_pan_mode_class_on_container (owner: rust-viewer-mermaid-fullscreen)
- covered: Display fullscreen and download buttons on diagram hover; Open diagram in fullscreen modal; Close modal with ESC key; Close modal with backdrop click; Zoom in with mouse wheel; Zoom out with mouse wheel; Pan diagram with horizontal scroll; Switch to pan mode with Space key; Zoom in with zoom button; Reset zoom to default; Enforce maximum zoom limit; Enforce minimum zoom limit
- not covered: Display mode indicator in zoom mode -> write (indicator text, in the owner); Display mode indicator in pan mode -> write ('Pan Mode' + active class + opacity 1, in the owner); Download diagram as SVG -> write (filename pattern mermaid-diagram-{timestamp}.svg, in the owner)
- action: merge into rust-viewer-mermaid-fullscreen. Add the 3 assertions to rust/attachment-viewer/tests/viewer_mermaid_fullscreen.rs as owner scenarios. Consistent with sibling VIEW-002.

#### mermaid-diagram-zoom-not-centered-on-mouse-and-trackpad-causes-jittering
- class: merge
- area: attachment-viewer
- scenarios: 5 mapped to deleted files
- rust code: `rust/attachment-viewer/src/markdown/template/mermaid_wheel.rs:46-60` (locked zoom point at the cursor, percentage-based pan recompute). mermaid_wheel.rs:50 (`-deltaY * (deltaMode === 1 ? 0.05 : deltaMode ? 1 : 0.002)`). mermaid_wheel.rs:16-19 (zoom session lock, ZOOM_SESSION_TIMEOUT_MS = 150; no delta accumulation).
- rust tests: rust/attachment-viewer/tests/viewer_mermaid_fullscreen.rs::cursor_centered_wheel_zoom_is_clamped, ::horizontal_scroll_pans_in_zoom_mode (owner: rust-viewer-mermaid-fullscreen)
- covered: Zoom in centered on cursor position with mouse wheel; Zoom out centered on cursor position with mouse wheel; Horizontal pan with trackpad
- not covered: Handle different mouse wheel deltaMode values -> write (assert the 0.002 / 0.05 / 1 multipliers, in the owner); Smooth zoom with trackpad vertical scroll -> write (assert the 150 ms locked zoom-point session, in the owner; reword the "accumulated deltaY" step, because Rust locks the zoom point and does not accumulate)
- action: merge into rust-viewer-mermaid-fullscreen. Add 2 owner scenarios and tests in rust/attachment-viewer/tests/viewer_mermaid_fullscreen.rs.

#### font-size-controls-for-attachment-viewer
- class: merge
- area: attachment-viewer
- scenarios: 6 mapped to deleted files
- rust code: `rust/attachment-viewer/src/markdown/template/scripts.rs:99-140` (MIN 10, MAX 24, STEP 2, DEFAULT 16, localStorage `fspec-base-font-size`, buttons disabled at the bounds). `rust/attachment-viewer/src/markdown/template/styles.rs:72-75` (`.font-size-controls` fixed at top:1rem; right:5rem).
- rust tests: rust/attachment-viewer/tests/viewer_prism_theme_fonts.rs::page_provides_clamped_font_size_controls (owner: rust-viewer-prism-theme-fonts)
- covered: Font size respects minimum bound; Font size respects maximum bound; Font size persists across browser sessions
- not covered: Default code block font size is larger than current implementation -> write (assert DEFAULT_FONT_SIZE = 16 in the owner); Increase font size with plus button -> write (assert FONT_SIZE_STEP = 2 in the owner); Font size controls display in top right corner -> write (assert the .font-size-controls position in the owner)
- action: merge into rust-viewer-prism-theme-fonts. Add the 3 assertions as owner scenarios and tests in rust/attachment-viewer/tests/viewer_prism_theme_fonts.rs, then drop this feature.

#### improve-scenario-similarity-matching-accuracy
- class: write
- area: fspec-core
- scenarios: 14 mapped to deleted files
- rust code: rust/fspec-core/src/commands/generate_scenarios.rs:713-1120 (private `mod similarity`: hybrid weights 0.3/0.25/0.2/0.15/0.1 :741, SHORT_CONFIG :750, jaro_winkler :835 (empty-empty is 1.0), token_set_ratio :909, trigram :938, jaccard :972 (no tokens gives 0), gherkin_structural :992 (Then ×1.5), strip_keywords :1065, alnum_tokens :1093). Weights are hard-coded ("do not tune"). There is no weight-config API and no legacy Levenshtein mode.
- rust tests: rust/fspec-core/src/commands/generate_scenarios.rs::identical_explicit_gwt_scenarios_match_above_threshold only (owner: none; other inline tests belong to generate-scenarios behaviour)
- covered: none
- not covered: S1-S11 -> write; Hybrid algorithm achieves 88% accuracy -> delete (TS benchmark dataset); Configure algorithm weights dynamically -> delete (Rust weights are fixed constants); Maintain backward compatibility with Levenshtein algorithm -> delete (no legacy mode)
- action: new rust/fspec-core/src/commands/generate_scenarios/similarity_tests.rs (a #[cfg(test)] #[path] child of `mod similarity`, since the functions are private), owned by this feature.

#### support-multiple-ai-agents-beyond-claude
- class: remap
- area: fspec-core (init)
- scenarios: 3 mapped to deleted files (13 others have no mapping at all)
- rust code: rust/fspec-core/src/commands/init/registry.rs:43 (AGENT_REGISTRY with supports_system_reminders, supports_meta_cognition, category, slash_command_format), rust/fspec-core/src/commands/init/templates.rs:110/124 (doc prose groups: plain_cli for aider has meta-cognition removed and plain "**IMPORTANT:**", plain_ide has "**⚠️ IMPORTANT:**")
- rust tests: rust/fspec-core/src/commands/init/tests.rs::ide_agent_uses_emoji_important_prefix, ::registry_contains_claude_and_gemini, ::available_agents_lists_all_registry_agents_in_order, ::meta_cognitive_phrases_removed_for_gemini (owner: none)
- covered: Transform system-reminders for IDE agents with emoji (ide_agent_uses_emoji_important_prefix). Agent registry with all capabilities is partly covered (registry_contains_claude_and_gemini; it needs asserts for supportsSystemReminders, supportsMetaCognition, category=cli and slashCommandFormat=markdown for claude).
- not covered: Install Aider-specific documentation -> write (add an aider case to init/tests.rs. Today the gemini meta-cognition test is the nearest equivalent)
- action: map rust/fspec-core/src/commands/init/tests.rs to this feature (needs @step comments, plus an aider test and stronger registry asserts). Note: fix-multi-agent-support-critical-issues (COV-072) shares templateTransformation.test.ts. Give it a different file so init/tests.rs stays single-owner.

#### spec-agent-md-files-contain-stub-instead-of-comprehensive-workflow
- class: write
- area: fspec-core (init)
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-core/src/commands/init/templates.rs:110 generate_agent_doc embeds rust/fspec-core/src/commands/init_doc_{sysrem_cli,plain_ide,plain_ext,plain_cli}.md (2072-2080 lines, titled "Project Management and Specification Guidelines for fspec", with Reverse ACDD, coverage, hooks and checkpoints sections). rust/fspec-core/src/commands/init.rs:139 writes spec/<DOC_TEMPLATE>. Rust writes no root stub file.
- rust tests: rust/fspec-core/src/commands/init/tests.rs::ide_agent_uses_emoji_important_prefix, placeholders_are_replaced_for_claude (unowned, but assigned to support-multiple-ai-agents-beyond-claude below)
- covered: none in a file this feature would own
- not covered: spec/AGENT.md contains comprehensive Project Management Guidelines -> write, Agent-specific transformations are applied to Project Management template -> write, Root stub files remain concise -> retire (root stub removed in TS 8fcf476b7, "fix(init): remove root stub installation logic"), Fix eliminates circular reference in spec/AGENTS.md -> write
- action: write rust/fspec-core/tests/bug028_agent_doc_comprehensive.rs (init claude/cursor and assert title, sections, cursor transforms, no "See spec/AGENTS.md"). Delete or @deprecated scenario 3 citing 8fcf476b7. Shares init-multi-agent.test.ts with support-multiple-ai-agents-beyond-claude.

#### step-validation-hardcoded-to-javascript-comment-syntax-should-be-language-agnostic
- class: write
- area: fspec-core
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-core/src/commands/link_coverage/step.rs:119-158 (extract_step_comments finds "@step" anywhere on the line, so any comment marker works). step_part :162-165 cuts "*/" only at end of line (BUG-499).
- rust tests: rust/fspec-core/tests/bug509_coverage_step_checks_are_exact.rs::an_exact_step_comment_after_any_documented_comment_marker_covers_the_step (owner: coverage-step-checks-accept-only-exact-step-comments; //, #, --, indentation). rust/fspec-core/tests/bug499_step_comments_keep_block_comment_markers.rs::a_one_line_block_comment_still_matches_without_its_terminator (owner: step-comments-keep-text-after-a-block-comment-marker). step.rs inline exact_step_matches / missing_step_not_matched (owner: none).
- covered: none at extractor level (S1-S4 and S7 are proven end-to-end by the bug509/bug499 owners)
- not covered: S1-S7 (JS, Python, SQL, block, MATLAB %, VB ', mid-line) -> write; Ignore trailing comment delimiters from block comments -> retire (Rust keeps "*/ // trailing comment" in the text on purpose, BUG-499)
- action: write S1-S7 as unit tests in rust/fspec-core/src/commands/link_coverage/step.rs (inline module, unowned; this feature takes ownership). Mark S8 @deprecated citing BUG-499.

#### design-generic-foundation-schema
- class: remap
- area: fspec-core
- scenarios: 10 mapped to deleted files
- rust code: `rust/fspec-core/src/generators/generic-foundation.schema.json` (required version/project/problemSpace/solutionSpace; subFoundations at :114; problem impact/frequency/cost at :205-222; capabilities minItems 1 at :93). `rust/fspec-core/src/generators/foundation_schema.rs:31` validate_foundation (jsonschema crate, Ajv-style `<instancePath>: <message>`).
- rust tests: rust/fspec-core/src/generators/foundation_schema.rs::tests::rejects_missing_required_top_level, ::rejects_empty_capabilities, ::accepts_valid_minimal_foundation (owner: none; unowned; needs @step comments)
- covered: Validate required vs optional sections (rejects_missing_required_top_level + accepts_valid_minimal_foundation)
- not covered: Define foundation schema for web application -> write; Define foundation schema for CLI tool -> write; Define foundation schema for library/SDK -> write; Support multiple problems with impact ratings -> write; Define solution space with high-level capabilities -> write (schema enforces minItems 1 only; drop the "3-7" step); Support hierarchical foundation documents with sub-foundations -> write; Enforce WHY/WHAT boundary (no HOW) -> delete (a schema cannot detect "HOW" prose; Rust has additionalProperties:false only); Preserve Mermaid diagram validation -> delete (mermaid.parse/jsdom is TS; Rust mermaid validation is rust/fspec-core/src/utils/mermaid_validation.rs, unowned, and belongs to a mermaid feature); Use Ajv for JSON Schema validation -> delete (Ajv/TS interfaces are TS tooling; Rust uses the jsonschema crate, foundation_schema.rs:16)
- action: map rust/fspec-core/src/generators/foundation_schema.rs (add @step comments) for the required-sections scenario. Write rust/fspec-core/tests/design_generic_foundation_schema.rs for the 6 write scenarios.

#### project-root-detection
- class: remap
- area: fspec-core
- scenarios: 6 mapped to deleted files
- rust code: rust/fspec-core/src/io/project_root.rs:14-23 (markers, MAX_SEARCH_DEPTH = 10), :37-55 (find_or_create_spec_directory with cwd fallback on create error), :88-130 (upward search for existing spec / nearest marker)
- rust tests: rust/fspec-core/src/io/project_root.rs::creates_spec_at_marker_root_when_found_via_parents, ::creates_spec_at_cwd_when_no_marker_anywhere, ::uses_existing_cwd_spec_when_present (owner: none)
- covered: Create spec at project root when .git marker exists; Fallback to cwd when no project boundary markers found (both need @step comments)
- not covered: Use existing spec directory within project boundary -> write; Handle monorepo with nested package.json correctly -> write; Stop search after maximum directory traversal limit -> write; Gracefully handle permission errors when searching parent directories -> write (create_dir_all failure fallback, :46-53)
- action: map rust/fspec-core/src/io/project_root.rs (inline tests) to this feature and add 4 tests there. The "all existing tests should continue passing" steps are meta; reword or drop them. Feature has no work-unit tag; tag it @COV-074.

#### generate-foundation-md-fails-with-mermaid-jsdom-parentrule-error-when-bounded-contexts-exist
- class: remap
- area: fspec-core
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-core/src/utils/mermaid_validation.rs:37 (validate_mermaid_syntax via merman; no jsdom, so no parentRule)
- rust tests: rust/fspec-core/src/utils/mermaid_validation.rs::accepts_valid_flowchart, ::rejects_unterminated_node_label, ::accepts_generator_event_flow_shape (subgraphs) (owner: none)
- covered: Mermaid validation succeeds with valid diagram after parentRule fix; Mermaid validation still rejects invalid diagrams after parentRule fix; Bounded context mermaid diagram renders successfully. All need @step comments.
- not covered: none
- action: map rust/fspec-core/src/utils/mermaid_validation.rs (inline tests module) to this feature and add @step comments. This feature owns that file; sibling mermaid features use their own files.

#### validate-json-schema
- class: remap
- area: fspec-core
- scenarios: 7 mapped to deleted files
- rust code: rust/fspec-core/src/validators/json_schema.rs:65 (validate_against_schema, Ajv-worded errors), :142 (join_errors). rust/fspec-core/src/generators/tags_schema.rs:27/:33 (validate_tags, format_tags_errors). foundation via generators/foundation_schema.rs:31. Rust has no generic "validate JSON file from path" helper; validate-foundation-schema reads the file itself.
- rust tests: rust/fspec-core/src/validators/json_schema.rs::valid_instance_yields_no_errors, ::required_property_uses_ajv_wording, ::pattern_uses_real_regex_matching, ::errors_are_ordered_shallowest_instance_path_first (owner: none). rust/fspec-core/src/generators/tags_schema.rs::accepts_valid_tags, ::rejects_bad_tag_name_pattern (owner: none). rust/fspec-core/tests/validate_foundation_schema.rs::validates_a_schema_valid_foundation_and_reports_success, ::reports_a_friendly_error_when_foundation_json_contains_malformed_json (owner: validate-foundation-schema-rust-port).
- covered: Validate valid tags.json; Detect invalid tag name format (tags_schema.rs); Format validation errors for display (json_schema.rs; needs a join_errors assertion). Validate from file path and Handle malformed JSON file (dups of validate-foundation-schema-rust-port; "SyntaxError" is TS wording).
- not covered: Validate valid foundation.json -> write (json_schema.rs inline, generic-foundation schema); Detect missing required field in foundation.json -> write (json_schema.rs inline, /project required 'name')
- action: map rust/fspec-core/src/validators/json_schema.rs and rust/fspec-core/src/generators/tags_schema.rs (inline tests) to this feature with @step comments, and add the 2 foundation tests to json_schema.rs. Drop S5/S6 as duplicates of validate-foundation-schema-rust-port. Do not use foundation_schema.rs (claimed by design-generic-foundation-schema).

#### mermaid-validation-fails-with-dompurify-error
- class: merge
- area: fspec-core
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-core/src/utils/mermaid_validation.rs:141-178 (validates .md fences and .mmd/.mermaid with the merman parser; there is no DOM, so no DOMPurify). rust/fspec-core/src/commands/add_attachment.rs:50-53 (validate before copy).
- rust tests: rust/fspec-core/tests/add_attachment.rs::validates_mmd_attachment_and_rejects_invalid_mermaid_before_copy, ::validates_md_mermaid_fences_and_accepts_fence_free_markdown (owner: add-attachment-rust-port)
- covered: Attach Mermaid diagram with invalid syntax shows clear error
- not covered: Attach markdown file with valid Mermaid code block -> write in owner (drop the DOMPurify step); Attach Mermaid diagram file with valid syntax -> write in owner (valid .mmd copied; drop the DOMPurify.addHook step)
- action: merge into add-attachment-rust-port. Add one "valid .md fence / valid .mmd attach and copy" test to rust/fspec-core/tests/add_attachment.rs, then @deprecated / remove this feature (the DOMPurify bug is jsdom-only).

#### codex-init-writes-prompt-to-home-directory
- class: write
- area: fspec-core (init)
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-core/src/commands/init/templates.rs:44 (codex and codex-cli write $HOME/.codex/prompts/fspec.md and report "~/.codex/prompts/fspec.md"). rust/fspec-core/src/commands/init/files.rs:45 home_dir reads only HOME and falls back to ".". rust/fspec-core/src/commands/init/agents.rs:59 ("Run /prompts:fspec in Codex to activate")
- rust tests: rust/fspec-core/tests/init.rs::writes_the_codex_slash_command_under_an_injectable_home_directory (owner: init-rust-port, same behaviour as scenario 1, but the file is owned). rust/fspec-core/tests/codex_agent_initialization.rs (unowned, but its Feature headers name codex-agent-initialization and codex-cli-agent-initialization, so leave it to them).
- covered: none in a file this feature can own
- not covered: Generate Codex prompt in user home directory on Unix-like systems -> write, Resolve Codex prompt location using os.homedir on Windows -> write after fix or rewrite (Rust has no USERPROFILE fallback, so on Windows without HOME it writes ./.codex/prompts), Re-running fspec init keeps project-level prompt intact -> write, Generated Codex instructions reference /prompts:fspec command -> write
- action: write rust/fspec-core/tests/agent018_codex_home_prompt.rs (redirect HOME). For scenario 2, file a bug for the missing USERPROFILE fallback (files.rs:45), or rewrite the scenario as "resolves from HOME". activationMessage.test.ts is shared with agent-specific-activation-message-not-customized-in-fspec-init-success-output (COV-074, another chunk), which should own its own activation-message test.

#### agent-specific-activation-message-not-customized-in-fspec-init-success-output
- class: write
- area: fspec-core
- scenarios: 4 mapped to deleted files
- rust code: `rust/fspec-core/src/commands/init/agents.rs:56-76` (activation_message: claude/codex/codex-cli/cursor/aider, then category fallbacks, then generic "Refer to your AI agent documentation to activate fspec"). `rust/fspec-core/src/commands/init.rs:110` (`activationMessage` in the init result). No registry agent reaches the generic fallback (no agent has an empty slash_command_path).
- rust tests: rust/fspec/tests/cli_init.rs::scenario_cli_installs_the_claude_agent_and_prints_the_success_summary (owner: init-cli-subcommand) checks the Claude message only. Codex messages are in rust/fspec-core/tests/codex_agent_initialization.rs (unowned; belongs to codex-agent-initialization).
- covered: Claude Code user sees agent-specific activation message (CLI half, in the init-cli-subcommand file)
- not covered: Claude Code user sees agent-specific activation message -> write (own copy); Cursor user sees IDE-specific activation instructions -> write; Aider user sees CLI-specific activation instructions -> write; Unknown agent receives generic fallback message -> write (in-module unit test; activation_message is pub(super)). Delete the "UI component should display" steps in all 4 (Ink UI removed, 103a73072).
- action: write rust/fspec-core/tests/agent_specific_activation_message.rs (dispatch init with claude/cursor/aider and assert activationMessage) plus rust/fspec-core/src/commands/init/activation_message_tests.rs (fallback arm). Sibling codex-init-writes-prompt-to-home-directory shares activationMessage.test.ts and needs its own file.

#### agent-runtime-detection-for-context-aware-cli-output
- class: human
- area: fspec-core
- scenarios: 6 mapped to deleted files
- rust code: FSPEC_AGENT > spec/fspec-config.json > default resolution exists only in `rust/fspec-core/src/commands/review.rs:148-195` (`**⚠️ IMPORTANT:**` vs `**IMPORTANT:**`), `rust/fspec-core/src/foundation/guidance.rs:462-479` and update_foundation.rs:30. init writes `{"agent": ...}` (`rust/fspec-core/src/commands/init/files.rs:13`). update-work-unit-status always wraps reminders in `<system-reminder>` whatever the agent (`rust/fspec-core/src/commands/update_work_unit_status/reminders.rs:33`). validate has no agent-aware output.
- rust tests: rust/fspec-core/tests/init.rs (owner: init-rust-port) proves the init config write. rust/fspec-core/tests/review.rs (owner: review-rust-port) covers agent-aware review output.
- covered: Agent detected during init and written to config file (dup of init-rust-port)
- not covered: FSPEC_AGENT environment variable overrides config file -> write or rewrite (exists for review/update-foundation, not validate); Claude Code agent receives system-reminder output -> write (Rust already does this for every agent); Cursor IDE agent receives bold text with emoji output -> write or delete (absent for update-work-unit-status); Aider CLI agent receives plain text output -> write or delete (absent); Fallback to safe default when no config exists -> delete or write (Rust defaults to system-reminder, not plain text)
- action: depends on the answer. With (a): merge the init scenario into init-rust-port and rewrite the rest against review (rust/fspec-core/tests/agent_runtime_detection.rs). With (b): open a port card.
- question: Rust formats update-work-unit-status and validate output the same for every agent (always `<system-reminder>`); per-agent formatting only exists in review, update-foundation and foundation guidance. Should INIT-008 be (a) rewritten to what Rust does: merge the init-writes-config scenario into init-rust-port, retarget the env-override scenario at `fspec review`, and delete the Cursor/Aider/plain-default status/validate scenarios [recommended], or (b) kept as-is, with a card to port agent-aware output for status/validate?

### Batch COV-074-B2

#### update-agents-md-with-big-picture-event-storming-workflow-documentation
- class: write
- area: fspec-core
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-core/src/commands/bootstrap_doc.txt:175-362 (DOC-016 renamed the section to "Step 3: Foundation Event Storm", between Step 2 and "Step 4: Feature Event Storm"; when-to section :194; commands :211; comparison table :252-262; session flow with commands and dialogue :264-305; EXMAP-004 integration :319-333). :325 still tells agents to run `fspec derive-tags-from-foundation`, a command that never existed in TS or Rust. DISC-003 (rust/tools/tests/foundation_guidance_doc_test.rs:77) removed it as a "phantom command" from the tool guidance only.
- rust tests: none
- covered: none
- not covered: AI agent reads Step 1.5a documentation after foundation discovery -> write (rewrite the titles to the DOC-016 names); Developer distinguishes foundation vs work unit Event Storming commands -> write; AI agent follows Big Picture Event Storm session flow guidance -> write; AI agent integrates Big Picture Event Storm with tag ontology -> delete (phantom derive-tags-from-foundation)
- action: new rust/fspec-core/tests/big_picture_event_storm_docs.rs (bootstrap output). Open a bug to replace bootstrap_doc.txt:319-333 with generate-tags-md, as DISC-003 did for the tool guidance.

#### prefill-detection-incorrectly-flags-code-syntax-as-placeholders
- class: write
- area: fspec-core
- scenarios: 3 mapped to deleted files
- rust code: `rust/fspec-core/src/commands/update_work_unit_status/prefill.rs:62-101` (detect_prefill matches only the fixed literals `[role]`, `[action]`, `[benefit]`, `[precondition]`, `[expected outcome]`, `[scenario name]`, `TODO:` and the bare @component/@feature-group tags, so `workUnits[id]` or `array[0]` cannot match). It gates update-work-unit-status (prefill.rs:117-150).
- rust tests: none for false positives (rust/fspec-core/tests/update_work_unit_status.rs::specifying_to_testing_blocked_by_prefill_placeholders, owner: update-work-unit-status-rust-port, proves only the positive case)
- covered: none
- not covered: Code syntax with brackets should not trigger prefill detection -> write; Actual placeholder patterns should trigger prefill detection -> write; Array access syntax should not trigger prefill detection -> write
- action: write rust/fspec-core/tests/bug079_prefill_false_positives.rs (linked feature containing code-bracket / array-index steps moves specifying->testing; one containing `[role]` is blocked). Note: the feature's step texts have lost their brackets ("Given I have role placeholder"), so the @step comments must copy them exactly.

#### bounded-context-map-ui-duplicates-each-bounded-context-name-in-visualization
- class: write
- area: fspec-core (generators)
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-core/src/generators/foundation_md_diagrams.rs:16 bounded_context_mermaid (hardcoded names get "<text><br/><description>", others get the text only, nodes BC1..n)
- rust tests: none (foundation_md.rs inline tests cover only the header)
- covered: none
- not covered: Hardcoded bounded context generates name and description on two lines -> write, Non-hardcoded bounded context generates only context name without duplication -> write, Generate bounded context map for multiple contexts without duplication -> write
- action: write rust/fspec-core/tests/bug085_bounded_context_map_labels.rs (drive generate-foundation-md via dispatch, assert node labels). Shares foundation-md.test.ts with enhance-foundation-md-event-storm-visualization, which gets its own file.

#### enhance-foundation-md-event-storm-visualization
- class: write
- area: fspec-core (generators)
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-core/src/generators/foundation_md.rs:236 render_domain_architecture (bounded contexts filtered !deleted at :250, per-context Aggregates/Events/Commands sections at :290 via render_context_items :343)
- rust tests: none
- covered: none
- not covered: Render bounded context with aggregates, events, and commands -> write, Render all bounded contexts with complete sections -> write, Exclude deleted items from visualization -> write
- action: write rust/fspec-core/tests/found042_foundation_md_event_storm_sections.rs. Sibling BUG-085 uses its own file.

#### checkpoint-creation-fails-when-deleted-files-exist
- class: write
- area: git / fspec-core
- scenarios: 4 mapped to deleted files
- rust code: rust/git/src/ghost_commit.rs:154-172 (a file deleted in the worktree counts as changed and is left out of the tree). rust/fspec-core/src/commands/checkpoint.rs:246 ("Captured {} file(s)"). rust/git/src/ghost_commit.rs:495-626 (restore writes the checkpoint tree back, recreating deleted files). Pure gix, no git.remove().
- rust tests: none for deleted files. rust/git/tests/ghost_commit_tests.rs (owner: ghost-commit-checkpoints) covers modified/untracked only.
- covered: none
- not covered: Create checkpoint with deleted file -> write; Create checkpoint with mixed changes -> write; Create checkpoint with multiple deleted files -> write (drop the "staged using git.remove()" step: isomorphic-git detail); Restore checkpoint that includes deleted files -> write
- action: new rust/fspec-core/tests/checkpoint_deleted_files.rs (dispatch checkpoint / restore-checkpoint against a temp repo).

#### documentation-clarity-improvements
- class: write
- area: fspec-core
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-core/src/commands/bootstrap_doc.txt:175/:252-262 ("Step 3: Foundation Event Storm", "Step 4: Feature Event Storm", comparison table). :372-383 (Research-First Workflow, AST block via __AST_RESEARCH_BLOCK__, which now means astgrep). :1634-1650 (✨ AUTOMATIC / 🔧 MANUAL). :1726-1749 (@step matches step.text only, data-table example). :136-140 (Foundation Discovery Iteration).
- rust tests: none. rust/fspec-core/tests/bootstrap.rs (owner: bootstrap-rust-port) checks only markers and placeholders.
- covered: none
- not covered: all 5 -> write
- action: new rust/fspec-core/tests/documentation_clarity_improvements.rs that runs bootstrap and asserts each section. The steps are written as agent narrative ("the AI might think…"), so rewrite them as doc-content checks. In S2 change "AST research" to `fspec astgrep` (CLI-015).

#### generate-event-storm-section-in-foundation-md-with-validated-mermaid-diagram
- class: write
- area: fspec-core (generators)
- scenarios: 4 mapped to deleted files
- rust code: rust/fspec-core/src/generators/foundation_md.rs:236-282 (Domain Architecture, Bounded Contexts list, Bounded Context Map, "Mermaid validation failed: ..." Err at :282, section skipped when there is no eventStorm or no contexts at :241/:258). rust/fspec-core/src/commands/add_foundation_bounded_context.rs:144 (regenerate after add). rust/fspec-core/src/utils/mermaid_validation.rs:37 (merman parser)
- rust tests: none for the Event Storm section (generate_foundation_md.rs tests are owned by generate-foundation-md-rust-port and cover diagrams in general)
- covered: none
- not covered: FOUNDATION.md auto-regenerates with Event Storm section when bounded context added -> write, FOUNDATION.md includes Mermaid graph with all bounded contexts -> write, Mermaid validation error prevents FOUNDATION.md generation -> write, Event Storm section only appears when eventStorm data exists -> write
- action: write rust/fspec-core/tests/found017_event_storm_section_mermaid.rs. Sibling mermaid-validation-test-uses-invalid-test-data (same deleted TS file) uses its own file.

#### mermaid-validation-test-uses-invalid-test-data
- class: write
- area: fspec-core (generators)
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-core/src/generators/foundation_md.rs:280-282 (bounded_context_mermaid then validate_mermaid, Err "Mermaid validation failed: <merman msg>"). rust/fspec-core/src/commands/generate_foundation_md.rs writes only on Ok.
- rust tests: none
- covered: none
- not covered: Bounded context with malicious code injection causes validation error -> write, generateFoundationMd throws error with helpful message pointing to invalid syntax -> write, FOUNDATION.md is not written when Event Storm diagram validation fails -> write
- action: write rust/fspec-core/tests/test009_mermaid_injection_rejected.rs, with a bounded context 'Context"];malicious[("code'. Confirm merman rejects it when written. If not, this becomes a bug, not a test.

#### complete-big-picture-event-storm-schema-integration
- class: write
- area: fspec-core
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-core/src/generators/generic-foundation.schema.json:160-190 (eventStorm, level const "big_picture"), :312-320 (eventStormItem oneOf), :391-404 (eventStormCommand.actor). rust/fspec-core/src/generators/foundation_schema.rs:31 (validate_foundation). rust/fspec-core/src/commands/discover_foundation.rs:342-360 (finalize schema gate: "Schema validation failed", returns before writing foundation.json or deleting the draft).
- rust tests: foundation_schema.rs::accepts_valid_event_storm_bounded_context / ::rejects_bounded_context_with_non_null_color (inline; claimed by design-generic-foundation-schema in COV-072)
- covered: none (the inline bounded-context tests sit in a file another feature is taking)
- not covered: all 5 -> write
- action: new rust/fspec-core/tests/foundation_event_storm_schema.rs (S1-S4 through validate_foundation; S5 through dispatch discover-foundation --finalize with an invalid item type).

#### foundation-missing-error-message-is-not-imperative-enough
- class: write
- area: fspec-core
- scenarios: 2 mapped to deleted files
- rust code: rust/fspec-core/src/io/ensure.rs:157-200 (check_foundation_exists / build_foundation_missing_error: "CRITICAL: NEVER manually create foundation.json - you MUST use discover-foundation workflow!", Step 1/2/3). Step 2 reads "Step 2: AI analyzes codebase and fills fields", not "Step 2: Fill [QUESTION:] placeholders in draft".
- rust tests: none. rust/fspec-core/tests/create_story.rs::dispatcher_rejects_a_missing_foundation_with_the_foundation_missing_message (owner: create-story-rust-port) does not assert this text.
- covered: none
- not covered: Error forbids manual foundation.json creation -> write; Error shows complete workflow steps -> write (rewrite the Step 2 step text to match ensure.rs)
- action: new rust/fspec-core/tests/foundation_missing_error_message.rs, as agreed with sibling foundation-existence-check-in-commands (COV-072), which uses foundation_existence_check.rs.

#### hook-configuration-schema-and-validation
- class: human
- area: fspec-core (command hooks)
- scenarios: 6 mapped to deleted files
- rust code: rust/fspec-core/src/commands/validate_hooks.rs:40-64 (bad JSON gives "Failed to load hook configuration". A missing script gives "Hook command not found: <cmd>"). There is no kanban-hook loader with default timeout 60, override, conditions or global defaults. rust/core/src/lifecycle_hooks/loader.rs:128 does global timeout and shell, but only for agent lifecycle events.
- rust tests: rust/fspec-core/tests/validate_hooks.rs (owner: validate-hooks-rust-port)
- covered: Reject configuration with non-existent hook command (validate-hooks-rust-port, same "Hook command not found" text)
- not covered: Load valid hook configuration with single hook -> human, Load hook configuration with timeout override -> human, Load hook configuration with conditions -> human, Reject invalid JSON configuration -> delete or rewrite (Rust says "Failed to load hook configuration", not "Invalid JSON"), Load configuration with global defaults -> human
- action: merge scenario 5 into validate-hooks-rust-port. The rest follow the command-hook answer (see hook-condition-evaluation). If ported, write rust/fspec-core/tests/hook002_command_hook_config.rs.
- question: Same as hook-condition-evaluation: port command-hook execution (a, recommended) or @deprecated (b)?

#### hook-discovery-and-event-naming
- class: human
- area: fspec-core (command hooks)
- scenarios: 6 mapped to deleted files
- rust code: none found. Grepped discover/event-name generation (pre-/post-<command>) and "setup"/"cleanup" lifecycle events. The only event strings built are pre-/post-<status> for virtual hooks (board_state.rs:83, virtual_hooks_exec.rs:49).
- rust tests: none
- covered: none
- not covered: all 6 -> retire or write, depending on the answer
- action: follows the command-hook answer. If ported, write rust/fspec-core/tests/hook004_discovery_event_naming.rs.
- question: Same as hook-condition-evaluation: port command-hook execution (a, recommended) or @deprecated (b)?

#### hook-condition-evaluation
- class: human
- area: fspec-core (command hooks)
- scenarios: 10 mapped to deleted files
- rust code: none found. Grepped condition/tags/prefix/epic/estimate evaluation and "condition" in fspec-core/core. Rust never runs spec/fspec-hooks.json command hooks. rust/core/src/lifecycle_hooks/config.rs:15 says "fspec CLI events ... (ignored by Rust engine)". Only virtual hooks (board_state.rs:69, virtual_hooks_exec.rs:34) and agent lifecycle hooks run. add-hook, list-hooks, remove-hook and validate-hooks were ported, but nothing executes their entries.
- rust tests: none
- covered: none
- not covered: all 10 -> retire or write, depending on the answer
- action: if (a), @deprecated citing the new card's outcome. If (b), a port card writes rust/fspec-core/tests/hook005_condition_evaluation.rs. Same question for HOOK-002/004/009 here and HOOK-001/003/006/007/008 elsewhere.
- question: The Rust port never executes the kanban command hooks in spec/fspec-hooks.json (pre-/post-<command>, conditions, global timeout/shell), although add-hook, list-hooks, remove-hook and validate-hooks were ported and help still documents them. Should the command-hook features (HOOK-002/004/005/009, plus HOOK-001/003/006/007/008) be (a) kept, with a new card to port command-hook execution [recommended: users can configure hooks that silently never run], or (b) @deprecated, with the add-hook/list-hooks help updated to say only agent lifecycle and virtual hooks run?

#### hook-system-documentation-and-examples
- class: human
- area: docs / examples
- scenarios: 6 mapped to deleted files
- rust code: n/a. docs/hooks/configuration.md and troubleshooting.md were removed in 051c9db83 ("migrate documentation to fspec.dev"). examples/hooks/ still has validate-feature.sh, run-tests.py, notify-slack.js, lint.sh and fspec-hooks.json.example. The README and example json now describe agent lifecycle events, not kanban hooks.
- rust tests: none
- covered: none
- not covered: Configuration documentation shows complete JSON schema -> retire (051c9db83), Troubleshooting section explains common errors -> retire (051c9db83), Bash hook example reads context and validates feature file -> per answer, Python hook example parses stdin and runs tests -> per answer, Node.js hook example reads stdin and sends notifications -> per answer, Lint hook example demonstrates proper exit codes -> per answer
- action: if (a), @deprecated citing 051c9db83. If (b), write rust/fspec-core/tests/hook009_hook_examples.rs that reads examples/hooks/*.
- question: The hook docs moved to fspec.dev (051c9db83), and the remaining kanban-hook examples target command hooks that Rust never runs. Should HOOK-009 be (a) @deprecated citing 051c9db83 [recommended], or (b) kept, with a Rust test that checks the four examples in examples/hooks/?

#### hook-execution-engine
- class: human
- area: fspec-core
- scenarios: 7 mapped to deleted files
- rust code: none for global hooks. `rust/core/src/lifecycle_hooks/config.rs:15` says fspec CLI events in spec/fspec-hooks.json are "ignored by Rust engine". The only fspec-core hook execution is work-unit *virtual* hooks. `rust/fspec-core/src/commands/update_work_unit_status/board_state.rs:70` (`run_pre_hooks`) runs `sh -c` per hook with `.output()`, with no timeout, no stdin JSON context and no stdout/stderr display. `rust/fspec-core/src/virtual_hooks_exec.rs:34` (`run_for_transition`, post-hooks) has no caller. The agent lifecycle hook executor `rust/core/src/lifecycle_hooks/executor.rs:23` (stdin, timeout, capture) serves agent events only (HOOK-013/015), not fspec commands. add-hook, list-hooks and validate-hooks were ported and still write and read spec/fspec-hooks.json (`rust/fspec-core/src/commands/add_hook.rs:85`).
- rust tests: none
- covered: none
- not covered: all 7 -> write (if ported) or retire
- action: shares the decision with system-reminder-formatting-for-ai and integrate-hooks-into-all-commands. If ported: new card; tests go in rust/fspec-core/tests/hook_execution_engine.rs. If retired: @deprecated citing 103a73072 + rust/core/src/lifecycle_hooks/config.rs:15.
- question: Rust never runs the global hooks in spec/fspec-hooks.json (pre-/post-<command>), even though add-hook, list-hooks and validate-hooks still manage that file. Should we (a) open a card to port global command-hook execution and keep HOOK-003/006/008 until it lands [recommended: the hook config is a live product surface that currently does nothing], or (b) mark HOOK-003/006/008 @deprecated citing 103a73072 (global CLI hooks dropped in the port; agent lifecycle hooks HOOK-013+ replace them)?

### Batch COV-074-B3

#### system-reminder-formatting-for-ai
- class: human
- area: fspec-core
- scenarios: 5 mapped to deleted files
- rust code: no "Hook: <name>" / "Exit code: N" formatter. The only analogue is the blocking virtual pre-hook error at `rust/fspec-core/src/commands/update_work_unit_status/board_state.rs:108`, which emits `<system-reminder>BLOCKING HOOK '{name}' failed for {id} ({event}): {stderr}</system-reminder>`. It has no exit code and is emitted even when stderr is empty. `rust/fspec-core/src/virtual_hooks_exec.rs:71` has the same shape without stderr and is never called. Non-blocking hook stderr is discarded, not shown.
- rust tests: none
- covered: none
- not covered: all 5 -> write (if ported) or retire
- action: same decision as hook-execution-engine (shared src/hooks/__tests__ suite). If ported: rust/fspec-core/tests/hook_system_reminder_formatting.rs. If retired: @deprecated citing 103a73072.
- question: Same as hook-execution-engine. Port the hook-failure system-reminder format ("Hook: X / Exit code: N / stderr", blocking only, none when stderr is empty) with the global-hook card [recommended], or @deprecated citing 103a73072?

#### mermaid-validation-using-parse-instead-of-render-misses-semantic-errors
- class: write
- area: fspec-core
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-core/src/utils/mermaid_validation.rs:52-80 (precheck: "Quoted subgraph titles are not supported. Use: subgraph ID[Title]", "Invalid subgraph identifier '…'"). :37-48 (merman strict parse). :141-150 ("Mermaid code block N is invalid: …"). Called from add_attachment before copy.
- rust tests: unit-level only, in mermaid_validation.rs inline (unowned; proposed for sibling BUG-123). rust/fspec-core/tests/add_attachment.rs::validates_mmd_attachment_and_rejects_invalid_mermaid_before_copy / ::validates_md_mermaid_fences_and_accepts_fence_free_markdown (owner: add-attachment-rust-port) do not check subgraph messages.
- covered: none
- not covered: all 5 -> write
- action: new rust/fspec-core/tests/mermaid_attachment_semantic_validation.rs that dispatches add-attachment with .md files: quoted subgraph, ID[Title] accepted, bad identifier, broken syntax, "block 2" in the error.

#### improve-step-workflow-clarity-to-prevent-wrong-test-file-and-missing-comments
- class: write
- area: fspec-core
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-core/src/commands/update_work_unit_status/reminders/status.rs:11 (TESTING reminder, line 3 is "CRITICAL: MANDATORY @step COMMENTS REQUIRED" plus "ONE scenario = ONE test"). rust/fspec-core/src/commands/update_work_unit_status/step_docstrings.rs:182 (validation error prefixed "Test file: {files}"). rust/fspec-core/src/commands/link_coverage/step.rs:210 and update_work_unit_status/step_docstrings/steps.rs:28 ("DELETE and RECREATE test if created in current work unit", "use checkpoint restore", "recreation is better than editing")
- rust tests: none assert these texts. Inline tests in step_docstrings/steps.rs only check matching.
- covered: none
- not covered: Testing state reminder makes @step requirement prominent -> write, Validation error shows which test file is being checked -> write (the path is shown, but Rust has no "CORRECT test file" warning, so delete that step or add the text), Reminder guides recreation when tests created without @step comments -> write
- action: write rust/fspec-core/tests/refac003_step_workflow_clarity.rs (drive update-work-unit-status to testing, then a failing validation).

#### configuration-management-with-tui-integration
- class: human
- area: fspec-core
- scenarios: 5 mapped to deleted files
- rust code: `rust/fspec-core/src/commands/research.rs:285-312` resolve_config: ENV > user > project > default (`perplexity.model = "sonar"` at :290). It is used only for the LIST-mode `configured` flag and `configGuidance` (research.rs:149-163, 227-250). The resolved values, any "config source" label (ENV/PROJECT/DEFAULT) and `.env`/dotenv loading are absent. The execute-time "Missing required configuration" validation is absent because the execute path is deferred (research.rs:201, RPC-286).
- rust tests: rust/fspec-core/tests/research.rs::list_mode_reflects_configured_perplexity_api_key_from_project_config (owner: research-rust-port)
- covered: Project config used when env var and user config don't exist (dup of research-rust-port, which uses perplexity instead of jira)
- not covered: Environment variable overrides user config file -> write (code research.rs:305; no test); .env file loads environment variables with precedence over config files -> delete (no dotenv in Rust); Validation detects missing required configuration -> delete (execute path deferred, RPC-286); Default values used when no configuration exists -> delete (default exists but is not observable; no source label)
- action: with (a): merge into research-rust-port; the project-config scenario is a dup and env-override becomes a new owner test in rust/fspec-core/tests/research.rs. Consistent with BUG-076 (same RPC-286 deferral).
- question: Rust resolves research config only to report "configured" in `fspec research` LIST mode. It has no config-source labels, no .env loading, and no execute-time validation (RPC-286 LIST-only). Should RES-012 be (a) merged into research-rust-port (keep project-config as a dup, add an env-over-user test there, delete the .env/source/validation/default scenarios) [recommended], or (b) kept, pending a card that ports research execution, .env and source reporting?

#### search-scenarios-returns-no-results-when-it-should-find-matching-scenarios
- class: merge
- area: fspec-core
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec-core/src/commands/search_scenarios.rs:106-142 (matches feature name, description, file path and work-unit title from work-units.json (BUG-059), then scenario name)
- rust tests: rust/fspec-core/tests/search_scenarios.rs::feature_name_match_returns_all_of_that_features_scenarios, ::literal_query_matches_a_scenario_by_name (owner: search-scenarios-rust-port)
- covered: Search by feature name; Search by scenario name (existing functionality)
- not covered: Search by feature file name (kebab-case) -> write in owner; Search by feature description -> write in owner; Search by work unit title -> write in owner
- action: merge into search-scenarios-rust-port. Add 3 tests to rust/fspec-core/tests/search_scenarios.rs, drop the 2 duplicates, then @deprecated / remove the feature.

#### duplicate-work-unit-ids-in-state-arrays-when-moving-backward-or-to-same-state
- class: write
- area: fspec-core
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-core/src/commands/update_work_unit_status/board_state.rs:12-29 (move_state: no-op when from == to; retain to remove from the source; push only if absent). transitions.rs:17-41 (implementing→implementing is not an allowed transition, so the command rejects same-state moves).
- rust tests: none
- covered: none
- not covered: Moving work unit backward through states should not create duplicates -> write; Moving work unit to same state should be idempotent -> write (rewrite: same-state move is rejected and the arrays are unchanged); Work unit should appear in exactly one state array after any transition -> write
- action: new rust/fspec-core/tests/state_arrays_no_duplicates.rs (dispatch update-work-unit-status, then inspect states.*).

#### step-validation-system-reminder-doesn-t-explain-proper-comment-placement-and-deduplication
- class: write
- area: fspec-core
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-core/src/commands/link_coverage/step.rs:210-282 (format_validation_error: "3. Place step comments NEAR the code that executes each step", "⚠️  If you have duplicate Given/When/Then comments, remove them first:", "Example:" placement block). A twin is in update_work_unit_status/step_docstrings/steps.rs.
- rust tests: none (no test asserts these lines)
- covered: none
- not covered: all 3 -> write
- action: new rust/fspec-core/tests/step_validation_reminder_guidance.rs (dispatch link-coverage on a story with a test missing @step comments; assert the 3 guidance blocks). This keeps step.rs inline tests for the BUG-063 sibling.

#### system-reminder-anti-drift-pattern
- class: write
- area: fspec-core
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-core/src/commands/update_work_unit_status/reminders.rs:26-30 (FSPEC_DISABLE_REMINDERS gate) and reminders/status.rs:11/:68/:217-220 (TESTING, implementing, VALIDATING and done reminders; done emits a "FINAL CHECK" reminder, reminders/enhanced.rs:177). rust/fspec-core/src/commands/show_work_unit.rs:25 (missing-estimate Fibonacci reminder). list-work-units has no empty-backlog reminder; TS getEmptyBacklogReminder had no caller at 103a73072^.
- rust tests: none for the status-transition reminders. rust/fspec-core/tests/show_work_unit.rs::honours_fspec_disable_reminders_env_gate (owner: show-work-unit-rust-port) covers show-work-unit only.
- covered: none
- not covered: Remind about failing tests when moving to testing phase -> write; Remind about minimal code when moving to implementing phase -> write; Remind about running all tests when moving to validating phase -> write; Remind about Fibonacci scale when estimate is missing -> write; No estimate reminder when estimate exists -> write; Disable reminders with environment variable -> write; No reminder when moving to done status -> delete (Rust and TS both emit the done reminder); Remind about empty backlog -> delete (never wired in TS; absent in Rust)
- action: new rust/fspec-core/tests/system_reminder_anti_drift.rs.

#### coverage-file-synchronization
- class: merge
- area: fspec-core
- scenarios: 6 mapped to deleted files
- rust code: rust/fspec-core/src/commands/delete_scenario.rs:109-150 (prunes the sidecar and recomputes stats). rust/fspec-core/src/commands/delete_scenarios.rs:193 (same for bulk delete). rust/fspec-core/src/commands/generate_coverage/global.rs:85-108 (drops unmapped stale scenarios and keeps mapped ones, BUG-301; there is no "ℹ Removed N stale scenario" line). rust/fspec-core/src/commands/update_work_unit_status/coverage_gate.rs:79-112 (stale-scenario gate: "Coverage file out of sync. Run 'fspec generate-coverage' …", lists the names). rust/fspec-core/src/commands/update_scenario.rs:138 (renames the coverage entry and keeps the mappings).
- rust tests: rust/fspec-core/tests/delete_scenario.rs::scenario_dispatcher_updates_the_coverage_sidecar_when_present (owner: delete-scenario-rust-port); rust/fspec-core/tests/bug374_coverage_stats_after_deletion.rs::deleting_scenarios_by_tag_drops_their_files_from_the_stats (owner: coverage-stats-after-scenario-deletion); rust/fspec-core/tests/generate_coverage.rs::updates_a_sidecar_when_scenarios_were_added_and_removed (owner: generate-coverage-rust-port); rust/fspec-core/tests/update_scenario.rs::scenario_rename_a_scenario_and_update_its_coverage_entry (owner: update-scenario-rust-port)
- covered: Delete scenario removes coverage entry (Rust prints "Successfully deleted scenario … Updated coverage file", not "✓ Deleted scenario"); Bulk delete scenarios by tag removes coverage entries; Generate coverage syncs deleted scenarios (no "ℹ Removed 1 stale scenario" text in Rust); Rename scenario preserves test mappings
- not covered: Coverage validation detects stale scenarios -> write (no test asserts the coverage_gate.rs:93 out-of-sync error); Deleted scenario no longer causes uncovered error -> write
- action: drop the 4 covered scenarios as duplicates. Move the 2 gate scenarios into update-work-unit-status-rust-port with new tests in rust/fspec-core/tests/update_work_unit_status.rs. The feature is then empty, so mark it @deprecated or remove it.

#### work-unit-scoped-hooks-for-dynamic-validation
- class: human
- area: fspec-core (virtual hooks)
- scenarios: 12 mapped to deleted files
- rust code: rust/fspec-core/src/commands/update_work_unit_status/board_state.rs:69 (pre-<status> virtual hooks, blocking: "BLOCKING HOOK '<name>' failed"). rust/fspec-core/src/virtual_hooks_exec.rs:34 (post-<status>). virtual_hooks_exec.rs:65 is "todo: gitContext support": stagedFiles/unstagedFiles are never passed and no stdin context is sent. add_virtual_hook.rs:201 (script spec/hooks/.virtual/<id>-<name>.sh, mode 0o755). remove_virtual_hook.rs:170 (deletes script). reminders/status.rs:148 ("VIRTUAL HOOKS: Consider quality checks") and :246 (done cleanup keep/remove reminder). show_work_unit.rs:924 ("Virtual Hooks:" grouped by event). No isShellCommand: validate_hooks.rs:62 treats every command as a path.
- rust tests: rust/fspec-core/tests/update_work_unit_status.rs::blocking_pre_hook_failure_prevents_transition (owner: update-work-unit-status-rust-port), rust/fspec-core/tests/add_virtual_hook.rs::scenario_git_context_generates_script_and_stores_relative_path (owner: add-virtual-hook-rust-port), rust/fspec-core/tests/copy_virtual_hooks.rs (owner: copy-virtual-hooks-rust-port), rust/fspec-core/tests/remove_virtual_hook.rs (owner: remove-virtual-hook-rust-port), rust/fspec-core/tests/clear_virtual_hooks.rs (owner: clear-virtual-hooks-rust-port)
- covered: Copy virtual hooks from one work unit to another (copy-virtual-hooks-rust-port) and Generate executable script files for complex virtual hooks (add-/remove-virtual-hook-rust-port). Blocking virtual hook prevents workflow transition until fixed belongs to HOOK-012 virtual-hooks-don-t-execute-during-status-transitions (COV-072-c4); the Rust message differs from "✗ Blocked by virtual hook".
- not covered: AI prompts for virtual hooks at end of specifying phase -> write (reminder text only), Convert successful virtual hook to permanent global hook -> delete (AI dialogue; global hooks never run in Rust, see HOOK question), Clean up virtual hooks when work unit reaches done status -> write (cleanup reminder; clear-virtual-hooks owns the clearing), Handle virtual hook execution failure gracefully -> delete (Rust has no "Hook failed to execute ... check PATH" text, and sh exit 127 is reported as a plain blocking failure), Display virtual hooks in work unit details -> write, Create virtual hook with git context for changed files only -> human, Detect shell commands vs script paths correctly -> delete (no isShellCommand in Rust), Detect git staged and unstaged files for hooks -> human, System reminder prompts AI about virtual hooks at phase transitions -> write
- action: write rust/fspec-core/tests/virtual_hooks_reminders_and_display.rs (scenarios 1, 4, 7, 12). Merge scenario 2 into HOOK-012, and scenarios 5 and 10 into the copy/add/remove twins. Delete 3, 6 and 9. Scenarios 8 and 11 depend on the answer below.
- question: Rust stores `gitContext: true` and generates the script, but never passes staged/unstaged files to virtual hooks (virtual_hooks_exec.rs:65 "todo: gitContext support"). Should the two git-context execution scenarios be (a) kept, with a card to implement gitContext execution [recommended, since add-virtual-hook advertises --git-context], or (b) deleted, with --git-context documented as script-generation only?

#### work-units-json-not-stamped-with-current-version-on-initial-creation
- class: write
- area: fspec-core
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-core/src/io/ensure.rs:40-45 (ensure_work_units_file creates WorkUnitsData::initial). rust/fspec-core/src/types/work_unit.rs:12 (single CURRENT_VERSION = "0.7.1") and :69 (initial stamps it). Rust has no migration system, so "no migration / no backup" holds trivially.
- rust tests: rust/fspec-core/src/io/ensure.rs::ensure_work_units_creates_canonical_initial_when_missing asserts version 0.7.1. Sibling automatic-json-file-initialization (COV-071) plans to own these ensure tests in rust/fspec-core/src/io/ensure/init_tests.rs.
- covered: none
- not covered: Create work-units.json with current version on first run -> write; Recreate work-units.json with current version after deletion -> write; Version constant shared across ensure-files and migrations -> delete (names ensure-files.ts and the TS migrations, which no longer exist; the single constant is work_unit.rs:12)
- action: new rust/fspec-core/tests/work_units_version_stamp.rs (first command creates the file at CURRENT_VERSION; delete then recreate). Keep it separate from the sibling's ensure/init_tests.rs (shared ensure-files.test.ts).

#### unified-shift-arrow-navigation-across-boardview-agentview-and-splitpaneview
- class: merge
- area: fspec-tui
- scenarios: 15 mapped to deleted files
- rust code: rust/fspec-tui/src/store/agent_view/navigation.rs:41/57/78 (navigate_next/prev, first_open_session_id), rust/fspec-tui/src/app/dispatch_session_cycle.rs, rust/sessions/src/navigation.rs:48 (flat list, BUG-124)
- rust tests: rust/fspec-tui/tests/shift_arrow_end_of_list_rpc096.rs (owner: agentview-shift-arrow-end-of-list-parity), rust/fspec-tui/tests/shift_right_create_session_dialog_rpc097.rs (owner: agentview-shift-right-create-session-dialog)
- covered: these are already proven by the owners and can be dropped as duplicates. RPC-097 boardview_shift_right_zero_open_sessions_still_mounts_dialog_rpc097_reopen2 covers "Shift+Right from BoardView with no sessions shows create session dialog". RPC-097 boardview_shift_right_with_two_open_sessions_resumes_first_rpc097_reopen2 covers "...with sessions navigates to first session". RPC-096 shift_left_at_first_index_snapshots_draft_then_exits_to_board covers "Shift+Left from first session returns to BoardView". RPC-096 mid-list Shift+Right covers "Shift+Right from session without watchers navigates to next session". RPC-097 shift_right_at_last_index_of_three_mounts_create_session_dialog covers "Shift+Right from last session without watchers shows create session dialog". RPC-097 boardview_shift_right_then_enter_yes_switches_to_agentview_and_submits covers "Confirming create session dialog creates new unattached session". RPC-097 esc_emits_cancelled_and_dismisses / enter_on_cancel_emits_cancelled_and_preserves_input cover "Canceling create session dialog stays at current position".
- not covered: Shift+Left from BoardView does nothing -> write (new test in shift_arrow_end_of_list_rpc096.rs under RPC-096), Shift+Right from session with watchers navigates to first watcher -> retire, Shift+Left from first watcher returns to parent session -> retire, Shift+Right from watcher navigates to next sibling watcher -> retire, Shift+Left from watcher navigates to previous sibling watcher -> retire, Shift+Right from last watcher navigates to next session -> retire, Shift+Right from last watcher of last session shows create session dialog -> retire, /parent command still works alongside Shift+Left navigation -> retire (no /parent and no SplitSessionView in Rust)
- action: merge into agentview-shift-arrow-end-of-list-parity and agentview-shift-right-create-session-dialog. Mark the 7 watcher/split/parent scenarios @deprecated citing BUG-124 (navigation flattened). Same reasoning as sibling-watcher-navigation.

#### command-help-system
- class: write
- area: fspec CLI
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec/src/cli_registration/entry.rs:11 + support_03.rs:10-270 (intercept_ts_help: `fspec <cmd> … --help|-h` with the flag anywhere after the subcommand renders the static help config; unknown commands return None and clap prints its default help). Configs are compiled in statically (no dynamic import). `fspec --help list-work-units` goes to clap, not custom help.
- rust tests: rust/fspec/tests/cli_remove_tag_from_scenario.rs::scenario_remove_tag_from_scenario_help_matches_ts_formatcommandhelp_reference (owner: remove-tag-from-scenario-cli-subcommand). rust/fspec/tests/cli_tag_stats.rs main `fspec --help` lists subcommands (owner: tag-stats-cli-subcommand).
- covered: Display custom help when help config exists (dup); Handle main program help without command (dup)
- not covered: Fall back to Commander.js help when no custom help exists -> write (as clap fallback); Handle help flag in any position -> write (drop the `fspec --help list-work-units` form, which Rust does not support); Gracefully handle missing help config file -> delete (TS dynamic import; Rust configs are static)
- action: new rust/fspec/tests/cli_help_interceptor.rs for S2/S3. Drop S1/S4 as duplicates and delete S5. Feature has no work-unit tag; tag it @COV-074.

#### integrate-hooks-into-all-commands
- class: human
- area: fspec-core
- scenarios: 7 mapped to deleted files
- rust code: no pre-/post-<command> hook calls in any Rust command (grep for `fspec-hooks` and `run_hooks` in rust/fspec/src and rust/fspec-core/src/commands finds only the add/list/remove/validate-hook config commands). `update_work_unit_status.rs:166` runs only work-unit virtual `pre-<status>` hooks (board_state.rs:70). No context JSON is passed, there are no tag conditions (`condition.tags`), and no post-hooks run.
- rust tests: none
- covered: none
- not covered: all 7 -> write (if ported) or retire
- action: same decision as hook-execution-engine. If ported: rust/fspec/tests/cli_integrate_hooks_into_commands.rs. If retired: @deprecated citing 103a73072.
- question: Same as hook-execution-engine. Port "every command runs its pre-/post-<command> global hooks, with context and tag conditions" [recommended], or @deprecated citing 103a73072?

#### model-selector-keyboard-navigation-e2e
- class: write
- area: fspec CLI (PTY e2e of fspec-tui)
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-tui/src/views/model_selector/navigation.rs:32 (move_down); state.rs:143 (adjust_scroll, scroll-follow); dispatch.rs:101 (Right expands a section)
- rust tests: these are unit-level only, in files other features own: rust/fspec-tui/src/views/model_selector/scroll_tests.rs (owner: model-view-scroll-viewport) and tests_collapse.rs (owner: model-selector-default-expansion). There is no real-binary test since e2e/prov-104-model-nav.test.ts was removed in 103a73072.
- covered: none at the e2e tier
- not covered: Pressing Down in the live /model view moves the highlight to the next model -> write; Down to the bottom of a tall list keeps the selected model painted in the viewport -> write; Fresh open with all sections collapsed lets Down reach a section to expand -> write
- action: write rust/fspec/tests/model_nav_pty_prov104.rs. Follow the rust/fspec/tests/scrollback_pty_rpc078.rs pattern (portable_pty + vt100, #[ignore], FSPEC_USER_DIR temp config with a local-server profile). Business rule 6 requires a real-binary test. If PTY tests are not wanted, the alternative is to merge into model-view-scroll-viewport.

### Batch COV-074-B4

#### enter-key-model-selection-end-to-end-in-the-model-view
- class: write
- area: fspec CLI (PTY e2e of fspec-tui)
- scenarios: 1 mapped to deleted files
- rust code: rust/fspec-tui/src/views/model_selector/dispatch.rs:175 (Enter emits Action::ModelSelected); model_id.rs:35 (model_ids_match normalizes date-suffixed ids)
- rust tests: unit-level only: rust/fspec-tui/src/views/model_selector/tests_enter_expand.rs (owner: model-selector-enter-key-behavior). Nothing drives the real binary.
- covered: none at the e2e tier
- not covered: End-to-end: pressing Enter on a model row selects the model and the selector closes against the real binary -> write
- action: write rust/fspec/tests/model_select_pty_prov117.rs, using the PTY pattern from scrollback_pty_rpc078.rs and a temp config with a dated custom model id. Treat it as a sibling of model-selector-keyboard-navigation-e2e and decide the two together.

#### rpc-074-e2e-clear-no-notice
- class: write
- area: fspec CLI (PTY e2e of fspec-tui)
- scenarios: 1 mapped to deleted files
- rust code: rust/fspec-tui/src/app/dispatch_slash_clear.rs:44 (handle_slash_clear calls backend.clear_history and appends no notice line; doc at :12)
- rust tests: unit-level only, in files other features own: rust/fspec-tui/tests/slash_clear_ts_parity_rpc074.rs::clear_appends_no_notice_line_on_ok (owner: rpc074-clear-ts-parity) and rust/fspec-tui/tests/rpc074_source_shape.rs (owner: rpc-074-source-shape-regression-divergent-clear-strings)
- covered: none at the e2e tier
- not covered: Real fspec binary /clear keystroke leaves no divergent notice in rendered scrollback -> write
- action: write rust/fspec/tests/slash_clear_pty_rpc074.rs (--features test-stub-provider, PTY + vt100, matching scrollback_pty_rpc078.rs). Alternative: merge into rpc074-clear-ts-parity if the e2e tier is dropped.

#### changed-files-view-missing-unstaged-files
- class: merge
- area: git / fspec-tui
- scenarios: 4 mapped to deleted files
- rust code: rust/git/src/change_type.rs (A/M/D derivation), rust/rpc/src/changed_files.rs (staged flag), rust/fspec-tui/src/views/diff_common/row.rs:23 (A green, M yellow, D red). The Rust view is one list with a staged flag, not staged/unstaged sections.
- rust tests: rust/git/tests/changed_files_rpc355.rs (owner: git-change-type-derivation), rust/fspec-tui/src/views/changed_files/tests.rs + diff_common/row.rs (owner: rust-changed-files-view)
- covered: View untracked file in changed files view (untracked_working_tree_file_appears_as_change_type_a_and_unstaged), View staged file in changed files view (staged_tracked_but_modified_file_is_reported_as_change_type_m), View deleted file in changed files view (unstaged_file_deleted_from_the_working_directory_is_reported_as_change_type_d). Colours are covered by the rust-changed-files-view "colored status letters" scenario.
- not covered: View unstaged modification in changed files view -> write (new test in rust/git/tests/changed_files_rpc355.rs under git-change-type-derivation: modified tracked file gives M, staged=false)
- action: merge into git-change-type-derivation (plus rust-changed-files-view for colours). Sibling replace-git-cli-usage-with-isomorphic-git-library shares status.test.ts and is retired.

#### replace-git-cli-usage-with-isomorphic-git-library
- class: retire
- area: none (TS library migration)
- scenarios: 5 mapped to deleted files
- rust code: the status behaviour lives on in rust/git/src/status.rs:41/181/252 (gix), but every scenario is framed as git-context.ts / isomorphic-git / execa
- rust tests: none direct
- covered: none
- not covered: Get staged and unstaged files for virtual hooks -> retire (and Rust never passes them to hooks, see work-unit-scoped-hooks question), Detect all file changes for checkpoint system -> retire, Handle empty repository without crashing -> retire (or move to git-repository-operations as a write), Respect .gitignore when listing files -> retire (or move to git-repository-operations), Return empty arrays for clean repository -> retire (or move to git-repository-operations)
- action: @deprecated citing 103a73072 (isomorphic-git/TS removed) and GIT-039. Live status behaviour gets tested under git-repository-operations (rust/git/tests/git013_repository_operations.rs). Sibling changed-files-view-missing-unstaged-files (shares status.test.ts) is merged.

#### checkpoint-restore-shows-file-not-found-but-doesn-t-delete-files-added-after-checkpoint
- class: write
- area: git
- scenarios: 3 mapped to deleted files
- rust code: rust/git/src/ghost_commit.rs:601-616 (restore deletes files not in the checkpoint). rust/git/src/diff.rs:172-173 (get_checkpoint_file_diff returns "[File will be deleted on restore]"; used by rust/rpc/src/checkpoints.rs:122).
- rust tests: rust/git/tests/ghost_commit_tests.rs::test_restore_checkpoint_deletes_files_added_after_checkpoint (owner: ghost-commit-checkpoints)
- covered: Restore checkpoint deletes files added after checkpoint creation (dup of ghost-commit-checkpoints)
- not covered: Diff viewer shows clear deletion message for files not in checkpoint -> write (step text must say "[File will be deleted on restore]"); Restore checkpoint deletes multiple new files and restores modified files -> write
- action: drop S1 as a duplicate. New rust/git/tests/checkpoint_restore_deletes_new_files.rs for S2 (get_checkpoint_file_diff) and S3.

#### git-repository-operations
- class: write
- area: git
- scenarios: 7 mapped to deleted files
- rust code: rust/git/src/status.rs:41 get_staged_files, :181 get_unstaged_files, :252 get_untracked_files, :323 get_current_branch; rust/git/src/diff.rs:17 get_file_diff (unified diff and binary sentinel)
- rust tests: none direct (callers are tested via change-type and capture features: changed_files_rpc355.rs owner git-change-type-derivation, bug189_single_pass_stat_first.rs owner git-capture-single-pass-stat-first)
- covered: none
- not covered: Query staged files from repository -> write, Query unstaged files from repository -> write, Query untracked files from repository -> write, Request unified diff for changed file -> write, Detect and handle binary files in diff -> write (or drop as a duplicate of TUI-030), Query current branch name -> write, Maintain TypeScript API compatibility -> retire (NAPI/TS removed, REFAC-027 / 103a73072)
- action: write rust/git/tests/git013_repository_operations.rs. Drop the @napi tag and @deprecated-or-delete the TS API scenario. This file also gives replace-git-cli-usage-with-isomorphic-git-library's live behaviours (empty repo, .gitignore, clean repo) a home if the product owner wants them kept.

#### handle-binary-files-and-large-files-in-diff-display
- class: write
- area: git
- scenarios: 4 mapped to deleted files
- rust code: rust/git/src/diff.rs:17 get_file_diff (":35/:45 [Binary file - no diff available]"), diff.rs:221 MAX_LINES 20000 with "[File truncated - showing first 20000 of N lines]", rust/git/src/utils.rs is_binary_content (null byte in the first 8000 bytes)
- rust tests: rust/fspec-tui/tests/changed_files_rpc355.rs binary sentinel test (owner: git-changed-files-transport). The rust/git/src/utils.rs inline binary tests are unowned but test the helper only.
- covered: none in an ownable file
- not covered: Display binary file message for PNG image -> write, Truncate large file with over 20,000 lines -> write, Display complete diff for normal-sized text file -> write, Display binary file message for executable binary -> write
- action: write rust/git/tests/tui030_diff_binary_and_truncation.rs (calls get_file_diff on a temp repo). Also proves the truncation scenario that native-rust-diff-operations defers here.

#### isolated-session-worktree-initialization
- class: remap
- area: git
- scenarios: 3 mapped to deleted files
- rust code: rust/git/src/worktree.rs:65 create_worktree, rust/git/src/session_diff.rs:30 compute_session_diff
- rust tests: rust/git/tests/worktree_index_initialization_tests.rs::test_worktree_has_all_tracked_files_in_git_index, ::test_worktree_has_clean_git_status_after_creation, ::test_session_diff_shows_accurate_file_change_count (owner: isolated-session-worktree-initialization, this feature only)
- covered: all 3 (the file already carries /// @step doc comments with the exact scenario text. Inline // @step comments on 2 steps are paraphrased: "should show clean state" and "(call get_session_diff)". Fix them if the exact-match check reads the inline ones.)
- not covered: none
- action: remap the 3 scenarios to rust/git/tests/worktree_index_initialization_tests.rs (already mapped for scenario 4). Drop the src/git/__tests__/worktree-index-initialization.test.ts mappings.

#### native-rust-diff-operations
- class: merge
- area: git / rpc
- scenarios: 9 mapped to deleted files
- rust code: rust/git/src/diff.rs:17 (working-tree diff), :158 get_checkpoint_file_diff (restore preview, "[File will be deleted on restore]" at :173/:183, binary sentinel), :221 (20000-line truncation)
- rust tests: rust/fspec-tui/tests/changed_files_rpc355.rs (owner: git-changed-files-transport: file_diff unified and binary sentinel), rust/rpc/tests/checkpoint_transport_rpc362.rs::checkpoint_file_diff_returns_the_unified_diff_for_a_changed_file (owner: checkpoint-transport)
- covered: Working directory diff via NAPI returns unified diff (git-changed-files-transport), Checkpoint file diff via NAPI returns restore preview (checkpoint-transport), Binary file diff returns binary indicator (git-changed-files-transport). All duplicates.
- not covered: Checkpoint diff for file not in checkpoint returns deletion message -> write (new test in checkpoint_transport_rpc362.rs under checkpoint-transport. Note the Rust text is "[File will be deleted on restore]", not "Will be deleted"), Large diff is truncated at 20000 lines -> delete (duplicate of TUI-030 write), FileDiffViewer uses direct NAPI call instead of worker thread -> retire, CheckpointViewer uses direct NAPI call instead of worker thread -> retire, Build pipeline does not include esbuild diff-worker step -> retire
- action: merge into git-changed-files-transport and checkpoint-transport. Mark the 3 NAPI/worker/esbuild scenarios @deprecated citing REFAC-027 (f25b90b2e) and 103a73072.

#### blocklist-core-command-tool-filtering
- class: remap
- area: tools
- scenarios: 3 mapped to deleted files
- rust code: rust/tools/src/blocklist/middleware.rs:148 check_bash_command; middleware.rs:98 init_blocklist (system plus project hierarchy)
- rust tests: rust/tools/src/blocklist/mod.rs::test_block_git_checkout_with_guidance, ::test_block_cat_command_with_read_tool_guidance, ::test_project_config_overrides_system_config (owner: blocklist-core-command-tool-filtering, already mapped, has @step comments)
- covered: all
- not covered: none
- action: drop the deleted src/__tests__/blocklist-napi-integration.test.ts mapping. COV-073's sensitive-path-prompts maps the same TS file and should drop it the same way.

#### unicode-path-normalization
- class: remap
- area: tools
- scenarios: 12 mapped to deleted files
- rust code: rust/tools/src/unicode_path.rs:22-60 (normalize_unicode_whitespace) and resolve_unicode_path (exact → normalized → dir scan, :52-58). rust/fspec-core/src/commands/add_attachment.rs:57-61 deliberately has no Unicode fuzzy resolution (RPC-170 architecture note). rust/attachment-viewer/src/handlers/path.rs does lexical normalization only.
- rust tests: rust/tools/src/unicode_path.rs::test_normalize_u202f, ::test_normalize_all_variants, ::test_idempotent, ::test_path_separators_preserved, ::test_ascii_only_unchanged (owner: none). rust/tools/tests/unicode_path_test.rs::test_resolve_file_with_u202f_when_user_types_regular_space, ::test_resolve_file_with_regular_space_when_user_pastes_nbsp (owner: unicode-path-normalization-rust).
- covered: S1-S5 (normalize scenarios; need @step comments). Resolve file with U+202F when user types regular space and Resolve file with regular space when user pastes U+00A0 (dups of unicode-path-normalization-rust).
- not covered: Resolve returns exact path when file exists as-is -> write (inline in unicode_path.rs); NAPI callback normalizes positional args… -> retire (REFAC-027, f25b90b2e); NAPI callback normalizes named option values… -> retire (REFAC-027); Attachment server resolves Unicode-encoded paths via HTTP request -> delete (the Rust viewer serves registered paths verbatim; same scope cut as RPC-170); add-attachment resolves file with Unicode whitespace via resolveFilePath -> delete (deliberate divergence, add_attachment.rs:57-61, RPC-170)
- action: map rust/tools/src/unicode_path.rs (inline tests) to this feature with @step comments and add an exact-path test there. Drop the 2 resolve duplicates. Mark the 2 NAPI scenarios @deprecated (REFAC-027). Delete the 2 attachment scenarios, citing RPC-170.

#### block-notifications
- class: remap
- area: tools
- scenarios: 1 mapped to deleted files
- rust code: rust/tools/src/facade/wrapper.rs:790 emit_block_notification ("AI was blocked from {action} - {reason}"); rust/sessions/src/session_tool_callbacks.rs:163
- rust tests: rust/tools/tests/block_notifications_integration_test.rs::test_blocklist_check_returns_blocked_result and ::test_stage_permissions_block_impl_files_in_testing_stage (owner: block-notifications, already mapped, has @step comments)
- covered: all (the notification text and auto-dismiss steps are only commented, not asserted)
- not covered: none
- action: drop the deleted src/__tests__/block-notifications-napi-integration.test.ts mapping. Keep the existing Rust mapping (lines 65-124).

#### blocklist-system-integration-tests
- class: write
- area: tools (plus fspec-tui for one scenario)
- scenarios: 5 mapped to deleted files
- rust code: rust/tools/src/blocklist/middleware.rs:98 init_blocklist, :148 check_bash_command, :291 check_file_path (Prompt action, :315), :387 allow_for_session, :407 clear_session_allowances (restart resets); rust/tools/src/stage_permissions/mod.rs:134 check_write_permission; rust/fspec/src/common.rs:92 init_blocklist at startup; rust/fspec-tui/src/views/blocklist/panes.rs:81 shows session-disabled rules
- rust tests: only per-component tests owned by other features (blocklist-core, stage-permissions-acdd-file-write-enforcement, rpc056-blocklist-view-dispatch, project-blocklist-initialization). Nothing tests the end-to-end flows.
- covered: none
- not covered: Config hierarchy with session override -> write; Stage permissions block and allow based on work unit state -> write; Sensitive path prompts with session memory -> write; Blocklist system initializes automatically at TUI startup -> write; TUI blocklist view with session toggle -> write (fspec-tui)
- action: write rust/tools/tests/blocklist_system_integration.rs (scenarios 1, 2, 3, 5; "TUI restarted" = clear_session_allowances + init_blocklist) and rust/fspec-tui/tests/blocklist_system_integration_view.rs (scenario 4). Scenario 1's step order is odd ("a project blocklist rule allows" comes after "When") and needs a reorder.

#### core-fspectool-implementation
- class: write
- area: tools
- scenarios: 3 mapped to deleted files
- rust code: `rust/tools/src/facade/fspec_facade.rs:91-103` (definition name "Fspec", ACDD description, schema from FspecArgs). `rust/tools/src/fspec.rs:27-36` (command/args/project_root). `rust/tools/src/facade/wrapper.rs:1173-1250` (FspecToolFacadeWrapper::call; a failed result becomes `ToolError::Execution`). `rust/agent-loop/src/fspec_handler.rs:47` (in-process `codelet_fspec_core::dispatch_command`, no CLI spawn). Note: `rust/tools/src/fspec.rs:68` (`FspecTool::call`) itself is a stub that returns an Execution error. The live path is the facade wrapper, so the scenario steps naming `FspecTool.call()` need rewording.
- rust tests: none (rust/tools/tests/fspec_facade_wrapper_test.rs is unowned but its header names structured-fspectool-results-via-streamchunk-discriminated-union / CODE-009, so it belongs to that feature)
- covered: none
- not covered: Developer uses FspecTool alongside other codelet tools in same session -> write; Agent system tool initialization provides FspecTool definition -> write; Fspec command failures return consistent ToolError format -> write
- action: write rust/tools/tests/core_fspectool_implementation.rs (definition name/params; a handler returning success=false gives ToolError::Execution with the original message; list-work-units routed through a registered handler).

#### system-reminder-preservation
- class: write
- area: tools
- scenarios: 4 mapped to deleted files
- rust code: `rust/fspec-core/src/dispatch.rs:82` (DispatchResult.system_reminder) -> `rust/agent-loop/src/fspec_handler.rs:56` -> `rust/tools/src/facade/wrapper.rs:1237-1240`, which appends `system_reminder` after the data in the tool output. Rust has no console.error capture: commands return reminders as values.
- rust tests: none (fspec_facade_wrapper_test.rs always sets system_reminder: None)
- covered: none
- not covered: Parse result.systemReminder property from command response -> write; Capture system reminder from console.error during command execution -> delete (TS fspecCallback stderr capture, 103a73072); Parse raw <system-reminder> tags from console.error output -> delete (TS-only, 103a73072); Combine multiple system reminders in tool response -> delete (TS-only, 103a73072)
- action: write rust/tools/tests/system_reminder_preservation.rs (a handler returning a system_reminder shows up appended in the wrapper output). Delete the 3 console.error scenarios citing 103a73072/TOOL-019. Consistent with sibling CODE-005 (retire).

### Batch COV-074-B5

#### dynamic-thinking-level-detection-via-keywords
- class: retire
- area: agent-loop
- scenarios: 9 mapped to deleted files
- rust code: keyword detection was removed. spec/features/prompt-text-never-changes-the-thinking-level.feature (MODEL-013) says detect_thinking_level, has_disable_keywords and compute_effective_thinking_level are gone and that it supersedes this feature.
- rust tests: rust/agent-loop/tests/model013_prompt_text_never_changes_the_thinking_level.rs (owner: prompt-text-never-changes-the-thinking-level), which proves the removal
- covered: none
- not covered: all 9 -> retire
- action: @deprecated citing MODEL-013 (product owner, 2026-11-04; MODEL-009 research §8 item 8).

#### local-web-server-for-attachment-viewing-with-markdown-and-mermaid-rendering
- class: merge
- area: attachment-viewer
- scenarios: 7 mapped to deleted files
- rust code: `rust/attachment-viewer/src/lib.rs:68` start_viewer (random port) and :60 stop. `rust/fspec-tui/src/app/board_card_attachments.rs:213` starts a viewer lazily on the first attachment open (per workspace), not at TUI start. Theme toggle, Prism, copy button and badge are in `rust/attachment-viewer/src/markdown/template/scripts.rs`.
- rust tests: rust/attachment-viewer/tests/viewer_server.rs::block_directory_traversal_outside_the_project_directory, ::serve_a_binary_image_attachment_raw_with_the_correct_content_type, ::render_mermaid_code_blocks_for_client_side_rendering, ::start_on_a_random_local_port_and_stop_cleanly (owner: rust-attachment-viewer-server). rust/attachment-viewer/tests/viewer_prism_theme_fonts.rs::page_provides_theme_toggle_persisted_to_local_storage, ::page_adds_copy_buttons_and_language_badges, ::page_highlights_code_blocks_with_alias_map (owner: rust-viewer-prism-theme-fonts).
- covered: Block directory traversal attacks; Serve non-markdown files directly; Open markdown attachment with mermaid diagram in browser (server side, pre.mermaid); Theme detection and persistence; Syntax-highlighted code blocks with UI features
- not covered: Server starts automatically when TUI opens -> delete (Rust starts the viewer lazily on first attachment open; start and port are proven by rust-attachment-viewer-server); Server stops automatically when TUI exits -> delete (stop is proven by start_on_a_random_local_port_and_stop_cleanly)
- action: merge into rust-attachment-viewer-server (server scenarios) and rust-viewer-prism-theme-fonts (theme/highlighting). Drop all 7 as duplicates or deleted, then @deprecated / remove the feature.

#### anchor-links-not-working-in-markdown-attachment-viewer
- class: merge
- area: attachment-viewer
- scenarios: 7 mapped to deleted files
- rust code: `rust/attachment-viewer/src/markdown/slug.rs` (GitHub slugs with -N dedupe) and `rust/attachment-viewer/src/markdown/render.rs`
- rust tests: rust/attachment-viewer/tests/markdown_heading_anchors.rs::render_heading_with_single_word_title, ::render_heading_with_multi_word_title, ::deduplicate_repeated_heading_slugs, ::strip_special_characters_from_slug, ::anchor_link_round_trips_to_heading_id, ::code_and_mermaid_blocks_unaffected_by_heading_ids (owner: markdown-heading-anchors)
- covered: all (the code-block regression is proven via data-language="python" in code_and_mermaid_blocks_unaffected_by_heading_ids)
- not covered: none
- action: merge into markdown-heading-anchors and drop all 7 as duplicates, then @deprecated / remove the feature.

#### shared-configuration-management-utilities-for-user-and-project-config-files
- class: merge
- area: common
- scenarios: 5 mapped to deleted files
- rust code: `rust/common/src/fspec_config.rs:43-123` (load_config/load_config_with_dirs, deep_merge :78, "Invalid JSON in <path>: <msg>" :57, write per scope)
- rust tests: rust/common/tests/config008_shared_config_merge.rs::missing_files_load_as_an_empty_object_without_error, ::user_only_config_loads_when_no_project_file_exists, ::nested_objects_merge_recursively_preserving_sibling_keys, ::arrays_and_scalars_replace_instead_of_merging, ::invalid_json_returns_an_error_naming_the_offending_path, ::writing_user_scope_creates_the_data_directory_and_file (owner: shared-config-merge)
- covered: all (differences: the user scope is `<data_dir>/fspec-config.json`, not ~/.fspec, and the JS "Unexpected token" text is a serde message in Rust)
- not covered: none
- action: merge into shared-config-merge (CONFIG-008) and drop all 5 as duplicates, then @deprecated / remove the feature.

#### fspec-help-displays-hardcoded-version-0-0-1-instead-of-package-json-version
- class: merge
- area: fspec CLI
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec/src/cli_registration/cli.rs:5. clap `version` takes CARGO_PKG_VERSION at compile time, and `fspec --version` prints it. Rust `fspec --help` has no "Version" line.
- rust tests: rust/fspec/tests/cargo_shape/fspec_version_prints_the.rs::scenario_fspec_version_prints_the_workspace_version (owner: fspec-binary-cargo-shape-rpc010)
- covered: Display current version from package.json and Display updated version after package.json change. The intent (version comes from the manifest, not a hardcoded value) is proven by the RPC-010 test.
- not covered: Omit version line when version cannot be read -> delete (env! is resolved at compile time, so it cannot fail at runtime)
- action: merge into fspec-binary-cargo-shape-rpc010 and delete this feature (or @deprecated citing 103a73072, since package.json and `npm run build` are gone).

#### commonpatterns-help-formatting
- class: merge
- area: fspec CLI / fspec-core help
- scenarios: 3 mapped to deleted files
- rust code: rust/fspec-core/src/help/mod.rs:165-180 (COMMON PATTERNS renders Bullet as "  • s"; Structured as pattern, "Example:" and description). The typed CommonPatternEntry (:42-52) makes "[object Object]" impossible.
- rust tests: rust/fspec/tests/cli_add_virtual_hook.rs::scenario_add_virtual_hook_help_matches_ts_formatcommandhelp_reference (owner: add-virtual-hook-cli-subcommand; byte-for-byte vs fixtures/help/add-virtual-hook.txt, which has structured patterns). rust/fspec/tests/cli_add_command.rs help fixture test (owner: add-command-cli-subcommand; Bullet-style patterns). The other -cli-subcommand help-fixture tests.
- covered: all 3 (Display formatted patterns for object-style commonPatterns; All affected commands display COMMON PATTERNS correctly; Backward compatibility with string array format)
- action: drop all 3 as duplicates of the -cli-subcommand help-fixture tests, then @deprecated / remove the feature. Same approach as fix-help-formatter-option-name-rendering… (COV-072).

#### fix-fspec-research-tool-ast-help-command-should-display-tool-specific-help-instead-of-failing
- class: human
- area: fspec-core / fspec CLI (research)
- scenarios: 5 mapped to deleted files
- rust code: rust/fspec/src/cli_registration/mode_08.rs:276 (research subcommand is list-only, and `--help` renders the generic research help via support_03.rs:235). rust/fspec-core/src/commands/research.rs:195 (unknown tool gives "Research tool not found" plus the list). The ast tool was removed from the registry (CLI-015). There is no per-tool help, no getHelpConfig and no custom JS tools.
- rust tests: rust/fspec/tests/cli_research.rs::cli_fails_with_not_found_error_for_unknown_tool (owner: research-cli-subcommand), rust/fspec-core/tests/research_registry_drops_ast_tool.rs (owner: research-registry-drops-ast-tool)
- covered: Error when requesting help for nonexistent tool is close to the research-cli-subcommand test, but Rust has no `--help` interplay (clap prints help first)
- not covered: Display help for bundled tool with standard sections -> retire (ast removed, CLI-015), Display help for tool requiring configuration -> retire or write (per answer), Display help for custom tool in spec/research-tools/ -> retire (TS custom tools), Refactor tool from string-based help to structured config -> retire (TS refactor)
- action: if (a), @deprecated citing CLI-015 and RPC-286. If (b), write per-tool help in the research port card. Keep consistent with sibling tool-specific-help-system (RES-011, COV-072), which shares research-help.test.ts, and with RES-006 above.
- question: Rust research has no per-tool help because tool execution is deferred (RPC-286) and ast moved to `fspec astgrep` (CLI-015). Should BUG-074 be (a) @deprecated citing CLI-015 and RPC-286 [recommended], or (b) kept, with per-tool `--help` added when research execution is ported?

#### confluence-research-tool-integration
- class: human
- area: fspec-core
- scenarios: 10 mapped to deleted files
- rust code: rust/fspec-core/src/commands/research.rs:207. A known tool returns "Research tool execution is not yet ported to the Rust binary". :70 has a static REGISTRY with no spec/research-scripts discovery.
- rust tests: none for execution. List and config status are in rust/fspec-core/tests/research.rs (owner: research-rust-port).
- covered: none
- not covered: all 10 -> retire or write, depending on the answer
- action: if yes, @deprecated citing RPC-286 rule [4] ("Actual execution of network/NAPI/dynamic-JS research tools is OUT OF SCOPE"). If no, leave it with a backlog card to port execution.
- question: The Rust `fspec research` only lists tools, by RPC-286 rule [4]. Running a tool fails with "not yet ported". Should the Confluence/Jira/Perplexity research-tool features be marked @deprecated, citing RPC-286? Recommended: yes, deprecate. Or no: keep them and open a card to port research tool execution.

#### jira-research-tool-integration
- class: human
- area: fspec-core
- scenarios: 10 mapped to deleted files
- rust code: rust/fspec-core/src/commands/research.rs:207 (execution "not yet ported"); :70 static REGISTRY lists jira with url+token config status only
- rust tests: none for execution
- covered: none
- not covered: all 10 -> retire or write, depending on the answer
- action: same decision as confluence-research-tool-integration. If yes, @deprecated citing RPC-286 rule [4].
- question: Same question as confluence-research-tool-integration: deprecate the Jira research-tool feature, citing RPC-286 rule [4] (Rust research only lists tools)? Recommended: yes. Or no: port execution.

#### perplexity-research-tool-integration
- class: human
- area: fspec-core
- scenarios: 10 mapped to deleted files
- rust code: rust/fspec-core/src/commands/research.rs:207 (execution "not yet ported"); :70 static REGISTRY lists perplexity with apiKey config status only
- rust tests: none for execution. rust/fspec-core/tests/research.rs (owner: research-rust-port) proves only the "configured" status.
- covered: none
- not covered: all 10 -> retire or write, depending on the answer
- action: same decision as confluence-research-tool-integration. If yes, @deprecated citing RPC-286 rule [4].
- question: Same question as confluence-research-tool-integration: deprecate the Perplexity research-tool feature, citing RPC-286 rule [4]? Recommended: yes. Or no: port execution.

#### interactive-research-command-with-multiple-backend-support
- class: human
- area: fspec-core / fspec CLI
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-core/src/commands/research.rs:70 (list mode), :195 (unknown-tool error), :207 (execute "not yet ported"); no spec/research-scripts auto-discovery and no attachment prompt
- rust tests: rust/fspec-core/tests/research.rs (owner: research-rust-port); rust/fspec/tests/cli_research.rs (owner: research-cli-subcommand)
- covered: List available research tools without arguments (proved by research-cli-subcommand "CLI lists available research tools..." / "...includes per-tool usage guidance"); Error on invalid research tool (proved by research-rust-port "Execute mode rejects an unknown tool name..."). Rust does not print the step 'suggest running "fspec research"'; its message lists the bundled tools instead.
- not covered: Execute perplexity research tool with query, Execute jira research tool with issue key, Execute confluence research tool with page title, Attach research results to work unit, Decline attachment prompt -> retire or write, depending on the answer; Auto-discover new research tool -> retire (Rust uses a static registry)
- action: drop the 2 covered scenarios as duplicates of research-rust-port and research-cli-subcommand. If yes, @deprecated citing RPC-286 rule [4] for the rest.
- question: Same question as confluence-research-tool-integration: Rust `fspec research` only lists tools (RPC-286 rule [4]). Deprecate RES-001's execute, attach and auto-discover scenarios and drop its list/invalid-tool scenarios as duplicates of research-rust-port/research-cli-subcommand? Recommended: yes. Or no: port execution and attachment.

#### stakeholder-communication-research-script-for-unanswered-questions
- class: human
- area: fspec-core (research)
- scenarios: 7 mapped to deleted files
- rust code: rust/fspec-core/src/commands/research.rs:207. The Rust research command only lists tools. Any known --tool returns "Research tool execution is not yet ported to the Rust binary". "stakeholder" is listed with a teamsWebhook config check (research.rs:85). There is no plugin discovery, no Teams or Slack sending and no message formatting. Help text still advertises `--tool=stakeholder` (help/configs/research.rs:49).
- rust tests: none for sending. rust/fspec-core/tests/research.rs (owner: research-rust-port) covers listing only.
- covered: none
- not covered: all 7 -> retire or write, depending on the answer
- action: if (a), @deprecated citing RPC-286 (execute path deferred, LIST-only port) and remove the stakeholder example from help/configs/research.rs. If (b), open a port card. Same decision as BUG-074 in this chunk and the other research-tool features (RES-001/002/003).
- question: Rust `fspec research` is list-only (RPC-286 deferred tool execution, and no card ports it). Should RES-006 stakeholder messaging (Teams/Slack plugins) be (a) @deprecated citing RPC-286 [recommended], or (b) kept, with a new card to port research tool execution?

#### build-migration-system
- class: human
- area: fspec-core
- scenarios: 10 mapped to deleted files
- rust code: none found. There is no `migrate` subcommand or dispatch entry (grep `migrate` in rust/fspec/src, dispatch.rs, help_dispatch_table.rs) and no backup, up()/down(), or auto-migrate on first command. Rust only pins `CURRENT_VERSION = "0.7.1"` (`rust/fspec-core/src/types/work_unit.rs:12`) and round-trips an existing `migrationHistory` key unchanged (work_unit.rs:25-48).
- rust tests: none
- covered: none
- not covered: all 10 -> retire (or write if ported)
- action: if retired: @deprecated citing 103a73072 (TS src/migrations removed; the Rust port has no migration system). If ported: new card plus rust/fspec/tests/cli_migrate.rs.
- question: The Rust binary has no migration system and no `fspec migrate` command (TS src/migrations was removed in 103a73072). Should build-migration-system be (a) marked @deprecated citing 103a73072 [recommended: every on-disk file is already at 0.7.x and Rust preserves migrationHistory], or (b) kept, with a card to port `fspec migrate` and auto-migration?

#### ast-research-tool-fails-with-file-is-required-error-even-when-file-flag-is-provided
- class: retire
- area: fspec-core
- scenarios: 2 mapped to deleted files
- rust code: `rust/fspec-core/src/commands/research.rs:180-200` rejects `--tool=ast` ("Research tool not found: ast … use the `fspec astgrep` command instead"). The research ast tool no longer exists (CLI-015).
- rust tests: rust/fspec-core/tests/research_registry_drops_ast_tool.rs::execute_mode_rejects_ast_tool_and_points_to_fspec_astgrep (owner: research-registry-drops-ast-tool), which proves the removal
- covered: none
- not covered: both -> retire
- action: @deprecated citing CLI-015 (research registry drops the ast tool; AST search moved to `fspec astgrep` / the AstGrep tool).

#### refactor-ast-tool-to-use-deterministic-tree-sitter-queries-instead-of-semantic-natural-language-parsing
- class: retire
- area: fspec-core
- scenarios: 7 mapped to deleted files
- rust code: `rust/fspec-core/src/commands/research.rs:180-200` rejects `--tool=ast`. None of `--operation=list-functions/find-class/find-functions/find-identifiers/find-async-functions/find-exports` or `--query-file` exist anywhere in Rust.
- rust tests: research_registry_drops_ast_tool.rs (owner: research-registry-drops-ast-tool), which proves the removal
- covered: none
- not covered: all 7 -> retire
- action: @deprecated citing CLI-015.

#### ast-tool-returns-stub-instead-of-actual-parsing
- class: retire
- area: fspec-core
- scenarios: 1 mapped to deleted files
- rust code: `rust/fspec-core/src/commands/research.rs:180-200` rejects `--tool=ast`
- rust tests: research_registry_drops_ast_tool.rs (owner: research-registry-drops-ast-tool), which proves the removal
- covered: none
- not covered: AST tool executes actual tree-sitter parsing instead of returning stub -> retire
- action: @deprecated citing CLI-015.

### Batch COV-074-B6

#### ast-research-tool-fails-with-relative-file-paths
- class: retire
- area: fspec-core
- scenarios: 2 mapped to deleted files
- rust code: `rust/fspec-core/src/commands/research.rs:180-200` rejects `--tool=ast`
- rust tests: research_registry_drops_ast_tool.rs (owner: research-registry-drops-ast-tool), which proves the removal
- covered: none
- not covered: both -> retire
- action: @deprecated citing CLI-015.

#### jira-research-tool-outputs-object-object-in-markdown-format-for-description-field
- class: human
- area: fspec-core
- scenarios: 3 mapped to deleted files
- rust code: none found. The execute path is deferred: `rust/fspec-core/src/commands/research.rs:201-208` returns "Research tool execution is not yet ported to the Rust binary (tool 'jira')". There is no ADF parsing and no "No description provided" text (grep adf, `[object Object]`, jira in research.rs). Module doc research.rs:5-13 records the RPC-286 supervisor decision "Option 1, LIST-only … EXECUTE path DEFERRED". No backlog card exists to port it.
- rust tests: none
- covered: none
- not covered: all 3 -> retire (or write if execution is ported)
- action: if retired: @deprecated citing RPC-286 (research execute path not ported). If kept: needs a port card first.
- question: `fspec research --tool=jira` cannot run in Rust (RPC-286 ported LIST mode only, and no card exists to port execution). Should BUG-076 be (a) marked @deprecated citing RPC-286 [recommended], or (b) kept, with a new card to port research tool execution (jira/confluence/perplexity)?

#### add-kotlin-language-support-to-ast-research-tool
- class: retire
- area: fspec-core
- scenarios: 1 mapped to deleted files
- rust code: `rust/fspec-core/src/commands/research.rs:180-200` rejects `--tool=ast`. Kotlin AST search lives on in the AstGrep tool (`rust/tools/src/astgrep.rs`), but not as `fspec research --tool=ast --operation=list-functions`.
- rust tests: research_registry_drops_ast_tool.rs (owner: research-registry-drops-ast-tool), which proves the removal
- covered: none
- not covered: Parse Kotlin file and list functions -> retire
- action: @deprecated citing CLI-015. The feature has no work-unit tag, so the helper may also need to add one.

#### add-java-php-ruby-bash-support-to-ast-research-tool
- class: retire
- area: fspec-core
- scenarios: 7 mapped to deleted files
- rust code: `rust/fspec-core/src/commands/research.rs:180-200` rejects `--tool=ast`. The research list has no ast entry (research.rs:67, 4 tools).
- rust tests: research_registry_drops_ast_tool.rs (owner: research-registry-drops-ast-tool), which proves the removal
- covered: none
- not covered: all 7 -> retire
- action: @deprecated citing CLI-015. Same decision as sibling add-multi-language-support-to-ast-research-tool-kotlin-dart-python-go-rust-swift-c-c-c (shared multi-language-support.test.ts).

#### add-multi-language-support-to-ast-research-tool-kotlin-dart-python-go-rust-swift-c-c-c
- class: retire
- area: fspec-core
- scenarios: 7 mapped to deleted files
- rust code: `rust/fspec-core/src/commands/research.rs:180-200` rejects `--tool=ast`
- rust tests: research_registry_drops_ast_tool.rs (owner: research-registry-drops-ast-tool), which proves the removal
- covered: none
- not covered: all 7 -> retire
- action: @deprecated citing CLI-015. Same decision as sibling add-java-php-ruby-bash-support-to-ast-research-tool.

#### event-storm-data-model-for-work-units
- class: merge
- area: fspec-core
- scenarios: 4 mapped to deleted files
- rust code: `rust/fspec-core/src/commands/add_domain_event.rs:200-209` (id, type event, color orange, deleted false, createdAt, optional timestamp; nextItemId post-increment). There is no Rust code for `triggersEvent`/`relatedTo` linking, the `suggestedTags` algorithm, or a work-unit Event Storm soft-delete/restore command (grep across rust/). In TS these existed only as interface fields (`src/types/index.ts:65,128` at 103a73072^), with no behaviour.
- rust tests: rust/fspec-core/tests/add_domain_event.rs (owner: add-domain-event-rust-port)
- covered: Store Process Modeling Event Storm with multiple domain events (dup of add-domain-event-rust-port)
- not covered: Create relationships between commands and events -> delete (type-only field, no behaviour in TS or Rust); Generate suggested tags from Event Storm artifacts -> delete (no algorithm in TS or Rust); Soft-delete Event Storm item with stable ID preservation -> delete (no work-unit event-storm delete/restore command in Rust)
- action: merge scenario 1 into add-domain-event-rust-port as a duplicate, delete the other 3, then @deprecated / remove the feature.

#### enhanced-research-tool-guidance-in-system-reminders
- class: human
- area: fspec-core
- scenarios: 7 mapped to deleted files
- rust code: none found. grep for "CODE-RELATED", "RESEARCH-HEAVY", "USE NATURAL LANGUAGE", "(configured)" and "No configuration required" finds nothing in fspec-core. Rust create-bug/create-story port the older fixed reminders (create_bug.rs:311-339). The specifying reminder names astgrep instead of the ast tool (update_work_unit_status/reminders.rs:44, CLI-015). The research tool list has no ast tool (research.rs:180-200, CLI-015), and research execution is deferred (research.rs:201-208, RPC-286). TS had this code until 103a73072.
- rust tests: none
- covered: none
- not covered: all 7 -> retire (or write if ported). S7 "getStatusChangeReminder is fully async" -> retire either way (TS Promise internals).
- action: if retired, @deprecated citing CLI-015 (ast tool removed) + RPC-286 (research execution not ported).
- question: The context-aware research-tool reminders (REMIND-013: "CODE-RELATED task" / "RESEARCH-HEAVY story", AST emphasis, the 5-tool configured/not-configured list) were not ported. The AST tool they push was removed (CLI-015), and research tool execution is deferred (RPC-286). Should REMIND-013 be (a) marked @deprecated citing CLI-015 + RPC-286 [recommended; the Rust specifying reminder already points at `fspec astgrep`], or (b) kept, with a card to port the reminders rewritten around astgrep and the 4 remaining research tools?

#### implement-file-locking-system-lockedfilemanager-refactoring
- class: retire
- area: fspec-core
- scenarios: 1 mapped to deleted files (Create comprehensive unit tests for LockedFileManager)
- rust code: rust/fspec-core/src/io/locked_file.rs:1-24 (BUG-477 design: one exclusive reentrant spec lock for writes, lock-free readers through atomic rename, no reader/writer locks, no exponential backoff). rust/common/src/spec_lock.rs:47-55 (60s timeout, FSPEC_SPEC_LOCK_TIMEOUT_MS).
- rust tests: rust/common/src/spec_lock.rs inline (unowned) and rust/common/tests/bug477_spec_lock_reentrancy.rs (owner: the-spec-lock-is-reentrant-within-a-thread). These cover the new design, not this scenario.
- covered: none
- not covered: Create comprehensive unit tests for LockedFileManager -> retire (meta scenario about writing src/utils/__tests__/file-manager.test.ts)
- action: @deprecated on that scenario, citing BUG-477 (concurrent-commands-never-corrupt-spec-json-files replaced the reader/writer-lock design) + 103a73072.

#### shift-arrow-session-switching
- class: retire
- area: fspec-tui
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-tui/src/store/agent_view/navigation.rs:41 (navigate_next: CreateDialog at the last index, no wrap) and :57 (navigate_prev: Board at the first index, no wrap). cycle_session was removed (RPC-096).
- rust tests: rust/fspec-tui/tests/shift_arrow_end_of_list_rpc096.rs (owner: agentview-shift-arrow-end-of-list-parity), rust/fspec-tui/tests/pending_input_durability_rpc052.rs (owner: pending-input-draft-persistence)
- covered: Switch to next session with Shift+Right, Switch to previous session with Shift+Left and Input text preserved when switching sessions are already proven by RPC-096 (mid-list switch plus draft round-trip) and RPC-052. Duplicates.
- not covered: Wrap around from last to first session with Shift+Right -> retire (now opens the Create Session dialog), Wrap around from first to last session with Shift+Left -> retire (now exits to Board), No action with only one session -> retire (RPC-096: Shift+Right opens the dialog, Shift+Left exits to Board), No action when in resume mode -> retire, Running session continues in background after switch -> retire (duplicate of the background-session features)
- action: @deprecated citing VIEWNV-001 and RPC-096. They replaced the wrap-around cycle with Board at the left end and the Create Session dialog at the right end.

#### refactor-checkpoint-counts-to-be-command-triggered-instead-of-file-watching
- class: retire
- area: fspec-tui
- scenarios: 6 mapped to deleted files
- rust code: rust/fspec-core/src/commands/checkpoint.rs:23-25 (IPC "checkpoint-changed" is a documented no-op; there is no /tmp/fspec-tui.sock or named pipe). Live counts come from GitStateWatcher push (codelet_core::git_state, BUG-181/BUG-182).
- rust tests: rust/fspec-tui/tests/bug181_checkpoint_watcher_push.rs (owner: live-checkpoint-counts-watcher-push; manual, auto and delete publish fresh counts)
- covered: none (S1-S3 count updates are proven by the watcher owner via a different mechanism)
- not covered: all 6 -> retire (IPC socket, zustand store)
- action: @deprecated citing BUG-181 (watcher push replaced the TUI-016 IPC trigger) + checkpoint.rs:23.

#### demo-scripts-incorrectly-try-to-import-non-existent-testcontroller-class
- class: retire
- area: none
- scenarios: 6 mapped to deleted files
- rust code: none found: the feature is about TS demo scripts in the separate fspec.videos repo using @microsoft/tui-test, node-pty and npx tsx. The mapped file ../fspec.videos/src/__tests__/demo-script-api-fix.test.ts is outside this repo, and /home/rquast/projects/fspec.videos does not exist. No work unit mentions TestController.
- rust tests: none
- covered: none
- not covered: all 6 -> retire
- action: @deprecated: TypeScript tooling (@microsoft/tui-test) in the external fspec.videos repo. The tui-test patch and TS e2e tooling were removed in 103a73072. Drop the coverage mapping.

#### browser-tab-creation
- class: retire
- area: none
- scenarios: 8 mapped to deleted files
- rust code: none found. I grepped rust/ for browser_create_tab, NATIVE_TOOLS, native messaging and webmcp. The only hits are generic MCP fixture names in rust/tools/src/mcp/helper_tests_0{4,5}.rs.
- rust tests: none
- covered: none
- not covered: all 8 -> retire
- action: @deprecated citing 14cb69eb6 ("remove: Chrome extension ... no longer used"). The extension and spec/skills/extension-skill.md were removed on purpose.

#### browser-script-execution
- class: retire
- area: none
- scenarios: 6 mapped to deleted files
- rust code: none found: browser_execute_script, userScripts and USER_SCRIPT world are absent from rust/. rust/tools/src/chrome_browser.rs:185 evaluate_js is a headless web_search helper, not this extension tool.
- rust tests: none
- covered: none
- not covered: all 6 -> retire
- action: @deprecated citing 14cb69eb6 (Chrome extension removed).

#### screenshot-native-messaging-resilience
- class: retire
- area: none
- scenarios: 6 mapped to deleted files
- rust code: none found. The native-messaging reader and encoder lived in extension/host/lib/native-messaging.mjs, which was deleted. rust/tools/src/chrome_browser.rs:217 capture_screenshot is the web_search tool's, with no tiling, 1568px resizing or native-messaging framing.
- rust tests: none
- covered: none
- not covered: all 6 -> retire
- action: @deprecated citing 14cb69eb6 (Chrome extension removed).

#### dom-scanning
- class: retire
- area: none
- scenarios: 13 mapped to deleted files
- rust code: none found: no browser_scan, data-fspec-ref or @ref scanner in rust/
- rust tests: none
- covered: none
- not covered: all 13 -> retire
- action: @deprecated citing 14cb69eb6 (Chrome extension removed).

#### element-targeted-screenshot
- class: retire
- area: none
- scenarios: 8 mapped to deleted files
- rust code: none found: no @ref or selector element screenshot in rust/
- rust tests: none
- covered: none
- not covered: all 8 -> retire
- action: @deprecated citing 14cb69eb6 (Chrome extension removed). Decide together with dom-scanning and ref-state-management, which share the @ref model.

### Batch COV-074-B7

#### iframe-aware-dom-scanning
- class: retire
- area: none
- scenarios: 15 mapped to deleted files
- rust code: none found: no frame-aware scanner in rust/
- rust tests: none
- covered: none
- not covered: all 15 -> retire
- action: @deprecated citing 14cb69eb6 (Chrome extension removed).

#### advanced-interactivity-heuristics
- class: retire
- area: none
- scenarios: 9 mapped to deleted files
- rust code: none found. The @codelet tag is historical; there is no interactivity heuristics code in rust/.
- rust tests: none
- covered: none
- not covered: all 9 -> retire
- action: @deprecated citing 14cb69eb6 (Chrome extension removed).

#### page-diff
- class: retire
- area: none
- scenarios: 8 mapped to deleted files
- rust code: none found: no browser_diff or page diff in rust/
- rust tests: none
- covered: none
- not covered: all 8 -> retire
- action: @deprecated citing 14cb69eb6 (Chrome extension removed).

#### ref-resolution-click-fill
- class: retire
- area: none
- scenarios: 7 mapped to deleted files
- rust code: none found: no @ref click/fill resolution in rust/
- rust tests: none
- covered: none
- not covered: all 7 -> retire
- action: @deprecated citing 14cb69eb6 (Chrome extension removed).

#### ref-state-management
- class: retire
- area: none
- scenarios: 8 mapped to deleted files
- rust code: none found. The @codelet tag is historical; there is no ref state store in rust/.
- rust tests: none
- covered: none
- not covered: all 8 -> retire
- action: @deprecated citing 14cb69eb6 (Chrome extension removed).

#### browser-scan-diff-documentation
- class: retire
- area: none
- scenarios: 7 mapped to deleted files
- rust code: none found. The skill doc spec/skills/extension-skill.md and the MCP server NATIVE_TOOLS (extension/host/lib/mcp-server.mjs) were deleted.
- rust tests: none
- covered: none
- not covered: all 7 -> retire
- action: @deprecated citing 14cb69eb6 (Chrome extension and extension-skill.md removed).

#### webmcp-dynamic-tool-discovery
- class: retire
- area: none
- scenarios: 9 mapped to deleted files
- rust code: none found: no navigator.modelContext or WebMCP discovery script in rust/. "webmcp" appears only as a generic MCP server name in rust/tools/src/mcp/helper_tests_05.rs:13.
- rust tests: none
- covered: none
- not covered: all 9 -> retire
- action: @deprecated citing 14cb69eb6 (Chrome extension removed).

#### ast-code-researcher-for-pattern-detection-and-deep-analysis
- class: retire
- area: none (superseded by tools AstGrep / fspec astgrep)
- scenarios: 8 mapped to deleted files
- rust code: rust/fspec-core/src/commands/research.rs:70 (REGISTRY has no `ast` tool); research.rs:195 (an unknown tool, including `ast`, gets a message pointing to `fspec astgrep`). The tree-sitter Node script in spec/research-scripts/ast no longer exists.
- rust tests: rust/fspec-core/tests/research_registry_drops_ast_tool.rs (owner: research-registry-drops-ast-tool) proves the removal
- covered: none
- not covered: all 8 -> retire
- action: @deprecated citing CLI-015 ("Replace research --tool=ast with AstGrep tool guidance and new fspec astgrep command"). The TS test was already deleted in 561c28783. The COV-074 multi-language AST siblings, which share src/research-tools/__tests__/multi-language-support.test.ts, should cite the same card.

#### codelet-napi-rs-native-module-bindings
- class: retire
- area: none
- scenarios: 10 mapped to deleted files
- rust code: none found: the rust/napi crate (BackgroundSession JS getters and the prompt callback) was removed. The session engine lives in rust/sessions under other features.
- rust tests: none
- covered: none
- not covered: all 10 -> retire
- action: @deprecated citing REFAC-027 (f25b90b2e, "remove the legacy codelet-napi crate"). The TS test was already deleted in b8613f4d3.

#### github-actions-ci-cd-for-codelet-napi-cross-platform-builds
- class: retire
- area: none
- scenarios: 11 mapped to deleted files
- rust code: none found. .github/workflows/build-codelet-napi.yml was deleted in 5969cfb58, and only release.yml remains. The NAPI crate was removed by REFAC-027.
- rust tests: none
- covered: none
- not covered: all 11 -> retire
- action: @deprecated citing REFAC-027 (f25b90b2e) and 5969cfb58 (workflow removed). The TS test was already deleted in 5a78fe15b.

#### jsonl-format-investigation-for-work-units-json-scalability
- class: retire
- area: none
- scenarios: 5 mapped to deleted files
- rust code: none found. There is no work-units.jsonl, migrate-to-jsonl or JSONL streaming in rust/, and fspec-core still reads and writes the monolithic spec/work-units.json. This was a TECH-001 investigation spike whose proof-of-concept was never adopted.
- rust tests: none
- covered: none
- not covered: all 5 -> retire
- action: @deprecated citing TECH-001 (investigation only, JSONL never adopted). The investigation test was already deleted in 67b5947d8.

#### napi-session-co-listener-parity
- class: retire
- area: none
- scenarios: 1 mapped to deleted files
- rust code: none found: sessionSetGlobalChunkCallback and the NAPI host are gone. The Rust embedded chunks broadcast is owned by replace-global-chunk-callback-background-session (rust/sessions/tests/background_session_shape/chunk_broadcast_*.rs).
- rust tests: none for the NAPI half
- covered: none
- not covered: the single scenario -> retire
- action: @deprecated citing REFAC-027 (f25b90b2e, NAPI crate removed).

#### napi-work-units-watcher-callback
- class: retire
- area: none
- scenarios: 1 mapped to deleted files
- rust code: none found: the NAPI startWorkUnitsWatcher shim and its ThreadsafeFunction were removed with rust/napi. The Rust WorkUnitsWatcher is covered by its own features.
- rust tests: none
- covered: none
- not covered: the single scenario -> retire
- action: @deprecated citing REFAC-027 (f25b90b2e, NAPI crate removed).

#### parent-command-for-quick-return
- class: retire
- area: none
- scenarios: 3 mapped to deleted files
- rust code: none found: there is no "/parent", "Switched to parent session" or "This session has no parent" in rust/fspec-tui, rust/tui or rust/sessions
- rust tests: none
- covered: none
- not covered: all 3 -> retire
- action: @deprecated citing WATCH-024 (rule 14 / answered question: "Remove the /parent command entirely — it's pointless. Users can navigate with Shift+Arrow").

#### rpc-068-final-ts-regression-and-boundary-audit
- class: retire
- area: none (TS/NAPI audit)
- scenarios: 6 mapped to deleted files
- rust code: none for 5 of 6. Rust has no NAPI crate, no index.d.ts, no rust/napi/src/persistence and no vitest suite. Scenario 1 (no_napi_dependency targets) still exists as rust/{core,sessions,rpc-types,fspec,fspec-tui,agent-loop}/tests/no_napi_dependency.rs. spec/attachments/RPC-068/boundary-audit-report.md exists.
- rust tests: rust/fspec/tests/no_napi_dependency.rs (owner: codelet-fspec-no-napi-regression), rust/fspec-tui/tests/no_napi_dependency.rs (owner: codelet-fspec-tui-no-napi-regression), rust/sessions/tests/no_napi_dependency.rs (owner: codelet-sessions-no-napi-regression)
- covered: none (the dependency rule in scenario 1 is already proven, crate by crate, by the *-no-napi-regression features)
- not covered: Dependency-rule regression tests pass across every forbidden crate -> retire (duplicate of the per-crate no-napi features), GLOBAL_CHUNK_CALLBACK is removed from executable code -> retire, TS-facing NAPI export surface is a strict superset... -> retire, codelet-napi persistence collapses to a thin adapter -> retire, TS test suite remains green after the watch-024 path fix -> retire, Boundary audit report is committed for future verification -> retire
- action: @deprecated citing REFAC-027 (f25b90b2e, codelet-napi removed) and 103a73072 (TS client removed). This was a one-off audit of a NAPI/TS surface that no longer exists.

#### fix-all-failing-tests-15-failures-across-3-test-suites
- class: retire
- area: none (vitest maintenance)
- scenarios: 4 mapped to deleted files
- rust code: none. Every scenario is about `npm test`, TS AST research tool tests, Ink TUI formatting tests and TS research integration tests.
- rust tests: none
- covered: none
- not covered: all 4 -> retire
- action: @deprecated citing 103a73072 (TS client and vitest suite removed). This was a one-off test-maintenance task for TypeScript.

### Batch COV-074-B8

#### eliminate-tree-sitter-legacy-peer-deps-requirement
- class: retire
- area: none (npm dependency management)
- scenarios: 4 mapped to deleted files
- rust code: none. npm install, package.json overrides and node_modules tree-sitter@0.25.0 no longer exist. Rust parsing lives in rust/graph (cargo crates).
- rust tests: none
- covered: none
- not covered: all 4 -> retire
- action: @deprecated citing 103a73072 (npm/package.json build removed). Parser behaviour is owned by the graph AST features.

#### remove-isomorphic-git-dependency
- class: retire
- area: none (TS dependency)
- scenarios: 4 mapped to deleted files
- rust code: none. CheckpointViewer NAPI resolveRef, fspecStore stash loading, TS test infra and the package.json isomorphic-git dependency are all TS. Rust uses gix (rust/git/src/repo_ops.rs:24 resolve_ref).
- rust tests: none
- covered: none
- not covered: all 4 -> retire
- action: @deprecated citing 103a73072 (TS client and package.json removed) and REFAC-027 (NAPI removed).

#### implement-automated-discovery-code-analysis
- class: retire
- area: none
- scenarios: 5 mapped to deleted files
- rust code: none. Automated project-type and persona inference was deleted in TS by d6d3169ba (src/guidance/automated-discovery-code-analysis.ts removed). The test was rewritten for FOUND-010 to assert that removal ("discover-foundation must be 100% interactive questionnaire").
- rust tests: none
- covered: none
- not covered: all 5 -> retire
- action: @deprecated citing FOUND-010 (d6d3169ba: automated detection removed, so discover-foundation is interactive only).

#### migrate-hook-execution-to-use-execa-library
- class: retire
- area: none (Node library migration)
- scenarios: 5 mapped to deleted files
- rust code: none for execa. Rust virtual hooks run `sh -c` with no timeout and no stdin context (board_state.rs:96, virtual_hooks_exec.rs:91). Agent lifecycle hooks do have stdin JSON, a timeout with kill, and capture (rust/core/src/lifecycle_hooks/executor.rs:23), owned by agent-lifecycle-hooks (HOOK-015).
- rust tests: rust/core/tests/lifecycle_hooks_engine_test.rs (owner: agent-lifecycle-hooks), for agent hooks only
- covered: none
- not covered: all 5 -> retire
- action: @deprecated citing 103a73072 (execa/Node removed). Generic executor behaviour is owned by agent-lifecycle-hooks. Command-hook execution follows the hook-condition-evaluation question.

#### napi-binding-setup-for-fspectool
- class: retire
- area: none
- scenarios: 5 mapped to deleted files
- rust code: none. rust/napi (callFspecCommand) was removed. The Fspec tool now uses the Rust dispatcher (`rust/agent-loop/src/fspec_handler.rs:47`, TOOL-019).
- rust tests: none
- covered: none
- not covered: all 5 -> retire
- action: @deprecated citing REFAC-027 (f25b90b2e, codelet-napi crate removed) and TOOL-019 (the Rust dispatcher replaced the TS callback).

#### typescript-function-call-integration
- class: retire
- area: none
- scenarios: 5 mapped to deleted files
- rust code: none. The NAPI + TypeScript callback path is gone; commands run in-process via `codelet_fspec_core::dispatch_command` (`rust/agent-loop/src/fspec_handler.rs:47`). Rust deliberately differs on "bootstrap excluded": `rust/fspec-core/src/dispatch.rs:625` dispatches bootstrap (RPC-200).
- rust tests: none
- covered: none
- not covered: all 5 -> retire
- action: @deprecated citing REFAC-027 (f25b90b2e) + TOOL-019 (FspecTool runs on the Rust dispatcher, no TS function calls). The in-process dispatch behaviour is owned by fspec-tool-rust-dispatcher (rust/fspec-core/tests/dispatcher_test.rs).

#### ansi-escape-codes-and-tui-content-leaking-into-fspec-tool-call-results
- class: retire
- area: none
- scenarios: 4 mapped to deleted files
- rust code: none needed. createCaptureContext, chalk, stripAnsi, Commander configureOutput and fspecCallback were all TS. fspec-core emits no ANSI (no `\x1b[`/colour crates in rust/fspec-core/src), and the Fspec tool gets plain strings from `codelet_fspec_core::dispatch_command` (`rust/agent-loop/src/fspec_handler.rs:47`).
- rust tests: none
- covered: none
- not covered: Capture context strips ANSI color codes from output.log -> retire; Capture context strips ANSI codes from output.error and output.warn -> retire; Strip ANSI handles all CSI sequences not just SGR colors -> retire; End-to-end fspec callback returns clean output -> retire; (unmapped) Commander configureOutput propagated to subcommands -> retire
- action: @deprecated citing 103a73072 (TS capture context / Commander removed) + TOOL-019 (Rust dispatcher returns plain text).

#### capture-all-console-methods-and-redirect-to-winston-logger
- class: retire
- area: none
- scenarios: 9 mapped to deleted files
- rust code: none. Patching console.* into winston is Node-only. Rust logs via `tracing` (rust/common/src/logging/mod.rs).
- rust tests: none
- covered: none
- not covered: all 9 -> retire
- action: @deprecated citing 103a73072 (TS client removed; winston/console capture is TypeScript tooling).

#### wire-lockedfilemanager-errors-to-winston-logger
- class: retire
- area: none
- scenarios: 2 mapped to deleted files (Log lock metrics in debug mode; Replace console calls with winston)
- rust code: none found. grep for FSPEC_DEBUG_LOCKS, "Acquired .* lock" and "waited .*ms" finds nothing. Rust locking (rust/fspec-core/src/io/locked_file.rs, rust/common/src/spec_lock.rs) does no winston-style logging.
- rust tests: none
- covered: none
- not covered: both -> retire (winston and file-manager.ts are TS-only)
- action: @deprecated citing 103a73072 + 828fe3306 (winston dependency pruned). The other 3 scenarios are winston too, so deprecate the whole feature (consistent with add-winston-universal-logger-for-fspec).

#### add-winston-universal-logger-for-fspec
- class: retire
- area: none
- scenarios: 4 mapped to deleted files
- rust code: none for winston. Rust logs with tracing to ~/.fspec/logs/fspec-combined.log (rust/common/src/logging/, tracing_appender), not ~/.fspec/fspec.log.
- rust tests: none
- covered: none
- not covered: all 4 -> retire
- action: @deprecated citing 103a73072 + 828fe3306 (winston pruned; Rust uses tracing). Consistent with wire-lockedfilemanager-errors-to-winston-logger.

#### sibling-watcher-navigation
- class: retire
- area: sessions / fspec-tui
- scenarios: 5 mapped to deleted files
- rust code: rust/sessions/src/navigation.rs:48 build_navigation_list returns a flat list in spawn order and ignores chain_of_command (module header: "BUG-124 ... replaces the traversal with a flat insertion-order walk"). There is no sibling or watcher grouping, and no SplitSessionView in fspec-tui.
- rust tests: rust/sessions/tests/cov057_shift_arrow_cycling.rs (owner: subordinate-agent-shift-arrow-cycling)
- covered: Regular session navigates through all sessions is already proven by BUG-124 (every session visited once). Duplicate.
- not covered: Navigate forward through sibling watchers -> retire, Single watcher has no siblings to navigate -> retire, Navigate backward through sibling watchers -> retire, Watchers of different parents are isolated -> retire
- action: @deprecated citing BUG-124 (navigation flattened, so hierarchy and sibling traversal are gone). README watcher section was also removed in 3d2d4a866 ("feature not implemented").

#### provider-configuration-display-and-profiles
- class: merge
- area: sessions / fspec-tui
- scenarios: 14 mapped to deleted files
- rust code: rust/sessions/src/profile_sections/ (profile and cloud sections, unreachable marking). rust/sessions/src/startup_model_resolution.rs ("openai:work-vllm/…" profile-qualified IDs). The profile CRUD forms live in rust/fspec-tui/src/views/provider_settings.
- rust tests: rust/sessions/src/profile_sections/tests.rs (owner: model-selector-profile-server); rust/fspec-tui/tests/provider_settings_profile_crud_prov111.rs (owner: provider-settings-profile-crud); rust/sessions/tests/prov121_profile_credential_bridge.rs::resolving_profile_model_sets_base_url_and_api_key (owner: openai-profile-credential-injection); rust/sessions/tests/prov131_local_profile_probe_resilience.rs (owner: local-profile-model-probe-resilience); rust/sessions/tests/profile_rpc_surface.rs (owner: provider-config-profile-persistence); rust/sessions/tests/prov130_section_ordering.rs (owner: model-selector-section-ordering)
- covered: Profiles appear as separate sections in model selector; Cloud provider section uses models.dev when no profile; View list of profiles for a provider; Create a new profile; Edit an existing profile; Delete a profile; Profile config structure; Selecting model from profile creates session with profile config; Profile settings flow through to Rust provider; Selecting profile model passes profile config to Rust session; Handle unreachable local server gracefully; Profile model selection saves with profile-qualified ID; Restoring persisted model finds correct profile section
- not covered: Model selector has unique keys for cloud and profile sections -> delete (React keys)
- action: drop the 13 as duplicates of the PROV-1xx owners, delete the React-key scenario, then @deprecated / remove the feature (superseded by the Rust profile cards PROV-111/121/131).

#### provider-configuration-and-credentials-management
- class: merge
- area: sessions / providers / fspec-tui
- scenarios: 19 mapped to deleted files
- rust code: rust/sessions/src/credentials/resolver.rs:1-6 (file > env > .env) and writer.rs:55-64 (0700 dir, 0600 file). rust/providers/src/credentials.rs:62 (mask_api_key "sk-ant-••••••••mnop"). rust/rpc-types/src/credentials.rs:137 (TestConnectionResult). rust/providers/src/catalog.rs (17-provider canonical list, RPC-107). There is no Azure endpoint/api-version config (grep api_version / AZURE_ENDPOINT finds nothing).
- rust tests: rust/sessions/tests/rpc054_credential_persistence.rs (owner: provider-credential-persistence); rust/sessions/tests/credential_resolution/priority_chain.rs (owner: move-credential-management-to-rust); rust/sessions/tests/provider_credential_deletion_prov133.rs (owner: provider-credential-deletion); rust/fspec-tui/tests/provider_credentials_masking_rpc108.rs (owner: provider-settings-credential-masking-with-prefix-aware-mask-helper-and-source-tag); rust/fspec-tui/tests/prov_098_nav_tree_credential_status_test.rs (owner: provider-settings-nav-tree-credential-status); rust/fspec-tui/tests/provider_dispatch_rpc054/status.rs::test_provider_connection_success_renders_ok_latency (owner: rpc054-provider-settings-dispatch); rust/fspec-tui/tests/provider_settings_tab_switch_to_models_rpc160.rs (owner: rpc160-provider-settings-tab-switch-to-models); rust/providers/tests/openai_local_provider_test.rs (owner: openai-compatible-local-model-support-vllm-ollama)
- covered: Save API key with secure file permissions; Credentials file takes precedence over environment variables; Environment variables continue to work; .env file is lowest priority fallback; TUI session creation uses credentials file (all via priority_chain); Delete a provider credential; API keys display with masked format; Display provider configuration status in model selection; Test provider connection before saving; Navigate to Settings view with Tab key (Rust lists 17 providers, not 19); Configure Ollama without API key
- not covered: Create session with programmatic credentials via NAPI -> retire (REFAC-027); Explicit credentials take highest priority -> retire (NAPI new_with_credentials, REFAC-027); All 19 rig providers are registered -> delete (superseded by RPC-107's 17-provider catalog); Configure provider with custom base URL -> delete (~/.fspec/fspec-config.json providers.X.baseUrl replaced by provider profiles, PROV-111); Configure Azure OpenAI with endpoint and version -> delete (no Azure endpoint/version config in Rust); Gemini uses query parameter authentication -> delete (rig-core HTTP client internals, not fspec code); Anthropic uses x-api-key header authentication -> delete (rig-core internals); API keys are never logged -> delete (no single owner; masking is proven by rpc108)
- action: drop the 11 covered scenarios as duplicates of the owners listed. Deprecate the 2 NAPI scenarios (REFAC-027) and delete the other 6. Then @deprecated / remove the feature. Unmapped "sessionService reads credentials file…" is TS-only: @deprecated (103a73072).

#### background-session-management-with-attach-detach
- class: retire
- area: sessions / fspec-tui
- scenarios: 11 mapped to deleted files
- rust code: the feature specifies the NAPI binding layer (architecture notes name rust/napi/src/session_manager.rs, ThreadsafeFunction attach callbacks and AgentView.tsx). The ESC "Detach" option is gone: rust/fspec-tui/src/components/exit_confirmation_dialog.rs:13 now offers only [Close Session, Cancel]. The underlying session engine exists in rust/sessions: session_manager/manager_impl_3.rs:36 create_session_with_id; background_session/messaging.rs:61 send_input(input, thinking_config); messaging.rs:106 interrupt.
- rust tests: the session behaviours are owned by Rust-era features: rust/sessions/tests/background_session_shape/*.rs (owner: move-background-session-into-codelet-sessions), rust/sessions/tests/rpc422_session_lifecycle_persistence.rs (owner: session-persistence-lifecycle), rust/sessions/tests/cmpct062_token_restore.rs (owner: committed-token-state-restore), rust/sessions/tests/bug288_destroy_releases_ownership.rs (owner: destroyed-session-releases-ownership)
- covered: none by this feature's own tests
- not covered: Detach while agent is running -> retire (BUG-205); Full detach and reattach workflow -> retire (BUG-205); the other 9 -> retire (NAPI surface, REFAC-027; the behaviour is owned by the features listed above)
- action: @deprecated citing REFAC-027 (f25b90b2e, NAPI crate removed) and BUG-205 (Detach option removed). Drop the src/__tests__/background-session.test.ts mapping.

#### migrate-fspec-research-ast-tool-from-tree-sitter-to-codelet-ast-grep
- class: merge
- area: tools
- scenarios: 6 mapped to deleted files
- rust code: `rust/tools/src/astgrep.rs:342-348` (file:line:column:text search output). `rust/tools/src/astgrep_refactor.rs:383` (multi-match error "Multiple matches found ({count}). Refactor requires exactly one match:\n{locations}"). `rust/tools/src/astgrep_refactor.rs:430` (removes the matched code from the source and writes it to the target).
- rust tests: rust/fspec/tests/cli_astgrep.rs::scenario_astgrep_runs_an_ast_search_and_prints_matches (owner: fspec-astgrep-cli-subcommand). rust/tools/tests/astgrep_refactor_test.rs::test_extract_code_matching_ast_pattern_to_new_file and ::test_error_when_pattern_matches_multiple_nodes (owner: ast-code-refactor-tool-for-codelet).
- covered: Search for functions using pattern matching (dup of fspec-astgrep-cli-subcommand). Refactor moves matched code to new file; Refactor errors when pattern matches multiple nodes (dups of ast-code-refactor-tool-for-codelet; the Rust error text differs from the scenario's).
- not covered: NAPI exports astGrepSearch function -> retire (REFAC-027, f25b90b2e); NAPI exports astGrepRefactor function -> retire (REFAC-027, f25b90b2e); Tree-sitter dependencies are removed -> retire (package.json removed, 103a73072)
- action: drop the 3 duplicates. Once the 3 retired scenarios are dropped too, nothing is left, so mark the feature @deprecated citing CLI-015 (research ast tool removed) + REFAC-027. Owners already prove the AstGrep/AstGrepRefactor behaviour.

