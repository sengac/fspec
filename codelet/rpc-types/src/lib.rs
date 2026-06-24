//! codelet-rpc-types: shared serde types for the fspec dual-transport RPC.
//!
//! Single source of truth for any type that crosses the RPC boundary in
//! either direction. Default builds have zero dependencies on tarpc, tokio,
//! or napi — those crates depend on us, never the other way around.
//!
//! ## NAPI feature gate
//!
//! Enabling the `napi` feature applies `#[napi(object)]` (or
//! `#[napi(discriminant = "type")]` / `#[napi(string_enum)]`) to types that
//! cross the JS boundary so that `codelet/napi` can re-export them
//! verbatim and preserve the existing TypeScript shape (most notably
//! `correlationId`/`observedCorrelationIds`/`toolCall` etc. via
//! `#[napi(js_name = ...)]`). The feature is off by default; only
//! `codelet-napi` opts in.
//!
//! RPC-005 lifted only `WorkUnitInfo` from `codelet/napi/src/types.rs:182`.
//! RPC-007 lifts five additional types that the session REPL needs as a
//! single source of truth shared by the embedded transport, the WebSocket
//! transport, and the NAPI surface:
//!   * [`SessionId`] — newtype around String
//!   * [`SessionInfo`] — opaque metadata returned by `list_sessions`
//!   * [`SessionStatus`] — coarse session lifecycle state
//!   * [`StreamChunk`] — the 23-variant streaming chunk discriminated union
//!     plus its supporting structs
//!   * [`LogRecord`] — structured tracing event payload

use serde::{Deserialize, Serialize};

/// Work unit information shared across all transports and the NAPI surface.
///
/// Field order and naming match the original NAPI definition so that the
/// `napi` feature gate can preserve the existing TypeScript shape without
/// breaking changes.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkUnitInfo {
    pub id: String,
    pub title: String,
    #[serde(rename = "workType")]
    pub work_type: String,
    pub status: String,
    pub description: Option<String>,
    pub estimate: Option<i32>,
    pub epic: Option<String>,
    /// RPC-014: attachment file paths (basenames are rendered in the
    /// BoardView details strip). Empty vec when the work unit has no
    /// attachments. The NAPI surface preserves the existing TS shape
    /// `attachments: string[]`.
    pub attachments: Vec<String>,
    /// RPC-016: ISO-8601 UTC timestamp of the latest entry in this work
    /// unit's `stateHistory` array (i.e. when the unit most recently
    /// changed status). `None` for legacy records without
    /// `stateHistory`. Drives the `⏩` last-changed indicator in the
    /// Rust BoardView. Additive — the TS Ink BoardView continues to
    /// derive its `lastChangedWorkUnit` from `stateHistory[last]`
    /// directly so this field is invisible on the TS side.
    #[serde(rename = "lastStateChangeAt")]
    pub last_state_change_at: Option<String>,
}

/// RPC-015: paired manual + automatic checkpoint counts across all work
/// units in a workspace.
///
/// Mirrors the TS interface `{ manual: number; auto: number }` from
/// `src/utils/checkpoint-index.ts` so both the existing Ink TUI (which
/// reads via the pure-JS `countCheckpoints` helper) and the new Rust
/// ratatui TUI (which reads via `FspecService::checkpoint_counts`)
/// converge on the same shape. The `napi` cfg-gate preserves the JS
/// shape so the additive `napi::count_checkpoints` export can return
/// the same type verbatim.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointCounts {
    /// Count of manual (user-created) checkpoints.
    pub manual: u32,
    /// Count of automatic (state-transition) checkpoints — those whose
    /// names contain the `-auto-` substring per
    /// `src/utils/checkpoint-index.ts::AUTO_CHECKPOINT_PATTERN`.
    pub auto: u32,
}

// ============================================================================
// RPC-007: Session types
// ============================================================================

/// Stable identifier for a session. Newtype around `String` so the wire
/// shape stays a plain string but the type system distinguishes session
/// IDs from arbitrary strings.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SessionId {
    pub value: String,
}

impl SessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl From<String> for SessionId {
    fn from(value: String) -> Self {
        Self { value }
    }
}

impl From<&str> for SessionId {
    fn from(value: &str) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.value)
    }
}

/// Coarse session lifecycle state.
///
/// Variant ORDER is preserved exactly so that `as u8` casts in
/// `codelet/napi/src/session_manager.rs`
/// (`AtomicU8::new(SessionStatus::Idle as u8)` /
/// `status.swap(status as u8, ...)`) keep the historical discriminant
/// values 0..=4 stable after the type was lifted out of NAPI. `Cleared`
/// is appended (5) as RPC-007's only new variant.
#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SessionStatus {
    #[default]
    Idle,
    Running,
    Interrupted,
    /// PAUSE-001: Session is paused waiting for user input (Enter/Y/N/Esc)
    Paused,
    /// PERF-002: Session is compacting context - supports progress tracking
    Compacting,
    /// RPC-007: Session has been cleared (post-cleanup terminal state).
    Cleared,
}

impl SessionStatus {
    /// Convert status to string representation for TypeScript / log output.
    /// Lifted from `codelet/napi/src/session_manager.rs` as part of the
    /// RPC-007 type-uniqueness rule so callers in `codelet/napi` can use
    /// the inherent method via the re-export.
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Idle => "idle",
            SessionStatus::Running => "running",
            SessionStatus::Interrupted => "interrupted",
            SessionStatus::Paused => "paused",
            SessionStatus::Compacting => "compacting",
            SessionStatus::Cleared => "cleared",
        }
    }
}

impl From<u8> for SessionStatus {
    /// Inverse of `SessionStatus as u8`, lifted from
    /// `codelet/napi/src/session_manager.rs` so the historical
    /// `AtomicU8`-based round-trip continues to compile after the
    /// lift. Unknown values fall back to `Idle` (matches the pre-lift
    /// behaviour).
    fn from(v: u8) -> Self {
        match v {
            0 => SessionStatus::Idle,
            1 => SessionStatus::Running,
            2 => SessionStatus::Interrupted,
            3 => SessionStatus::Paused,
            4 => SessionStatus::Compacting,
            5 => SessionStatus::Cleared,
            _ => SessionStatus::Idle,
        }
    }
}

