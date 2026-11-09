# COV-075 plan: give each shared test file a single feature

Read-only prep, 2026-11-04, against HEAD 5e50f0658 plus the uncommitted BUG-477 working tree. No repo files were changed. Computed from every `spec/features/**/*.feature.coverage` `testMappings[].file` (the same source `coverage_sharing.rs` reads). Scripts are in /tmp/asst/prep/COV-075-scripts/.

## Totals

| | count |
|---|---|
| Test files shared between features today | **120** |
| From the attached 83-file list, still shared | 82 |
| From the attached list, no longer shared | 1 (`rust/providers/tests/claude_opus_4_7_adaptive_thinking_test.rs`: MODEL-010, 83958288f, moved opus-4-7-thinking-mode-appears-inactive... off it) |
| Shared but missing from the list | 38, all removed TypeScript paths under `src/` (the files are deleted; the coverage still maps them) |
| **In scope for COV-075** | **82** = 80 Rust files in crates + 2 live TypeScript files in `bridge/__tests__/` |
| Excluded as TypeScript-era (COV-071..074) | 38 |
| Features involved (in-scope files) | 180 |

Fix types for the 82 in-scope files (primary type per file):

| fix | files | meaning |
|---|---|---|
| SPLIT | 51 | tests are disjoint per feature; move one feature's tests to a new file |
| SPLIT + DEDUP | 6 | one test proves scenarios of two features; the borrowing feature gets its own focused test in a new file |
| REHOME | 8 | the borrowing feature already owns another test file; write or move its test there and nothing else changes |
| RELINK | 9 | coverage line ranges are stale (tests moved or renamed); re-link to the current location, possibly with no code moves |
| MERGE | 6 | the features describe one behaviour; fold one into the other (needs product-owner sign-off) |
| DEPRECATE | 2 | one feature describes behaviour a later card retired; mark it @deprecated (COV-076 rule) |

By crate: fspec-tui 19, agent-loop 13, fspec 12, sessions 9, tools 8, cli 6, fspec-core 3, patches/rig-core 3, providers 3, core 2, graph 2, bridge 2.

## Is the list current?

- 82 of the 83 listed files are still shared. The one that is not is claude_opus_4_7_adaptive_thinking_test.rs, fixed in passing by MODEL-010.
- No Rust or live-TypeScript shared file is missing from the list. The 38 extra shared files are all removed `src/**` TypeScript tests. The list left them out (it only lists files that exist on disk). The BUG-357 gate does not check existence, though, so those 38 still block their features until COV-071..074 re-map them. They are listed at the end and excluded here.
- **device-auth-flow (BUG-489): not a shared test file, so no overlap with COV-075.** device-auth-flow is the only feature mapping `rust/providers/tests/device_protocol/legacy_{happy,poll,failures}.rs`. Under the product-owner rule (2026-11-04: a feature may have several test files; a test file may not have several features) that is allowed. `workspace-is-rustfmt-formatted` lists legacy_failures.rs only as an *implMapping*, which does not count. BUG-489's "breaks the one-feature-one-test-file rule" is the old VAL-005 direction. The installed `~/.local/bin/fspec` (0.10.9, built 2026-10-30) predates b0141eb56, so it still enforces the old rule, and that is most likely what refused BUG-475. The still-valid half of BUG-489 is the unmapped `test_request_device_code_sends_correct_request` and its missing @step comments. BUG-489 should be re-scoped to that.
- The BUG-477 agent has uncommitted edits in 84 `spec/features/*-rust-port.feature.coverage` files. Re-running the computation against that live tree gives the same 120-file result, so those edits neither add nor resolve any shared file. Re-run `COV-075-scripts/shared.py` after BUG-477 lands.
- **Use a binary built from HEAD for every gate check in this work.** The installed fspec does not report shared test files (`fspec audit-coverage deep-search` prints "All mappings valid" today), so it will neither confirm a fix nor show the refusal.

## Needs a human (product-owner) decision

These shape 27 files (each such per-file entry has a "Decision needed" line). The recommended answer comes first in each item.

1. **inject-summary-handler (CMPCT-009) cluster**: 4 files (cmpct045/part_03, cmpct054_cancellation_cleanup, cmpct054_terminal_dependencies, cli/cmpct059_runtime_commit). Four of its five scenarios borrow tests from CMPCT-045/054/059. Two of them ("Partition, clear, restore, and inject DAG", "Turns and token tracker are reset") describe the raw-DAG success path, which inject_summary_handler_test.rs says was dropped when the handler became fail-closed. Recommended: @deprecate those two, and write registration/teardown tests for the other two in inject_summary_handler_test.rs. Alternative: deprecate the whole feature.
2. **BUG-205 exit-session cluster**: 4 files (agentview_esc_exit_confirmation_rpc098, bug164_mux_retained_on_session_exit, tui112_dialog_click_activation, slash_detach_rpc050). BUG-205's features re-state behaviour already owned by RPC-098, rust-mux-mode (BUG-164), the left-click feature (TUI-112) and RPC-050. Recommended: merge exit-session-dialog-two-button-options, exit-session-close-retains-mux-grid and exit-session-dialog-click-activation into their parents. Deprecate slash-command-detach-and-work-unit-binding (RPC-050), whose /detach behaviour BUG-205 removed.
3. **CONT-011 guard features**: 3 files (cmpct041, auto_continue_engine_test, cont006). Each 1-scenario guard borrows a parent test. Recommended: one source-scan guard test file per guard feature. cmpct041 already has the right test, unmapped. Alternative: fold each guard scenario into its parent feature.
4. **Duplicate-behaviour merges**: BUG-144 into CMPCT-025 (error_classifiers.rs); BUG-446 into REFAC-019 (refac019_contracts); BUG-106's duplicated "non-streaming providers" scenario (deep_search_handler); RPC-069's 3 borrowed scenarios (cross_frontend_parity). For each: merge, or write duplicate focused tests.
5. **Superseded or retired behaviour, deprecate or rewrite**: RPC-008 help dialog (superseded by RPC-009); BUG-153 TS reference files (TypeScript TUI removed); MENU-001's 3 category scenarios (superseded by MENU-008); RIG-012 reasoning-token NAPI bridge (NAPI removed; 2 files); WATCH-004 session role (RoleAuthority/NAPI gone; get_role is now Option<String>); WATCH-019 watcher loop (mapped to an unrelated test); RPC-040's two NAPI/TypeScript scenarios.
6. **Vendored rig-core patch**: is restructuring tests inside `rust/patches/rig-core/src` acceptable, given the larger diff against upstream? (2 files)
7. **Live TypeScript `bridge/`**: is it in COV-075 scope? It is not TypeScript-era, but it needs vitest rather than cargo. (2 files)
8. **Untagged features** touched here: git-state-watcher-dedup-signature, defense-in-depth-pdf-image-count-cap-in-rig-patch, left-click-activates-buttons-rows... These have no work-unit tag, so no card move will trigger the gate for them. Should they be tagged COV-075?

## Batches (one agent, one ACDD sitting each)

