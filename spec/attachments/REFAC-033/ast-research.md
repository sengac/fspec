# REFAC-033 AST research

## fspec astgrep --pattern 'fn $NAME() { $$$B }' --lang rust --path rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:427:1:fn generate_compaction_tool_wired_into_every_provider_tool_chain() {
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:999:1:fn sub_agent_dag_validation_uses_parse_dag_nodes() {
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:1243:1:fn fresh_compaction_prompt_names_the_target_and_forbids_inject_summa
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:1281:1:fn incremental_compaction_prompt_embeds_existing_dag_and_turn_offset
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:1341:1:fn compactor_sub_agent_has_only_the_read_only_tool_surface() {
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:1389:1:fn generate_compaction_handler_registration_lifecycle_mirrors_deep_s
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:1648:1:fn compactor_sub_agent_provider_arms_match_the_deep_search_clone() {


## fspec astgrep --pattern 'async fn $NAME() { $$$B }' --lang rust --path rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:476:1:async fn successful_dag_is_pinned_to_the_target_and_sub_agent_session
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:667:1:async fn calling_session_target_stashes_pending_dag_instead_of_lockin
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:722:1:async fn unknown_target_session_is_rejected_with_an_execution_error()
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:809:1:async fn existing_dag_is_captured_before_any_clear_and_drives_increme
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:891:1:async fn compaction_flag_true_during_run_and_false_after_pin() {
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:931:1:async fn compaction_lifecycle_events_carry_the_recalculated_post_pin_
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:1050:1:async fn generate_compaction_implements_the_rig_tool_trait() {
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:1093:1:async fn invalid_session_id_returns_a_validation_error() {
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:1146:1:async fn omitted_session_id_targets_the_calling_session() {
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:1175:1:async fn missing_generate_compaction_handler_returns_an_execution_er
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:1205:1:async fn tool_result_is_the_pinned_dag_text() {
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:1464:1:async fn self_target_capture_never_awaits_the_callers_inner_lock() {
rust/agent-loop/tests/cmpct045_generate_compaction_behavioral.rs:1571:1:async fn a_zero_tracker_basis_falls_back_to_a_content_estimate() {


## Findings

- The file is one test-target crate root: top-level helper fns and #[test]/#[tokio::test] fns, no
  nested test module, so cuts between top-level items keep every item at crate-root scope.