/// Public metadata about a session returned by `list_sessions`.
///
/// Field names match the NAPI surface verbatim (see
/// codelet/napi/src/session_manager.rs:380) so that codelet/napi can
/// re-export this type without changing the existing TypeScript shape.
/// `id` is a plain `String` rather than a [`SessionId`] newtype so the
/// TS shape stays a flat string.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub project: String,
    pub message_count: u32,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    /// GIT-029: Whether this is an isolated session with a git worktree
    pub is_isolated: bool,
    /// GIT-029: Path to the worktree (if isolated)
    pub worktree_path: Option<String>,
    /// RPC-007: optional role string the session was created with.
    pub role: Option<String>,
}

/// Structured log event payload pushed to subscribers via the
/// `logs_rx` broadcast channel.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogRecord {
    pub level: String,
    pub target: String,
    pub message: String,
    /// Unix epoch milliseconds when the event was captured.
    #[serde(rename = "timestampMs")]
    pub timestamp_ms: i64,
}

/// RPC-011: live health summary returned by `FspecService::health` and
/// reused by the `fspec status` subcommand for human-readable output.
///
/// All counters are point-in-time reads from `ServerStats`. `version`
/// is the daemon process's `env!("CARGO_PKG_VERSION")` so the caller
/// can sanity-check protocol compatibility.
///
/// Lifted into `codelet-rpc-types` (rather than living on the server
/// crate) so both transports — `EmbeddedFspecBackend` (which reads
/// `ServerStats` directly) and `WebSocketFspecBackend` (which receives
/// the struct over tarpc) — share the SAME wire shape. The napi
/// feature gate follows the existing `WorkUnitInfo`/`SessionInfo`
/// pattern so the type can be re-exported through `codelet-napi`
/// verbatim if a future JS surface needs it.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthInfo {
    /// Seconds since the daemon's `ServerStats::started_at` instant.
    ///
    /// Typed `i64` (rather than `u64`) so the `napi(object)` cfg-gate
    /// compiles under napi-derive v3 + `napi4` feature, which does not
    /// support `u64` in `napi(object)` field positions. The wire format
    /// (tarpc bincode) carries the same 8 bytes either way, and uptime
    /// can never exceed 2^63 seconds in practice.
    pub uptime_secs: i64,
    /// Live count of attached WebSocket clients (decremented via the
    /// `ConnectedClientGuard` Drop impl when each connection task
    /// exits).
    pub connected_clients: i64,
    /// Elapsed seconds since the workspace watcher last fired an Ok
    /// snapshot into the work-units fanout task. `None` if no
    /// snapshot has ever been observed by this daemon.
    pub last_watcher_event_secs_ago: Option<i64>,
    /// Cumulative `RecvError::Lagged` count surfaced by the chunks
    /// broadcast fanout.
    pub lag_chunks: i64,
    /// Cumulative `RecvError::Lagged` count surfaced by the logs
    /// broadcast fanout.
    pub lag_logs: i64,
    /// Cumulative `RecvError::Lagged` count surfaced by the work-units
    /// broadcast fanout.
    pub lag_work_units: i64,
    /// Daemon process's `env!("CARGO_PKG_VERSION")`.
    pub version: String,
}

// ============================================================================
// RPC-018: AgentView chrome shared types (ModelInfo, ThinkingLevel,
// WorkspaceInfo) consumed by the new `get_model_info`,
// `get_thinking_level`, `get_workspace_info` RPC methods.
// ============================================================================

/// Capability + display metadata for the model attached to a session.
///
/// Mirrors the TS `useModelStore.getCapabilities` shape so the SessionHeader
/// in the Rust ratatui TUI can paint `[R]` / `[V]` / `[Nk]` badges using
/// the SAME source of truth that the Ink SessionHeader.tsx consumes.
///
/// `Default` returns an "unknown model" sentinel — `display_name` is empty,
/// every capability is false, and `context_window` is 0. The SessionHeader
/// widget hides empty badges so the UI degrades gracefully when no
/// session manager is attached (the RPC-018 default-impl path).
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInfo {
    pub display_name: String,
    pub supports_reasoning: bool,
    pub supports_vision: bool,
    pub context_window: u32,
}

/// Per-session thinking/reasoning level. Mirrors the
/// `JsThinkingLevel` enum that the existing TS code consumes via
/// `@sengac/codelet-napi` (`codelet/napi/src/thinking_config.rs`).
#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThinkingLevel {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

/// Workspace snapshot — cwd + git branch — returned by
/// `FspecService::get_workspace_info`. Mirrors the data the TS
/// `SessionFooter.tsx` derives from a Rust-side `FooterStateUpdate`
/// poller; the new RPC method is a single-shot pull alternative that the
/// Rust ratatui SessionFooter widget consumes via `Action::WorkspaceInfoLoaded`.
///
/// `cwd` is returned RAW (not `~`-shortened) — the SessionFooter widget
/// performs the `home::home_dir()` substitution at render time so the
/// wire shape stays portable across hosts whose `$HOME` differs.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub cwd: String,
    pub git_branch: Option<String>,
}

// ============================================================================
// RPC-022: Modal dialog shared types — ProviderInfo + ModelEntry consumed by
// the new `list_providers`, `set_session_model`, `set_thinking_level`,
// `get_session_role`, `set_session_role` RPC methods.
// ============================================================================

/// One entry in the per-provider model list returned by
/// `FspecService::list_providers`. Mirrors the relevant fields of the
/// TS `NapiModelInfo` shape (codelet/napi/src/models/napi_bindings.rs)
/// so the Rust ratatui `ModelSelectorDialog` can paint capability
/// badges using the SAME source of truth that the Ink
/// `ModelSelectorView.tsx` consumes.
///
/// `Default` returns a "blank" entry — `display_name` empty, every
/// capability false, `context_window` 0, `is_custom` false. The
/// `ModelSelectorDialog` hides empty rows so the UI degrades
/// gracefully when no session manager is attached (the RPC-022
/// default-impl path).
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub display_name: String,
    pub context_window: u32,
    pub supports_reasoning: bool,
    pub supports_vision: bool,
    pub is_custom: bool,
}

/// RPC-347: transport-portable definition of a custom model declared on a
/// local-server profile (`profile.customModels[]`, MODEL-004). Maps 1:1 to
/// the persistence-layer `codelet_sessions::profile_sections::CustomModelDef`
/// — the `codelet-sessions` conversion module (`custom_model_def_from_wire`)
/// is the single place the two meet, keeping `codelet-core` /
/// `codelet-rpc-types` free of any dependency on `codelet-sessions`.
///
/// The CTX-008 compaction-threshold override is carried as two flat optional
/// fields (`compaction_threshold_type` / `compaction_threshold_value`) rather
/// than a nested object so the `napi(object)` projection stays a plain struct
/// (mirroring the `session_set_model` NAPI binding's
/// `compaction_threshold_type` / `compaction_threshold_value` parameters).
/// `id` is the only required field; every other field is optional and, when
/// `None`, is omitted from the persisted `CustomModelDef` JSON.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomModelDefinition {
    pub id: String,
    pub display_name: Option<String>,
    pub facade: Option<String>,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub compaction_threshold_type: Option<String>,
    pub compaction_threshold_value: Option<u32>,
    pub reasoning: Option<bool>,
    pub has_vision: Option<bool>,
}