Order: B2, B7 and B3 are mostly mechanical and can start now. B1, B4 and B5 depend on decisions 1-5. B6 depends on decision 6. B8 depends on decision 7. Batches touch disjoint crates, except B5 (watch004 test in sessions, from B1's decision) and B2 (adds files to fspec-core/tests, which B7 also touches; there are no shared files between them).

### B1 agent-loop (13)

- `rust/agent-loop/src/deep_search_handler/tests.rs`: SPLIT + MERGE one duplicate scenario
- `rust/agent-loop/src/session_search_handler.rs`: SPLIT
- `rust/agent-loop/tests/bug210_completion_metrics.rs`: REHOME
- `rust/agent-loop/tests/cmpct041_pre_compaction_basis_test.rs`: SPLIT (mis-mapping)
- `rust/agent-loop/tests/cmpct045/part_03.rs`: REHOME or DEPRECATE (inject-summary-handler)
- `rust/agent-loop/tests/cmpct045/part_06.rs`: SPLIT
- `rust/agent-loop/tests/cmpct045/part_07.rs`: SPLIT (same as part_06)
- `rust/agent-loop/tests/cmpct054_cancellation_cleanup.rs`: REHOME or DEPRECATE (inject-summary-handler)
- `rust/agent-loop/tests/cmpct054_terminal_dependencies.rs`: REHOME or DEPRECATE (inject-summary-handler)
- `rust/agent-loop/tests/cmpct069_token_forwarding.rs`: REHOME or DEPRECATE (reasoning-token-napi-bridge)
- `rust/agent-loop/tests/compaction_post_inject_loading_test.rs`: SPLIT / REHOME
- `rust/agent-loop/tests/rpc082_role_injection.rs`: REHOME or DEPRECATE (session-role-and-authority-model)
- `rust/agent-loop/tests/rpc086_token_tracking.rs`: REHOME or DEPRECATE (reasoning-token-napi-bridge)

### B2 fspec (12)

- `rust/fspec/src/common/tests/part_1.rs`: SPLIT
- `rust/fspec/src/daemon_directory/tests.rs`: SPLIT
- `rust/fspec/tests/cli_query_bottlenecks.rs`: SPLIT (mechanical)
- `rust/fspec/tests/cli_query_estimation_guide.rs`: SPLIT (mechanical)
- `rust/fspec/tests/cli_query_example_mapping_stats.rs`: SPLIT (mechanical)
- `rust/fspec/tests/cli_query_orphans.rs`: SPLIT (mechanical)
- `rust/fspec/tests/cli_show_acceptance_criteria.rs`: SPLIT (mechanical)
- `rust/fspec/tests/cli_show_event_storm.rs`: SPLIT (mechanical)
- `rust/fspec/tests/cli_show_foundation.rs`: SPLIT (mechanical)
- `rust/fspec/tests/cli_show_foundation_event_storm.rs`: SPLIT (mechanical)
- `rust/fspec/tests/cli_show_test_patterns.rs`: SPLIT (mechanical)
- `rust/fspec/tests/cross_frontend_parity.rs`: SPLIT + DEDUP

### B3 fspec-tui A: views and mechanical splits (10)

- `rust/fspec-tui/src/components/help_dialog.rs`: MERGE / DEPRECATE (recommended), else SPLIT
- `rust/fspec-tui/src/views/blocklist/tests.rs`: SPLIT
- `rust/fspec-tui/src/views/changed_files/tests.rs`: SPLIT
- `rust/fspec-tui/src/views/checkpoints/tests.rs`: SPLIT
- `rust/fspec-tui/src/views/full_screen_shell.rs`: SPLIT
- `rust/fspec-tui/src/views/model_selector/mod.rs`: RELINK (stale)
- `rust/fspec-tui/src/views/model_selector/rows.rs`: RELINK (stale)
- `rust/fspec-tui/tests/keyboard_cascade_rpc051.rs`: SPLIT (+ relink drift)
- `rust/fspec-tui/tests/menu002_board_surface.rs`: SPLIT
- `rust/fspec-tui/tests/menu005_menu_bar_help.rs`: SPLIT

### B4 fspec-tui B: BUG-205 cluster and supersessions (9)

- `rust/fspec-tui/tests/agentview_esc_exit_confirmation_rpc098.rs`: MERGE (recommended), else SPLIT + DEDUP
- `rust/fspec-tui/tests/bug164_mux_retained_on_session_exit.rs`: MERGE (recommended)
- `rust/fspec-tui/tests/menu008_menu_bar_reorg.rs`: RELINK or DEPRECATE (MENU-001 scenarios)
- `rust/fspec-tui/tests/refac019_contracts.rs`: MERGE (recommended)
- `rust/fspec-tui/tests/rpc027_dialog_parity_ij.rs`: DEPRECATE (recommended)
- `rust/fspec-tui/tests/rpc028_popup_scroll.rs`: SPLIT (+ relink drift)
- `rust/fspec-tui/tests/slash_clear_rpc046.rs`: SPLIT
- `rust/fspec-tui/tests/slash_detach_rpc050.rs`: DEPRECATE (recommended)
- `rust/fspec-tui/tests/tui112_dialog_click_activation.rs`: MERGE (recommended)

### B5 sessions + providers (12)

- `rust/providers/src/manager/tests_3.rs`: SPLIT
- `rust/providers/src/manager/tests_4.rs`: RELINK (stale) + SPLIT
- `rust/providers/src/manager/tests_5.rs`: RELINK (drift) + SPLIT
- `rust/sessions/src/profile_sections/tests.rs`: SPLIT
- `rust/sessions/tests/background_session_shape/api.rs`: SPLIT
- `rust/sessions/tests/background_session_shape/broadcast.rs`: SPLIT
- `rust/sessions/tests/background_session_shape/isolation.rs`: RELINK (stale)
- `rust/sessions/tests/bug251_multimodal_archives.rs`: RELINK or DEPRECATE (WATCH-019)
- `rust/sessions/tests/cont009_completion_contract_sync.rs`: SPLIT + DEDUP
- `rust/sessions/tests/exec_stdin_prompt_p2.rs`: SPLIT
- `rust/sessions/tests/rpc418_compact_session/compact_populated_clears_conversation.rs`: SPLIT + DEDUP (3-way)
- `rust/sessions/tests/session_manager_shape/codelet_sessions_tests_assert.rs`: SPLIT (+ DEPRECATE one RPC-040 scenario)

### B6 tools + rig-core patches (11)

- `rust/patches/rig-core/src/agent/prompt_request/streaming.rs`: SPLIT
- `rust/patches/rig-core/src/providers/openai/completion/mod.rs`: SPLIT (mechanical)
- `rust/patches/rig-core/tests/cmpct065_openai_usage_evidence.rs`: SPLIT
- `rust/tools/src/agent_manager/tests.rs`: SPLIT (5-way)
- `rust/tools/src/bridge_relay.rs`: RELINK (stale) + SPLIT
- `rust/tools/src/deep_search/tests.rs`: SPLIT
- `rust/tools/src/facade/codex.rs`: SPLIT (8-way) + RELINK
- `rust/tools/src/facade/wrapper.rs`: SPLIT
- `rust/tools/src/fspec_workflow_guidance.rs`: SPLIT
- `rust/tools/src/unified_exec/exec_stdin/tests.rs`: SPLIT
- `rust/tools/tests/tool_facade_test.rs`: SPLIT (6-way)

### B7 cli + core + fspec-core + graph (13)

- `rust/cli/src/compaction_threshold.rs`: RELINK (stale)
- `rust/cli/src/interactive/error_classifiers.rs`: MERGE (recommended), else SPLIT + DEDUP
- `rust/cli/tests/auto_continue_engine_test.rs`: SPLIT + DEDUP (CONT-011 guard)
- `rust/cli/tests/cmpct059_runtime_commit.rs`: REHOME or DEPRECATE (inject-summary-handler)
- `rust/cli/tests/cont006_goal_immediate_termination.rs`: SPLIT + DEDUP (CONT-011 guard)
- `rust/cli/tests/image_content_recovery_test.rs`: SPLIT
- `rust/core/src/compaction/__tests__/dag_node_proptest.test.rs`: SPLIT
- `rust/core/tests/git_state_watcher.rs`: SPLIT
- `rust/fspec-core/src/io/gherkin_format.rs`: SPLIT
- `rust/fspec-core/tests/list_work_units.rs`: SPLIT
- `rust/fspec-core/tests/research.rs`: SPLIT + DEDUP
- `rust/graph/tests/ast_extraction_pipeline_test/walk_project_directory_with.rs`: SPLIT
- `rust/graph/tests/graph_reset_test.rs`: SPLIT

### B8 live TypeScript bridge (2), only if in scope

- `bridge/__tests__/relay-endpoint.test.ts`: SPLIT
- `bridge/__tests__/telegram-endpoint.test.ts`: SPLIT

Per-batch ACDD loop: for each file, (1) move tests along with their @step comments into the new file and register the module; (2) `cargo test -p <crate> --test <name>` (or `--lib <module>`); (3) re-link coverage for each moved scenario; (4) run `audit-coverage <feature>` with a HEAD-built fspec for every feature the file touched (it must report no shared test files); (5) run `validate`. Keep every new file under 300 lines (AGENTS.md).

## Per-file plan

Test-function attribution comes from each scenario's coverage line range (a test function is credited when the range covers most of its body). Where ranges have drifted, the notes say so and give the real owner from the `// Scenario:` / `@step` comments. "unmapped" means test functions no feature maps; they stay in the original file unless noted.

### B1 agent-loop (13)

#### `rust/agent-loop/src/deep_search_handler/tests.rs`

- Kind: in-crate test module (#[path] from deep_search_handler.rs) (Rust, crate agent-loop)
- **deepsearch-codex-sub-agent-still-fails-because-responses-api-requires-stream-true** [BUG-104:done] 5 scen; 1 stale mapping(s): codex_requires_streaming_execution_path, collect_final_response_from_stream_uses_final_response_only, zai_config_includes_max_tokens_in_additional_params, zai_requires_streaming_execution_path
- **glm-zai-deepsearch-fails-with-500-internal-server-error** [BUG-106:done] 6 scen: zai_requires_streaming_execution_path, codex_requires_streaming_execution_path, non_streaming_providers_remain_unchanged, zai_config_includes_max_tokens_in_additional_params
- Same test credited to 2+ features: codex_requires_streaming_execution_path (deepsearch-codex-sub-agent-still-fails-because-responses-api-requires-stream-true, glm-zai-deepsearch-fails-with-500-internal-server-error); zai_config_includes_max_tokens_in_additional_params (deepsearch-codex-sub-agent-still-fails-because-responses-api-requires-stream-true, glm-zai-deepsearch-fails-with-500-internal-server-error); zai_requires_streaming_execution_path (deepsearch-codex-sub-agent-still-fails-because-responses-api-requires-stream-true, glm-zai-deepsearch-fails-with-500-internal-server-error)
- Unmapped (6): collect_final_response_from_stream_errors_when_final_missing, collect_final_response_from_stream_propagates_stream_errors, deep_search_wall_clock_timeout_fires_on_stalled_sub_agent, deep_search_wall_clock_timeout_defaults_to_600_seconds, deep_search_timeout_message_is_not_classified_as_stall_timeout, collect_final_response_stalled_stream_aborted_by_wall_clock_timeout
- **Fix: SPLIT + MERGE one duplicate scenario**
- New file(s): rust/agent-loop/src/deep_search_handler/zai_tests.rs (BUG-106: zai_requires_streaming_execution_path, zai_config_includes_max_tokens_in_additional_params)
- Moves / wiring: Register `#[cfg(test)] #[path = "deep_search_handler/zai_tests.rs"] mod zai_tests;` in deep_search_handler.rs next to the existing tests module. tests.rs keeps BUG-104 (codex_requires_streaming_execution_path, collect_final_response_from_stream_uses_final_response_only, non_streaming_providers_remain_unchanged).
- Notes: Line ranges have drifted: the Scenario comments put codex_*/collect_* under BUG-104 and zai_*/non_streaming under BUG-106. Both features carry a "non-streaming providers keep the non-streaming path" scenario proved by the same test (non_streaming_providers_remain_unchanged). The 6 wall-clock-timeout / collect_* error tests are mapped by no feature.
- Decision needed: BUG-104 "Non-Codex providers keep the existing non-streaming execution path" and BUG-106 "Non-streaming providers remain unchanged" say the same thing. Drop BUG-106's copy (recommended), or add a second test in zai_tests.rs.

#### `rust/agent-loop/src/session_search_handler.rs`

- Kind: source file with inline #[cfg(test)] mod tests (line 828) (Rust, crate agent-loop)
- **session-search-trimming** [CMPCT-010:done] 9 scen: test_flag_false_returns_full_untrimmed_content, test_flag_true_trims_read_tool_results, test_flag_true_preserves_plain_user_messages, test_flag_true_preserves_assistant_reasoning, test_trimmer_processes_messages_in_order, test_trimming_applied_after_blob_resolution, test_default_flag_false_returns_untrimmed, test_create_handler_accepts_compaction_flag, test_handler_registration_with_flag
- **session-search-turn-range** [CMPCT-018:done] 10 scen: test_filter_by_turn_range_basic, test_filter_by_turn_range_start_only, test_filter_by_turn_range_end_only, test_filter_by_turn_range_beyond_session, test_filter_by_turn_range_inverted, test_turn_range_then_max_turns, test_turn_range_then_user_only, test_search_restricts_matches_to_turn_range, test_search_context_extends_outside_turn_range, test_tool_definition_includes_turn_range_params
- Unmapped (16): test_ripgrep_matcher_case_insensitive_regex, test_ripgrep_matcher_literal_query, test_ripgrep_find_returns_offsets, test_ripgrep_find_no_match, test_ripgrep_invalid_regex_returns_error, test_extract_match_preview_with_ripgrep, test_work_unit_id_extraction_via_ripgrep, test_floor_char_boundary_ascii, test_floor_char_boundary_multibyte, test_ceil_char_boundary_multibyte, test_truncate_preview_multibyte_safe, test_extract_match_preview_multibyte ...
- **Fix: SPLIT**
- New file(s): rust/agent-loop/src/session_search_handler/trimming_tests.rs (CMPCT-010, 9 tests); rust/agent-loop/src/session_search_handler/turn_range_tests.rs (CMPCT-018, 10 tests)
- Moves / wiring: Move the inline tests out using the `#[cfg(test)] #[path = "session_search_handler/<x>_tests.rs"] mod <x>_tests;` pattern (precedent: cli/src/compaction_threshold.rs:121-130). Shared fixtures inside the inline module (message/blob builders) go to session_search_handler/test_support.rs. The 10+ unmapped ripgrep/preview tests can stay inline.
- Notes: Afterwards session_search_handler.rs is still an impl mapping for both features. That is allowed: the rule only constrains test files.

#### `rust/agent-loop/tests/bug210_completion_metrics.rs`

- Kind: integration test (Rust, crate agent-loop)
- **automatic-compaction-completion-metrics** [BUG-210:done] 5 scen: scenario_completion_reports_the_percentage_removed_rather_than_retained, scenario_completion_coverage_matches_the_durable_generation, scenario_successful_publication_produces_only_one_completion_notice, scenario_failed_publication_produces_no_successful_completion, scenario_self_compaction_completion_waits_for_durable_reconciliation
- **compaction-complete-measurement-basis** [CMPCT-038:done] 1 scen: scenario_completion_reports_the_percentage_removed_rather_than_retained
- Same test credited to 2+ features: scenario_completion_reports_the_percentage_removed_rather_than_retained (automatic-compaction-completion-metrics, compaction-complete-measurement-basis)
- **Fix: REHOME**
- New file(s): none (add one test to the existing rust/agent-loop/tests/cmpct038_measurement_basis_test.rs, owned only by CMPCT-038)
- Moves / wiring: Nothing moves out of bug210. Write a CMPCT-038 test for "A real ~60 percent reduction is reported as ~60, never ~99" in cmpct038_measurement_basis_test.rs (117 lines) and re-link that scenario there.
- Notes: CMPCT-038 borrows scenario_completion_reports_the_percentage_removed_rather_than_retained from BUG-210.

#### `rust/agent-loop/tests/cmpct041_pre_compaction_basis_test.rs`

- Kind: integration test (Rust, crate agent-loop)
- **completion-snapshot-delegation-guards** [CONT-011:done] 1 scen: auto_and_manual_snapshots_agree_on_the_same_basis
- **pre-compaction-snapshot-basis-unification** [CMPCT-041:done] 2 scen: overflow_recovery_snapshot_records_true_total_in_seed_window, auto_and_manual_snapshots_agree_on_the_same_basis
- Same test credited to 2+ features: auto_and_manual_snapshots_agree_on_the_same_basis (completion-snapshot-delegation-guards, pre-compaction-snapshot-basis-unification)
- Unmapped (1): all_four_pre_compaction_writers_route_through_shared_accessors
- **Fix: SPLIT (mis-mapping)**
- New file(s): rust/agent-loop/tests/cont011_completion_snapshot_delegation_guards.rs (CONT-011: all_four_pre_compaction_writers_route_through_shared_accessors)
- Moves / wiring: Move the unmapped source-guard test all_four_pre_compaction_writers_route_through_shared_accessors (it checks the shared snapshot accessors, which is the CONT-011 guard) into the new file. Re-link "Shared snapshot delegation retains semantic and writer guards" to it.
- Notes: The CONT-011 guard scenario is mapped to CMPCT-041's auto_and_manual_snapshots_agree_on_the_same_basis, but its real test is the unmapped guard in the same file.

#### `rust/agent-loop/tests/cmpct045/part_03.rs`

- Kind: include!() fragment of tests/cmpct045_generate_compaction_behavioral.rs (Rust, crate agent-loop)
- **generate-compaction-tool** [CMPCT-046:done,CMPCT-047:done,CMPCT-048:done,CMPCT-045:done,BUG-311:done] 6 scen: successful_dag_is_pinned_to_the_target_and_sub_agent_session_is_ephemeral, calling_session_target_stashes_pending_dag_instead_of_locking_inner
- **inject-summary-handler** [CMPCT-009:done] 1 scen: successful_dag_is_pinned_to_the_target_and_sub_agent_session_is_ephemeral
- Same test credited to 2+ features: successful_dag_is_pinned_to_the_target_and_sub_agent_session_is_ephemeral (generate-compaction-tool, inject-summary-handler)
- **Fix: REHOME or DEPRECATE (inject-summary-handler)**
- New file(s): none
- Moves / wiring: part_03 stays with generate-compaction-tool. inject-summary-handler's "Turns and token tracker are reset after injection" moves to its own file, rust/agent-loop/tests/inject_summary_handler_test.rs, or is retired.
- Notes: Header of inject_summary_handler_test.rs: "Scenarios that asserted the legacy raw-DAG success path were dropped when the handler became fail-closed". The borrowed scenario describes that dropped path.
- Decision needed: inject-summary-handler (CMPCT-009) cluster: see the decisions section.

#### `rust/agent-loop/tests/cmpct045/part_06.rs`

- Kind: include!() fragment of tests/cmpct045_generate_compaction_behavioral.rs (Rust, crate agent-loop)
- **generate-compaction-tool-wiring** [CMPCT-045:done,CMPCT-046:done,REFAC-034:done,BUG-345:done] 1 scen: generate_compaction_handler_registration_lifecycle_mirrors_deep_search
- **generate-compaction-tool** [CMPCT-046:done,CMPCT-047:done,CMPCT-048:done,CMPCT-045:done,BUG-311:done] 2 scen: compactor_sub_agent_has_only_the_read_only_tool_surface, fresh_compaction_prompt_names_the_target_and_forbids_inject_summary
- Unmapped (1): incremental_compaction_prompt_embeds_existing_dag_and_turn_offset
- **Fix: SPLIT**
- New file(s): rust/agent-loop/tests/cmpct045/part_08.rs (generate-compaction-tool-wiring: generate_compaction_handler_registration_lifecycle_mirrors_deep_search from part_06 + compactor_sub_agent_provider_arms_match_the_deep_search_clone from part_07). Alternative: append both to part_02.rs (144 lines, already wiring-only) if it stays < 300 lines.
- Moves / wiring: Add `include!("cmpct045/part_08.rs");` to cmpct045_generate_compaction_behavioral.rs. Every file in tests/cmpct045/ must stay under 300 lines (REFAC-033 guard: tests/refac033_cmpct045_split.rs).
- Notes: part_06 and part_07 are handled together.

#### `rust/agent-loop/tests/cmpct045/part_07.rs`

- Kind: include!() fragment (Rust, crate agent-loop)
- **generate-compaction-tool-wiring** [CMPCT-045:done,CMPCT-046:done,REFAC-034:done,BUG-345:done] 1 scen: compactor_sub_agent_provider_arms_match_the_deep_search_clone
- **generate-compaction-tool** [CMPCT-046:done,CMPCT-047:done,CMPCT-048:done,CMPCT-045:done,BUG-311:done] 2 scen: self_target_capture_never_awaits_the_callers_inner_lock, a_zero_tracker_basis_falls_back_to_a_content_estimate
- **Fix: SPLIT (same as part_06)**
- New file(s): (see part_06)
- Moves / wiring: (see part_06)

#### `rust/agent-loop/tests/cmpct054_cancellation_cleanup.rs`

- Kind: integration test (Rust, crate agent-loop)
- **aborted-turn-compaction-dependency-cleanup** [CMPCT-054:done] 1 scen: scenario_aborting_a_real_turn_releases_every_compaction_dependency
- **inject-summary-handler** [CMPCT-009:done] 1 scen: scenario_aborting_a_real_turn_releases_every_compaction_dependency
- Same test credited to 2+ features: scenario_aborting_a_real_turn_releases_every_compaction_dependency (aborted-turn-compaction-dependency-cleanup, inject-summary-handler)
- **Fix: REHOME or DEPRECATE (inject-summary-handler)**
- New file(s): none
- Moves / wiring: inject-summary-handler's "Handler is registered alongside SessionSearch handler in agent loop setup" gets a test in inject_summary_handler_test.rs (registration still exists: set_inject_summary_handler), or is retired.
- Decision needed: inject-summary-handler cluster

#### `rust/agent-loop/tests/cmpct054_terminal_dependencies.rs`

- Kind: integration test (Rust, crate agent-loop)
- **inject-summary-handler** [CMPCT-009:done] 1 scen: scenario_terminal_recovery_invokes_the_turn_builder_before_cleanup
- **terminal-compaction-dependency-lifetime** [CMPCT-054:done,BUG-310:done] 3 scen: scenario_terminal_recovery_invokes_the_turn_builder_before_cleanup, scenario_missing_recovery_dependencies_fail_without_replacing_context, scenario_cancellation_releases_the_operation_registration
- Same test credited to 2+ features: scenario_terminal_recovery_invokes_the_turn_builder_before_cleanup (inject-summary-handler, terminal-compaction-dependency-lifetime)
- **Fix: REHOME or DEPRECATE (inject-summary-handler)**
- New file(s): none
- Moves / wiring: inject-summary-handler's "Handler is cleaned up on session teardown" gets a test in inject_summary_handler_test.rs, or is retired.
- Decision needed: inject-summary-handler cluster

#### `rust/agent-loop/tests/cmpct069_token_forwarding.rs`

- Kind: integration test (Rust, crate agent-loop)
- **complete-token-event-forwarding** [CMPCT-069:done] 2 scen: measured_token_fields_survive_json_forwarding, unknown_token_fields_remain_unknown_in_json
- **reasoning-token-napi-bridge** [RIG-012:done] 1 scen: measured_token_fields_survive_json_forwarding
- Same test credited to 2+ features: measured_token_fields_survive_json_forwarding (complete-token-event-forwarding, reasoning-token-napi-bridge)
- **Fix: REHOME or DEPRECATE (reasoning-token-napi-bridge)**
- New file(s): rust/agent-loop/tests/rig012_reasoning_token_forwarding.rs (only if RIG-012 is kept)
- Moves / wiring: RIG-012's "NAPI TokenTracker includes reasoning_tokens field" borrows measured_token_fields_survive_json_forwarding.
- Notes: reasoning-token-napi-bridge (RIG-012) describes the NAPI bridge, which no longer exists. All 3 of its scenarios borrow tests from CMPCT-069/RPC-086.
- Decision needed: RIG-012: rewrite to the Rust StreamChunk path with its own test file, or @deprecated.

#### `rust/agent-loop/tests/compaction_post_inject_loading_test.rs`

- Kind: integration test (Rust, crate agent-loop)
- **compaction-complete-measurement-basis** [CMPCT-038:done] 1 scen: test_emit_post_injection_events_metrics
- **compaction-post-inject-loading-state** [CMPCT-015:done] 2 scen: test_done_handler_keeps_running_when_pending_dag_exists, test_agent_loop_cleanup_clears_compaction_flag_on_error
- Unmapped (1): test_done_handler_idle_guard_truth_table
- **Fix: SPLIT / REHOME**
- New file(s): none (move test_emit_post_injection_events_metrics into the existing cmpct038_measurement_basis_test.rs)
- Moves / wiring: test_emit_post_injection_events_metrics is claimed only by CMPCT-038 ("Running is emitted before CompactionComplete at the apply site"), so move it with its @step comments and helpers. CMPCT-015 keeps the rest, including the unmapped test_done_handler_idle_guard_truth_table.

#### `rust/agent-loop/tests/rpc082_role_injection.rs`

- Kind: integration test (Rust, crate agent-loop)
- **agent-loop-role-injection** [RPC-082:done,BUG-344:done] 5 scen: background_session_round_trips_set_get_clear_role, run_with_provider_macro_reads_session_role_and_passes_as_preamble, openai_turn_reads_session_role_and_passes_as_preamble, custom_provider_turn_reads_session_role_and_passes_as_preamble, every_provider_create_rig_agent_signature_accepts_option_str_preamble
- **session-role-and-authority-model** [WATCH-004:done] 2 scen: custom_provider_turn_reads_session_role_and_passes_as_preamble, every_provider_create_rig_agent_signature_accepts_option_str_preamble
- Same test credited to 2+ features: custom_provider_turn_reads_session_role_and_passes_as_preamble (agent-loop-role-injection, session-role-and-authority-model); every_provider_create_rig_agent_signature_accepts_option_str_preamble (agent-loop-role-injection, session-role-and-authority-model)
- **Fix: REHOME or DEPRECATE (session-role-and-authority-model)**
- New file(s): rust/sessions/tests/watch004_session_role.rs (BackgroundSession::get_role None / Some; the methods live in rust/sessions/src/background_session/messaging.rs)
- Moves / wiring: Nothing moves out of rpc082. WATCH-004's two scenarios are mapped to a drifted range (159-233) covering three RPC-082 tests.
- Notes: WATCH-004's description still names SessionRole/RoleAuthority structs and NAPI functions. Today get_role returns Option<String>.
- Decision needed: WATCH-004: rewrite the 2 scenarios to current get_role behaviour (recommended) or @deprecated.

#### `rust/agent-loop/tests/rpc086_token_tracking.rs`

- Kind: integration test (Rust, crate agent-loop)
- **agent-loop-token-tracking** [RPC-086:done] 5 scen: background_output_tokens_arm_updates_session_and_emits_token_update, background_output_context_fill_arm_emits_context_fill_update, background_session_round_trips_update_tokens_and_get_tokens, stream_chunk_declares_token_and_context_fill_variants_and_constructors, token_info_and_context_fill_info_declare_expected_fields_and_stream_event_variants
- **reasoning-token-napi-bridge** [RIG-012:done] 2 scen: background_output_tokens_arm_updates_session_and_emits_token_update, background_session_round_trips_update_tokens_and_get_tokens
- Same test credited to 2+ features: background_output_tokens_arm_updates_session_and_emits_token_update (agent-loop-token-tracking, reasoning-token-napi-bridge); background_session_round_trips_update_tokens_and_get_tokens (agent-loop-token-tracking, reasoning-token-napi-bridge)
- **Fix: REHOME or DEPRECATE (reasoning-token-napi-bridge)**
- New file(s): rust/agent-loop/tests/rig012_reasoning_token_forwarding.rs (only if RIG-012 is kept)
- Moves / wiring: RIG-012 borrows two RPC-086 tests.
- Decision needed: RIG-012 (see cmpct069)

### B2 fspec (12)

#### `rust/fspec/src/common/tests/part_1.rs`

- Kind: in-crate test part (mod part_1; in common/tests/mod.rs) (Rust, crate fspec)
- **rpc017-build-service-cwd-attachment** [RPC-017:done] 1 scen: build_service_attaches_workspace_cwd
- **rpc025-fspec-binary-persistence-bootstrap** [RPC-025:done] 1 scen: build_service_initializes_global_data_directory_for_persistence
- **wire-codelet-sessions-into-fspec-build-service** [RPC-044:done] 3 scen: build_service_wires_session_manager_into_shared_service
- Unmapped (1): fspec_cargo_toml_declares_sessions_dep_and_not_napi
- **Fix: SPLIT**
- New file(s): rust/fspec/src/common/tests/build_service_cwd.rs (RPC-017: build_service_attaches_workspace_cwd); rust/fspec/src/common/tests/build_service_persistence.rs (RPC-025: build_service_initializes_global_data_directory_for_persistence); part_1.rs keeps RPC-044
- Moves / wiring: Declare the new modules in common/tests/mod.rs. Keep #[serial] (blocklist root is process-global, RPC-407 note in mod.rs).

#### `rust/fspec/src/daemon_directory/tests.rs`

- Kind: in-crate test module (Rust, crate fspec)
- **combined-mode-daemon-switcher** [BUG-430:done] 1 scen: the_switcher_opens_with_the_in_process_daemon_connected
- **daemon-switcher-local-token** [BUG-428:done] 2 scen: a_local_daemon_is_reached_with_the_fspec_daemon_token_bearer, a_protected_local_daemon_refuses_a_switcher_without_a_token
- **Fix: SPLIT**
- New file(s): rust/fspec/src/daemon_directory/local_token_tests.rs (BUG-428, 2 tests); tests.rs keeps BUG-430
- Moves / wiring: Declare it in daemon_directory.rs next to `mod tests;`.

#### `rust/fspec/tests/cli_query_bottlenecks.rs`

- Kind: integration test (Rust, crate fspec)
- **query-bottlenecks-cli-subcommand** [RPC-256:done] 7 scen: scenario_clap_exposes_query_bottlenecks_with_help, scenario_cli_no_options_empty_workspace_prints_success, scenario_cli_output_json_empty_workspace_prints_empty_array, scenario_cli_text_output_lists_qualifying_bottleneck, scenario_cli_malformed_work_units_json_exits_1_with_stderr, scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher, scenario_query_bottlenecks_help_matches_ts_reference
- **query-bottlenecks-rust-port** [RPC-256:done] 10 scen: scenario_dispatcher_returns_empty_bottlenecks_in_empty_workspace, scenario_dispatcher_excludes_done_units_with_blocks, scenario_dispatcher_excludes_blocked_status_units, scenario_dispatcher_excludes_empty_or_missing_blocks, scenario_dispatcher_single_block_below_threshold, scenario_dispatcher_score_three_with_direct_and_transitive, scenario_dispatcher_cycle_yields_score_two, scenario_dispatcher_two_qualifying_bottlenecks_sorted_descending, scenario_dispatcher_field_declaration_order, scenario_dispatcher_malformed_work_units_json_yields_parse_error
- **Fix: SPLIT (mechanical)**
- New file(s): rust/fspec-core/tests/query_bottlenecks.rs (query-bottlenecks-rust-port: the 10 dispatcher-level tests)
- Moves / wiring: Repo convention: <cmd>-rust-port maps rust/fspec-core/tests/<cmd>.rs and <cmd>-cli-subcommand maps rust/fspec/tests/cli_<cmd>.rs (e.g. add-aggregate). Move the dispatch_command-based tests and their workspace-seeding helpers. Drop the second `//! Feature:` header line from each file. scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher stays in CLI.

#### `rust/fspec/tests/cli_query_estimation_guide.rs`

- Kind: integration test (Rust, crate fspec)
- **query-estimation-guide-cli-subcommand** [RPC-259:done] 8 scen: scenario_clap_exposes_query_estimation_guide_with_help, scenario_cli_without_format_prints_nothing, scenario_cli_format_text_prints_nothing, scenario_cli_format_json_empty_workspace_prints_empty_array, scenario_cli_requires_positional_work_unit_id, scenario_cli_malformed_work_units_json_exits_1_with_stderr, scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher, scenario_query_estimation_guide_help_matches_ts_reference
- **query-estimation-guide-rust-port** [RPC-259:done] 10 scen: scenario_dispatcher_empty_workspace_returns_empty_patterns, scenario_dispatcher_ignores_non_done_units, scenario_dispatcher_skips_done_unit_missing_iterations, scenario_dispatcher_skips_done_unit_missing_estimate, scenario_dispatcher_single_done_unit_yields_low_confidence, scenario_dispatcher_two_done_units_yield_medium_confidence, scenario_dispatcher_four_done_units_yield_high_confidence, scenario_dispatcher_two_buckets_sorted_ascending, scenario_dispatcher_pattern_field_declaration_order, scenario_dispatcher_malformed_work_units_json_yields_parse_error
- **Fix: SPLIT (mechanical)**
- New file(s): rust/fspec-core/tests/query_estimation_guide.rs (query-estimation-guide-rust-port: the 10 dispatcher-level tests)
- Moves / wiring: Repo convention: <cmd>-rust-port maps rust/fspec-core/tests/<cmd>.rs and <cmd>-cli-subcommand maps rust/fspec/tests/cli_<cmd>.rs (e.g. add-aggregate). Move the dispatch_command-based tests and their workspace-seeding helpers. Drop the second `//! Feature:` header line from each file. scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher stays in CLI.

#### `rust/fspec/tests/cli_query_example_mapping_stats.rs`

- Kind: integration test (Rust, crate fspec)
- **query-example-mapping-stats-cli-subcommand** [RPC-260:done] 7 scen: scenario_clap_exposes_query_example_mapping_stats_with_help, scenario_cli_format_json_prints_canonical_empty_stats, scenario_cli_without_format_prints_nothing_to_stdout, scenario_cli_format_text_prints_nothing_to_stdout, scenario_cli_malformed_work_units_json_exits_1_with_stderr, scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher, scenario_query_example_mapping_stats_help_matches_ts_reference
- **query-example-mapping-stats-rust-port** [RPC-260:done] 14 scen: scenario_returns_empty_stats_when_work_units_auto_created_in_empty_workspace, scenario_completeness_score_100_when_rules_and_examples_nonempty_and_no_questions, scenario_completeness_score_0_when_only_questions, scenario_completeness_score_66_with_rules_only_and_no_questions, scenario_completeness_score_67_with_examples_only_and_no_questions, scenario_aggregate_counts_and_averages_reflect_every_retained_work_unit, scenario_work_unit_id_filter_narrows_to_single_work_unit, scenario_work_unit_id_filter_against_missing_id_surfaces_error, scenario_has_questions_true_keeps_only_units_with_questions, scenario_has_questions_false_keeps_only_units_with_zero_questions, scenario_questions_for_alice_keeps_only_units_mentioning_alice, scenario_result_json_field_order_matches_ts_shape, scenario_escalates_malformed_work_units_json_dispatcher, scenario_per_work_unit_stats_record_carries_title_and_status
- **Fix: SPLIT (mechanical)**
- New file(s): rust/fspec-core/tests/query_example_mapping_stats.rs (query-example-mapping-stats-rust-port: the 14 dispatcher-level tests)
- Moves / wiring: Repo convention: <cmd>-rust-port maps rust/fspec-core/tests/<cmd>.rs and <cmd>-cli-subcommand maps rust/fspec/tests/cli_<cmd>.rs (e.g. add-aggregate). Move the dispatch_command-based tests and their workspace-seeding helpers. Drop the second `//! Feature:` header line from each file. scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher stays in CLI.

#### `rust/fspec/tests/cli_query_orphans.rs`

- Kind: integration test (Rust, crate fspec)
- **query-orphans-cli-subcommand** [RPC-262:done] 8 scen: scenario_clap_exposes_query_orphans_with_help, scenario_cli_no_options_empty_workspace_prints_success, scenario_cli_output_json_empty_workspace_prints_empty_array, scenario_cli_text_output_lists_orphan_with_suggested_actions, scenario_cli_exclude_done_flag_suppresses_done_orphans, scenario_cli_malformed_work_units_json_exits_1_with_stderr, scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher, scenario_query_orphans_help_matches_ts_reference
- **query-orphans-rust-port** [RPC-262:done] 11 scen: scenario_dispatcher_empty_workspace_returns_empty_orphans, scenario_dispatcher_non_blank_epic_not_orphaned, scenario_dispatcher_non_empty_blocks_not_orphaned, scenario_dispatcher_no_epic_no_relations_is_orphaned, scenario_dispatcher_whitespace_epic_is_treated_as_no_epic, scenario_dispatcher_empty_relationship_arrays_treated_as_no_relationships, scenario_dispatcher_exclude_done_filters_done_orphans, scenario_dispatcher_orphans_in_insertion_order, scenario_dispatcher_orphan_field_declaration_order, scenario_dispatcher_malformed_work_units_json_yields_parse_error
- **Fix: SPLIT (mechanical)**
- New file(s): rust/fspec-core/tests/query_orphans.rs (query-orphans-rust-port: the 10 dispatcher-level tests)
- Moves / wiring: Repo convention: <cmd>-rust-port maps rust/fspec-core/tests/<cmd>.rs and <cmd>-cli-subcommand maps rust/fspec/tests/cli_<cmd>.rs (e.g. add-aggregate). Move the dispatch_command-based tests and their workspace-seeding helpers. Drop the second `//! Feature:` header line from each file. scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher stays in CLI.

#### `rust/fspec/tests/cli_show_acceptance_criteria.rs`

- Kind: integration test (Rust, crate fspec)
- **show-acceptance-criteria-cli-subcommand** [RPC-299:done] 10 scen: cli_clap_exposes_subcommand_with_flag_help, cli_against_workspace_with_no_spec_features_exits_1, cli_default_text_output_prints_feature_name_and_scenario_steps, cli_format_markdown_prints_h1_h2_bullet_step_output, cli_format_json_prints_two_space_json_array, cli_output_writes_file_and_prints_message_without_dumping_content, cli_tag_matching_zero_features_prints_no_features_found, cli_help_matches_ts_formatcommandhelp_reference, cli_default_combined_tui_mode_preserved, cli_delegates_to_same_fspec_core_function_as_dispatcher
- **show-acceptance-criteria-rust-port** [RPC-299:done] 12 scen: rust_port_missing_spec_features_directory_returns_structured_error, rust_port_empty_spec_features_directory_returns_success_with_message, rust_port_tag_filter_selects_only_features_carrying_that_tag, rust_port_multiple_tag_filters_require_all_tags_present, rust_port_format_markdown_renders_h1_blockquote_h2_bullet_steps, rust_port_format_json_renders_two_space_json_array, rust_port_tag_matches_zero_features_returns_no_features_message, rust_port_feature_without_background_rendered_with_no_background_block, rust_port_feature_with_no_scenarios_shows_no_scenarios_defined_marker, rust_port_output_path_writes_rendered_content_to_disk_and_changes_message, rust_port_summary_line_reports_scenario_and_feature_counts, rust_port_shared_infrastructure_module_is_registered
- **Fix: SPLIT (mechanical)**
- New file(s): rust/fspec-core/tests/show_acceptance_criteria.rs (show-acceptance-criteria-rust-port: the 12 dispatcher-level tests)
- Moves / wiring: Repo convention: <cmd>-rust-port maps rust/fspec-core/tests/<cmd>.rs and <cmd>-cli-subcommand maps rust/fspec/tests/cli_<cmd>.rs (e.g. add-aggregate). Move the dispatch_command-based tests and their workspace-seeding helpers. Drop the second `//! Feature:` header line from each file. scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher stays in CLI.

#### `rust/fspec/tests/cli_show_event_storm.rs`

- Kind: integration test (Rust, crate fspec)
- **show-event-storm-cli-subcommand** [RPC-303:done] 7 scen: scenario_clap_exposes_show_event_storm_with_help, scenario_cli_empty_workspace_exits_1_with_not_found, scenario_cli_exits_1_when_no_event_storm_data, scenario_cli_prints_json_array_of_active_items_on_success, scenario_cli_exits_1_when_malformed_work_units, scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher, scenario_show_event_storm_help_matches_ts_reference
- **show-event-storm-rust-port** [RPC-303:done] 11 scen: scenario_returns_work_unit_not_found_when_empty_workspace, scenario_returns_work_unit_not_found_when_id_missing, scenario_returns_no_event_storm_data_when_no_event_storm_field, scenario_returns_no_event_storm_data_when_no_items_array, scenario_returns_active_items_as_pretty_printed_json_array, scenario_filters_out_soft_deleted_items, scenario_treats_missing_deleted_field_as_retained, scenario_returns_empty_array_when_event_storm_items_empty, scenario_preserves_every_field_on_every_retained_item, scenario_escalates_malformed_work_units_dispatcher, scenario_missing_work_unit_id_surfaces_invalid_args_error
- **Fix: SPLIT (mechanical)**
- New file(s): rust/fspec-core/tests/show_event_storm.rs (show-event-storm-rust-port: the 11 dispatcher-level tests)
- Moves / wiring: Repo convention: <cmd>-rust-port maps rust/fspec-core/tests/<cmd>.rs and <cmd>-cli-subcommand maps rust/fspec/tests/cli_<cmd>.rs (e.g. add-aggregate). Move the dispatch_command-based tests and their workspace-seeding helpers. Drop the second `//! Feature:` header line from each file. scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher stays in CLI.

#### `rust/fspec/tests/cli_show_foundation.rs`

- Kind: integration test (Rust, crate fspec)
- **show-foundation-cli-subcommand** [RPC-305:done] 12 scen: scenario_clap_exposes_show_foundation_with_help, scenario_cli_default_render_prints_project_section, scenario_cli_positional_section_emits_raw_string_in_text_format, scenario_cli_format_json_emits_json, scenario_cli_exits_1_when_section_unknown, scenario_cli_draft_surfaces_missing_draft_error, scenario_cli_output_writes_file_and_prints_success_line, scenario_cli_exits_1_when_foundation_malformed, scenario_cli_list_sections_is_parsed_but_ignored, scenario_cli_line_numbers_is_parsed_but_ignored, scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher, scenario_show_foundation_help_matches_ts_reference
- **show-foundation-rust-port** [RPC-305:done] 13 scen: scenario_returns_text_render_with_project_and_other_sections, scenario_returns_entire_foundation_as_pretty_json_when_format_json, scenario_resolves_project_name_via_field_map_text_format, scenario_resolves_project_name_via_field_map_json_format, scenario_section_pointing_to_object_emits_pretty_json_in_text_format, scenario_missing_section_returns_field_not_found_error, scenario_dotted_path_bypasses_field_map_for_unmapped_sections, scenario_draft_true_with_no_draft_file_returns_missing_draft_error, scenario_draft_true_reads_draft_file_instead_of_foundation, scenario_empty_workspace_auto_creates_foundation_json, scenario_escalates_malformed_foundation_json_dispatcher, scenario_output_writes_formatted_content_to_disk_via_dispatcher, scenario_default_format_is_text_when_format_flag_omitted
- **Fix: SPLIT (mechanical)**
- New file(s): rust/fspec-core/tests/show_foundation.rs (show-foundation-rust-port: the 13 dispatcher-level tests)
- Moves / wiring: Repo convention: <cmd>-rust-port maps rust/fspec-core/tests/<cmd>.rs and <cmd>-cli-subcommand maps rust/fspec/tests/cli_<cmd>.rs (e.g. add-aggregate). Move the dispatch_command-based tests and their workspace-seeding helpers. Drop the second `//! Feature:` header line from each file. scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher stays in CLI.

#### `rust/fspec/tests/cli_show_foundation_event_storm.rs`

- Kind: integration test (Rust, crate fspec)
- **show-foundation-event-storm-cli-subcommand** [RPC-306:done] 10 scen: cli_clap_exposes_subcommand_with_flag_help, cli_against_workspace_with_no_foundation_exits_1_with_error, cli_prints_empty_array_when_no_event_storm, cli_prints_all_active_items_when_no_filters, cli_type_filter_narrows_to_matching_items, cli_context_filter_returns_bc_plus_linked_items, cli_context_unknown_errors_and_lists_available_contexts, cli_help_matches_ts_formatcommandhelp_reference, cli_default_combined_tui_mode_preserved, cli_delegates_to_same_fspec_core_function_as_dispatcher
- **show-foundation-event-storm-rust-port** [RPC-306:done] 8 scen: rust_port_missing_foundation_json_surfaces_error, rust_port_no_event_storm_field_returns_empty_data_and_message, rust_port_soft_deleted_items_filtered_out, rust_port_filtering_by_type_returns_matching_items, rust_port_filtering_by_context_returns_bounded_context_plus_linked_items, rust_port_filtering_by_unknown_context_errors_and_lists_available_contexts, rust_port_combined_context_and_type_filters_compose, rust_port_shared_infrastructure_module_is_registered
- **Fix: SPLIT (mechanical)**
- New file(s): rust/fspec-core/tests/show_foundation_event_storm.rs (show-foundation-event-storm-rust-port: the 8 dispatcher-level tests)
- Moves / wiring: Repo convention: <cmd>-rust-port maps rust/fspec-core/tests/<cmd>.rs and <cmd>-cli-subcommand maps rust/fspec/tests/cli_<cmd>.rs (e.g. add-aggregate). Move the dispatch_command-based tests and their workspace-seeding helpers. Drop the second `//! Feature:` header line from each file. scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher stays in CLI.

#### `rust/fspec/tests/cli_show_test_patterns.rs`

- Kind: integration test (Rust, crate fspec)
- **show-test-patterns-cli-subcommand** [RPC-307:done] 9 scen: cli_clap_exposes_subcommand_with_flag_help, cli_without_tag_flag_exits_non_zero, cli_default_output_prints_green_summary_line, cli_json_flag_prints_json_envelope_to_stdout, cli_include_coverage_includes_deduplicated_test_file_paths, cli_exits_1_when_work_units_json_missing, cli_help_matches_ts_formatcommandhelp_reference, cli_default_combined_tui_mode_preserved, cli_delegates_to_same_fspec_core_function_as_dispatcher
- **show-test-patterns-rust-port** [RPC-307:done] 8 scen: rust_port_missing_tag_argument_surfaces_invalid_args_error, rust_port_tag_matches_zero_work_units_returns_empty_data, rust_port_tag_matches_work_units_returns_their_tags_arrays, rust_port_include_coverage_true_reads_and_dedupes_test_files, rust_port_include_coverage_false_leaves_test_files_empty, rust_port_json_format_sets_data_format_to_json, rust_port_default_format_flag_yields_table_format, rust_port_shared_infrastructure_module_is_registered
- **Fix: SPLIT (mechanical)**
- New file(s): rust/fspec-core/tests/show_test_patterns.rs (show-test-patterns-rust-port: the 8 dispatcher-level tests)
- Moves / wiring: Repo convention: <cmd>-rust-port maps rust/fspec-core/tests/<cmd>.rs and <cmd>-cli-subcommand maps rust/fspec/tests/cli_<cmd>.rs (e.g. add-aggregate). Move the dispatch_command-based tests and their workspace-seeding helpers. Drop the second `//! Feature:` header line from each file. scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher stays in CLI.

#### `rust/fspec/tests/cross_frontend_parity.rs`

- Kind: integration test (Rust, crate fspec)
- **cross-frontend-integration-test-against-stub-provider** [RPC-066:done] 11 scen: scenario_fspec_daemon_boots_and_emits_a_port, scenario_workspace_registers_the_stub_provider, scenario_send_input_hello_yields_canned_stream, scenario_scripted_run_matches_golden, scenario_normalise_chunk_stream_substitutes_volatile_fields, scenario_regenerate_env_var_recorded_in_test_source, scenario_missing_fixture_fails_with_clear_hint, scenario_regression_catch_documented, scenario_runtime_budget_is_enforced, scenario_tests_readme_documents_regeneration
- **stub-provider-rig-dispatch** [RPC-069:done] 5 scen: scenario_send_input_hello_yields_canned_stream, scenario_fspec_daemon_boots_and_emits_a_port, scenario_scripted_run_matches_golden, scenario_deny_network_egress_still_yields_canned_chunks, scenario_agent_loop_dispatch_supports_stub_arm
- Same test credited to 2+ features: scenario_fspec_daemon_boots_and_emits_a_port (cross-frontend-integration-test-against-stub-provider, stub-provider-rig-dispatch); scenario_send_input_hello_yields_canned_stream (cross-frontend-integration-test-against-stub-provider, stub-provider-rig-dispatch); scenario_scripted_run_matches_golden (cross-frontend-integration-test-against-stub-provider, stub-provider-rig-dispatch)
- **Fix: SPLIT + DEDUP**
- New file(s): rust/fspec/tests/rpc069_stub_provider_rig_dispatch.rs (scenario_deny_network_egress_still_yields_canned_chunks, scenario_agent_loop_dispatch_supports_stub_arm); rust/fspec/tests/cross_frontend_support/mod.rs (daemon boot + stub harness + golden normaliser, shared by both)
- Moves / wiring: RPC-069 borrows 3 RPC-066 tests (daemon boots, hello canned stream, scripted run golden).
- Decision needed: RPC-069's 3 borrowed scenarios: write focused stub-dispatch tests in the new file (recommended), or drop them as duplicates of RPC-066.

### B3 fspec-tui A: views and mechanical splits (10)

#### `rust/fspec-tui/src/components/help_dialog.rs`

- Kind: source file with inline tests (line 222) (Rust, crate fspec-tui)
- **fspec-tui-help-dialog-body-rpc009** [RPC-009:done] 3 scen; 1 stale mapping(s): help_dialog_is_critical_priority_with_canonical_id, help_dialog_body_lists_the_board_keybindings, help_dialog_rendering_is_byte_equal_across_runs_insta_snapshot
- **fspec-tui-help-dialog** [RPC-008:done] 3 scen; 3 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **Fix: MERGE / DEPRECATE (recommended), else SPLIT**
- New file(s): if split: rust/fspec-tui/src/components/help_dialog/popup_tests.rs (RPC-008)
- Moves / wiring: All 3 RPC-008 mappings point at impl lines (handle_event/render), not tests.
- Notes: RPC-008 "static body lists exactly the ?, ESC and q keybindings" contradicts RPC-009's body. RPC-009 restates RPC-008's popup/priority and snapshot invariants.
- Decision needed: Retire fspec-tui-help-dialog (RPC-008) as superseded by fspec-tui-help-dialog-body-rpc009, or rewrite it.

#### `rust/fspec-tui/src/views/blocklist/tests.rs`

- Kind: in-crate test module (mod tests; in blocklist/mod.rs) (Rust, crate fspec-tui)
- **blocklist-view-framing** [BLOCK-009:done] 4 scen: header_shows_the_rules_count, a_vertical_divider_separates_the_list_and_details_panes, footer_shows_the_reference_parity_hint, rpc056_rendering_behaviour_is_preserved_after_the_framing_change
- **blocklist-view-keybindings** [BLOCK-010:done] 8 scen: vim_keys_do_not_move_the_selection, arrow_keys_move_the_selection_one_rule_at_a_time, page_down_advances_the_selection_by_one_viewport, page_down_clamps_at_the_last_rule, page_up_retreats_the_selection_by_one_viewport, end_selects_the_last_rule_and_home_selects_the_first_rule, space_toggles_the_focused_rule, footer_hint_no_longer_advertises_vim_keys
- **blocklist-view-mouse-scroll** [BLOCK-011:done] 5 scen: wheel_down_moves_the_selection_down, wheel_down_clamps_at_the_last_rule, wheel_up_clamps_at_the_first_rule, non_wheel_mouse_events_are_ignored, the_navigator_routes_wheel_events_to_the_view
- **blocklist-view-scrolling** [BLOCK-008:done] 6 scen: scrolling_down_keeps_focused_row_inside_window, scrolling_back_up_above_window_scrolls_offset_back, scroll_offset_clamps_at_total_minus_visible_on_last_row, rendering_overflowing_list_windows_rows_and_shows_indicator, re_seeding_with_shorter_list_resets_scroll_offset, overflowing_list_renders_scrollbar_gutter_and_fitting_list_does_not
- Unmapped (2): set_rules_resets_selection_when_index_exceeds_new_len, derive_category_classifies_bash_and_file_path
- **Fix: SPLIT**
- New file(s): rust/fspec-tui/src/views/blocklist/framing_tests.rs (BLOCK-009); rust/fspec-tui/src/views/blocklist/keybindings_tests.rs (BLOCK-010); rust/fspec-tui/src/views/blocklist/mouse_scroll_tests.rs (BLOCK-011); tests.rs keeps BLOCK-008 scrolling; rust/fspec-tui/src/views/blocklist/test_support.rs (seeded view/rule fixtures)
- Moves / wiring: Declare each `#[cfg(test)] mod <x>_tests;` in blocklist/mod.rs (the <topic>_tests.rs naming follows checkpoints/delete_tests.rs).

#### `rust/fspec-tui/src/views/changed_files/tests.rs`

- Kind: in-crate test module (Rust, crate fspec-tui)
- **changed-files-view-click-to-select** [RPC-368:done] 5 scen: clicking_an_unselected_file_row_selects_it_and_reloads_its_diff, clicking_the_top_visible_row_selects_the_file_at_the_scroll_offset, clicking_the_already_selected_file_row_changes_nothing, clicking_inside_the_diff_pane_focuses_it_without_changing_the_selection, clicking_empty_space_below_the_last_file_changes_nothing
- **changed-files-view-f-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-changed-files-empty-state** [TUI-108:done] 7 scen: pre_load_render_shows_loading_dialog_not_empty_message, loaded_but_empty_shows_real_empty_message, diff_stage_shows_its_label_until_the_diff_folds_in, stale_diff_result_does_not_clear_the_current_stage, esc_ignored_while_loading_and_closes_after_flush, loading_dialog_renders_through_the_canonical_dialog_theme, arrowing_while_a_diff_is_in_flight_is_swallowed_so_the_selection_stays_put
- **rust-changed-files-view** [RPC-359:done,RPC-358:done,RPC-357:done,RPC-356:done] 18 scen: pressing_f_on_the_board_emits_open_changed_files_view, open_flips_navigator_to_changed_files_and_close_returns_to_board, file_list_renders_colored_status_letters_and_cursor, diff_pane_renders_colored_add_remove_and_hunk_lines, moving_selection_down_reloads_diff_for_new_file, tab_moves_focus_to_diff_then_pgdn_scrolls_diff_not_list, empty_repo_shows_message_and_esc_closes, mouse_wheel_scrolls_focused_diff_pane_by_wheel_velocity_step, diff_pane_scroll_stops_at_the_last_full_page, mouse_wheel_over_file_list_reloads_diff_for_new_file, mouse_wheel_over_diff_pane_scrolls_diff_without_changing_selection, diff_focused_down_scrolls_diff_one_line, diff_focused_up_at_top_clamps_scroll_at_zero, diff_focused_down_leaves_selection_unchanged, files_focused_down_moves_selection_and_reloads_diff, file_list_taller_than_pane_renders_scrollbar, diff_longer_than_pane_renders_scrollbar, file_list_that_fits_renders_no_scrollbar
- **rust-tui-pane-borders-changed-files** [RPC-367:done] 3 scen: changed_files_view_shows_vertical_divider_between_files_and_diff_panes, each_pane_shows_horizontal_underline_rule_beneath_its_heading, empty_changed_files_view_still_shows_its_empty_state_message
- **Fix: SPLIT**
- New file(s): rust/fspec-tui/src/views/changed_files/click_tests.rs (RPC-368); rust/fspec-tui/src/views/changed_files/loading_tests.rs (TUI-108); rust/fspec-tui/src/views/changed_files/borders_tests.rs (RPC-367); tests.rs keeps rust-changed-files-view (RPC-356..359); rust/fspec-tui/src/views/changed_files/test_support.rs

#### `rust/fspec-tui/src/views/checkpoints/tests.rs`

- Kind: in-crate test module (Rust, crate fspec-tui)
- **checkpoints-view-c-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-checkpoints-empty-state** [TUI-107:done] 7 scen: pre_load_render_shows_loading_dialog_not_empty_message, loaded_but_empty_shows_real_empty_message, files_stage_shows_its_own_label, diff_stage_shows_its_label_until_the_diff_folds_in, stale_files_result_does_not_clear_the_current_stage, esc_ignored_while_loading_and_closes_after_flush, loading_dialog_renders_through_the_canonical_dialog_theme
- **checkpoints-view-click-to-select** [RPC-369:done] 6 scen: clicking_a_checkpoint_name_row_selects_it_and_loads_its_files, clicking_a_file_row_selects_it_and_loads_its_diff, clicking_inside_the_diff_pane_focuses_it_without_changing_any_selection, a_click_is_swallowed_while_the_restore_dialog_is_open, clicking_the_already_selected_checkpoint_row_changes_nothing, clicking_empty_space_below_the_last_checkpoint_row_changes_nothing
- **rust-checkpoints-view** [RPC-364:done] 7 scen: pressing_c_on_the_board_emits_open_checkpoints_view, open_flips_navigator_to_checkpoints_and_close_returns_to_board, auto_and_manual_checkpoints_render_their_labels, tab_moves_focus_from_checkpoints_to_files_and_highlights_heading, selecting_checkpoint_then_file_shows_colored_diff, arrow_keys_act_on_the_focused_pane, empty_repo_shows_message_and_esc_closes
- **rust-tui-pane-borders-checkpoints** [RPC-367:done] 1 scen: checkpoints_view_shows_vertical_divider_between_checkpoints_and_files_panes
- **Fix: SPLIT**
- New file(s): rust/fspec-tui/src/views/checkpoints/click_tests.rs (RPC-369); rust/fspec-tui/src/views/checkpoints/loading_tests.rs (TUI-107); rust/fspec-tui/src/views/checkpoints/borders_tests.rs (RPC-367); tests.rs keeps rust-checkpoints-view (RPC-364); rust/fspec-tui/src/views/checkpoints/test_support.rs
- Moves / wiring: Follows the existing delete_tests.rs / restore_tests.rs siblings.

#### `rust/fspec-tui/src/views/full_screen_shell.rs`

- Kind: source file with inline tests (line 165) (Rust, crate fspec-tui)
- **full-screen-shell-title-closure** [RPC-339:done] 2 scen: shell_title_closure_paints_title_body_and_footer_no_overlay, count_wrapper_preserves_title_count_format
- **full-screen-shell** [RPC-337:done] 5 scen: shell_renders_title_count_body_and_footer_with_no_overlay, shell_paints_optional_confirm_dialog_overlay_over_full_area, shell_skips_overlay_slot_when_none, shell_reports_body_height_to_body_renderer, shell_collapses_body_gracefully_on_tiny_area
- **Fix: SPLIT**
- New file(s): rust/fspec-tui/src/views/full_screen_shell/tests.rs (RPC-337, 5); rust/fspec-tui/src/views/full_screen_shell/title_closure_tests.rs (RPC-339, 2)
- Moves / wiring: Move with `#[cfg(test)] #[path]`.

#### `rust/fspec-tui/src/views/model_selector/mod.rs`

- Kind: module root; tests moved to sibling tests_*.rs files (Rust, crate fspec-tui)
- **model-selector-custom-model-crud** [RPC-344:done] 15 scen; 30 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **model-selector-default-expansion** [RPC-342:done] 5 scen; 10 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **model-selector-no-auto-select** [PROV-101:done] 1 scen; 2 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **model-selector-open-on-current-model** [RPC-341:done] 4 scen; 8 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **model-selector-scroll** [RPC-340:done] 8 scen; 16 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **model-selector-tab-to-providers** [RPC-345:done] 5 scen; 10 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **model-selector-view** [RPC-337:done,BUG-300:done] 8 scen; 16 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **Fix: RELINK (stale)**
- New file(s): none expected
- Moves / wiring: All 54 mappings point past the end of the 148-line file. Re-link each scenario by its `/// Scenario:` comment: custom-model-crud -> tests_crud_add.rs + tests_crud_delete.rs; default-expansion -> tests_collapse.rs; open-on-current-model -> tests_current_model.rs; scroll -> tests_scroll.rs; tab-to-providers -> tests_tab.rs; view -> tests_core.rs (already maps it). None of these targets is mapped by any other feature today.
- Notes: Trap: tests_current_model.rs also holds the PROV-101 "No active selection ..." tests (model-selector-no-auto-select). Move those into a new tests_no_auto_select.rs, or the relink creates a new shared file. tests_crud_delete.rs has a "Feature: model-selector-no-auto-select" header, so check which feature it really belongs to.

#### `rust/fspec-tui/src/views/model_selector/rows.rs`

- Kind: module; tests moved to rows_tests*.rs (Rust, crate fspec-tui)
- **model-selector-profile-rendering** [RPC-338:done] 5 scen; 10 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **model-selector-rows** [RPC-337:done] 4 scen; 5 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **Fix: RELINK (stale)**
- New file(s): none expected
- Moves / wiring: model-selector-profile-rendering -> rows_tests_profile.rs (unmapped today). model-selector-rows -> rows_tests.rs, but rows_tests.rs is already mapped by model-selector-header-count-label, so either move the RPC-337 row tests into a new rows_tests_core.rs or check whether that feature took them over.

#### `rust/fspec-tui/tests/keyboard_cascade_rpc051.rs`

- Kind: integration test (Rust, crate fspec-tui)
- **agentview-esc-cascade-input-clear** [RPC-095:done] 3 scen: shift_up_at_end_of_history_clamps_at_tail, esc_level_6_idle_session_with_nonempty_input_clears_buffer, esc_level_5_unknown_status_navigates_back_to_board
- **keyboard-shortcut-cascade-parity** [RPC-051:done] 10 scen: esc_level_1_slash_popup_dismiss_takes_precedence_over_interrupt, esc_level_2_help_dialog_dismiss_takes_precedence_over_interrupt, esc_level_3_resume_mode_view_dismiss_takes_precedence, esc_level_3_search_mode_view_dismiss_takes_precedence, esc_level_4_running_session_interrupts_without_navigating_back, esc_level_4_compacting_session_interrupts_without_navigating, esc_level_5_unknown_status_navigates_back_to_board, esc_level_5_no_current_session_navigates_back_to_board, ctrl_r_opens_search_history_view_with_input_focused, shift_up_down_snapshots_walks_and_restores_draft, shift_up_at_end_of_history_clamps_at_tail
- Same test credited to 2+ features: shift_up_at_end_of_history_clamps_at_tail (agentview-esc-cascade-input-clear, keyboard-shortcut-cascade-parity); esc_level_5_unknown_status_navigates_back_to_board (agentview-esc-cascade-input-clear, keyboard-shortcut-cascade-parity)
- Unmapped (2): esc_level_5_idle_session_navigates_back_to_board, esc_idle_session_with_whitespace_only_input_navigates_back
- **Fix: SPLIT (+ relink drift)**
- New file(s): rust/fspec-tui/tests/agentview_esc_cascade_input_clear_rpc095.rs (RPC-095: esc_level_6_idle_session_with_nonempty_input_clears_buffer, esc_idle_session_with_whitespace_only_input_navigates_back)
- Moves / wiring: Uses tests/common. RPC-095's mappings onto shift_up_at_end_of_history_clamps_at_tail and esc_level_5_unknown_status_navigates_back_to_board are drift; its real tests are the two unmapped esc_* tests. Naming follows multilineinput_parity_rpc095.rs.

#### `rust/fspec-tui/tests/menu002_board_surface.rs`

- Kind: integration test (Rust, crate fspec-tui)
- **board-surface-header-row-replaced-by-the-2-zone-bar-column-menu-chip-continuous-ring-keys-wheel-mouse** [MENU-002:done] 19 scen: scenario_left_from_the_new_agent_button_focuses_the_last_chip, scenario_enter_on_an_open_dropdown_executes_the_highlighted_row, scenario_enter_on_a_chip_opens_that_session_agent_view, scenario_up_and_down_from_the_bar_drop_focus_back_into_the_focused_column, scenario_the_board_header_row_3_paints_the_2_zone_menu_bar, scenario_right_from_the_last_column_focuses_the_first_menu_item, scenario_the_ring_walks_items_then_chips_then_wraps_to_the_first_column, scenario_esc_closes_the_open_dropdown_and_keeps_the_item_focused, scenario_clicking_a_menu_item_focuses_it_and_opens_its_dropdown, scenario_the_board_footer_hint_describes_the_ring, scenario_the_header_bar_omits_the_separator_and_chips_when_no_sessions_are_open, scenario_while_a_dropdown_is_open_all_other_keys_are_swallowed, scenario_wheel_left_and_right_over_the_bar_walk_the_ring, scenario_with_no_open_sessions_the_ring_wraps_from_the_last_item_to_the_first_column, scenario_clicking_the_already_open_item_closes_the_dropdown, scenario_wheel_up_and_down_over_an_open_dropdown_move_the_cursor, scenario_enter_on_a_menu_item_opens_its_dropdown_at_row_0
- **menu-dropdown-cycles-with-the-board-ring-walk** [BUG-198:done] 5 scen: scenario_clicking_outside_the_dropdown_closes_it_while_the_click_still_works, scenario_while_a_dropdown_is_open_left_and_right_walk_the_ring_and_reanchor, scenario_left_with_an_open_dropdown_moves_the_panel_back_onto_the_previous_item, scenario_left_from_the_first_item_with_an_open_dropdown_closes_the_panel
- Unmapped (5): scenario_left_from_the_first_column_focuses_the_last_chip, scenario_esc_with_no_dropdown_open_is_unchanged, scenario_clicking_a_chip_activates_that_session, scenario_right_from_the_last_item_with_an_open_dropdown_closes_the_panel, scenario_wheel_over_the_bar_with_an_open_dropdown_reanchors_the_panel
- **Fix: SPLIT**
- New file(s): rust/fspec-tui/tests/bug198_menu_dropdown_ring_walk.rs (4 tests); rust/fspec-tui/tests/menu002_support/mod.rs (board-surface harness; precedent tests/menu007_support/)

#### `rust/fspec-tui/tests/menu005_menu_bar_help.rs`

- Kind: integration test (Rust, crate fspec-tui)
- **board-new-agent-gesture-prompts-create-session-dialog** [BUG-203:done] 11 scen: scenario_the_mux_surface_renders_one_top_row_bar_with_per_pane_suppression_wide_and_narrow, scenario_bug203_shift_right_from_board_still_resumes_first_open_session, scenario_bug203_board_period_key_with_attached_session_mounts_dialog_not_jump, scenario_bug203_board_period_key_with_unattached_open_session_does_not_resume, scenario_bug203_board_zone_c_button_with_running_agent_mounts_dialog, scenario_bug203_confirm_yes_from_board_flips_to_agent_without_binding_unit, scenario_bug203_cancel_from_board_prompt_keeps_board_and_sessions, scenario_bug203_shift_right_from_board_with_attached_session_still_jumps_into_it, scenario_bug203_kanban_dropdown_new_agent_row_mounts_dialog, scenario_bug203_u_help_dialog_new_agent_row_mounts_dialog
- **u-menu-bar-help-re-purpose-help-content-snapshot-shape-test-migration-tag** [MENU-005:done] 11 scen: scenario_enter_on_the_a_attachments_row_fires_the_attachment_picker, scenario_up_from_the_first_row_wraps_to_the_last_row_and_down_wraps_back, scenario_the_board_help_dialog_lists_the_updated_keybinding_rows, scenario_the_agent_help_lists_the_empty_input_left_arrow_menu_bar_entry, scenario_the_board_footer_hint_keeps_the_ring_wording, scenario_the_dead_chord_shim_files_are_deleted_and_the_dialog_module_renamed, scenario_the_agent_pane_flag_on_layout_pins_the_6_constraint_list, scenario_the_board_surface_renders_the_bar_wide_and_narrow_snapshots, scenario_the_agent_surface_renders_the_bar_row_under_the_header_wide_and_narrow_snapshots, scenario_u_opens_the_menu_bar_dialog_listing_all_ten_registry_rows_in_order
- Unmapped (2): scenario_enter_on_the_new_agent_row_mounts_the_create_session_dialog, scenario_bug203_mux_top_bar_kanban_new_agent_row_mounts_dialog
- **Fix: SPLIT**
- New file(s): rust/fspec-tui/tests/bug203_board_new_agent_gesture.rs (11 tests); rust/fspec-tui/tests/menu005_support/mod.rs

### B4 fspec-tui B: BUG-205 cluster and supersessions (9)

#### `rust/fspec-tui/tests/agentview_esc_exit_confirmation_rpc098.rs`

- Kind: integration test (Rust, crate fspec-tui)
- **agentview-esc-exit-confirmation-dialog** [RPC-098:done,BUG-299:done] 16 scen: idle_session_esc_opens_dialog_with_idle_description_and_close_session_focused, running_session_first_esc_interrupts_second_esc_opens_dialog, compacting_session_esc_routes_to_interrupt_and_not_dialog, no_active_session_esc_dispatches_back_to_board_without_dialog, cyclic_left_right_navigation_across_two_buttons, enter_on_preselected_close_session_destroys_the_session_and_returns_to_board, enter_on_close_session_destroys_then_dispatches_back_to_board, enter_on_cancel_removes_dialog_and_stays_on_agentview, esc_inside_dialog_is_equivalent_to_cancel, pressing_esc_twice_from_l7_only_opens_one_dialog, cycling_sessions_in_agent_view_after_close_session_does_not_list_destroyed_session, shift_right_after_close_session_does_not_navigate_back_to_destroyed_session, end_to_end_app_render_overlays_dialog_on_top_of_agentview_chrome, dialog_priority_is_critical_and_id_matches_const, end_to_end_close_session_purges_destroyed_session_from_every_cycle_path
- **exit-session-dialog-two-button-options** [BUG-205:done] 5 scen: idle_session_esc_opens_dialog_with_idle_description_and_close_session_focused, cyclic_left_right_navigation_across_two_buttons, enter_on_preselected_close_session_destroys_the_session_and_returns_to_board, enter_on_cancel_removes_dialog_and_stays_on_agentview
- Same test credited to 2+ features: idle_session_esc_opens_dialog_with_idle_description_and_close_session_focused (agentview-esc-exit-confirmation-dialog, exit-session-dialog-two-button-options); cyclic_left_right_navigation_across_two_buttons (agentview-esc-exit-confirmation-dialog, exit-session-dialog-two-button-options); enter_on_preselected_close_session_destroys_the_session_and_returns_to_board (agentview-esc-exit-confirmation-dialog, exit-session-dialog-two-button-options); enter_on_cancel_removes_dialog_and_stays_on_agentview (agentview-esc-exit-confirmation-dialog, exit-session-dialog-two-button-options)
- Unmapped (2): snapshot_dialog_80x24_is_busy_true, snapshot_dialog_80x24_is_busy_false
- **Fix: MERGE (recommended), else SPLIT + DEDUP**
- New file(s): if split: rust/fspec-tui/tests/bug205_exit_session_dialog_two_buttons.rs
- Moves / wiring: BUG-205 exit-session-dialog-two-button-options (5 scenarios) maps 4 RPC-098 tests. RPC-098 (with BUG-299) already specifies the two-button dialog.
- Decision needed: BUG-205 exit-session cluster

#### `rust/fspec-tui/tests/bug164_mux_retained_on_session_exit.rs`

- Kind: integration test (Rust, crate fspec-tui)
- **exit-session-close-retains-mux-grid** [BUG-205:done] 1 scen: closing_a_session_in_mux_mode_retains_the_mux_and_focuses_the_board_pane
- **rust-mux-mode** [MUX-001:done,BUG-164:done,BUG-300:done,BUG-165:done] 2 scen: closing_a_session_in_mux_mode_retains_the_mux_and_focuses_the_board_pane, cancelling_the_exit_dialog_in_mux_mode_retains_the_mux_and_keeps_the_agent_pane_focused
- Same test credited to 2+ features: closing_a_session_in_mux_mode_retains_the_mux_and_focuses_the_board_pane (exit-session-close-retains-mux-grid, rust-mux-mode)
- **Fix: MERGE (recommended)**
- New file(s): none
- Moves / wiring: The file is named for BUG-164, which tags rust-mux-mode, so rust-mux-mode keeps it. BUG-205 exit-session-close-retains-mux-grid (1 scenario) duplicates the BUG-164 scenario.
- Decision needed: BUG-205 exit-session cluster

#### `rust/fspec-tui/tests/menu008_menu_bar_reorg.rs`

- Kind: integration test (Rust, crate fspec-tui)
- **board-menu-bar-reorganize-zone-a-into-kanban-tools-settings-categories** [MENU-008:done] 15 scen: the_kanban_category_lists_the_work_unit_workflow_entries_in_order, the_tools_category_lists_the_git_tools_in_order, the_settings_category_lists_the_configuration_entries_in_order, the_help_category_is_unchanged, every_registry_entry_has_a_key_hint_label_description_and_a_valid_action, the_bar_row_paints_the_four_zone_a_items, the_ring_walks_across_all_four_items_before_reaching_zone_b, scenario_executing_the_tools_row_opens_the_changed_files_view, scenario_executing_the_settings_providers_row_opens_the_provider_settings_view, focusing_the_kanban_item_opens_a_5_row_dropdown, the_lowercase_p_key_opens_the_provider_settings_view_from_the_board, p_opens_the_provider_settings_view_from_the_focused_board_pane_in_mux_mode, scenario_the_u_help_dialog_lists_all_ten_registry_rows_in_registry_order, tight_width_still_degrades_safely_with_the_four_items
- **menubar-component-2-zone-bar-gui-menu-items-anchored-dropdown-with-view-chip-switcher-menucategories-registry-pure-painter** [MENU-001:done] 3 scen: the_kanban_category_lists_the_work_unit_workflow_entries_in_order, the_tools_category_lists_the_git_tools_in_order, the_settings_category_lists_the_configuration_entries_in_order
- Same test credited to 2+ features: the_kanban_category_lists_the_work_unit_workflow_entries_in_order (board-menu-bar-reorganize-zone-a-into-kanban-tools-settings-categories, menubar-component-2-zone-bar-gui-menu-items-anchored-dropdown-with-view-chip-switcher-menucategories-registry-pure-painter); the_tools_category_lists_the_git_tools_in_order (board-menu-bar-reorganize-zone-a-into-kanban-tools-settings-categories, menubar-component-2-zone-bar-gui-menu-items-anchored-dropdown-with-view-chip-switcher-menucategories-registry-pure-painter); the_settings_category_lists_the_configuration_entries_in_order (board-menu-bar-reorganize-zone-a-into-kanban-tools-settings-categories, menubar-component-2-zone-bar-gui-menu-items-anchored-dropdown-with-view-chip-switcher-menucategories-registry-pure-painter)
- Unmapped (3): the_registry_defines_the_four_categories_in_kanban_tools_settings_help_order, the_p_key_opens_the_provider_settings_view_from_the_board, ctrl_p_does_not_open_the_provider_settings_view
- **Fix: RELINK or DEPRECATE (MENU-001 scenarios)**
- New file(s): none
- Moves / wiring: MENU-001 maps 3 category-registry scenarios onto MENU-008's Kanban/Tools/Settings tests. MENU-001 owns menu001_menu_bar.rs (19 mappings, 1016 lines). Re-link those 3 scenarios to registry tests there if they exist, otherwise rewrite them.
- Notes: MENU-008 reorganised the categories, so MENU-001's category lists are superseded.
- Decision needed: MENU-001's 3 category scenarios: rewrite to the MENU-008 categories, or retire them.

#### `rust/fspec-tui/tests/refac019_contracts.rs`

- Kind: integration test (dir tests/refac019/ for support) (Rust, crate fspec-tui)
- **preserve-contracts-while-decomposing-native-board-integration-surfaces** [REFAC-019:done,BUG-370:done] 4 scen: ordered_public_and_implementation_contracts_retain_their_baseline, backend_consumers_retain_object_safety_and_existing_defaults, decomposed_integration_source_remains_below_the_module_ceiling, mcp_mappings_equal_the_resolved_service_methods_exactly
- **refac019-contracts-compile** [BUG-446:done] 1 scen: mcp_mappings_equal_the_resolved_service_methods_exactly
- Same test credited to 2+ features: mcp_mappings_equal_the_resolved_service_methods_exactly (preserve-contracts-while-decomposing-native-board-integration-surfaces, refac019-contracts-compile)
- **Fix: MERGE (recommended)**
- New file(s): none
- Moves / wiring: BUG-446 refac019-contracts-compile (1 scenario) and REFAC-019's MCP scenario are the same check (mcp_mappings_equal_the_resolved_service_methods_exactly).
- Decision needed: Retire refac019-contracts-compile (BUG-446) into REFAC-019.

#### `rust/fspec-tui/tests/rpc027_dialog_parity_ij.rs`

- Kind: integration test (Rust, crate fspec-tui)
- **rpc027-refactor-invariants** [RPC-027:done] 5 scen: popup_body_rs_is_deleted_from_the_codebase, no_dialog_module_imports_tui_popup_popup, every_refactored_dialog_file_remains_under_300_lines, insta_snapshot_for_help_dialog_is_regenerated_against_the_new_theme, a_new_insta_snapshot_exists_for_every_migrated_dialog
- **ts-reference-files-exist-on-disk-without-git** [BUG-153:done] 1 scen: a_new_insta_snapshot_exists_for_every_migrated_dialog
- Same test credited to 2+ features: a_new_insta_snapshot_exists_for_every_migrated_dialog (rpc027-refactor-invariants, ts-reference-files-exist-on-disk-without-git)
- **Fix: DEPRECATE (recommended)**
- New file(s): none
- Moves / wiring: BUG-153 ts-reference-files-exist-on-disk-without-git checks that Dialog.tsx etc. exist on disk. The TypeScript TUI has been removed, and its scenario borrows RPC-027's a_new_insta_snapshot_exists_for_every_migrated_dialog.
- Decision needed: Retire BUG-153's feature (behaviour removed along with the TypeScript TUI).

#### `rust/fspec-tui/tests/rpc028_popup_scroll.rs`

- Kind: integration test (Rust, crate fspec-tui)
- **double-click-to-resume-session-in-resume-view** [TUI-098:done] 4 scen: rpc028_scroll_viewport_ensure_visible_scrolls_down_past_window, rpc028_wheel_velocity_ramps_up_then_resets_after_gap, tui098_double_click_same_row_resumes_session, tui098_footer_hint_text_indicates_double_click_resumes_session, tui098_two_clicks_over_300ms_are_single_clicks
- **rpc028-scroll-mouse-wrap-parity** [RPC-028:done] 13 scen: rpc028_slash_scroll_offset_advances_past_visible_rows_so_selection_stays_visible, rpc028_slash_down_at_last_match_wraps_to_first, rpc028_slash_up_at_first_match_wraps_to_last_and_scrolls_to_show_it, rpc028_file_mouse_wheel_up_moves_selection_up_one, rpc028_file_mouse_wheel_up_at_first_wraps_to_last, rpc028_resume_left_click_on_row_selects_that_row, rpc028_resume_home_jumps_to_first_session_and_scrolls_to_top, rpc028_search_end_jumps_to_last_match_and_scrolls, rpc028_slash_mouse_wheel_outside_popup_rect_is_ignored, rpc028_thinking_level_mouse_wheel_down_advances_and_wraps, rpc028_scroll_viewport_wrap_index_wraps_both_directions
- Unmapped (23): rpc028_slash_mouse_wheel_inside_popup_rect_moves_selection, rpc028_slash_page_down_jumps_by_visible_rows, rpc028_slash_page_up_jumps_back_by_visible_rows, rpc028_slash_home_jumps_to_first_with_zero_scroll, rpc028_slash_end_jumps_to_last_with_scroll_so_last_row_visible, rpc028_file_mouse_wheel_down_moves_selection_down_one, rpc028_file_scroll_offset_advances_past_visible_rows, rpc028_file_home_and_end_keys, legacy_slash_new_popup_has_full_registry_and_first_selected, legacy_slash_set_filter_narrows_matches, legacy_slash_down_wraps_around, legacy_slash_up_wraps_to_end ...
- **Fix: SPLIT (+ relink drift)**
- New file(s): rust/fspec-tui/tests/tui098_double_click_resume.rs (tui098_double_click_same_row_resumes_session, tui098_footer_hint_text_indicates_double_click_resumes_session, tui098_two_clicks_over_300ms_are_single_clicks)
- Moves / wiring: TUI-098's mappings onto rpc028_scroll_viewport_ensure_visible_scrolls_down_past_window and rpc028_wheel_velocity_ramps_up_then_resets_after_gap are drift. 23 unmapped rpc028 tests stay.

#### `rust/fspec-tui/tests/slash_clear_rpc046.rs`

- Kind: integration test (Rust, crate fspec-tui)
- **rpc074-clear-ts-parity** [RPC-074:done] 2 scen: clear_appends_no_notice_line_on_ok
- **slash-command-clear** [RPC-046:done] 5 scen: clear_resets_local_scrollback_synchronously_for_focused_session, clear_calls_backend_clear_history_for_focused_session, clear_with_no_current_session_is_a_silent_no_op
- Unmapped (2): clear_appends_no_error_line_on_err, clear_only_affects_focused_session_background_untouched
- **Fix: SPLIT**
- New file(s): rust/fspec-tui/tests/slash_clear_ts_parity_rpc074.rs (RPC-074: clear_appends_no_notice_line_on_ok)
- Moves / wiring: Uses tests/common.

#### `rust/fspec-tui/tests/slash_detach_rpc050.rs`

- Kind: integration test (Rust, crate fspec-tui)
- **remove-detach-slash-command-with-attach-path-intact** [BUG-205:done] 3 scen: slash_command_registry_no_longer_offers_detach, typed_detach_falls_through_to_send_input_and_leaves_the_binding_alone, attach_path_still_binds_and_folds_the_work_unit_context
- **slash-command-detach-and-work-unit-binding** [RPC-050:done] 4 scen; 5 stale mapping(s): typed_detach_falls_through_to_send_input_and_leaves_the_binding_alone, attach_path_still_binds_and_folds_the_work_unit_context
- Same test credited to 2+ features: typed_detach_falls_through_to_send_input_and_leaves_the_binding_alone (remove-detach-slash-command-with-attach-path-intact, slash-command-detach-and-work-unit-binding); attach_path_still_binds_and_folds_the_work_unit_context (remove-detach-slash-command-with-attach-path-intact, slash-command-detach-and-work-unit-binding)
- Unmapped (1): parse_slash_command_keeps_detach_as_plain_input
- **Fix: DEPRECATE (recommended)**
- New file(s): none
- Moves / wiring: RPC-050 slash-command-detach-and-work-unit-binding specifies /detach, which BUG-205 removed. All 4 of its mappings are stale (the /detach tests are gone). The file stays with BUG-205 remove-detach-slash-command-with-attach-path-intact.
- Decision needed: Retire RPC-050's /detach feature (COV-076 rule: superseded behaviour is @deprecated).

#### `rust/fspec-tui/tests/tui112_dialog_click_activation.rs`

- Kind: integration test (Rust, crate fspec-tui)
- **exit-session-dialog-click-activation** [BUG-205:done] 2 scen: exit_session_clicking_close_session_commits_it, exit_session_clicking_cancel_dismisses_it
- **left-click-activates-buttons-rows-in-all-enter-selectable-modal-dialogs-new-agent-exit-session-etc** [] 22 scen: start_new_agent_clicking_yes_isolated_commits_it, start_new_agent_clicking_cancel_dismisses_without_creating, start_new_agent_keyboard_behavior_is_unchanged, exit_session_clicking_close_session_commits_it, exit_session_clicking_cancel_dismisses_it, exit_fspec_clicking_exit_commits_quit, exit_fspec_clicking_cancel_stays_on_the_board, merge_confirm_clicking_discard_returns_discard, merge_confirm_clicking_cancel_returns_cancel, session_worktrees_clicking_prune_returns_prune, session_worktrees_clicking_cancel_returns_cancel, confirm_dialog_clicking_the_primary_button_returns_primary, thinking_level_clicking_high_commits_high, mux_layout_clicking_pane_2_applies_with_the_cursor_on_pane_2, mux_layout_clicking_orientation_applies_with_the_cursor_on_orientation, role_dialog_clicking_the_draft_row_saves_the_role, attachment_picker_clicking_notes_pdf_opens_it, work_unit_search_clicking_a_result_row_commits_it, a_click_outside_the_dialog_rect_is_ignored, a_click_in_the_gap_between_two_buttons_does_nothing, a_click_on_the_title_description_or_footer_does_nothing
- Same test credited to 2+ features: exit_session_clicking_close_session_commits_it (exit-session-dialog-click-activation, left-click-activates-buttons-rows-in-all-enter-selectable-modal-dialogs-new-agent-exit-session-etc); exit_session_clicking_cancel_dismisses_it (exit-session-dialog-click-activation, left-click-activates-buttons-rows-in-all-enter-selectable-modal-dialogs-new-agent-exit-session-etc)
- Unmapped (1): wheel_navigation_behavior_is_unchanged
- **Fix: MERGE (recommended)**
- New file(s): none
- Moves / wiring: BUG-205 exit-session-dialog-click-activation (2 scenarios) is a subset of the left-click-activates-buttons feature (22 scenarios, untagged; TUI-112 by file name).
- Decision needed: BUG-205 exit-session cluster

### B5 sessions + providers (12)

#### `rust/providers/src/manager/tests_3.rs`

- Kind: in-crate test part (tests_N.rs) (Rust, crate providers)
- **fix-providermanager-resolution-chain-use-modellimitsresolver** [LIMITS-004:done,BUG-309:done] 1 scen: test_codex_model_gets_context_window_through_napi_params
- **per-model-context-window-and-max-output-configuration** [MODEL-005:done] 1 scen: test_env_var_override_works_when_no_per_model_data
- Unmapped (9): test_claude_model_gets_per_model_context_from_registry, test_context_window_falls_back_to_provider_constant, test_context_window_returns_model_specific_value, test_max_output_tokens_returns_model_specific_value, test_set_model_direct_stores_optional_context_params, test_set_model_direct_without_context_params_leaves_none, test_for_testing_with_custom_context, test_with_provider_and_model_accepts_context_params, test_with_provider_and_model_without_context_falls_back
- **Fix: SPLIT**
- New file(s): move test_env_var_override_works_when_no_per_model_data into the MODEL-005-only rust/providers/src/manager/tests_7.rs (60 lines, unmapped) or a new tests_8.rs
- Moves / wiring: tests_3 keeps LIMITS-004 (test_codex_model_gets_context_window_through_napi_params). Its 9 unmapped tests stay.

#### `rust/providers/src/manager/tests_4.rs`

- Kind: in-crate test part (Rust, crate providers)
- **fix-providermanager-resolution-chain-use-modellimitsresolver** [LIMITS-004:done,BUG-309:done] 2 scen; 2 stale mapping(s): test_claude_context_window_retains_1m_registry_value
- **set-model-direct-stores-provider-prefixed-model-string** [BUG-136:done] 3 scen; 3 stale mapping(s): test_claude_context_window_retains_1m_registry_value
- Same test credited to 2+ features: test_claude_context_window_retains_1m_registry_value (fix-providermanager-resolution-chain-use-modellimitsresolver, set-model-direct-stores-provider-prefixed-model-string)
- Unmapped (10): test_napi_override_takes_priority_over_registry, test_provider_constants_remain_unchanged, test_compaction_threshold_large_context_model, test_compaction_threshold_small_context_model, test_session_set_model_passes_context_params, test_session_set_model_profile_passes_context_params, test_set_model_direct_stores_facade_override, test_set_model_direct_without_facade_leaves_none, test_set_facade_override_setter, test_facade_override_initialized_none_in_all_constructors
- **Fix: RELINK (stale) + SPLIT**
- New file(s): rust/providers/src/manager/tests_8.rs or tests_set_model_direct.rs (BUG-136 tests)
- Moves / wiring: BUG-136's tests_4 mappings (230-250) and LIMITS-004's point at drifted ranges. Re-link by Scenario comment.

#### `rust/providers/src/manager/tests_5.rs`

- Kind: in-crate test part (Rust, crate providers)
- **agentmanager-spawn-fails-for-profile-qualified-openai-fireworks-models-e-g-kimi-k2-6** [BUG-137:done] 2 scen: test_bug136_agent_manager_round_trips_slashed_model_id, test_bug136_set_model_direct_stores_provider_prefixed_model_string
- **fix-providermanager-resolution-chain-use-modellimitsresolver** [LIMITS-004:done,BUG-309:done] 5 scen: test_openai_context_window_passes_through_unclamped, test_codex_no_registry_returns_default, test_user_override_takes_priority_over_registry, test_openai_env_var_override_when_no_registry
- **set-model-direct-stores-provider-prefixed-model-string** [BUG-136:done] 3 scen: test_openai_context_window_passes_through_unclamped, test_codex_no_registry_returns_default, test_user_override_takes_priority_over_registry
- Same test credited to 2+ features: test_openai_context_window_passes_through_unclamped (fix-providermanager-resolution-chain-use-modellimitsresolver, set-model-direct-stores-provider-prefixed-model-string); test_codex_no_registry_returns_default (fix-providermanager-resolution-chain-use-modellimitsresolver, set-model-direct-stores-provider-prefixed-model-string); test_user_override_takes_priority_over_registry (fix-providermanager-resolution-chain-use-modellimitsresolver, set-model-direct-stores-provider-prefixed-model-string)
- Unmapped (3): test_claude_maximum_output_retains_128k_registry_value, test_claude_user_override_retained_without_blanket_ceiling, test_sub_agent_propagation_returns_effective_model_limits
- **Fix: RELINK (drift) + SPLIT**
- New file(s): BUG-136 tests (test_bug136_*) -> the same new BUG-136 file as tests_4; BUG-137 tests -> existing tests_6.rs (BUG-137-only, 212 lines) if they fit under 300 lines
- Moves / wiring: BUG-136's 3 tests_5 scenarios are mapped onto LIMITS-004 context-window tests (drift), and BUG-137 maps test_bug136_*. Settle ownership from the Scenario comments before moving anything. tests_5 then keeps LIMITS-004.
- Notes: tests_3/4/5 are one job: re-link LIMITS-004, BUG-136, BUG-137 and MODEL-005 together.

#### `rust/sessions/src/profile_sections/tests.rs`

- Kind: in-crate test module (Rust, crate sessions)
- **model-selector-cloud-section-assembly** [PROV-127:done] 3 scen: empty_cloud_section_is_dropped, populated_cloud_section_is_kept_unchanged, empty_local_profile_section_is_not_dropped
- **model-selector-profile-server** [RPC-338:done] 4 scen: cloud_provider_has_no_profile_and_is_reachable, reachable_profile_is_a_profile_section, unreachable_profile_without_custom_models_is_marked, failed_probe_with_custom_models_is_not_unreachable
- **Fix: SPLIT**
- New file(s): rust/sessions/src/profile_sections/cloud_section_tests.rs (PROV-127, 3 tests); tests.rs keeps RPC-338 profile-server
- Moves / wiring: Follows persistence_tests.rs in the same dir.

#### `rust/sessions/tests/background_session_shape/api.rs`

- Kind: #[path] part of tests/background_session_shape.rs (Rust, crate sessions)
- **move-background-session-into-codelet-sessions** [RPC-039:done,BUG-298:done] 2 scen: scenario_send_input_is_rewritten_to_a_non_napi_result_type, scenario_codelet_sessions_tests_assert_background_session_is_reachable_from_its_new_home
- **replace-global-chunk-callback-background-session** [RPC-041:done] 1 scen: scenario_background_session_new_accepts_broadcast_senders_as_trailing_parameters
- **Fix: SPLIT**
- New file(s): rust/sessions/tests/background_session_shape/broadcast_senders.rs (RPC-041: scenario_background_session_new_accepts_broadcast_senders_as_trailing_parameters)
- Moves / wiring: Register with `#[path]` in tests/background_session_shape.rs. Parent-level helpers are shared by every part.

#### `rust/sessions/tests/background_session_shape/broadcast.rs`

- Kind: #[path] part (Rust, crate sessions)
- **move-background-session-into-codelet-sessions** [RPC-039:done,BUG-298:done] 1 scen: scenario_handle_output_uses_the_new_chunks_tx_broadcast_and_no_longer_touches_global_chunk_callback
- **replace-global-chunk-callback-background-session** [RPC-041:done] 2 scen: scenario_handle_output_emits_unconditionally_on_chunks_tx, scenario_set_status_emits_on_status_changes_tx
- **Fix: SPLIT**
- New file(s): rust/sessions/tests/background_session_shape/chunk_broadcast.rs (RPC-041: scenario_handle_output_emits_unconditionally_on_chunks_tx, scenario_set_status_emits_on_status_changes_tx)

#### `rust/sessions/tests/background_session_shape/isolation.rs`

- Kind: #[path] part (65 lines) (Rust, crate sessions)
- **move-background-session-into-codelet-sessions** [RPC-039:done,BUG-298:done] 2 scen: scenario_the_moved_background_session_rs_has_no_napi_references, scenario_the_moved_background_session_rs_has_no_crate_persistence_or_crate_types_imports
- **replace-global-chunk-callback-background-session** [RPC-041:done] 1 scen; 2 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **Fix: RELINK (stale)**
- New file(s): rust/sessions/tests/background_session_shape/no_global_chunk_callback.rs (if no current test covers it)
- Moves / wiring: RPC-041 "The GLOBAL_CHUNK_CALLBACK static ... is deleted" is mapped to lines 66-91 of a 65-line file. Find or write its test, then re-link.

#### `rust/sessions/tests/bug251_multimodal_archives.rs`

- Kind: integration test (Rust, crate sessions)
- **faithful-multimodal-archives** [BUG-251:done,BUG-433:done] 6 scen: accepted_images_survive_originals_and_cold_next_request, stored_documents_and_remote_media_retain_supported_source_identities, invalid_or_unsupported_restored_media_fails_visibly, referenced_media_survives_cleanup_and_missing_blobs_fail_visibly, legacy_unicode_text_remains_compatible, tool_image_budgets_do_not_discard_original_attachments
- **watcher-loop-and-input-channel-not-integrated** [WATCH-019:done] 1 scen: accepted_images_survive_originals_and_cold_next_request
- Same test credited to 2+ features: accepted_images_survive_originals_and_cold_next_request (faithful-multimodal-archives, watcher-loop-and-input-channel-not-integrated)
- **Fix: RELINK or DEPRECATE (WATCH-019)**
- New file(s): none
- Moves / wiring: WATCH-019 watcher-loop-and-input-channel-not-integrated maps accepted_images_survive_originals_and_cold_next_request, which is unrelated to watchers. Re-link it to a watcher test (candidate: rust/sessions/tests/cov057_watcher_injection_format.rs, if that is single-feature) or retire it.
- Decision needed: WATCH-019: does the parent watcher_input path still exist (WatcherInput/watcher_inject)? If not, @deprecated.

#### `rust/sessions/tests/cont009_completion_contract_sync.rs`

- Kind: integration test (Rust, crate sessions)
- **completion-contract-dispatch-arming** [CONT-009:done,BUG-228:done] 6 scen: continue_state_syncs_inner_and_arms_registry, goal_alone_arms_registry_and_registers_goal_spec, neither_continue_nor_goal_disarms_registry, new_user_turn_resets_nudge_counter, clearing_chrome_goal_clears_inner_and_registry, unchanged_chrome_goal_is_not_reapplied
- **cont009-completion-contract-sync-tests-fail-because-fresh-session-returns-empty-session-id** [BUG-155:done] 1 scen; 1 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **Fix: SPLIT + DEDUP**
- New file(s): rust/sessions/tests/bug155_fresh_session_seeds_models_cache.rs (assert a session created via the shared helper has a non-empty id)
- Moves / wiring: BUG-155's only scenario is mapped to the fresh_session() helper (lines 17-61), not a test. If both files need the helper, move it to rust/sessions/tests/cont009_support/mod.rs.

#### `rust/sessions/tests/exec_stdin_prompt_p2.rs`

- Kind: integration test (Rust, crate sessions)
- **exec-stdin-prompt** [TOOL-022:done,BUG-456:done] 4 scen: scenario_write_exec_stdin_appends_newline_to_typed_text, scenario_backend_round_trip_surfaces_request_only_while_live_exec_is_quiet, scenario_write_exec_stdin_unknown_session_returns_clean_error, scenario_exec_stdin_overlay_does_not_flip_agent_session_status
- **exec-stdin-request-wire-shape** [TOOL-022:done,BUG-415:done] 1 scen: wire_exec_stdin_request_round_trips_through_serde_json
- **Fix: SPLIT**
- New file(s): rust/sessions/tests/bug415_exec_stdin_request_wire_shape.rs (wire_exec_stdin_request_round_trips_through_serde_json)

#### `rust/sessions/tests/rpc418_compact_session/compact_populated_clears_conversation.rs`

- Kind: #[path] part of tests/rpc418_compact_session.rs (Rust, crate sessions)
- **compression-ratio-clamping** [CMPCT-039:done] 1 scen: compact_populated_reports_real_token_counts
- **honest-compaction-acknowledgement** [RPC-421:done] 1 scen: compact_populated_reports_real_token_counts
- **rust-tui-compact-real-compaction** [RPC-418:done,BUG-432:done] 4 scen: compact_populated_clears_conversation_and_kicks_agent_loop, compact_empty_returns_error_and_leaves_untouched, compact_populated_reports_real_token_counts, compact_unknown_session_id_returns_not_found
- Same test credited to 2+ features: compact_populated_reports_real_token_counts (compression-ratio-clamping, honest-compaction-acknowledgement, rust-tui-compact-real-compaction)
- **Fix: SPLIT + DEDUP (3-way)**
- New file(s): rust/sessions/tests/rpc418_compact_session/compression_ratio_reported.rs (CMPCT-039 scenario, focused test); rust/sessions/tests/rpc418_compact_session/honest_acknowledgement.rs (RPC-421 scenario, focused test)
- Moves / wiring: All three features map compact_populated_reports_real_token_counts. RPC-418 keeps it. CMPCT-039 (which also owns cli/tests/cmpct039_ratio_clamp_test.rs) and RPC-421 each get a focused assertion in a sibling part file that reuses the parent helpers.
- Decision needed: Alternative: re-point the CMPCT-039 scenario into cmpct039_ratio_clamp_test.rs (cli crate). Confirm whether RPC-421 is already covered by single_sourced_compaction_notice_rpc421.rs.

#### `rust/sessions/tests/session_manager_shape/codelet_sessions_tests_assert.rs`

- Kind: #[path] part of tests/session_manager_shape.rs (Rust, crate sessions)
- **move-sessionmanager-from-codelet-napi-into-codelet-sessions** [RPC-040:done] 3 scen: scenario_codelet_sessions_tests_assert_session_manager_is_reachable_from_its_new_home_and_the_hooks_design_is_sound, scenario_session_manager_passes_broadcast_senders_to_background_session
- **replace-global-chunk-callback-napi-shell** [RPC-041:done] 2 scen: scenario_session_manager_passes_broadcast_senders_to_background_session, scenario_session_manager_hooks_trait_no_longer_carries_emit_isolation_state_change
- Same test credited to 2+ features: scenario_session_manager_passes_broadcast_senders_to_background_session (move-sessionmanager-from-codelet-napi-into-codelet-sessions, replace-global-chunk-callback-napi-shell)
- **Fix: SPLIT (+ DEPRECATE one RPC-040 scenario)**
- New file(s): rust/sessions/tests/session_manager_shape/broadcast_senders.rs (RPC-041: scenario_session_manager_passes_broadcast_senders_to_background_session)
- Moves / wiring: RPC-040 maps that test to "The napi side installs its hooks at startup so existing TS behaviour is preserved", and "NAPI TypeScript surface is byte-stable" is mapped to the reachability test. Both are NAPI/TypeScript claims that no longer apply.
- Decision needed: Retire RPC-040's two NAPI/TypeScript scenarios.

### B6 tools + rig-core patches (11)

#### `rust/patches/rig-core/src/agent/prompt_request/streaming.rs`

- Kind: vendored rig-core source with inline tests (line 1130) (Rust, crate patches/rig-core)
- **defense-in-depth-bound-tool-result-text-size-in-rig-patch-parse-tool-result-content** [CMPCT-031:done] 6 scen: test_cmpct_031_200kib_text_replaced_by_truncation_marker, test_cmpct_031_32kib_text_stored_verbatim, test_cmpct_031_exact_bound_stored_verbatim, test_cmpct_031_oversized_image_still_rejected_by_existing_check, test_cmpct_031_truncation_marker_is_valid_utf8_json, test_cmpct_031_verbose_tool_output_is_bounded_before_chat_history
- **defense-in-depth-pdf-image-count-cap-in-rig-patch** [] 4 scen: test_bug_168_pdf_shape_result_exceeding_cap_is_truncated_with_continue_notice, test_bug_168_pdf_shape_result_at_or_under_cap_is_unchanged, test_bug_168_nested_text_variant_obeyes_the_same_cap, test_bug_168_per_image_dimension_defense_remains_independent_of_count_cap
- Unmapped (5): test_span_context_isolation, test_cmpct_031_bound_utf8_boundary_preview_does_not_split_codepoint, test_final_response_end_turn_stop_reason, test_final_response_carries_max_tokens_stop_reason, test_final_response_none_stop_reason
- **Fix: SPLIT**
- New file(s): rust/patches/rig-core/src/agent/prompt_request/streaming/tool_result_text_bound_tests.rs (CMPCT-031, 6); rust/patches/rig-core/src/agent/prompt_request/streaming/pdf_image_cap_tests.rs (BUG-168 tests, 4)
- Moves / wiring: Move with `#[cfg(test)] #[path]`. Shared test helpers in the inline module go to streaming/test_support.rs.
- Notes: defense-in-depth-pdf-image-count-cap-in-rig-patch has no work-unit tag.
- Decision needed: Restructuring the vendored rig-core patch makes the diff against upstream larger. Confirm that is acceptable.

#### `rust/patches/rig-core/src/providers/openai/completion/mod.rs`

- Kind: vendored rig-core source; already 3 per-feature inline modules (prov_081_tests, prov_083_tests, prov_084_tests) (Rust, crate patches/rig-core)
- **openai-provider-base64-image-inputs** [PROV-083:done] 5 scen: prov_083_base64_png_with_detail_none_defaults_to_auto, prov_083_base64_jpeg_with_detail_none_defaults_to_auto, prov_083_base64_png_with_explicit_high_preserves_value, prov_083_url_image_path_unchanged, prov_083_base64_with_media_type_none_still_errors_on_mime
- **openai-provider-reasoning-tokens** [PROV-081:done] 8 scen: prov_081_streaming_vllm_reasoning_field_surfaces_as_reasoning_delta, prov_081_streaming_glm_reasoning_content_field_surfaces_as_reasoning_delta, prov_081_streaming_concatenates_reasoning_when_both_fields_present, prov_081_streaming_content_chunk_passes_through_unchanged, prov_081_non_streaming_vllm_reasoning_field_surfaces_reasoning_and_content, prov_081_non_streaming_without_reasoning_field_still_works, prov_081_outgoing_request_body_never_contains_reasoning_suppression_keys
- **openai-provider-tool-result-images** [PROV-084:done] 4 scen: prov_084_single_base64_image_emits_tool_then_user, prov_084_text_only_tool_result_still_single_message, prov_084_three_images_yield_tool_plus_user_with_three_parts, prov_084_mixed_text_and_image_splits_correctly
- Unmapped (1): prov_081_non_streaming_glm_reasoning_content_field_surfaces_reasoning_and_content
- **Fix: SPLIT (mechanical)**
- New file(s): rust/patches/rig-core/src/providers/openai/completion/prov_081_tests.rs; rust/patches/rig-core/src/providers/openai/completion/prov_083_tests.rs; rust/patches/rig-core/src/providers/openai/completion/prov_084_tests.rs
- Moves / wiring: Turn each inline module into a `#[cfg(test)] mod prov_08x_tests;` file. The code inside does not change.
- Decision needed: Same vendored-patch question as streaming.rs.

#### `rust/patches/rig-core/tests/cmpct065_openai_usage_evidence.rs`

- Kind: integration test (Rust, crate patches/rig-core)
- **builtin-provider-nonstreaming-rejection-evidence** [CMPCT-071:done] 1 scen: nonstreaming_openai_chat_preserves_physical_http_rejection_evidence
- **builtin-provider-openai-usage-evidence** [CMPCT-065:done] 4 scen: streaming_chat_missing_usage_is_rejected, nonstreaming_chat_missing_usage_is_rejected, direct_chat_missing_usage_is_rejected, explicit_measured_zero_remains_valid_at_every_chat_boundary
- **Fix: SPLIT**
- New file(s): rust/patches/rig-core/tests/cmpct071_nonstreaming_rejection_evidence.rs (nonstreaming_openai_chat_preserves_physical_http_rejection_evidence)
- Moves / wiring: Copy the wiremock/usage helpers it needs.

#### `rust/tools/src/agent_manager/tests.rs`

- Kind: in-crate test module (3099 lines, 78 tests) (Rust, crate tools)
- **agent-manager-await-idle** [AMGR-015:done] 10 scen; 1 stale mapping(s): test_await_idle_all_complete, test_await_idle_already_idle, test_await_idle_timeout, test_await_idle_mixed_results, test_await_idle_nonexistent_session, test_await_idle_session_destroyed, test_await_idle_default_timeout, test_await_idle_single_string_vs_array, test_await_idle_interrupted
- **agent-manager-context-resolution** [AMGR-011:done] 9 scen: test_result_serialization_message_delivered, test_delivery_failed_error_helper, test_tool_definition_includes_message_action, test_context_specific_turns, test_context_turn_range, test_context_search_query, test_context_nonexistent_session_degrades, test_context_zero_match_query_degrades, test_context_mixed_array, test_message_without_context_is_plain_delivery, test_context_out_of_range_turns_skipped
- **agent-manager-core** [AMGR-009:done] 11 scen; 1 stale mapping(s): test_spawn_without_role, test_spawn_with_role, test_list_sessions_with_relationships, test_get_status_existing_session, test_get_status_nonexistent_session, test_close_as_spawner, test_close_without_permission, test_spawn_multiple_and_list, test_invalid_action_deserialization, test_concurrent_sessions_isolated
- **agent-manager-messaging** [AMGR-010:done] 11 scen: test_error_helpers, test_message_supervisor_to_subordinate, test_message_subordinate_to_supervisor, test_message_nonexistent_session, test_message_channel_full, test_message_peer_to_peer, test_message_missing_session_id, test_message_missing_message_text, test_message_includes_sender_identity, test_message_sender_without_role, test_message_self_messaging
- **role-management-set-role** [AMGR-012:done] 6 scen: test_tool_definition_includes_context_parameter, test_set_role_on_own_session_no_session_id, test_set_role_on_specific_session_by_id, test_set_role_clear_with_empty_string, test_set_role_result_serialization_cleared, test_get_status_includes_role
- Unmapped (31): test_handler_lifecycle, test_tool_definition_and_construction, test_spawn_action_deserializes, test_spawn_action_with_role_deserializes, test_list_action_deserializes, test_get_status_action_deserializes, test_close_action_deserializes, test_result_serialization_spawned, test_result_serialization_error, test_result_serialization_closed, test_message_action_deserializes, test_context_reference_deserialization ...
- **Fix: SPLIT (5-way)**
- New file(s): rust/tools/src/agent_manager/core_tests.rs (AMGR-009); rust/tools/src/agent_manager/messaging_tests.rs (AMGR-010); rust/tools/src/agent_manager/context_tests.rs (AMGR-011); rust/tools/src/agent_manager/set_role_tests.rs (AMGR-012); rust/tools/src/agent_manager/await_idle_tests.rs (AMGR-015); rust/tools/src/agent_manager/test_support.rs (mock manager/session fixtures)
- Moves / wiring: Assign tests by Scenario comment, not by the line ranges: several mappings have drifted (for example set-role maps test_tool_definition_includes_context_parameter, and context-resolution maps messaging serialisation tests). Each new file must stay under 300 lines, so the larger features may need _2 parts. 31 unmapped tests go with whichever feature their Scenario comment names, or to core_tests.rs.

#### `rust/tools/src/bridge_relay.rs`

- Kind: source file with inline tests (line 1163) (Rust, crate tools)
- **bridge-command-handling-pipeline** [BRIDGE-017:done] 6 scen; 4 stale mapping(s): test_injected_input_text_only, test_injected_input_with_images, test_injected_input_with_empty_images, test_get_instance_metadata_has_name, test_process_outbound_envelope_regular_chunk, test_process_outbound_envelope_skip_fspec_request
- **bridge-command-message-types** [BRIDGE-016:done] 4 scen; 4 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **telegram-stream-control-channel** [BRIDGE-008:done] 4 scen; 4 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- Unmapped (12): test_process_outbound_envelope_command_response, test_handle_multiplexed_inbound_input, test_handle_multiplexed_inbound_ping, test_handle_inbound_session_create_invokes_creator_and_responds, test_handle_inbound_terminal_create_spawns_pty_and_responds, test_sess018_session_input_routes_by_envelope_session_id, test_sess018_session_input_without_context_falls_back_to_parameter_injector, test_sess018_session_control_routes_by_envelope_session_id, test_sess018_terminal_input_writes_to_pty, test_sess018_terminal_resize_updates_pty_size, test_sess018_terminal_destroy_removes_and_responds, test_sess018_terminal_create_spawns_reader_emitting_data_envelopes
- **Fix: RELINK (stale) + SPLIT**
- New file(s): rust/tools/src/bridge_relay/command_pipeline_tests.rs (BRIDGE-017); rust/tools/src/bridge_relay/message_types_tests.rs (BRIDGE-016); rust/tools/src/bridge_relay/stream_control_tests.rs (BRIDGE-008)
- Moves / wiring: 12 of the 14 mappings point at drifted ranges (control-message, serde and command-pipeline tests have moved). Find each scenario's current test in bridge_relay.rs, bridge_integration_tests.rs or tests/bridge_handler_*.rs. Write any that are missing, then move per feature.
- Notes: bridge_relay.rs's header names bridge-relay-multiplexed-wiring, which is not a mapper of this file.

#### `rust/tools/src/deep_search/tests.rs`

- Kind: in-crate test module (Rust, crate tools)
- **deep-search-model-inheritance** [BUG-102:done] 3 scen: test_handler_captures_provider_and_model_claude, test_handler_captures_provider_and_model_openai, test_provider_manager_uses_with_provider_and_model
- **deep-search** [RLM-001:done] 17 scen: test_tool_trait_name_and_args, test_tool_definition_schema, test_session_search_handler_lifecycle, test_handler_dispatch_with_scope, test_handler_dispatch_error_propagation, test_narrow_scope_restricts_sub_agent, test_max_depth_passed_to_handler, test_default_max_depth_constant, test_scope_reflected_in_system_prompt, test_handler_dispatch_without_scope, test_system_prompt_with_code_scope, test_system_prompt_without_code_scope, test_missing_query_returns_deserialization_error, test_empty_query_returns_error_at_call_time, test_scope_is_optional, test_tool_stores_parent_session_id, test_read_only_tools_list, test_handler_dispatch_no_handler_returns_error, test_tool_construction_for_provider_wiring
- Unmapped (5): test_system_prompt_with_multiple_scopes, test_handler_registry_set_and_check, test_handler_registry_clear_all, test_handler_captures_provider_and_model_codex, test_handler_captures_provider_and_model_zai
- **Fix: SPLIT**
- New file(s): rust/tools/src/deep_search/model_inheritance_tests.rs (BUG-102, 3 tests; its 2 unmapped scenarios need tests or relink)
- Moves / wiring: Follows recursive_tests.rs in the same dir.

#### `rust/tools/src/facade/codex.rs`

- Kind: source file with inline tests (line 892, 85 tests) (Rust, crate tools)
- **codex-grep-files-include-limit** [BUG-111:done] 10 scen: test_codex_grep_files_facade_maps_include_param, test_codex_grep_files_facade_maps_limit_param, test_codex_grep_files_facade_maps_include_and_limit, test_codex_grep_files_facade_backward_compatible, test_internal_search_params_grep_has_include_and_limit_fields, test_wrapper_grep_args_construction_with_include, test_wrapper_grep_args_construction_with_limit, test_grep_args_supports_optional_glob_field, test_zai_grep_facade_uses_none_defaults_for_include_and_limit, test_gemini_grep_facade_uses_none_defaults_for_include_and_limit
- **codex-list-dir-pagination** [BUG-110:done] 5 scen: test_codex_list_dir_maps_offset_and_limit, test_codex_list_dir_backward_compatible_no_pagination, test_codex_list_dir_schema_has_pagination_params, test_codex_list_dir_maps_depth, test_zai_facade_provides_none_for_pagination_params
- **codex-native-tool-facades** [BUG-107:done,TOOL-015:done] 8 scen: test_codex_shell_command_facade, test_codex_read_file_facade_with_offset_limit, test_codex_list_dir_facade_with_dir_path, test_codex_grep_files_facade, test_codex_shell_command_missing_command, test_codex_read_file_facade_without_offset_limit, test_all_codex_schemas_have_additional_properties_false, test_codex_does_not_expose_non_native_glob
- **codex-read-file-mode-indentation** [BUG-109:done] 6 scen: test_codex_read_file_maps_mode_indentation, test_codex_read_file_maps_mode_slice, test_codex_read_file_backward_compatible_no_mode_no_indentation, test_codex_read_file_schema_has_mode_and_indentation, test_zai_facade_provides_none_mode_and_indentation, test_codex_read_file_extracts_all_indentation_fields
- **codex-shell-command-params** [BUG-108:done] 7 scen: test_codex_shell_command_maps_workdir_to_cwd, test_codex_shell_command_maps_timeout_ms, test_codex_shell_command_maps_both_workdir_and_timeout, test_codex_shell_command_without_optional_params, test_codex_shell_command_schema_has_approval_params, test_codex_shell_command_approval_params_ignored, test_gemini_facade_backward_compatible_with_new_fields
- **codex-shell-exec-facades** [BUG-114:done] 12 scen: test_codex_shell_facade_maps_command_array, test_codex_shell_facade_converts_timeout, test_codex_shell_facade_defaults_to_none, test_codex_shell_facade_missing_command, test_codex_shell_facade_schema, test_codex_exec_command_facade_maps_pty, test_codex_exec_command_facade_defaults_tty_false, test_codex_exec_command_facade_all_params, test_codex_exec_command_facade_missing_cmd, test_codex_exec_command_facade_ignores_approval_params, test_codex_exec_command_facade_schema, test_both_facades_tool_names_and_schemas
- **codex-view-image** [BUG-112:done] 5 scen: test_codex_view_image_schema_matches_codex_spec, test_codex_view_image_facade_maps_path_to_read, test_codex_view_image_facade_accepts_detail_param, test_codex_view_image_facade_missing_path, test_codex_view_image_facade_empty_path
- **codex-write-stdin-facade** [BUG-115:done] 9 scen: test_codex_shell_facade_schema, test_codex_exec_command_facade_maps_pty, test_codex_exec_command_facade_defaults_tty_false, test_codex_exec_command_facade_all_params, test_codex_exec_command_facade_missing_cmd, test_codex_write_stdin_validates_required_session_id, test_codex_write_stdin_rejects_null_session_id, test_codex_write_stdin_schema, test_codex_write_stdin_facade_tool_name_and_schema
- Same test credited to 2+ features: test_codex_shell_facade_schema (codex-shell-exec-facades, codex-write-stdin-facade); test_codex_exec_command_facade_maps_pty (codex-shell-exec-facades, codex-write-stdin-facade); test_codex_exec_command_facade_defaults_tty_false (codex-shell-exec-facades, codex-write-stdin-facade); test_codex_exec_command_facade_all_params (codex-shell-exec-facades, codex-write-stdin-facade); test_codex_exec_command_facade_missing_cmd (codex-shell-exec-facades, codex-write-stdin-facade)
- Unmapped (28): test_codex_shell_command_empty_command, test_codex_read_file_missing_path, test_codex_read_file_empty_path, test_codex_list_dir_facade_with_empty_dir_path, test_codex_list_dir_facade_with_null_dir_path, test_codex_list_dir_uses_dir_path_not_path, test_codex_grep_files_facade_no_path, test_codex_grep_files_facade_missing_pattern, test_codex_tools_use_correct_names, test_codex_tools_provider_name, test_codex_list_dir_schema_uses_dir_path_not_path, test_codex_grep_files_schema_has_include_param ...
- **Fix: SPLIT (8-way) + RELINK**
- New file(s): rust/tools/src/facade/codex/native_tool_tests.rs (BUG-107/TOOL-015); rust/tools/src/facade/codex/shell_command_params_tests.rs (BUG-108); rust/tools/src/facade/codex/read_file_mode_tests.rs (BUG-109); rust/tools/src/facade/codex/list_dir_pagination_tests.rs (BUG-110); rust/tools/src/facade/codex/grep_files_tests.rs (BUG-111); rust/tools/src/facade/codex/view_image_tests.rs (BUG-112); rust/tools/src/facade/codex/shell_exec_tests.rs (BUG-114); rust/tools/src/facade/codex/write_stdin_tests.rs (BUG-115)
- Moves / wiring: BUG-115's first 5 scenarios (CodexWriteStdinFacade maps chars / Poll / numeric session_id) are mapped to BUG-114 exec_command tests (drift). Their real tests are probably among the 28 unmapped tests, so re-link before splitting. Some BUG-108/109/111 scenarios map zai/gemini facade tests, which stay with the codex feature that specified them.

#### `rust/tools/src/facade/wrapper.rs`

- Kind: source file with inline tests (lines 299, 2164) (Rust, crate tools)
- **codex-request-user-input-facade** [BUG-116:done] 7 scen: test_hitl_wrapper_returns_answers, test_hitl_wrapper_headless_mode_error, test_hitl_wrapper_cancellation_to_error, test_hitl_wrapper_validation_error, test_hitl_wrapper_tool_name_and_definition, test_hitl_wrapper_freeform_only
- **require-session-id-for-all-tools-to-support-worktree-isolation** [TOOL-014:done,BUG-456:done] 29 scen: test_validate_and_resolve_relative_to_worktree, test_validate_and_resolve_path_resolves_relative_to_worktree, test_edit_tool_resolves_to_worktree, test_grep_tool_resolves_to_worktree, test_glob_tool_resolves_to_worktree, test_validate_and_resolve_dot_to_worktree, test_astgrep_tool_resolves_to_worktree, test_astgrep_refactor_tool_resolves_to_worktree, test_bash_tool_runs_in_the_session_worktree, test_validate_and_resolve_path_rejects_absolute_outside_worktree, test_validate_and_resolve_path_rejects_write_outside_worktree, test_validate_and_resolve_path_rejects_edit_outside_worktree, test_validate_and_resolve_path_rejects_grep_outside_worktree, test_validate_and_resolve_path_rejects_glob_outside_worktree, test_validate_and_resolve_path_rejects_ls_outside_worktree, test_validate_and_resolve_path_rejects_astgrep_outside_worktree, test_validate_and_resolve_path_rejects_astgrep_refactor_outside_worktree, test_validate_and_resolve_path_no_worktree, test_read_tool_requires_session_id, test_write_tool_requires_session_id, test_edit_tool_requires_session_id, test_grep_tool_requires_session_id, test_glob_tool_requires_session_id, test_ls_tool_requires_session_id, test_ast_grep_tool_requires_session_id, test_ast_grep_refactor_tool_requires_session_id, test_web_search_tool_requires_session_id, test_search_tool_facade_wrapper_requires_session_id, test_ls_tool_facade_wrapper_requires_session_id
- Unmapped (9): test_wrapper_returns_facade_tool_name, test_wrapper_returns_facade_tool_name_web_fetch, test_wrapper_returns_flat_schema_for_gemini, test_file_wrapper_returns_facade_tool_name, test_file_wrapper_returns_flat_schema, test_bridge_wrapper_uses_session_id_from_construction, test_bridge_wrapper_fails_without_handler, test_validate_and_resolve_path_allows_all_paths_with_nil_session, test_write_tool_resolves_to_worktree
- **Fix: SPLIT**
- New file(s): rust/tools/src/facade/wrapper/request_user_input_tests.rs (BUG-116, 6 tests); rust/tools/src/facade/wrapper/worktree_isolation_tests.rs (TOOL-014/BUG-456, 29 tests; split into _2 if > 300 lines)
- Moves / wiring: 9 unmapped tests stay inline or go to wrapper/tests.rs.

#### `rust/tools/src/fspec_workflow_guidance.rs`

- Kind: source file with inline tests (line 1187) (Rust, crate tools)
- **fspec-workflow-guidance-named-keys** [TOOL-020:done] 10 scen: test_guidance_does_not_use_positional_args_for_update_work_unit_status, test_guidance_does_not_use_positional_args_for_show_work_unit, test_guidance_does_not_use_positional_args_for_create_story, test_guidance_does_not_use_positional_args_for_add_rule, test_guidance_does_not_use_positional_args_for_add_dependency, test_guidance_does_not_use_positional_args_for_set_user_story, test_guidance_does_not_use_positional_args_for_add_attachment, test_guidance_does_not_use_positional_args_for_link_coverage, test_guidance_has_no_positional_args_patterns
- **harness-workflow-guidance-references-astgrep-tool** [CLI-015:done] 1 scen: test_harness_guidance_references_astgrep_not_research_ast
- Unmapped (27): Feature: User Authentication, Scenario: Login with valid credentials, should redirect to dashboard, test_guidance_contains_acdd_workflow, test_guidance_contains_fspec_tool_instruction, test_guidance_is_wrapped_in_system_reminder, test_guidance_contains_example_tool_usage, test_guidance_contains_foundation_discovery, test_guidance_contains_event_storm, test_guidance_contains_coverage_tracking, test_guidance_contains_file_naming_rules, test_guidance_contains_reverse_acdd ...
- **Fix: SPLIT**
- New file(s): rust/tools/src/fspec_workflow_guidance/named_keys_tests.rs (TOOL-020, 9 tests); rust/tools/src/fspec_workflow_guidance/astgrep_reference_tests.rs (CLI-015, 1 test)
- Moves / wiring: 27 unmapped tests stay inline.

#### `rust/tools/src/unified_exec/exec_stdin/tests.rs`

- Kind: in-crate test module (mod tests; in exec_stdin.rs) (Rust, crate tools)
- **exec-stdin-clock-and-emit-errors** [BUG-387:done] 3 scen: a_request_timestamp_before_the_epoch_is_an_error, the_refire_cooldown_runs_on_the_monotonic_clock, emitting_to_a_session_without_a_callback_is_an_error
- **exec-stdin-detector-fault-reporting** [BUG-376:done] 2 scen: the_refire_cooldown_keeps_working_after_its_lock_was_poisoned, a_request_for_a_session_without_a_callback_is_reported
- **exec-stdin-prompt** [TOOL-022:done,BUG-456:done] 1 scen: test_cooldown_state_machine
- Unmapped (3): test_emit_with_callback_and_clear, test_session_isolation, test_reset_cooldown_allows_immediate_refire
- **Fix: SPLIT**
- New file(s): rust/tools/src/unified_exec/exec_stdin/clock_and_emit_tests.rs (BUG-387, 3); rust/tools/src/unified_exec/exec_stdin/fault_reporting_tests.rs (BUG-376, 2); tests.rs keeps exec-stdin-prompt (test_cooldown_state_machine)

#### `rust/tools/tests/tool_facade_test.rs`

- Kind: integration test (789 lines, 6 features) (Rust, crate tools)
- **bash-facade** [TOOL-004:done] 3 scen: test_map_gemini_run_shell_command_parameters_to_internal_format, test_gemini_run_shell_command_facade_provides_flat_schema, test_bash_facade_wrapper_integrates_with_rig
- **directory-listing-facade** [TOOL-006:done] 4 scen: test_map_gemini_list_directory_parameters_with_path, test_map_gemini_list_directory_with_empty_parameters, test_gemini_list_directory_facade_provides_flat_schema, test_ls_tool_facade_wrapper_integrates_with_rig
- **facadetoolwrapper-for-rig-integration** [TOOL-002:done] 3 scen: test_facade_wrapper_integrates_with_rig_tool_trait, test_facade_wrapper_returns_definition_with_flat_schema
- **file-operation-facades** [TOOL-003:done] 7 scen: test_facade_wrapper_web_fetch_integrates_with_rig, test_map_gemini_read_file_parameters_to_internal_format, test_map_gemini_write_file_parameters_to_internal_format, test_gemini_replace_facade_provides_flat_schema, test_map_gemini_replace_parameters_to_internal_format, test_gemini_read_file_facade_provides_flat_schema
- **provider-specific-tool-facades** [TOOL-001:specifying] 6 scen: test_map_claude_web_search_parameters_to_internal_format, test_map_gemini_google_web_search_parameters_to_internal_format, test_registry_returns_only_facades_for_requested_provider, test_gemini_facade_provides_flat_json_schema_without_oneof
- **search-facades** [TOOL-005:done] 6 scen: test_bash_facade_wrapper_integrates_with_rig, test_map_gemini_search_file_content_parameters_to_internal_format, test_gemini_search_file_content_facade_provides_flat_schema, test_map_gemini_glob_parameters_to_internal_format, test_gemini_glob_facade_provides_flat_schema, test_search_facade_wrapper_integrates_with_rig_for_search_file_content
- Same test credited to 2+ features: test_bash_facade_wrapper_integrates_with_rig (bash-facade, search-facades)
- Unmapped (6): test_map_gemini_web_fetch_url_to_internal_open_page_format, test_claude_facade_provides_flat_schema_with_action_type_enum, test_gemini_write_file_facade_provides_flat_schema, test_facade_wrapper_read_file_integrates_with_rig, test_file_facades_available_for_gemini_provider, test_search_facade_wrapper_integrates_with_rig_for_glob
- **Fix: SPLIT (6-way)**
- New file(s): rust/tools/tests/tool002_facade_wrapper_rig.rs (TOOL-002); rust/tools/tests/tool003_file_operation_facades.rs (TOOL-003); rust/tools/tests/tool004_bash_facade.rs (TOOL-004); rust/tools/tests/tool005_search_facades.rs (TOOL-005); rust/tools/tests/tool006_directory_listing_facade.rs (TOOL-006); tool_facade_test.rs keeps TOOL-001 provider-specific-tool-facades
- Moves / wiring: search-facades' mappings are shifted one test up (it maps test_bash_facade_wrapper_integrates_with_rig). Re-link by Scenario comment. TOOL-001 is still in "specifying" status, so moving its card is refused until its other gates pass. This does not block the split.

### B7 cli + core + fspec-core + graph (13)

#### `rust/cli/src/compaction_threshold.rs`

- Kind: source file; its tests moved to compaction_threshold/{capacity,configuration,catalog}_tests.rs (Rust, crate cli)
- **compaction-threshold-clamped-inputs** [LIMITS-005:done] 7 scen; 14 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **optimized-compaction-trigger** [CTX-002:done] 12 scen; 24 stale mapping(s): (no test function: mapping points at non-test lines or past EOF)
- **Fix: RELINK (stale)**
- New file(s): none expected: re-link to rust/cli/src/compaction_threshold/capacity_tests.rs + configuration_tests.rs (CTX-002) and catalog_tests.rs (LIMITS-005)
- Moves / wiring: All 19 mappings point past the end of the 131-line file. The three sibling test files are mapped by no feature and have no @step comments, so add @step comments as you re-link. If one sibling ends up with tests for both features, move tests between them so each sibling stays single-feature.
- Notes: Some LIMITS-005 scenarios ("Sub-agent propagation uses effective context window", "Summarization budget uses clamped context window") may be tested elsewhere or not at all. Check before re-linking.

#### `rust/cli/src/interactive/error_classifiers.rs`

- Kind: source file with inline #[cfg(test)] tests (line 300) (Rust, crate cli)
- **promptcancelled-error-chain-preservation-during-compaction** [BUG-144:done] 3 scen: detects_streaming_error_wrapped_prompt_cancelled, does_not_match_bare_string, error_from_preserves_downcast_for_extract_prompt_cancelled
- **replace-stringly-typed-is-compaction-cancelled-with-structural-prompterror-downcast** [CMPCT-025:done] 5 scen: structurally_detects_prompt_cancelled, detects_wrapped_prompt_cancelled, stall_timeout_error_not_caught_by_truncation_classifier, stall_timeout_error_not_caught_by_prompt_too_long_classifier, does_not_match_bare_string, does_not_match_other_prompt_errors
- Same test credited to 2+ features: does_not_match_bare_string (promptcancelled-error-chain-preservation-during-compaction, replace-stringly-typed-is-compaction-cancelled-with-structural-prompterror-downcast)
- Unmapped (5): stall_timeout_error_is_identified_by_dedicated_classifier, stall_timeout_error_not_caught_by_network_classifier, stall_timeout_error_not_caught_by_image_content_classifier, network_errors_are_not_classified_as_stall, stall_classifier_does_not_match_unrelated_errors
- **Fix: MERGE (recommended), else SPLIT + DEDUP**
- New file(s): rust/cli/src/interactive/error_classifiers/prompt_cancelled_tests.rs (all PromptCancelled tests, one feature); stall-timeout tests stay inline (unmapped)
- Moves / wiring: Move the PromptCancelled tests out with `#[cfg(test)] #[path]`. If no merge: put BUG-144 tests (detects_streaming_error_wrapped_prompt_cancelled, error_from_preserves_downcast_for_extract_prompt_cancelled) in error_classifiers/prompt_cancelled_chain_tests.rs, and CMPCT-025 tests (structurally_detects_prompt_cancelled, detects_wrapped_prompt_cancelled, does_not_match_bare_string, does_not_match_other_prompt_errors) in error_classifiers/prompt_cancelled_downcast_tests.rs.
- Notes: CMPCT-025 "StreamingError::Prompt-wrapped PromptCancelled is detected" is mapped to stall-timeout tests (drift); its real test is the one BUG-144 owns. The "bare string" scenarios are word-for-word duplicates.
- Decision needed: Merge BUG-144 promptcancelled-error-chain-preservation into CMPCT-025 (one behaviour: structural PromptCancelled detection through Error::from / .context() / StreamingError).

#### `rust/cli/tests/auto_continue_engine_test.rs`

- Kind: integration test (Rust, crate cli)
- **auto-continue-engine** [CONT-002:done] 11 scen: session_auto_continue_state_defaults_and_per_user_turn_reset, off_mode_finishes_exactly_as_today_when_the_model_stops, armed_stop_without_done_produces_a_counted_nudge, accepted_done_finishes_the_turn_and_surfaces_the_summary, budget_exhaustion_finishes_with_a_visible_warning, user_interrupt_always_wins_over_nudging, truncation_recovery_takes_precedence_over_auto_continue, a_nudge_followed_by_tool_activity_is_refunded, done_tool_records_a_session_scoped_tier0_acceptance, done_is_registered_only_while_auto_continue_is_armed, stream_loop_nudges_only_at_the_clean_final_response_settle_point
- **completion-registration-layout-guards** [CONT-011:done] 1 scen: done_is_registered_only_while_auto_continue_is_armed
- Same test credited to 2+ features: done_is_registered_only_while_auto_continue_is_armed (auto-continue-engine, completion-registration-layout-guards)
- **Fix: SPLIT + DEDUP (CONT-011 guard)**
- New file(s): rust/cli/tests/cont011_completion_registration_layout_guards.rs (new source-shape guard: done registered conditionally in all 7 provider builders, never in DeepSearch)
- Moves / wiring: auto_continue_engine_test.rs keeps done_is_registered_only_while_auto_continue_is_armed for CONT-002. The guard feature gets its own source-scan test, as its description says ("These guards are supplemental").
- Decision needed: CONT-011 guard features (3): give each its own guard test file (recommended), or fold each 1-scenario guard into its parent feature.

#### `rust/cli/tests/cmpct059_runtime_commit.rs`

- Kind: integration test (Rust, crate cli)
- **inject-summary-handler** [CMPCT-009:done] 1 scen: runtime_installation_reconciles_only_the_durable_promoted_generation
- **runtime-durable-dag-installation** [CMPCT-059:done] 4 scen: runtime_installation_reconciles_only_the_durable_promoted_generation, runtime_publication_failure_cannot_authorize_ordinary_inference, runtime_rejects_a_candidate_that_drops_authoritative_file_identity, runtime_assembles_missing_file_identity_and_retains_mandatory_reminders
- Same test credited to 2+ features: runtime_installation_reconciles_only_the_durable_promoted_generation (inject-summary-handler, runtime-durable-dag-installation)
- **Fix: REHOME or DEPRECATE (inject-summary-handler)**
- New file(s): none
- Moves / wiring: inject-summary-handler's "Partition, clear, restore, and inject DAG into session messages" borrows runtime_installation_reconciles_only_the_durable_promoted_generation.
- Notes: This is the legacy success path that the fail-closed handler dropped.
- Decision needed: inject-summary-handler cluster

#### `rust/cli/tests/cont006_goal_immediate_termination.rs`

- Kind: integration test (Rust, crate cli)
- **completion-goal-teardown-layout-guards** [CONT-011:done] 1 scen: stream_loop_wiring_pins_goal_mode_early_exit_and_single_teardown_invariants
- **goal-immediate-termination** [CONT-006:done] 7 scen: accepted_goal_mode_done_terminates_the_turn_at_the_tool_result_arm_with_atomic_teardown, immediate_termination_prevents_repeat_verify_runs_and_acceptance_overwrite, tier_1_rejected_done_records_a_rejection_and_never_exits_early, failing_tier_2_verify_rejects_done_without_early_exit, settle_point_escalation_semantics_are_unchanged, stream_loop_wiring_pins_goal_mode_early_exit_and_single_teardown_invariants, verify_command_exceeding_the_timeout_rejects_done_without_early_exit
- Same test credited to 2+ features: stream_loop_wiring_pins_goal_mode_early_exit_and_single_teardown_invariants (completion-goal-teardown-layout-guards, goal-immediate-termination)
- **Fix: SPLIT + DEDUP (CONT-011 guard)**
- New file(s): rust/cli/tests/cont011_completion_goal_teardown_layout_guards.rs (source scan of interactive/stream_loop.rs + session reminders, per the scenario)
- Moves / wiring: cont006 keeps stream_loop_wiring_pins_goal_mode_early_exit_and_single_teardown_invariants for CONT-006.
- Decision needed: CONT-011 guard features

#### `rust/cli/tests/image_content_recovery_test.rs`

- Kind: integration test (Rust, crate cli)
- **image-content-recovery** [EXT-016:done] 2 scen: test_session_recovers_from_image_dimension_400_error, test_session_survives_unknown_content_400_error
- **image-dimension-validation** [EXT-016:done] 1 scen: test_build_user_content_rejects_oversized_jpeg_bridge_image
- Unmapped (12): test_is_image_content_error_detects_dimension_errors, test_is_image_content_error_detects_image_size_errors, test_is_image_content_error_false_positives, test_sanitize_replaces_image_in_user_message, test_sanitize_no_images_returns_false, test_sanitize_preserves_text_messages, test_sanitize_preserves_call_id_on_tool_result, test_build_user_content_rejects_oversized_png_bridge_image, test_build_user_content_allows_normal_bridge_image, test_build_user_content_mixed_oversized_and_normal_images, test_build_user_content_no_images, test_sanitize_preserves_none_call_id_on_tool_result
- **Fix: SPLIT**
- New file(s): rust/cli/tests/ext016_image_dimension_validation.rs (test_build_user_content_rejects_oversized_jpeg_bridge_image)
- Moves / wiring: Both features are EXT-016. image-dimension-validation already maps rust/tools/tests/read_image_dimension_validation_test.rs, but build_user_content is in the cli crate, so the new file goes in cli. Move any image-fixture helpers it uses (oversized JPEG builder) with it, or share them via rust/test-helpers.
- Notes: 12 unmapped classifier/sanitize tests stay with image-content-recovery.

#### `rust/core/src/compaction/__tests__/dag_node_proptest.test.rs`

- Kind: in-crate test module (registered in core/src/compaction/mod.rs:72) (Rust, crate core)
- **parse-dag-nodes-turn-range-validation** [CMPCT-035:done] 5 scen: cmpct_035_reversed_turn_range_is_rejected, cmpct_035_forward_turn_range_is_parsed_unchanged, cmpct_035_equal_start_and_end_is_accepted, cmpct_035_mixed_input_keeps_only_well_formed_nodes, cmpct_035_proptest_every_parsed_node_has_start_le_end
- **prevent-clamping-from-inverting-turn-ranges-in-parse-dag-nodes-fv-003-c** [CMPCT-037:done] 7 scen: cmpct_037_turn_start_beyond_message_count_drops_node_with_warning, cmpct_037_turn_end_above_message_count_is_clamped_when_start_in_range, cmpct_037_turn_start_equal_to_message_count_minus_one_is_accepted, cmpct_037_turn_start_equal_to_message_count_drops_node, cmpct_037_mixed_input_drops_only_out_of_range_node, cmpct_037_message_count_zero_rejects_every_node, cmpct_037_proptest_clamping_preserves_invariants
- **reject-overlapping-same-depth-turn-ranges-in-parse-dag-nodes-fv-003-b** [CMPCT-036:done] 7 scen: cmpct_036_same_depth_overlap_drops_later_node_and_warns, cmpct_036_disjoint_same_depth_ranges_are_both_kept, cmpct_036_boundary_touch_counts_as_overlap, cmpct_036_cross_depth_coverage_is_accepted, cmpct_036_containment_drops_inner_and_preserves_disjoint_neighbour, cmpct_036_empty_and_singleton_inputs_are_unaffected, cmpct_036_proptest_same_depth_pairs_are_disjoint
- Unmapped (8): parser_output_sorted_by_turn_start, parser_output_depth_bounded, parser_output_labels_nonempty, parser_clamps_turn_end_to_message_count, parser_empty_in_empty_out, parser_idempotent_roundtrip, parser_output_count_bounded_by_input_blocks, parser_preserves_start_le_end_when_input_well_formed
- **Fix: SPLIT**
- New file(s): rust/core/src/compaction/__tests__/dag_node_overlap.test.rs (CMPCT-036, 7 tests); rust/core/src/compaction/__tests__/dag_node_clamping.test.rs (CMPCT-037, 7 tests)
- Moves / wiring: Register each like the existing `mod dag_node_proptest { ... }` block in compaction/mod.rs. Shared proptest strategies/builders go to __tests__/dag_node_test_support.rs. dag_node_proptest keeps CMPCT-035 and the unmapped generic parser properties.

#### `rust/core/tests/git_state_watcher.rs`

- Kind: integration test (Rust, crate core)
- **git-state-watcher-core** [BUG-182:done] 3 scen: manual_checkpoint_creation_publishes_a_git_state_frame_with_incremented_counts_on_the_fs_watch, the_periodic_poll_catches_a_working_tree_change_that_emits_no_git_event, git_state_watcher_dedups_unchanged_snapshots_across_poll_ticks
- **git-state-watcher-dedup-signature** [] 4 scen: steady_state_silence_in_a_clean_repo, steady_state_silence_in_a_checkpointed_repo, a_new_checkpoint_ref_produces_a_frame_on_the_checkpointed_repo, two_captures_of_an_unchanged_checkpointed_repo_differ_raw_but_match_in_signature
- **Fix: SPLIT**
- New file(s): rust/core/tests/git_state_watcher_dedup_signature.rs (4 tests); rust/core/tests/git_state_watcher_support/mod.rs (repo/checkpoint fixtures used by both)
- Notes: git-state-watcher-dedup-signature has no work-unit tag (the BUG-188 regression lives in it). Tag it or attach it to COV-075 when it is next moved.

#### `rust/fspec-core/src/io/gherkin_format.rs`

- Kind: source file with inline #[cfg(test)] mod tests (line 733) (Rust, crate fspec-core)
- **gherkin-background-formatting** [BUG-157:done] 4 scen: bug157_background_prose_does_not_duplicate_scenarios, bug157_background_prose_formatting_is_idempotent, bug157_background_with_steps_is_formatted, bug157_rule_with_background_is_formatted_without_duplication
- **gherkin-description-formatting** [RPC-330:done] 6 scen; 1 stale mapping(s): rpc330_preserves_blank_line_between_feature_description_paragraphs, rpc330_preserves_blank_line_between_scenario_description_paragraphs, rpc330_single_paragraph_description_is_unchanged, rpc330_collapses_more_than_two_blank_lines_to_two, rpc330_multi_paragraph_description_is_idempotent
- **gherkin-stepless-scenario-formatting** [BUG-158:done] 4 scen: bug158_stepless_scenario_does_not_duplicate_scenarios, bug158_stepless_scenario_formatting_is_idempotent, bug158_lowercase_step_keywords_are_formatted_without_duplication, bug158_rule_with_stepless_scenario_is_formatted_without_duplication
- Unmapped (16): renormalises_step_indentation_to_four_spaces, single_trailing_newline, idempotent, preserves_tags, aligns_table_columns, blank_line_before_each_child, docstring_has_no_spurious_blank_lines, docstring_is_idempotent, docstring_media_type_preserved_on_opening_line, rpc330_step_docstring_with_internal_blank_line_is_not_regressed, preserves_comment_before_feature_keyword, preserves_comment_block_before_scenario ...
- **Fix: SPLIT**
- New file(s): rust/fspec-core/src/io/gherkin_format/background_tests.rs (BUG-157, 4); rust/fspec-core/src/io/gherkin_format/description_tests.rs (RPC-330, 5 + relink); rust/fspec-core/src/io/gherkin_format/stepless_scenario_tests.rs (BUG-158, 4)
- Moves / wiring: Move with `#[cfg(test)] #[path]`. The generic formatter tests (unmapped) stay inline or go to gherkin_format/tests.rs.
- Notes: RPC-330 "Step doc string with an internal blank line is not regressed" is mapped to format_table (impl), so re-link it to the docstring test. RPC-330 has 13 scenarios and only 6 map here.

#### `rust/fspec-core/tests/list_work_units.rs`

- Kind: integration test (Rust, crate fspec-core)
- **fspec-dispatcher-tokio-nesting-safety** [RPC-327:done] 3 scen: dispatch_command_does_not_hang_when_called_from_inside_a_tokio_runtime, dispatch_command_still_works_from_a_synchronous_test_context_for_backwards_compatibility, dispatch_ported_command_with_invalid_args_from_tokio_runtime_returns_structured_error_without_hanging
- **list-work-units-rust-port** [RPC-253:done] 11 scen: auto_creates_work_units_and_prefixes_files_on_first_run, lists_all_work_units_in_insertion_order_when_no_filters_applied, filters_by_status_when_status_flag_provided, filters_by_prefix_appending_hyphen_before_starts_with_match, filters_by_epic_with_exact_equality, filters_by_type_defaulting_missing_type_to_story, combines_multiple_filters_with_and_semantics, text_format_prints_no_work_units_found_for_empty_result, text_format_prints_work_units_header_and_entries_when_populated, returns_structured_error_when_work_units_json_is_malformed, shared_infrastructure_modules_exist_under_fspec_core
- **Fix: SPLIT**
- New file(s): rust/fspec-core/tests/dispatcher_tokio_nesting_safety.rs (RPC-327, 3 tests)
- Moves / wiring: Copy the small helpers it uses (req, seed_work_units, three_unit_store, parse_data) or move them to rust/test-helpers if they are used elsewhere.

#### `rust/fspec-core/tests/research.rs`

- Kind: integration test (Rust, crate fspec-core)
- **research-registry-drops-ast-tool** [CLI-015:done] 2 scen: list_mode_enumerates_bundled_registry_without_ast, execute_mode_rejects_ast_tool_and_points_to_fspec_astgrep
- **research-rust-port** [RPC-286:done] 6 scen: list_mode_reflects_configured_perplexity_api_key_from_project_config, list_mode_does_not_create_files_or_spawn_a_process, execute_mode_rejects_unknown_tool_before_doing_work, list_mode_reports_stakeholder_configured_when_required_webhook_present, list_mode_enumerates_bundled_registry_without_ast
- Same test credited to 2+ features: list_mode_enumerates_bundled_registry_without_ast (research-registry-drops-ast-tool, research-rust-port)
- Unmapped (1): both_front_doors_converge_on_same_function
- **Fix: SPLIT + DEDUP**
- New file(s): rust/fspec-core/tests/research_registry_drops_ast_tool.rs (CLI-015: list_mode_enumerates_bundled_registry_without_ast, execute_mode_rejects_ast_tool_and_points_to_fspec_astgrep)
- Moves / wiring: research.rs keeps RPC-286. Re-point RPC-286's "List mode enumerates the bundled tool registry without ast" to a new registry-listing test that does not assert on ast (or drop the "without ast" wording, which CLI-015 added). Re-point "Both front doors converge" to the unmapped both_front_doors_converge_on_same_function.

#### `rust/graph/tests/ast_extraction_pipeline_test/walk_project_directory_with.rs`

- Kind: #[path] part of tests/ast_extraction_pipeline_test.rs (Rust, crate graph)
- **ast-entity-deduplication** [KGRAPH-026:done] 2 scen: test_dedup_file_nodes_when_import_target_is_also_walked, test_preserve_stub_file_nodes_for_external_imports
- **ast-extraction-pipeline** [KGRAPH-017:done] 1 scen: test_walk_project_directory_with_gitignore_and_batch_load
- **Fix: SPLIT**
- New file(s): rust/graph/tests/ast_extraction_pipeline_test/file_node_deduplication.rs (KGRAPH-026: test_dedup_file_nodes_when_import_target_is_also_walked, test_preserve_stub_file_nodes_for_external_imports)
- Moves / wiring: Register with `#[path]` in tests/ast_extraction_pipeline_test.rs. Alternative: append to multiple_importers_produce_single.rs (265 lines, already KGRAPH-026-only), but that would pass 300 lines.

#### `rust/graph/tests/graph_reset_test.rs`

- Kind: integration test (Rust, crate graph)
- **graph-compaction-schema-migration** [KGRAPH-010:done] 1 scen: test_schema_match_passes_validation
- **graph-database-reset** [KGRAPH-071:done] 4 scen: test_reset_deletes_on_disk_database, test_reset_clears_in_memory_registry, test_schema_mismatch_returns_actionable_error, test_queries_work_after_reset_and_reindex
- Unmapped (3): test_ast_index_deserializes_with_reset_flag, test_ast_index_backwards_compatible_without_reset, test_delete_graph_data_returns_false_when_no_data
- **Fix: SPLIT**
- New file(s): rust/graph/tests/graph_schema_validation_test.rs (KGRAPH-010: test_schema_match_passes_validation)
- Moves / wiring: Uses graph_test_helpers (shared module already exists).

### B8 live TypeScript bridge (2), only if in scope

#### `bridge/__tests__/relay-endpoint.test.ts`

- Kind: vitest test (live TypeScript, not TypeScript-era)
- **relay-bridge-endpoint** [BRIDGE-015:done] 15 scen: Feature: Platform-Agnostic Relay Bridge Endpoint, Scenario: Successful authentication with relay server, should authenticate and start local WebSocket server, Scenario: Authentication failure with invalid API key, should log error and exit on auth failure, Scenario: Missing required configuration, should exit with clear error when RELAY_URL is missing, Scenario: Codelet BridgeTool establishes session connection, should store session_id and forward connected message to relay, Scenario: Multiple concurrent codelet sessions, should route messages to correct session, Scenario: Translate relay input message to fspec format, should unwrap data envelope and flatten message, Scenario: Translate relay sessionControl to fspec control format, should rename type and unwrap data envelope, Scenario: Translate relay command to fspec flat InboundMessage format, should unwrap data envelope and serialise args to args_json, Scenario: Drop messages for unknown session, should log warning and silently drop message, Scenario: Send periodic heartbeat to relay, should send ping every 30 seconds, Feature: relay-bridge-endpoint.feature — Command integration, Scenario: Execute fspec command via StreamChunk pipeline and return re, should translate command, forward to codelet, and relay commandRespons, Scenario: Command execution for unknown fspec command via StreamChunk , should relay error commandResponse back for unknown command, Scenario: Command execution timeout via StreamChunk pipeline, should relay timeout commandResponse back
- **relay-command-flow-through-bridge** [BRIDGE-018:done] 5 scen: should translate command, forward to codelet, and relay commandRespons, Scenario: Command execution for unknown fspec command via StreamChunk , should relay error commandResponse back for unknown command, Scenario: Command execution timeout via StreamChunk pipeline, should relay timeout commandResponse back, Scenario: FspecCommandResult chunks are intercepted and not forwarded , should not forward fspecCommandResult as regular chunk
- Same test credited to 2+ features: should translate command, forward to codelet, and relay commandRespons (relay-bridge-endpoint, relay-command-flow-through-bridge); Scenario: Command execution for unknown fspec command via StreamChunk  (relay-bridge-endpoint, relay-command-flow-through-bridge); should relay error commandResponse back for unknown command (relay-bridge-endpoint, relay-command-flow-through-bridge); Scenario: Command execution timeout via StreamChunk pipeline (relay-bridge-endpoint, relay-command-flow-through-bridge); should relay timeout commandResponse back (relay-bridge-endpoint, relay-command-flow-through-bridge)
- Unmapped (15): Scenario: Forward StreamChunk from codelet to relay without transforma, should pass through chunk data without modification, Scenario: Reconnect to relay after connection drops, should attempt reconnection with exponential backoff, Feature: Relay Command Flow Through Bridge WebSocket, Scenario: Translate relay command message to InboundMessage and forwar, should translate command to flat InboundMessage and send to codelet WS, Scenario: Drop command message for unknown session, should log warning and drop command for unknown session, Scenario: Forward commandResponse from codelet to relay unchanged, should pass through commandResponse from codelet to relay, Scenario: No bridge TypeScript files import fspecCallback ...
- **Fix: SPLIT**
- New file(s): bridge/__tests__/relay-command-flow.test.ts (BRIDGE-018: the "Relay Command Flow Through Bridge WebSocket" describe block, 5 tests)
- Moves / wiring: The file already has a separate describe block per feature. Some BRIDGE-015 line ranges overlap the BRIDGE-018 block (drift), so re-link both. Shared mock-WebSocket helpers go to bridge/__tests__/fixtures/.
- Notes: Not TypeScript-era: bridge/ is a live package and its files exist.
- Decision needed: Is the live bridge/ TypeScript in scope for COV-075 (the card does not say)? It needs vitest, not cargo.

#### `bridge/__tests__/telegram-endpoint.test.ts`

- Kind: vitest test (live TypeScript, not TypeScript-era)
- **telegram-bridge-endpoint** [BRIDGE-002:done] 19 scen: Feature: Telegram Bridge Endpoint, should start WebSocket server and Telegram bot when TELEGRAM_BOT_TOKEN, Scenario: Fail to start without required bot token, should exit with error when TELEGRAM_BOT_TOKEN is not set, Scenario: Use pre-configured chat ID for immediate message delivery, should send messages immediately when TELEGRAM_CHAT_ID is pre-configur, Scenario: Learn chat ID from first Telegram message, should drop chunks until chat ID is learned from Telegram message, Scenario: Reject additional codelet connections, should reject second connection when one is already active, Scenario: Accept new connection after session disconnect, should accept new connection after previous session disconnects, Scenario: Complete codelet auth handshake with authSuccess envelope, should respond with auth:authSuccess when codelet sends auth:authentic, should still respond with authSuccess when no instance metadata is pre, Scenario: Reply to system:ping with system:pong, should respond with system:pong when codelet sends system:ping, Scenario: Learn session ID from first relay chunk envelope, should store session ID when the first relay:chunk envelope arrives, should use learned session ID when relaying Telegram messages, should not overwrite an existing session_id on subsequent chunk envelo, Scenario: Unknown envelopes are logged and dropped, should drop envelopes with unknown service/type and not throw
- **telegram-user-whitelist** [BRIDGE-009:done] 7 scen: should preserve language markers in code blocks, Scenario: Relay Telegram message to codelet as input, should format Telegram message as input message and send via WebSocket, Scenario: Update active chat when user messages from different device, should update active chat ID when message arrives from different chat, Scenario: Route messages from multiple Telegram users to single sessio, should route all Telegram messages to the connected session, Scenario: Handle Telegram API errors gracefully, should log error and continue when Telegram API fails, Feature: User ID Whitelist for Telegram Bridge, Scenario: Authorized user message is forwarded to rust, should forward message when user ID is in whitelist, should drop message and log when user ID is not in whitelist
- Unmapped (35): Scenario: Start endpoint with required configuration, should invoke handleInboundEnvelope directly without needing a connect, Scenario: Relay text chunk to Telegram with MarkdownV2 formatting, should format text chunk with MarkdownV2 and send to Telegram, Scenario: Relay thinking chunk wrapped in think tags, should wrap thinking chunk in <think>...</think> tags (BRIDGE-006), Scenario: Relay tool_call chunk with tool indicator, should format tool_call chunk and store tool name for correlation, Scenario: Relay tool_result chunk with correlated tool name, should look up tool name and format result with tool indicator, Scenario: Relay error chunk with error indicator, should format error chunk with ❌ prefix ...
- **Fix: SPLIT**
- New file(s): bridge/__tests__/telegram-whitelist.test.ts (BRIDGE-009: the "User ID Whitelist for Telegram Bridge" describe block, 8 tests)
- Moves / wiring: Whitelist line ranges (935-1188) are drifted onto BRIDGE-002 tests. Re-link both features by describe/Scenario name.
- Decision needed: Same scope question.

## Excluded: TypeScript-era shared files (removed `src/**`, owned by COV-071..074)

Not in the attached list, because the files no longer exist. They still trip the BUG-357 gate for these features until the owning card re-maps or retires them. Card per feature from spec/attachments/COV-063/coverage-retirement-audit.md sections (commands-remap = COV-071, commands-review = COV-072, tui-review = COV-073, misc-review = COV-074).

| removed test file | features (card) |
|---|---|
| `src/__tests__/blocklist-napi-integration.test.ts` | blocklist-core-command-tool-filtering (COV-074); sensitive-path-prompts (COV-073) |
| `src/commands/__tests__/add-diagram.test.ts` | add-diagram-fails-with-screen-is-not-defined-error-for-valid-c4-diagrams (COV-071); add-diagram (COV-072) |
| `src/commands/__tests__/add-step.test.ts` | add-step (COV-071); fspec-add-step-appends-instead-of-replacing-prefill-placeholders (COV-071) |
| `src/commands/__tests__/configure-tools.test.ts` | conversational-test-and-quality-check-tool-detection (COV-071); tool-detection-check-functions-not-integrated-into-workflow (COV-071) |
| `src/commands/__tests__/documentation-validation.test.ts` | conversational-test-and-quality-check-tool-detection (COV-071); update-documentation-and-help (COV-072) |
| `src/commands/__tests__/epic-and-prefix-management.test.ts` | cli-command-registration (COV-072); epic-and-prefix-management (COV-072) |
| `src/commands/__tests__/event-storm-artifact-commands.test.ts` | event-storm-artifact-commands-events-commands-aggregates (COV-072); event-storm-artifact-commands-policies-hotspots-external-systems (COV-072) |
| `src/commands/__tests__/event-storm-help.test.ts` | fix-help-formatter-option-name-rendering-and-commonerrors-property-mismatch (COV-072); wire-up-comprehensive-help-for-all-event-storm-commands (COV-072) |
| `src/commands/__tests__/init-multi-agent.test.ts` | spec-agent-md-files-contain-stub-instead-of-comprehensive-workflow (COV-074); support-multiple-ai-agents-beyond-claude (COV-074) |
| `src/commands/__tests__/kanban-workflow-state-management.test.ts` | clarify-backward-movement-in-workflow-documentation (COV-072); kanban-workflow-state-management (COV-072) |
| `src/commands/__tests__/link-coverage.test.ts` | link-coverage-command (COV-072); system-reminder-for-missing-scenarios-in-link-coverage (COV-071) |
| `src/commands/__tests__/register-tag.test.ts` | register-tag-json-backed (COV-071); register-tag (COV-071) |
| `src/commands/__tests__/research-help.test.ts` | fix-fspec-research-tool-ast-help-command-should-display-tool-specific-help-instead-of-failing (COV-074); tool-specific-help-system (COV-072) |
| `src/commands/__tests__/show-coverage.test.ts` | show-coverage-crashes-with-undefined-coveragepercent (COV-071); show-coverage-statistics (COV-071) |
| `src/commands/__tests__/show-foundation.test.ts` | show-foundation-command-has-multiple-quality-issues (COV-071); show-foundation (COV-071) |
| `src/commands/__tests__/update-foundation.test.ts` | draft-file-update-priority (COV-071); update-foundation (COV-072) |
| `src/commands/__tests__/work-unit-dependency-management.test.ts` | cli-command-registration (COV-072); work-unit-dependency-management (COV-071) |
| `src/commands/__tests__/work-unit-estimation-and-metrics.test.ts` | cli-command-registration (COV-072); work-unit-estimation-and-metrics (COV-072) |
| `src/commands/__tests__/work-unit-query-and-reporting.test.ts` | cli-command-registration (COV-072); work-unit-query-and-reporting (COV-072) |
| `src/components/__tests__/ConfirmationDialog.test.tsx` | confirmation-dialog (COV-073); sensitive-path-prompts (COV-073) |
| `src/components/__tests__/RoleDialog.test.tsx` | role-dialog-remove-button (COV-073); role-dialog (COV-073) |
| `src/components/__tests__/agent-selector-duplicate-message.test.tsx` | duplicate-installation-success-message-in-fspec-init (COV-073); duplicate-next-steps-message-in-fspec-init-output (COV-073) |
| `src/generators/__tests__/foundation-md.test.ts` | bounded-context-map-ui-duplicates-each-bounded-context-name-in-visualization (COV-074); enhance-foundation-md-event-storm-visualization (COV-074) |
| `src/generators/__tests__/generate-event-storm-section-in-foundation-md.test.ts` | generate-event-storm-section-in-foundation-md-with-validated-mermaid-diagram (COV-074); mermaid-validation-test-uses-invalid-test-data (COV-074) |
| `src/git/__tests__/status.test.ts` | changed-files-view-missing-unstaged-files (COV-074); replace-git-cli-usage-with-isomorphic-git-library (COV-074) |
| `src/research-tools/__tests__/multi-language-support.test.ts` | add-java-php-ruby-bash-support-to-ast-research-tool (COV-074); add-multi-language-support-to-ast-research-tool-kotlin-dart-python-go-rust-swift-c-c-c (COV-074) |
| `src/tui/__tests__/AgentModal.test.tsx` | add-debug-slash-command-to-fspec-tui-agent (COV-073); manual-compaction-command (COV-073); tui-integration-for-codelet-ai-agent (COV-073) |
| `src/tui/__tests__/TUI-056-anchor-viewer.test.tsx` | fix-anchor-viewer-integration-with-thinking-dialog-system (COV-073); interactive-anchor-point-viewer-with-conversation-navigation (COV-073) |
| `src/tui/__tests__/context-window-fill-percentage.test.tsx` | context-fill-percentage-realtime-recompute-ui (COV-073); context-window-fill-percentage-indicator (COV-073) |
| `src/tui/components/__tests__/ModelSelectorScreen.integration.test.tsx` | model-screen-stale-profile-sections-from-non-openai-providers-footer-text-unreachable-section-filtering (COV-073); model-selector-screen (COV-073) |
| `src/tui/components/__tests__/SessionHeader.test.tsx` | session-header-isolation-badge (COV-073); session-header-realtime-status (COV-073) |
| `src/tui/components/__tests__/isolated-session-integration.test.tsx` | boardview-createsessiondialog-callback-ignores-isolated-parameter (COV-073); createsessiondialog-shows-wrong-text-when-pressing-enter-on-work-unit (COV-073); enter-key-on-board-bypasses-create-session-dialog-no-isolated-option (COV-073); sessionheader-missing-isisolated-prop-isolated-badge-never-shows (COV-073) |
| `src/tui/services/__tests__/globalSessionStreamManager.test.ts` | global-session-stream-manager (COV-073); isolation-state-stream-sync (COV-073) |
| `src/tui/services/__tests__/sessionService.test.ts` | fix-tui-068-code-smells-error-handling-console-statements-facade-completion (COV-073); refactor-session-work-unit-state-management (COV-073) |
| `src/utils/__tests__/activationMessage.test.ts` | agent-specific-activation-message-not-customized-in-fspec-init-success-output (COV-074); codex-init-writes-prompt-to-home-directory (COV-074) |
| `src/utils/__tests__/ensure-files.test.ts` | automatic-json-file-initialization (COV-071); work-units-json-not-stamped-with-current-version-on-initial-creation (COV-074) |
| `src/utils/__tests__/foundation-check.test.ts` | foundation-existence-check-in-commands (COV-072); foundation-missing-error-message-is-not-imperative-enough (COV-074) |
| `src/utils/__tests__/templateTransformation.test.ts` | fix-multi-agent-support-critical-issues (COV-072); support-multiple-ai-agents-beyond-claude (COV-074) |

Files per card (a file whose features sit in two cards counts for both): COV-071 11, COV-072 14, COV-073 12, COV-074 11. Ten files span two cards and need the two cards to agree.

