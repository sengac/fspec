use super::*;
pub async fn invoke(
    client: &FspecServiceClient,
    name: &str,
    args: Value,
) -> Result<Value, InvokeError> {
    match name {
        "list_work_units" => group_00::list_work_units::call(client, args).await,
        "list_sessions" => group_00::list_sessions::call(client, args).await,
        "list_live_sessions" => group_00::list_live_sessions::call(client, args).await,
        "create_session" => group_01::create_session::call(client, args).await,
        "send_input" => group_01::send_input::call(client, args).await,
        "interrupt" => group_01::interrupt::call(client, args).await,
        "get_session_status" => group_02::get_session_status::call(client, args).await,
        "health" => group_02::health::call(client, args).await,
        "checkpoint_counts" => group_02::checkpoint_counts::call(client, args).await,
        "changed_files" => group_03::changed_files::call(client, args).await,
        "file_diff" => group_03::file_diff::call(client, args).await,
        "list_checkpoints" => group_03::list_checkpoints::call(client, args).await,
        "checkpoint_diff_files" => group_04::checkpoint_diff_files::call(client, args).await,
        "checkpoint_file_diff" => group_04::checkpoint_file_diff::call(client, args).await,
        "restore_checkpoint_file" => group_04::restore_checkpoint_file::call(client, args).await,
        "restore_checkpoint_all" => group_05::restore_checkpoint_all::call(client, args).await,
        "delete_checkpoint" => group_05::delete_checkpoint::call(client, args).await,
        "delete_all_checkpoints" => group_05::delete_all_checkpoints::call(client, args).await,
        "move_work_unit_up" => group_06::move_work_unit_up::call(client, args).await,
        "move_work_unit_down" => group_06::move_work_unit_down::call(client, args).await,
        "get_model_info" => group_06::get_model_info::call(client, args).await,
        "get_thinking_level" => group_07::get_thinking_level::call(client, args).await,
        "get_workspace_info" => group_07::get_workspace_info::call(client, args).await,
        "search_files" => group_07::search_files::call(client, args).await,
        "persistence_add_history" => group_08::persistence_add_history::call(client, args).await,
        "persistence_get_history" => group_08::persistence_get_history::call(client, args).await,
        "persistence_search_history" => {
            group_08::persistence_search_history::call(client, args).await
        }
        "persistence_delete_session" => {
            group_09::persistence_delete_session::call(client, args).await
        }
        "list_providers" => group_09::list_providers::call(client, args).await,
        "set_session_model" => group_09::set_session_model::call(client, args).await,
        "set_default_model" => group_10::set_default_model::call(client, args).await,
        "add_custom_model" => group_10::add_custom_model::call(client, args).await,
        "update_custom_model" => group_10::update_custom_model::call(client, args).await,
        "delete_custom_model" => group_11::delete_custom_model::call(client, args).await,
        "list_provider_settings" => group_11::list_provider_settings::call(client, args).await,
        "save_provider_settings" => group_11::save_provider_settings::call(client, args).await,
        "save_profile" => group_12::save_profile::call(client, args).await,
        "delete_profile" => group_12::delete_profile::call(client, args).await,
        "rename_profile" => group_12::rename_profile::call(client, args).await,
        "set_thinking_level" => group_13::set_thinking_level::call(client, args).await,
        "set_continue_state" => group_13::set_continue_state::call(client, args).await,
        "get_continue_state" => group_13::get_continue_state::call(client, args).await,
        "set_goal_state" => group_14::set_goal_state::call(client, args).await,
        "get_goal_state" => group_14::get_goal_state::call(client, args).await,
        "get_session_role" => group_14::get_session_role::call(client, args).await,
        "set_session_role" => group_15::set_session_role::call(client, args).await,
        "send_input_with_thinking" => group_15::send_input_with_thinking::call(client, args).await,
        "get_session_tokens" => group_15::get_session_tokens::call(client, args).await,
        "get_session_model" => group_16::get_session_model::call(client, args).await,
        "get_compaction_progress" => group_16::get_compaction_progress::call(client, args).await,
        "get_buffered_output" => group_16::get_buffered_output::call(client, args).await,
        "clear_history" => group_17::clear_history::call(client, args).await,
        "compact_session" => group_17::compact_session::call(client, args).await,
        "restore_session_messages" => group_17::restore_session_messages::call(client, args).await,
        "restore_session_token_state" => {
            group_18::restore_session_token_state::call(client, args).await
        }
        "resume_session" => group_18::resume_session::call(client, args).await,
        "get_work_unit_focus" => group_18::get_work_unit_focus::call(client, args).await,
        "set_work_unit_focus" => group_19::set_work_unit_focus::call(client, args).await,
        "get_work_unit_context" => group_19::get_work_unit_context::call(client, args).await,
        "set_work_unit_context" => group_19::set_work_unit_context::call(client, args).await,
        "get_pending_input" => group_20::get_pending_input::call(client, args).await,
        "set_pending_input" => group_20::set_pending_input::call(client, args).await,
        "set_active_session" => group_20::set_active_session::call(client, args).await,
        "clear_active_session" => group_21::clear_active_session::call(client, args).await,
        "get_active_session" => group_21::get_active_session::call(client, args).await,
        "get_effective_cwd" => group_21::get_effective_cwd::call(client, args).await,
        "get_supervisors" => group_22::get_supervisors::call(client, args).await,
        "add_supervisor" => group_22::add_supervisor::call(client, args).await,
        "remove_supervisor" => group_22::remove_supervisor::call(client, args).await,
        "get_subordinate" => group_23::get_subordinate::call(client, args).await,
        "get_subordinates" => group_23::get_subordinates::call(client, args).await,
        "receive_incoming_message" => group_23::receive_incoming_message::call(client, args).await,
        "get_debug_enabled" => group_24::get_debug_enabled::call(client, args).await,
        "set_debug_enabled" => group_24::set_debug_enabled::call(client, args).await,
        "toggle_debug" => group_24::toggle_debug::call(client, args).await,
        "set_debug_directory" => group_25::set_debug_directory::call(client, args).await,
        "pause_resume" => group_25::pause_resume::call(client, args).await,
        "pause_confirm" => group_25::pause_confirm::call(client, args).await,
        "pause_triple" => group_26::pause_triple::call(client, args).await,
        "send_hitl_response" => group_26::send_hitl_response::call(client, args).await,
        "get_pause_state" => group_26::get_pause_state::call(client, args).await,
        "get_hitl_request" => group_27::get_hitl_request::call(client, args).await,
        "get_exec_stdin_request" => group_27::get_exec_stdin_request::call(client, args).await,
        "write_exec_stdin" => group_27::write_exec_stdin::call(client, args).await,
        "send_fspec_result" => group_28::send_fspec_result::call(client, args).await,
        "create_isolated_session" => group_28::create_isolated_session::call(client, args).await,
        "set_thinking_level_default" => {
            group_28::set_thinking_level_default::call(client, args).await
        }
        "destroy_session" => group_29::destroy_session::call(client, args).await,
        "list_provider_credentials" => {
            group_29::list_provider_credentials::call(client, args).await
        }
        "get_provider_credential" => group_29::get_provider_credential::call(client, args).await,
        "set_provider_credentials" => group_30::set_provider_credentials::call(client, args).await,
        "delete_provider_credentials" => {
            group_30::delete_provider_credentials::call(client, args).await
        }
        "test_provider_connection" => group_30::test_provider_connection::call(client, args).await,
        "refresh_models_cache" => group_31::refresh_models_cache::call(client, args).await,
        "oauth_clear_tokens" => group_31::oauth_clear_tokens::call(client, args).await,
        "oauth_get_tokens" => group_31::oauth_get_tokens::call(client, args).await,
        "oauth_browser_login" => group_32::oauth_browser_login::call(client, args).await,
        "oauth_headless_start" => group_32::oauth_headless_start::call(client, args).await,
        "oauth_headless_complete" => group_32::oauth_headless_complete::call(client, args).await,
        "oauth_device_start" => group_33::oauth_device_start::call(client, args).await,
        "oauth_device_poll" => group_33::oauth_device_poll::call(client, args).await,
        "oauth_copilot_device_start" => {
            group_33::oauth_copilot_device_start::call(client, args).await
        }
        "blocklist_list" => group_34::blocklist_list::call(client, args).await,
        "merge_session_worktree" => group_34::merge_session_worktree::call(client, args).await,
        "discard_session_worktree" => group_34::discard_session_worktree::call(client, args).await,
        "prune_orphaned_worktrees" => group_35::prune_orphaned_worktrees::call(client, args).await,
        "list_session_worktrees" => group_35::list_session_worktrees::call(client, args).await,
        "inspect_session_changes" => group_35::inspect_session_changes::call(client, args).await,
        "detach_session_worktree" => group_36::detach_session_worktree::call(client, args).await,
        "schedule_add" => group_36::schedule_add::call(client, args).await,
        "schedule_list" => group_36::schedule_list::call(client, args).await,
        "schedule_pause" => group_37::schedule_pause::call(client, args).await,
        "schedule_resume" => group_37::schedule_resume::call(client, args).await,
        "schedule_remove" => group_37::schedule_remove::call(client, args).await,
        "loop_add" => group_38::loop_add::call(client, args).await,
        "loop_cancel" => group_38::loop_cancel::call(client, args).await,
        "loop_list" => group_38::loop_list::call(client, args).await,
        "get_transcript_page" => group_39::get_transcript_page::call(client, args).await,
        "read_work_units_bounded" => group_39::read_work_units_bounded::call(client, args).await,
        "get_interaction_snapshot" => group_39::get_interaction_snapshot::call(client, args).await,
        "reply_interaction_checked" => {
            group_40::reply_interaction_checked::call(client, args).await
        }
        "create_session_checked" => group_40::create_session_checked::call(client, args).await,
        "session_manifest_issues" => group_40::session_manifest_issues::call(client, args).await,
        _ => Err(InvokeError::Unknown),
    }
}