/// PROV-108: transport-portable definition of a local-server profile's
/// connection settings (`providers.openai.profiles.<name>`, PROV-007). Maps to
/// the on-disk profile object written by
/// `codelet_sessions::profile_persistence::save_profile`; the
/// `codelet-sessions` conversion (`profile_def_from_wire`) is the single place
/// the wire shape and the on-disk shape meet, keeping `codelet-core` /
/// `codelet-rpc-types` free of any dependency on `codelet-sessions`.
///
/// `customModels` is deliberately NOT part of this definition: the
/// custom-model write path (RPC-347, `CustomModelDefinition`) owns that array,
/// and the profile read-modify-write preserves it. `base_url` and `api_key`
/// are required; the CTX-008 compaction-threshold override is carried as two
/// flat optional fields (`compaction_threshold_type` /
/// `compaction_threshold_value`) so the `napi(object)` projection stays a
/// plain struct (mirroring [`CustomModelDefinition`]).
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileDefinition {
    pub base_url: String,
    pub api_key: String,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub compaction_threshold_type: Option<String>,
    pub compaction_threshold_value: Option<u32>,
}

/// One provider's display metadata plus its set of available models —
/// returned (in a Vec) by `FspecService::list_providers`. Mirrors the
/// TS `NapiProviderModels` shape (codelet/napi/src/models/napi_bindings.rs)
/// so the Rust ratatui `ModelSelectorDialog` and the Ink
/// `ModelSelectorView.tsx` consume the same provider/model tree.
///
/// The `key` field is the stable provider identifier (e.g. "openai",
/// "anthropic") passed back through `set_session_model`. The
/// `display_name` is the human-readable label rendered in the
/// provider rows.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub key: String,
    pub display_name: String,
    pub models: Vec<ModelEntry>,
    /// RPC-338: `Some(profile)` marks this entry as a local-server profile
    /// section (drives the 📁 icon + `"provider: profile"` label in the
    /// model selector). `None` for cloud / custom providers. Stays
    /// `Option<String>` (not an enum) because `napi(object)` does not
    /// support discriminated enums — matches the `masked_key` / `source`
    /// convention on `ProviderCredentialInfo`.
    pub profile_name: Option<String>,
    /// RPC-338: `true` when a local-server profile's `/v1/models` probe
    /// failed and it has no custom models (MODEL-004). Drives the red
    /// `(unreachable)` marker. Defaults to `false` (reachable) so
    /// `derive(Default)` stays valid.
    pub is_unreachable: bool,
}

// ============================================================================
// RPC-054: Provider credentials surface — the wire shapes used by the
// `list_provider_credentials` / `set_provider_credentials` /
// `delete_provider_credentials` / `test_provider_connection` /
// `refresh_models_cache` RPCs that back the new Rust ratatui
// ProviderSettingsView (`/provider` slash command).
//
// All three types follow the same cfg-gated `napi(object)` + Serialize +
// Deserialize pattern as the surrounding `ProviderInfo` / `ModelEntry`
// shapes so codelet-napi can re-export them verbatim if a future JS
// surface needs them.
// ============================================================================

/// Summary entry per provider returned by
/// `FspecService::list_provider_credentials`. Drives the left-pane
/// provider list in `ProviderSettingsView`.
///
/// `credential_type` is one of `"api_key" | "oauth" | "custom"` and
/// matches the variant tag used in `ProviderCredentialInput::kind`.
/// `configured` reflects [`ProviderCredentials::detect`] (env var set,
/// auth file present, etc.). `model_count` is the number of models the
/// provider's config currently exposes — drives the
/// "(n models)" suffix in each row.
///
/// RPC-108: `masked_key` and `source` extend the wire surface so the
/// TUI can render `'sk-ant-••••mnop [env]'`-style indicators on
/// configured rows. Both stay `Option<String>` (not enums) because
/// `napi(object)` does not support discriminated enums — matches the
/// existing `credential_type` string convention at L407-409. Server-side
/// the masking happens inside `codelet-providers` via
/// `credentials::mask_api_key` BEFORE the data crosses the wire, so
/// raw key bytes never traverse either transport.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCredentialInfo {
    pub provider_id: String,
    pub display_name: String,
    pub configured: bool,
    /// One of "api_key", "oauth", or "custom" — mirrors the variant
    /// discriminant used in [`ProviderCredentialInput::kind`].
    pub credential_type: String,
    pub model_count: u32,
    /// RPC-108: Display-safe masked credential (e.g.
    /// `Some("sk-ant-••••••••mnop")`). `None` for unconfigured rows and
    /// for OAuth-only providers (the view layer renders `'OAuth'`).
    pub masked_key: Option<String>,
    /// RPC-108: Provenance tag — one of `"explicit" | "file" | "env" |
    /// "dotenv"` mirroring TS `ProviderConfigResult.source` at
    /// `src/utils/credentials.ts:56-59`. `None` when unconfigured.
    pub source: Option<String>,
}

/// PROV-113: result of the synchronous-ish headless OAuth start (anthropic).
/// `authorize_url` is shown to the user immediately and `pkce_verifier` is
/// round-tripped back to `oauth_headless_complete` for CSRF validation. Not
/// napi-exposed — the TS TUI has its own napi bindings; this shape only
/// crosses the Rust tarpc boundary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthHeadlessStart {
    pub authorize_url: String,
    pub pkce_verifier: String,
}

/// PROV-113: result of the codex device-auth start. `user_code` +
/// `verification_url` are displayed immediately; `device_auth_id` + `interval`
/// drive the follow-up poll. Not napi-exposed (Rust tarpc boundary only).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthDeviceStart {
    pub user_code: String,
    pub verification_url: String,
    pub device_auth_id: String,
    pub interval: u64,
}

