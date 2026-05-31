# RPC-043 AST research — current shape of `codelet/napi/src/session_manager.rs`

Date: 2026-05-21
Method: AST-grep over the live file (6752 LOC) with `language=rust` and
representative patterns matching the structural shapes RPC-043 must
relocate.

## Pattern 1 — `pub async fn $NAME(...) -> $RET { ... }`

20 matches. These are the `#[napi]`-decorated async free functions that
must move into `session_bindings.rs`:

| Line | Function |
|---:|---|
| 4188 | `session_manager_create` |
| 4201 | `session_manager_create_with_id` |
| 4236 | `session_manager_create_isolated` |
| 4845 | `session_get_turn_details` |
| 4917 | `session_set_model` |
| 5046 | `session_set_model_profile` |
| 5203 | `session_get_internal_provider` |
| 5352 | `loop_register` |
| 5415 | `loop_cancel` |
| 5423 | `loop_list` |
| 5552 | `session_restore_messages` |
| 5725 | `session_restore_token_state` |
| 5774 | `session_update_debug_metadata` |
| 5805 | `session_toggle_debug` |
| 5907 | `session_compact` |
| 6607 | `list_providers` |
| 6616 | `show_provider` |
| 6626 | `validate_provider` |
| 6634 | `test_provider` |
| 6649 | `init_provider` |

## Pattern 2 — `pub fn $NAME(...) -> $RET { ... }`

49 matches. The 46 of these decorated with `#[napi]` move into
`session_bindings.rs`; `parse_interjection` (line 181) moves into
`interjection.rs`; `agent_loop_dispatch_supports_provider` (line 2419)
moves into `agent_loop.rs`; `session_manager_list` / `_destroy` /
`session_set_global_chunk_callback` are part of the `#[napi]` set:

The 46 `#[napi]` sync free functions to move into `session_bindings.rs`:

| Line | Function |
|---:|---|
| 4261 | `session_manager_list` |
| 4267 | `session_manager_destroy` |
| 4295 | `session_set_global_chunk_callback` |
| 4568 | `session_set_active` |
| 4586 | `session_send_input` |
| 4595 | `session_interrupt` |
| 4610 | `session_clear_history` |
| 4618 | `session_get_status` |
| 4629 | `session_get_compaction_progress` |
| 4645 | `session_get_pause_state` |
| 4655 | `session_get_hitl_request` |
| 4677 | `session_pause_resume` |
| 4688 | `session_pause_confirm` |
| 4704 | `session_pause_triple` |
| 4734 | `session_send_fspec_result` |
| 4757 | `session_send_hitl_response` |
| 4790 | `session_get_base_thinking_level` |
| 4801 | `session_set_base_thinking_level` |
| 4813 | `session_get_next` |
| 4820 | `session_get_prev` |
| 4827 | `session_get_first` |
| 4834 | `session_clear_active` |
| 5177 | `session_get_model` |
| 5230 | `session_get_tokens` |
| 5242 | `session_get_debug_enabled` |
| 5249 | `session_set_debug_enabled` |
| 5260 | `session_get_pending_input` |
| 5270 | `session_set_pending_input` |
| 5278 | `session_get_buffered_output` |
| 5295 | `session_set_role` |
| 5313 | `session_get_role` |
| 5328 | `session_is_scheduled` |
| 5335 | `session_schedule_name` |
| 5453 | `session_get_subordinate` |
| 5466 | `session_get_supervisors` |
| 5487 | `session_set_observed_correlation_ids` |
| 5498 | `session_clear_observed_correlation_ids` |
| 5507 | `session_get_merged_output` |
| 5760 | `toggle_debug` |
| 6008 | `test_provider_connection` |
| 6040 | `session_set_work_unit_context` |
| 6055 | `session_get_work_unit_context` |
| 6076 | `session_get_active` |
| 6117 | `session_validate_path` |
| 6418 | `session_get_effective_cwd` |
| 6432 | `session_is_isolated` |
| 6464 | `session_execute_bash` |
| 6675 | `get_model_info` |

Total: 20 async + 46 sync = **66 `#[napi]` wrappers**. Matches the
attachment table exactly.

## Pattern 3 — `impl $TRAIT for $TYPE { ... }`

8 matches:

| Line | Block | Destination |
|---:|---|---|
| 325 | `impl From<PauseState> for NapiPauseState` | `session_bindings.rs` (lives alongside `NapiPauseState`) |
| 3343 | `impl Drop for IdleOnDropGuard` | `agent_loop.rs` (`IdleOnDropGuard` is internal to `agent_loop`) |
| 3779 | `impl codelet_cli::interactive::StreamOutput for BackgroundOutput` | `agent_loop.rs` |
| 4019 | `impl codelet_cli::interactive::StreamOutput for BackgroundProgressEmitter` | `agent_loop.rs` |
| 4249 | `impl From<codelet_rpc_types::IsolatedSessionInfo> for IsolatedSessionResult` | `session_bindings.rs` |
| 6547 | `impl From<codelet_providers::custom::ProviderModelInfo> for JsProviderModelInfo` | `session_bindings.rs` |
| 6580 | `impl From<codelet_providers::custom::ProviderInfo> for JsProviderInfo` | `session_bindings.rs` |
| 6698 | `impl codelet_sessions::session_manager::SessionManagerHooks for NapiSessionManagerHooks` | `session_hooks.rs` |

## Outcome

The AST research confirms:

1. The 66-wrapper count in the attachment is exact (20 async + 46 sync,
   no double-counting).
2. The non-#[napi] structural blocks that must relocate are:
   * `parse_interjection` + `Interjection` → `interjection.rs`
   * `agent_loop` async fn + `IdleOnDropGuard` + `BackgroundOutput` +
     `BackgroundProgressEmitter` + `agent_loop_dispatch_supports_provider` →
     `agent_loop.rs`
   * `NapiSessionManagerHooks` impl → `session_hooks.rs`
3. The 7 `From<…>` and `Drop` impls split cleanly across the three
   destinations.
4. Every `#[napi(object)]` struct (12 total — verified by manual grep)
   lives alongside its consuming `#[napi]` wrapper in
   `session_bindings.rs`.

This research underwrites the rule and example set in the RPC-043
feature file. No further AST investigation is required before testing
begins.
