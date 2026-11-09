//! The MockBackend fields of one group and their empty defaults.

use super::*;

pub(super) struct MockControls {
    /// RPC-051: per-session scripted return for
    /// `persistence_get_history`. Tests use `script_history` to seed
    /// the snapshot the App's Shift+↑ first-press fetch task receives.
    pub(super) scripted_history: Mutex<HashMap<SessionId, Vec<String>>>,
    /// RPC-052: per-call counter for `set_pending_input`.
    pub(super) set_pending_input_calls: AtomicUsize,
    /// RPC-052: per-call counter for `get_pending_input`.
    pub(super) get_pending_input_calls: AtomicUsize,
    /// RPC-052: capture of every `(SessionId, Option<String>)` passed to
    /// `set_pending_input` in call order so the debounce-coalescing
    /// scenarios can assert how many distinct writes landed AND the
    /// final value the backend was left holding.
    pub(super) pending_input_writes: Mutex<Vec<(SessionId, Option<String>)>>,
    /// RPC-052: in-memory pending-input store so `get_pending_input`
    /// reads back what `set_pending_input` wrote AND so test fixtures
    /// can seed a value via `script_pending_input`.
    pub(super) pending_input_store: Mutex<HashMap<SessionId, Option<String>>>,
    /// RPC-052: when `Some`, `get_pending_input` returns
    /// `Err(anyhow!(message))` so failure-path scenarios can exercise
    /// the hydration error branch.
    pub(super) get_pending_input_error: Mutex<Option<String>>,
    /// RPC-052: when `Some`, `set_pending_input` returns
    /// `Err(anyhow!(message))` so failure-path scenarios can exercise
    /// the silent-drop branch.
    pub(super) set_pending_input_error: Mutex<Option<String>>,
    // ── RPC-053 pause / HITL surface ─────────────────────────────────
    /// RPC-053: per-session scripted pause-state return from
    /// `get_pause_state`. None when no pause is active for the session.
    pub(super) pause_state_store: Mutex<HashMap<SessionId, Option<PendingRequest<PauseState>>>>,
    /// RPC-053: per-session scripted hitl-request return from
    /// `get_hitl_request`. None when no HITL request is pending.
    pub(super) hitl_request_store: Mutex<HashMap<SessionId, Option<PendingRequest<HitlRequest>>>>,
    /// RPC-053: per-call counters for the six pause / HITL methods.
    pub(super) get_pause_state_calls: AtomicUsize,
    pub(super) get_hitl_request_calls: AtomicUsize,
    pub(super) pause_human_action_calls: AtomicUsize,
    pub(super) pause_confirm_calls: AtomicUsize,
    pub(super) pause_triple_calls: AtomicUsize,
    pub(super) send_hitl_response_calls: AtomicUsize,
    /// RPC-053: capture of every `(SessionId, accept)` passed to
    /// `pause_confirm` in call order.
    pub(super) pause_confirm_calls_log: Mutex<Vec<(SessionId, bool)>>,
    /// RPC-053: capture of every `(SessionId, ApprovalChoice)` passed
    /// to `pause_triple` in call order.
    pub(super) pause_triple_calls_log: Mutex<Vec<(SessionId, ApprovalChoice)>>,
    /// AMGR-023: capture of every `(SessionId, reply)` passed to
    /// `pause_human_action`.
    pub(super) pause_human_action_calls_log:
        Mutex<Vec<(SessionId, codelet_rpc_types::HumanActionReply)>>,
    /// RPC-053: capture of every `(SessionId, HitlResponse)` passed to
    /// `send_hitl_response` in call order.
    pub(super) send_hitl_response_calls_log: Mutex<Vec<(SessionId, HitlResponse)>>,
    /// AMGR-020: `(method, session, request_id)` of every reply in call order.
    pub(super) reply_request_ids: Mutex<Vec<(&'static str, SessionId, String)>>,
    /// RPC-053: scripted error slots — when `Some`, the matching
    /// method returns `Err(anyhow!(message))`.
    pub(super) get_pause_state_error: Mutex<Option<String>>,
    pub(super) get_hitl_request_error: Mutex<Option<String>>,
    pub(super) pause_human_action_error: Mutex<Option<String>>,
    pub(super) pause_confirm_error: Mutex<Option<String>>,
    pub(super) pause_triple_error: Mutex<Option<String>>,
    pub(super) send_hitl_response_error: Mutex<Option<String>>,
    // ── TOOL-022 P2 exec-stdin overlay surface ────────────────────────
    /// Per-session scripted exec-stdin request return from
    /// `get_exec_stdin_request`. `None` when no request is pending.
    pub(super) exec_stdin_store: Mutex<HashMap<SessionId, Option<ExecStdinRequest>>>,
    /// Per-call counter for `get_exec_stdin_request`.
    pub(super) get_exec_stdin_request_calls: AtomicUsize,
    /// Per-call counter for `write_exec_stdin`.
    pub(super) write_exec_stdin_calls: AtomicUsize,
    /// Capture of every `(SessionId, exec_session_id, text)` passed to
    /// `write_exec_stdin` in call order.
    pub(super) write_exec_stdin_calls_log: Mutex<Vec<(SessionId, String, String)>>,
    /// Scripted error for `get_exec_stdin_request` — when `Some`, the
    /// method returns `Err(anyhow!(message))`.
    pub(super) get_exec_stdin_request_error: Mutex<Option<String>>,
    /// Scripted error for `write_exec_stdin` — when `Some`, the method
    /// returns `Err(message)` (a String, matching the trait's
    /// `Result<(), String>`).
    pub(super) write_exec_stdin_error: Mutex<Option<String>>,
    // ── RPC-054 provider-credentials surface ─────────────────────────
    /// RPC-054: ordered list of provider credential infos returned by
    /// `list_provider_credentials`. Tests use `seed_provider_credentials`
    /// to set the initial list AND `set_post_save_provider_credentials`
    /// / `set_post_delete_provider_credentials` / `set_post_refresh_provider_credentials`
    /// to script the next list returned after the App's follow-up refresh.
    pub(super) provider_credentials: Mutex<Vec<ProviderCredentialInfo>>,
    pub(super) provider_settings: Mutex<Vec<codelet_rpc_types::ProviderSettingsConfig>>,
    /// BUG-575: when set, `list_provider_settings` fails with it.
    pub(super) provider_settings_error: Mutex<Option<String>>,
    /// RPC-054: next-list-after-save override. When `Some`, the very next
    /// `list_provider_credentials` call after `set_provider_credentials`
    /// returns this value instead of the seeded list, then clears the
    /// override.
    pub(super) provider_credentials_after_save: Mutex<Option<Vec<ProviderCredentialInfo>>>,
    /// RPC-054: next-list-after-delete override.
    pub(super) provider_credentials_after_delete: Mutex<Option<Vec<ProviderCredentialInfo>>>,
    /// RPC-054: next-list-after-refresh override.
    pub(super) provider_credentials_after_refresh: Mutex<Option<Vec<ProviderCredentialInfo>>>,
    /// RPC-054: when set, the next `list_provider_credentials` call
    /// returns this override (one-shot).
    pub(super) next_list_override: Mutex<Option<Vec<ProviderCredentialInfo>>>,
    /// RPC-054: per-call counters for the credential surface.
    pub(super) list_provider_credentials_calls: AtomicUsize,
    pub(super) get_provider_credential_calls: AtomicUsize,
    pub(super) set_provider_credentials_calls: AtomicUsize,
    pub(super) delete_provider_credentials_calls: AtomicUsize,
    pub(super) test_provider_connection_calls: AtomicUsize,
    pub(super) refresh_models_cache_calls: AtomicUsize,
    /// RPC-054: capture of the last (provider_id, input) tuple passed to
    /// `set_provider_credentials`.
    pub(super) last_set_provider_credentials: Mutex<Option<(String, ProviderCredentialInput)>>,
    /// RPC-054: capture of the last provider_id passed to
    /// `delete_provider_credentials`.
    pub(super) last_delete_provider_credentials: Mutex<Option<String>>,
    /// RPC-054: capture of the last provider_id passed to
    /// `test_provider_connection`.
    pub(super) last_test_provider_connection: Mutex<Option<String>>,
    /// RPC-054: capture of the last provider_id passed to
    /// `refresh_models_cache`.
    pub(super) last_refresh_models_cache: Mutex<Option<String>>,
    /// RPC-054: per-provider scripted result for
    /// `test_provider_connection`. Defaults to `Ok(success: true, latency 0)`.
    pub(super) test_connection_results: Mutex<HashMap<String, TestConnectionResult>>,
    /// RPC-054: per-provider scripted model list returned by
    /// `refresh_models_cache`.
    pub(super) refresh_models_results: Mutex<HashMap<String, Vec<ModelEntry>>>,
    /// RPC-054: when `Some`, `set_provider_credentials` returns
    /// `Err(anyhow!(message))` so the silent-error scenario can exercise
    /// the tracing::warn! branch.
    pub(super) set_provider_credentials_error: Mutex<Option<String>>,
    /// RPC-054: when `Some`, `list_provider_credentials` returns
    /// `Err(anyhow!(message))`.
    pub(super) list_provider_credentials_error: Mutex<Option<String>>,
    /// RPC-054: when `Some`, `delete_provider_credentials` returns
    /// `Err(anyhow!(message))`.
    pub(super) delete_provider_credentials_error: Mutex<Option<String>>,
    /// RPC-054: when `Some`, `test_provider_connection` returns
    /// `Err(anyhow!(message))`.
    pub(super) test_provider_connection_error: Mutex<Option<String>>,
    /// RPC-054: when `Some`, `refresh_models_cache` returns
    /// `Err(anyhow!(message))`.
    pub(super) refresh_models_cache_error: Mutex<Option<String>>,
    // ── PROV-109 profile write surface ───────────────────────────────
    /// PROV-109: per-call counters for the profile write surface.
    pub(super) save_profile_calls: AtomicUsize,
    pub(super) delete_profile_calls: AtomicUsize,
    /// PROV-109: capture of the last `(provider_id, profile_name, definition)`
    /// tuple passed to `save_profile`.
    pub(super) last_save_profile:
        Mutex<Option<(String, String, codelet_rpc_types::ProfileDefinition)>>,
    /// PROV-109: capture of the last `(provider_id, profile_name)` pair passed
    /// to `delete_profile`.
    pub(super) last_delete_profile: Mutex<Option<(String, String)>>,
    /// PROV-109: when `Some`, `save_profile` returns `Err(anyhow!(message))`.
    pub(super) save_profile_error: Mutex<Option<String>>,
    /// PROV-109: when `Some`, `delete_profile` returns `Err(anyhow!(message))`.
    pub(super) delete_profile_error: Mutex<Option<String>>,
    /// PROV-112: per-call counter for `oauth_clear_tokens`.
    pub(super) oauth_clear_tokens_calls: AtomicUsize,
    /// PROV-112: capture of every `provider_id` passed to `oauth_clear_tokens`,
    /// in call order (so per-provider routing can be asserted).
    pub(super) oauth_clear_tokens_providers: Mutex<Vec<String>>,
    /// PROV-112: when `Some`, `oauth_clear_tokens` returns `Err(anyhow!(msg))`.
    pub(super) oauth_clear_tokens_error: Mutex<Option<String>>,
    /// PROV-112: per-call counter for `oauth_get_tokens`.
    pub(super) oauth_get_tokens_calls: AtomicUsize,
    /// PROV-112: seeded `provider_id → has_tokens` answers for
    /// `oauth_get_tokens` (defaults to `false` when unset).
    pub(super) oauth_get_tokens_results: Mutex<HashMap<String, bool>>,
    // ── PROV-113 OAuth login surface ─────────────────────────────────
    /// PROV-113: per-call counter for `oauth_headless_start`.
    pub(super) oauth_headless_start_calls: AtomicUsize,
    /// PROV-113: scripted `(authorize_url, pkce_verifier)` for
    /// `oauth_headless_start`.
    pub(super) oauth_headless_start_result: Mutex<(String, String)>,
    /// BUG-600: when `Some`, `oauth_headless_start` returns `Err(anyhow!(msg))`.
    pub(super) oauth_headless_start_error: Mutex<Option<String>>,
    /// PROV-113: per-call counter + `(provider, code, verifier)` capture for
    /// `oauth_headless_complete`.
    pub(super) oauth_headless_complete_calls: AtomicUsize,
    pub(super) oauth_headless_complete_args: Mutex<Vec<(String, String, String)>>,
    pub(super) oauth_headless_complete_error: Mutex<Option<String>>,
    /// PROV-113: per-call counter + provider capture for `oauth_device_start`.
    pub(super) oauth_device_start_calls: AtomicUsize,
    pub(super) oauth_device_start_providers: Mutex<Vec<String>>,
    /// PROV-113: scripted `(user_code, verification_url, device_auth_id,
    /// interval)` for `oauth_device_start`.
    pub(super) oauth_device_start_result: Mutex<(String, String, String, u64)>,
    pub(super) oauth_device_start_error: Mutex<Option<String>>,
    /// PROV-113: per-call counter for `oauth_device_poll`.
    pub(super) oauth_device_poll_calls: AtomicUsize,
    pub(super) oauth_device_poll_error: Mutex<Option<String>>,
    /// PROV-114: per-call counter for `oauth_copilot_device_start`.
    pub(super) oauth_copilot_device_start_calls: AtomicUsize,
    /// PROV-114: capture of every `enterprise_host` passed to
    /// `oauth_copilot_device_start`, in call order (asserts the normalized
    /// host is forwarded; `None` for GitHub.com).
    pub(super) oauth_copilot_device_start_hosts: Mutex<Vec<Option<String>>>,
    /// PROV-114: scripted `(user_code, verification_url, device_auth_id,
    /// interval)` returned by `oauth_copilot_device_start`.
    pub(super) oauth_copilot_device_start_result: Mutex<(String, String, String, u64)>,
    /// PROV-114: when `Some`, `oauth_copilot_device_start` returns
    /// `Err(anyhow!(msg))`.
    pub(super) oauth_copilot_device_start_error: Mutex<Option<String>>,
    // ── RPC-055 debug-capture surface ────────────────────────────────
    /// RPC-055: per-call counter for `toggle_debug`.
    pub(super) toggle_debug_calls: AtomicUsize,
    /// RPC-055: capture of the last `(SessionId, debug_dir)` pair passed
    /// to `toggle_debug`.
    pub(super) last_toggle_debug: Mutex<Option<(SessionId, String)>>,
    /// RPC-055: scripted result returned by `toggle_debug`. Default is
    /// `Ok(String::new())` so scenarios that don't care about the body
    /// still get a well-formed Ok reply.
    pub(super) toggle_debug_result: Mutex<Result<String, String>>,
    /// RPC-055: per-call counter for `set_debug_directory`.
    pub(super) set_debug_directory_calls: AtomicUsize,
    /// RPC-055: capture of the last path passed to `set_debug_directory`.
    pub(super) last_set_debug_directory: Mutex<Option<String>>,
    /// RPC-055: when `Some`, `set_debug_directory` returns
    /// `Err(anyhow!(message))`.
    pub(super) set_debug_directory_error: Mutex<Option<String>>,
    // ── RPC-430 debug hydration / propagation tracking ────────────────
    /// RPC-430: per-call counter for `get_debug_enabled`.
    pub(super) get_debug_enabled_calls: AtomicUsize,
    /// RPC-430: scripted result returned by `get_debug_enabled`.
    pub(super) get_debug_enabled_result: Mutex<Result<bool, String>>,
    /// RPC-430: per-call counter for `set_debug_enabled`.
    pub(super) set_debug_enabled_calls: AtomicUsize,
    /// RPC-430: capture of the last `(SessionId, bool)` passed to `set_debug_enabled`.
    pub(super) last_set_debug_enabled: Mutex<Option<(SessionId, bool)>>,
    // ── RPC-056 blocklist surface ────────────────────────────────────
    /// RPC-056: in-memory rule list returned by `blocklist_list`.
    pub(super) blocklist_rules: Mutex<Vec<BlocklistRuleInfo>>,
    /// RPC-056: per-call counter for `blocklist_list`.
    pub(super) blocklist_list_calls: AtomicUsize,
    /// RPC-056: when `Some`, `blocklist_list` returns
    /// `Err(anyhow!(message))`.
    pub(super) blocklist_list_error: Mutex<Option<String>>,
    // ── RPC-057 /merge-worktree surface ──────────────────────────────
    /// RPC-057: seeded `MergeOutcome` returned by `merge_session_worktree`.
    pub(super) merge_outcome: Mutex<codelet_rpc_types::MergeOutcome>,
    /// RPC-057: per-call counter for `merge_session_worktree`.
    pub(super) merge_session_worktree_calls: AtomicUsize,
    /// RPC-057: when `Some`, `merge_session_worktree` returns
    /// `Err(anyhow!(message))`.
    pub(super) merge_session_worktree_error: Mutex<Option<String>>,
    /// RPC-057: per-call counter for `discard_session_worktree`.
    pub(super) discard_session_worktree_calls: AtomicUsize,
    /// RPC-057: when `Some`, `discard_session_worktree` returns
    /// `Err(anyhow!(message))`.
    pub(super) discard_session_worktree_error: Mutex<Option<String>>,
    /// RPC-057: per-call counter for `prune_orphaned_worktrees`.
    pub(super) prune_orphaned_worktrees_calls: AtomicUsize,
}