/// Credential write payload consumed by
/// `FspecService::set_provider_credentials`. Encoded as a struct with a
/// `kind` discriminant (rather than a Rust enum) so that the
/// `napi(object)` derive remains valid — `napi_derive::napi(object)`
/// does not support discriminated enums and the JS surface needs to
/// re-export the same shape.
///
/// `kind` is one of:
///   * `"api_key"`     — `api_key` MUST be Some
///   * `"oauth"`       — `oauth_token` MUST be Some; `oauth_refresh_token` optional
///   * `"custom"`      — `custom_endpoint` MUST be Some; `custom_api_key` optional
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCredentialInput {
    /// "api_key" | "oauth" | "custom"
    pub kind: String,
    pub api_key: Option<String>,
    pub oauth_token: Option<String>,
    pub oauth_refresh_token: Option<String>,
    pub custom_endpoint: Option<String>,
    pub custom_api_key: Option<String>,
}

impl ProviderCredentialInput {
    /// Construct an `api_key` variant. The Rust API stays ergonomic
    /// even though the wire shape is a flat struct.
    pub fn api_key(key: impl Into<String>) -> Self {
        Self {
            kind: "api_key".to_string(),
            api_key: Some(key.into()),
            ..Self::default()
        }
    }

    /// Construct an `oauth` variant.
    pub fn oauth(token: impl Into<String>, refresh: Option<String>) -> Self {
        Self {
            kind: "oauth".to_string(),
            oauth_token: Some(token.into()),
            oauth_refresh_token: refresh,
            ..Self::default()
        }
    }

    /// Construct a `custom` variant.
    pub fn custom(endpoint: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            kind: "custom".to_string(),
            custom_endpoint: Some(endpoint.into()),
            custom_api_key: api_key,
            ..Self::default()
        }
    }
}

/// Result returned by `FspecService::test_provider_connection`. Drives
/// the right-pane status area in `ProviderSettingsView` — a `success:
/// true` value renders "✓ ok (latency_ms ms)" and a `success: false`
/// value renders "✗ <error>".
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestConnectionResult {
    pub success: bool,
    pub error: Option<String>,
    pub latency_ms: u32,
}

// ============================================================================
// RPC-056: Blocklist rule wire payload.
// ============================================================================

/// One row in the `/blocklist` view, surfaced by the `blocklist_list`
/// RPC method. Mirrors the TS `BlocklistRule` interface
/// (`src/tui/components/BlocklistListView.tsx` lines 20-34) field-for-field
/// with the addition of the explicit `source` provenance tag — the TS
/// frontend stamps `'system' | 'project'` on each rule client-side after
/// loading from `blocklistLoad(cwd)`; the Rust pipeline carries the
/// provenance over the wire so the frontend does not need to know how to
/// split the two configs.
///
/// `action` is one of `"block" | "allow" | "prompt"` (matches
/// `BlocklistAction` lowercased serde tag).
/// `source` is one of `"system" | "project"`.
/// `guidance` is `None` when the rule has no educational follow-up.
///
/// Wire shape is a flat struct so `napi_derive::napi(object)` stays valid;
/// no discriminated enums.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlocklistRuleInfo {
    pub id: String,
    pub pattern: String,
    /// "block" | "allow" | "prompt"
    pub action: String,
    pub reason: String,
    pub guidance: Option<String>,
    /// "system" | "project"
    pub source: String,
}

// ============================================================================
// RPC-057: /merge-worktree wire payloads.
// ============================================================================

/// The merge algorithm to use. Currently the codelet-git layer only
/// supports a single fast-forward-style merge; `Squash` and `ThreeWay`
/// are reserved on the wire so future cards can add support without
/// breaking the trait surface. Serialised as a snake_case string so
/// napi consumers receive plain strings.
#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    #[default]
    FastForward,
    Squash,
    ThreeWay,
}

/// Result status of a merge attempt. `Success` and `NoChanges` are
/// terminal outcomes; `Conflict` carries the conflicting file paths in
/// the surrounding `MergeOutcome` so the LLM can be seeded with a
/// context message and asked to resolve them. Serialised as a
/// snake_case string for napi compatibility.
#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStatus {
    Success,
    Conflict,
    #[default]
    NoChanges,
}

/// Outcome of a `merge_session_worktree` RPC call.
///
/// * `status` — terminal state classification.
/// * `conflicts` — non-empty only when `status == Conflict`; lists the
///   relative paths whose merge produced a conflict.
/// * `merge_commit` — short SHA of the resulting merge commit if the
///   underlying codelet-git layer surfaces one; `None` otherwise.
///
/// Wire shape is a flat struct so `napi_derive::napi(object)` stays
/// valid; no discriminated enums.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeOutcome {
    pub status: MergeStatus,
    pub conflicts: Vec<String>,
    pub merge_commit: Option<String>,
}

/// One row in the `list_session_worktrees` RPC response. Mirrors
/// `codelet_git::WorktreeInfo` plus the `derive_session_status` +
/// dirty heuristic computed from a non-empty `get_session_diff`.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionWorktreeInfo {
    pub session_id: SessionId,
    pub worktree_path: String,
    pub base_commit: String,
    pub head_commit: String,
    pub dirty: bool,
}

/// Summary payload returned by `inspect_session_changes`. The
/// MergeConfirmDialog renders the counts inline (e.g. "1 file changed,
/// +4 / -2, 1 commit") so the user has explicit feedback before
/// confirming a destructive merge.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionChangesSummary {
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
    /// Short SHAs (typically 7 chars) of every commit on the session
    /// branch that is not yet on the base branch.
    pub commits: Vec<String>,
}

// ============================================================================
// RPC-058: /schedule wire payloads.
// ============================================================================

/// Flat wire shape for a single scheduled job (agent or shell). Mirrors
/// `ScheduleEntry` from codelet/napi/src/scheduler/types.rs but flattens
/// the nested agent/shell config into top-level `role`/`prompt`/`command`
/// so the struct stays `napi_derive::napi(object)`-compatible (no
/// discriminated enums on the wire).
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub name: String,
    pub cron: String,
    pub timezone: String,
    /// "agent" | "shell"
    pub job_type: String,
    /// "active" | "paused"
    pub status: String,
    pub created_at: Option<String>,
    pub last_run_at: Option<String>,
    pub last_run_status: Option<String>,
    /// Agent-only — role text injected on session spawn.
    pub role: Option<String>,
    /// Agent-only — initial prompt sent to the spawned session.
    pub prompt: Option<String>,
    /// Shell-only — the command line to spawn.
    pub command: Option<String>,
    /// "skip" | "queue" (when None, the engine treats as "skip")
    pub overlap_policy: Option<String>,
}

// ============================================================================
// RPC-059: /loop wire payloads.
// ============================================================================

/// Flat wire shape for a single registered session-scoped loop. Mirrors
/// the internal `codelet_core::loops::LoopEntry` but flattens the
/// `chrono::DateTime<Utc>` fields into RFC-3339 String timestamps and
/// wraps the session UUID in a `SessionId` so the struct stays
/// `napi_derive::napi(object)`-compatible (no chrono on the JS side).
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredLoop {
    pub id: String,
    pub session_id: SessionId,
    pub prompt: String,
    pub interval_seconds: u32,
    /// RFC-3339 UTC timestamp.
    pub created_at: String,
    /// RFC-3339 UTC timestamp.
    pub expires_at: String,
    /// RFC-3339 UTC timestamp, or `None` if the loop has not fired yet.
    pub last_run_at: Option<String>,
}

// ============================================================================
// RPC-007: StreamChunk supporting types (lifted verbatim from
//   codelet/napi/src/types.rs)
// ============================================================================

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionProgress {
    pub phase: String,
    pub current: u32,
    pub total: u32,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub input: String,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultInfo {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProgressInfo {
    pub tool_call_id: String,
    pub tool_name: String,
    pub output_chunk: String,
    pub is_stderr: bool,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFillInfo {
    pub fill_percentage: u32,
    pub effective_tokens: f64,
    pub threshold: f64,
    pub context_window: f64,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorPendingInjectionInfo {
    pub urgent: bool,
    pub content: String,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncomingMessageImage {
    pub data: String,
    #[serde(rename = "mediaType")]
    pub media_type: String,
}

/// RPC-061: wire-portable payload for `receive_incoming_message`. Mirrors
/// the fields of `codelet_sessions::IncomingMessage` (formerly
/// `SupervisorInput` — see WATCH-003/006/008/011/019/020) so the
/// supervisor → subordinate injection path round-trips identically
/// across both embedded and WebSocket transports.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncomingMessageInput {
    #[serde(rename = "sourceSessionId")]
    pub source_session_id: String,
    #[serde(rename = "roleName")]
    pub role_name: String,
    pub message: String,
    pub images: Option<Vec<IncomingMessageImage>>,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    pub original_tokens: u32,
    pub compacted_tokens: u32,
    pub compression_ratio: f64,
    pub turns_summarized: u32,
    pub turns_kept: u32,
}

/// RPC-025: transport-portable match returned by
/// `FspecService::persistence_search_history`. Mirrors the relevant
/// fields of `codelet_core::persistence::history::HistoryEntry` but
/// formats the timestamp as an RFC3339 string so non-Rust consumers
/// don't need a chrono dependency.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryMatch {
    pub session_id: SessionId,
    pub text: String,
    pub timestamp_iso: String,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FspecRequest {
    pub command: String,
    #[serde(rename = "argsJson")]
    pub args_json: String,
    #[serde(rename = "projectRoot")]
    pub project_root: String,
    #[serde(rename = "toolCallId")]
    pub tool_call_id: String,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FspecResult {
    pub success: bool,
    pub data: String,
    pub error: Option<String>,
    #[serde(rename = "systemReminder")]
    pub system_reminder: Option<String>,
    #[serde(rename = "toolCallId")]
    pub tool_call_id: String,
}

#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTracker {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_input_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
    pub tokens_per_second: Option<f64>,
    pub cumulative_billed_input: Option<u32>,
    pub cumulative_billed_output: Option<u32>,
    pub reasoning_tokens: Option<u32>,
}

impl Default for TokenTracker {
    fn default() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_input_tokens: Some(0),
            cache_creation_input_tokens: Some(0),
            tokens_per_second: None,
            cumulative_billed_input: Some(0),
            cumulative_billed_output: Some(0),
            reasoning_tokens: None,
        }
    }
}

#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Idle,
    Running,
    Paused,
    Compacting,
    Interrupted,
    Cleared,
}

#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Error,
}

// ============================================================================
// RPC-036: per-session derived state + pause/HITL wire types + isolation
// supporting types. Added so the Rust AgentView and the TypeScript Ink
// frontend can share identical wire shapes via the `napi` feature gate.
// ============================================================================

// ---------------------------------------------------------------------------
// Phase 2.1 — Per-session derived state
// ---------------------------------------------------------------------------

/// Per-session token totals (input + output) as currently observed.
/// Mirrors what `BackgroundSession::get_tokens()` produces on the JS
/// side.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTokens {
    pub input_tokens: i64,
    pub output_tokens: i64,
}

/// Per-session token-restore state — used by `/resume` to rehydrate the
/// cumulative-billed counters and cache totals so the SessionFooter's
/// context-fill / billing badges keep their numbers across reopens.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenRestoreState {
    pub current_context: i64,
    pub cumulative_billed_output: i64,
    pub cache_read: i64,
    pub cache_creation: i64,
    pub cumulative_billed_input: i64,
    pub cumulative_billed_output_second: i64,
}

/// Per-session model binding: which provider+model the session uses
/// plus the derived limits (context window, max output tokens, and the
/// compaction threshold at which the session manager auto-compacts).
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionModel {
    pub provider_id: String,
    pub model_id: String,
    pub context_window: i64,
    pub max_output_tokens: i64,
    pub compaction_threshold: i64,
}

/// The work-unit this session is currently attached to (BoardView →
/// AgentView attach path). `None` when the session is detached.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkUnitContext {
    pub id: String,
    pub title: String,
    pub status: String,
}

/// Provider-specific thinking-config payload. The `config_json` field
/// is the JSON-encoded blob produced by `getThinkingConfig(providerId,
/// level)` on the JS side — store it as a string instead of
/// `serde_json::Value` to keep `codelet-rpc-types` free of any
/// `serde_json` runtime dependency. Mirrors the precedent set by
/// `FspecRequest::args_json`.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingConfig {
    pub provider_id: String,
    pub level: ThinkingLevel,
    pub config_json: String,
}

// ---------------------------------------------------------------------------
// Phase 2.2 — Pause & HITL wire types
// ---------------------------------------------------------------------------

/// Kind of pause the user-facing dialog should render. Wire-portable
/// slice of the internal `codelet_tools::tool_pause::PauseKind` —
/// `Continue` (a loop-control signal) is intentionally omitted.
#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PauseKind {
    Confirm,
    Triple,
}

/// Snapshot of the AgentView pause dialog state — the question to
/// render plus the tool-call ID (if any) the pause is gating.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseState {
    pub kind: PauseKind,
    pub prompt: String,
    pub tool_call_id: Option<String>,
}

/// Response the user can send back to dismiss the pause dialog. Maps
/// 1-to-1 onto the AgentView buttons (`Resume`, `ConfirmAccept`,
/// `ConfirmDeny` for two-choice prompts; `TripleApprove`,
/// `TripleApproveSession`, `TripleDeny` for three-choice prompts).
#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PauseResponse {
    Resume,
    ConfirmAccept,
    ConfirmDeny,
    TripleApprove,
    TripleApproveSession,
    TripleDeny,
}

/// Per-blocklist approval choice surfaced by the triple-pause dialog.
#[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalChoice {
    Approve,
    ApproveSession,
    Deny,
}

/// One option presented to the user inside an HITL question.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitlOption {
    pub label: String,
    pub description: String,
}

/// Single HITL question to render in the AgentView modal — wire-facing
/// slice of `codelet_tools::request_user_input::HitlQuestion`. The
/// internal type wraps multiple questions; the AgentView wire surface
/// represents one question per outgoing request.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitlRequest {
    pub id: String,
    pub question: String,
    pub header: String,
    pub options: Vec<HitlOption>,
    pub allow_text_input: bool,
}

/// User's response to an `HitlRequest`. The `value` field carries
/// either the selected option label or the freeform text the user
/// entered.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitlResponse {
    pub id: String,
    pub value: String,
}

// ---------------------------------------------------------------------------
// Phase 2.4 — Supporting types
// ---------------------------------------------------------------------------

/// Result of `create_isolated_session`: identifies the new session,
/// its git worktree path on disk, and the baseline commit SHA the
/// worktree was forked from. Drives the AgentView isolation badge and
/// the merge/discard flow.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolatedSessionInfo {
    pub session_id: SessionId,
    pub worktree_path: String,
    pub base_commit: String,
}

// ============================================================================
// RPC-007: StreamChunk (23 variants, lifted verbatim from
//   codelet/napi/src/types.rs:217 with #[napi(discriminant = "type")] and
//   every #[napi(js_name = ...)] rename preserved)
// ============================================================================

/// Streaming chunk discriminated union shared by the embedded transport,
/// the WebSocket transport, and the NAPI re-exports.
#[cfg_attr(feature = "napi", napi_derive::napi(discriminant = "type"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamChunk {
    Text {
        text: String,
        #[serde(rename = "correlationId")]
        correlation_id: Option<String>,
        #[serde(rename = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },
    Thinking {
        thinking: String,
        #[serde(rename = "correlationId")]
        correlation_id: Option<String>,
        #[serde(rename = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },
    ToolCall {
        #[serde(rename = "toolCall")]
        tool_call: ToolCallInfo,
        #[serde(rename = "correlationId")]
        correlation_id: Option<String>,
        #[serde(rename = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },
    ToolResult {
        #[serde(rename = "toolResult")]
        tool_result: ToolResultInfo,
        #[serde(rename = "correlationId")]
        correlation_id: Option<String>,
        #[serde(rename = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },
    ToolProgress {
        #[serde(rename = "toolProgress")]
        tool_progress: ToolProgressInfo,
        #[serde(rename = "correlationId")]
        correlation_id: Option<String>,
        #[serde(rename = "observedCorrelationIds")]
        observed_correlation_ids: Option<Vec<String>>,
    },
    SessionStateChange {
        state: SessionState,
    },
    UserNotification {
        message: String,
        severity: NotificationSeverity,
    },
    Interrupted {
        #[serde(rename = "queuedInputs")]
        queued_inputs: Vec<String>,
    },
    TokenUpdate {
        tokens: TokenTracker,
    },
    ContextFillUpdate {
        #[serde(rename = "contextFill")]
        context_fill: ContextFillInfo,
    },
    Done,
    Error {
        error: String,
    },
    UserInput {
        text: String,
    },
    IncomingMessage {
        text: String,
        images: Option<Vec<IncomingMessageImage>>,
    },
    SupervisorPendingInjection {
        #[serde(rename = "supervisorPendingInjection")]
        supervisor_pending_injection: SupervisorPendingInjectionInfo,
    },
    CompactionComplete {
        #[serde(rename = "compactionResult")]
        compaction_result: CompactionResult,
    },
    FspecCommandRequest {
        #[serde(rename = "fspecRequest")]
        fspec_request: FspecRequest,
    },
    FspecCommandResult {
        #[serde(rename = "fspecResult")]
        fspec_result: FspecResult,
    },
    WorkUnitsUpdate {
        #[serde(rename = "workUnits")]
        work_units: Vec<WorkUnitInfo>,
    },
    IsolationStateChange {
        #[serde(rename = "isIsolated")]
        is_isolated: bool,
        #[serde(rename = "worktreePath")]
        worktree_path: Option<String>,
        /// RPC-036: the git commit SHA the worktree was forked from.
        /// `None` when the chunk was emitted without baseline info (e.g.
        /// the legacy 2-arg constructor path). The Rust AgentView uses
        /// this to render the isolation diff against the origin commit.
        #[serde(rename = "baseCommit")]
        base_commit: Option<String>,
    },
    FooterStateUpdate {
        cwd: String,
        #[serde(rename = "displayPath")]
        display_path: String,
        #[serde(rename = "isGitRepo")]
        is_git_repo: bool,
        branch: Option<String>,
    },
    DebugStateChange {
        enabled: bool,
    },
}

impl StreamChunk {
    pub fn text(text: String) -> Self {
        Self::Text {
            text,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    /// Create a thinking/reasoning content chunk (TOOL-010)
    pub fn thinking(thinking: String) -> Self {
        Self::Thinking {
            thinking,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    pub fn tool_call(info: ToolCallInfo) -> Self {
        Self::ToolCall {
            tool_call: info,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    pub fn tool_result(info: ToolResultInfo) -> Self {
        Self::ToolResult {
            tool_result: info,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    /// Tool execution progress - streaming output from bash/shell tools (TOOL-011)
    pub fn tool_progress(info: ToolProgressInfo) -> Self {
        Self::ToolProgress {
            tool_progress: info,
            correlation_id: None,
            observed_correlation_ids: None,
        }
    }

    /// NAPI-010: Create a session state change chunk (internal state, not for conversation)
    pub fn session_state_change(state: SessionState) -> Self {
        Self::SessionStateChange { state }
    }

    /// NAPI-010: Create a user notification chunk (for conversation display)
    pub fn user_notification(message: String, severity: NotificationSeverity) -> Self {
        Self::UserNotification { message, severity }
    }

    pub fn interrupted(queued_inputs: Vec<String>) -> Self {
        Self::Interrupted { queued_inputs }
    }

    pub fn token_update(tokens: TokenTracker) -> Self {
        Self::TokenUpdate { tokens }
    }

    /// Context fill percentage update (TUI-033)
    pub fn context_fill_update(info: ContextFillInfo) -> Self {
        Self::ContextFillUpdate { context_fill: info }
    }

    pub fn done() -> Self {
        Self::Done
    }

    pub fn error(message: String) -> Self {
        Self::Error { error: message }
    }

    /// User input message (NAPI-009: for resume/attach to restore user messages)
    pub fn user_input(text: String) -> Self {
        Self::UserInput { text }
    }

    /// Supervisor input message (WATCH-006: for supervisor injection into subordinate session)
    /// BRIDGE-007: Extended to support optional images
    pub fn incoming_message(formatted_message: String) -> Self {
        Self::IncomingMessage {
            text: formatted_message,
            images: None,
        }
    }

    /// Supervisor input message with images (BRIDGE-007)
    pub fn incoming_message_with_images(
        formatted_message: String,
        images: Vec<IncomingMessageImage>,
    ) -> Self {
        Self::IncomingMessage {
            text: formatted_message,
            images: if images.is_empty() {
                None
            } else {
                Some(images)
            },
        }
    }

    /// Set correlation ID on the chunk (for variants that support it)
    pub fn with_correlation_id(mut self, id: String) -> Self {
        match &mut self {
            Self::Text { correlation_id, .. } => *correlation_id = Some(id),
            Self::Thinking { correlation_id, .. } => *correlation_id = Some(id),
            Self::ToolCall { correlation_id, .. } => *correlation_id = Some(id),
            Self::ToolResult { correlation_id, .. } => *correlation_id = Some(id),
            Self::ToolProgress { correlation_id, .. } => *correlation_id = Some(id),
            // Other variants don't have correlation_id
            _ => {}
        }
        self
    }

    /// Set observed correlation IDs for supervisor response chunks (WATCH-011)
    pub fn with_observed_correlation_ids(mut self, ids: Vec<String>) -> Self {
        match &mut self {
            Self::Text {
                observed_correlation_ids,
                ..
            } => *observed_correlation_ids = Some(ids),
            Self::Thinking {
                observed_correlation_ids,
                ..
            } => *observed_correlation_ids = Some(ids),
            Self::ToolCall {
                observed_correlation_ids,
                ..
            } => *observed_correlation_ids = Some(ids),
            Self::ToolResult {
                observed_correlation_ids,
                ..
            } => *observed_correlation_ids = Some(ids),
            Self::ToolProgress {
                observed_correlation_ids,
                ..
            } => *observed_correlation_ids = Some(ids),
            // Other variants don't have observed_correlation_ids
            _ => {}
        }
        self
    }

    /// Supervisor pending injection - when auto_inject=false (WATCH-020)
    pub fn supervisor_pending_injection(urgent: bool, content: String) -> Self {
        Self::SupervisorPendingInjection {
            supervisor_pending_injection: SupervisorPendingInjectionInfo { urgent, content },
        }
    }

    /// UX-002: Compaction completed with structured result
    pub fn compaction_complete(result: CompactionResult) -> Self {
        Self::CompactionComplete {
            compaction_result: result,
        }
    }

    /// CODE-009: Fspec command request - sent to TypeScript for execution
    pub fn fspec_command_request(request: FspecRequest) -> Self {
        Self::FspecCommandRequest {
            fspec_request: request,
        }
    }

    /// CODE-009: Fspec command result - sent after TypeScript executes command
    pub fn fspec_command_result(result: FspecResult) -> Self {
        Self::FspecCommandResult {
            fspec_result: result,
        }
    }

    /// Work units updated - emitted by global file watcher
    pub fn work_units_update(work_units: Vec<WorkUnitInfo>) -> Self {
        Self::WorkUnitsUpdate { work_units }
    }

    /// GIT-029 / RPC-036: Isolation state change - emitted when session
    /// isolation state changes. The optional `base_commit` parameter
    /// carries the git SHA the worktree was forked from; pre-RPC-036
    /// callers using the 2-arg form get `base_commit = None`.
    pub fn isolation_state_change(is_isolated: bool, worktree_path: Option<String>) -> Self {
        Self::IsolationStateChange {
            is_isolated,
            worktree_path,
            base_commit: None,
        }
    }

    /// RPC-036: Isolation state change including the baseline commit
    /// SHA the worktree was forked from. Use this constructor when
    /// emitting from a code path that has the baseline SHA in hand
    /// (e.g. `SessionManager::create_isolated_session`).
    pub fn isolation_state_change_with_base(
        is_isolated: bool,
        worktree_path: Option<String>,
        base_commit: Option<String>,
    ) -> Self {
        Self::IsolationStateChange {
            is_isolated,
            worktree_path,
            base_commit,
        }
    }

    /// TUI-091: Footer state update - emitted by background poller
    pub fn footer_state_update(
        cwd: String,
        display_path: String,
        is_git_repo: bool,
        branch: Option<String>,
    ) -> Self {
        Self::FooterStateUpdate {
            cwd,
            display_path,
            is_git_repo,
            branch,
        }
    }

    /// BUG-134: Debug state change - emitted when session debug capture toggles
    pub fn debug_state_change(enabled: bool) -> Self {
        Self::DebugStateChange { enabled }
    }
}

// ============================================================================
// RPC-036: JSON round-trip test suite for every wire-portable shape added by
// this card. Lives inline under `#[cfg(test)] mod tests` (per RPC-036 rule
// [6] / architecture note [4]) so that codelet-rpc-types ships its own first
// unit-test suite establishing the types are wire-portable by construction.
//
// The integration-test counterpart at codelet/rpc-types/tests/rpc036_widen_types.rs
// exercises the same surface from outside the crate (which additionally
// proves every type is publicly re-exported from the crate root).
// ============================================================================
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::uninlined_format_args
)]
mod tests {
    use super::*;

    /// Helper: serialize `value` to JSON, deserialize it back, and assert
    /// the round-tripped value equals the original. Every new RPC-036 type
    /// implements `PartialEq + Debug + Serialize + Deserialize`, so this
    /// helper is uniformly applicable.
    fn round_trip<T>(value: T)
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + PartialEq + core::fmt::Debug,
    {
        let json = serde_json::to_string(&value).expect("serialize must succeed");
        let back: T = serde_json::from_str(&json).expect("deserialize must succeed");
        assert_eq!(back, value);
    }

    #[test]
    fn session_tokens_round_trips() {
        round_trip(SessionTokens {
            input_tokens: 1024_i64,
            output_tokens: 512_i64,
        });
    }

    #[test]
    fn token_restore_state_round_trips() {
        round_trip(TokenRestoreState {
            current_context: 1,
            cumulative_billed_output: 2,
            cache_read: 3,
            cache_creation: 4,
            cumulative_billed_input: 5,
            cumulative_billed_output_second: 6,
        });
    }

    #[test]
    fn session_model_round_trips() {
        round_trip(SessionModel {
            provider_id: "openai".to_string(),
            model_id: "gpt-4o".to_string(),
            context_window: 128_000,
            max_output_tokens: 4_096,
            compaction_threshold: 96_000,
        });
    }

    #[test]
    fn work_unit_context_round_trips() {
        round_trip(WorkUnitContext {
            id: "RPC-036".to_string(),
            title: "Widen rpc-types".to_string(),
            status: "testing".to_string(),
        });
    }

    #[test]
    fn thinking_config_round_trips() {
        round_trip(ThinkingConfig {
            provider_id: "anthropic".to_string(),
            level: ThinkingLevel::High,
            config_json: r#"{"type":"enabled","budget_tokens":8000}"#.to_string(),
        });
    }

    #[test]
    fn pause_kind_round_trips_both_variants() {
        round_trip(PauseKind::Confirm);
        round_trip(PauseKind::Triple);

        // String-enum wire shape: `"Confirm"` / `"Triple"`.
        assert_eq!(
            serde_json::to_string(&PauseKind::Confirm).expect("serialize Confirm"),
            "\"Confirm\""
        );
        assert_eq!(
            serde_json::to_string(&PauseKind::Triple).expect("serialize Triple"),
            "\"Triple\""
        );
    }

    #[test]
    fn pause_state_round_trips_with_and_without_tool_call_id() {
        round_trip(PauseState {
            kind: PauseKind::Confirm,
            prompt: "Apply changes?".to_string(),
            tool_call_id: Some("tc-1".to_string()),
        });
        round_trip(PauseState {
            kind: PauseKind::Triple,
            prompt: "Run command?".to_string(),
            tool_call_id: None,
        });
    }

    #[test]
    fn pause_response_round_trips_every_variant() {
        round_trip(PauseResponse::Resume);
        round_trip(PauseResponse::ConfirmAccept);
        round_trip(PauseResponse::ConfirmDeny);
        round_trip(PauseResponse::TripleApprove);
        round_trip(PauseResponse::TripleApproveSession);
        round_trip(PauseResponse::TripleDeny);
    }

    #[test]
    fn approval_choice_round_trips_every_variant() {
        round_trip(ApprovalChoice::Approve);
        round_trip(ApprovalChoice::ApproveSession);
        round_trip(ApprovalChoice::Deny);
    }

    #[test]
    fn hitl_option_round_trips() {
        round_trip(HitlOption {
            label: "Yes".to_string(),
            description: "Proceed".to_string(),
        });
    }

    #[test]
    fn hitl_request_round_trips_with_multiple_options_and_text_input() {
        round_trip(HitlRequest {
            id: "q-1".to_string(),
            question: "Apply changes?".to_string(),
            header: "Apply".to_string(),
            options: vec![
                HitlOption {
                    label: "Yes".to_string(),
                    description: "Proceed".to_string(),
                },
                HitlOption {
                    label: "No".to_string(),
                    description: "Stop".to_string(),
                },
            ],
            allow_text_input: true,
        });
    }

    #[test]
    fn hitl_response_round_trips() {
        round_trip(HitlResponse {
            id: "q-1".to_string(),
            value: "Yes".to_string(),
        });
    }

    #[test]
    fn isolated_session_info_round_trips() {
        round_trip(IsolatedSessionInfo {
            session_id: SessionId::new("uuid-1"),
            worktree_path: "/tmp/wt".to_string(),
            base_commit: "abc1234".to_string(),
        });
    }

    #[test]
    fn isolation_state_change_round_trips_with_base_commit_some() {
        let chunk = StreamChunk::IsolationStateChange {
            is_isolated: true,
            worktree_path: Some("/tmp/wt".to_string()),
            base_commit: Some("abc1234".to_string()),
        };
        let json = serde_json::to_string(&chunk).expect("serialize");
        let back: StreamChunk = serde_json::from_str(&json).expect("deserialize");
        match back {
            StreamChunk::IsolationStateChange {
                is_isolated,
                worktree_path,
                base_commit,
            } => {
                assert!(is_isolated);
                assert_eq!(worktree_path.as_deref(), Some("/tmp/wt"));
                assert_eq!(base_commit.as_deref(), Some("abc1234"));
            }
            other => panic!("expected IsolationStateChange, got {other:?}"),
        }
    }

    #[test]
    fn isolation_state_change_round_trips_with_base_commit_none() {
        // Backward-compat: pre-RPC-036 callers using the 2-arg constructor
        // get `base_commit = None`; the variant must still round-trip.
        let chunk = StreamChunk::isolation_state_change(false, None);
        let json = serde_json::to_string(&chunk).expect("serialize");
        let back: StreamChunk = serde_json::from_str(&json).expect("deserialize");
        match back {
            StreamChunk::IsolationStateChange {
                is_isolated,
                worktree_path,
                base_commit,
            } => {
                assert!(!is_isolated);
                assert!(worktree_path.is_none());
                assert!(base_commit.is_none());
            }
            other => panic!("expected IsolationStateChange, got {other:?}"),
        }
    }

    // ========================================================================
    // RPC-338: ProviderInfo profile/unreachable wire fields.
    // Feature: spec/features/model-selector-profile-wire-types.feature
    // ========================================================================

    /// Scenario: ProviderInfo carries profile and reachability fields over the wire
    #[test]
    fn provider_info_carries_profile_and_reachability_fields() {
        // @step Given a ProviderInfo value constructed with its derived Default
        let info = ProviderInfo::default();

        // @step Then its profile_name field is None
        assert_eq!(info.profile_name, None);

        // @step And its is_unreachable field is false
        assert!(!info.is_unreachable);

        // @step And the field profile_name has type Option<String>
        let with_profile = ProviderInfo {
            key: "openai:my-profile".to_string(),
            display_name: "openai: my-profile".to_string(),
            models: Vec::new(),
            profile_name: Some("my-profile".to_string()),
            is_unreachable: true,
        };
        let _typed: Option<String> = with_profile.profile_name.clone();
        assert_eq!(with_profile.profile_name.as_deref(), Some("my-profile"));

        // @step And the field is_unreachable has type bool
        let _flag: bool = with_profile.is_unreachable;
        assert!(with_profile.is_unreachable);

        // The new fields must survive a JSON round-trip like every other
        // RPC wire type.
        round_trip(with_profile);
    }
}
