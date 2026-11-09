//! Modal-dialog wire types: providers and model entries (RPC-022).

use super::*;

// ============================================================================
// RPC-022: Modal dialog shared types — ProviderInfo + ModelEntry consumed by
// the new `list_providers`, `set_session_model`, `set_thinking_level`,
// `get_session_role`, `set_session_role` RPC methods.
// ============================================================================

/// One entry in the per-provider model list returned by
/// `FspecService::list_providers`. Mirrors the relevant fields of the
/// TS `NapiModelInfo` shape (rust/napi/src/models/napi_bindings.rs)
/// so the Rust ratatui `ModelSelectorDialog` can paint capability
/// badges using the SAME source of truth that the Ink
/// `ModelSelectorView.tsx` consumes.
///
/// `Default` returns a "blank" entry — `display_name` empty, every
/// capability false, `context_window` 0, `is_custom` false. The
/// `ModelSelectorDialog` hides empty rows so the UI degrades
/// gracefully when no session manager is attached (the RPC-022
/// default-impl path).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub display_name: String,
    /// PROV-169: `None` when neither the model nor its profile/catalog
    /// declares a context window.
    pub context_window: Option<u32>,
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ProfileDefinition {
    pub base_url: String,
    pub api_key: String,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub compaction_threshold_type: Option<String>,
    pub compaction_threshold_value: Option<u32>,
    /// PROV-139: per-profile streaming toggle. `None` (the on-disk default when
    /// the `streaming` key is absent) means streaming is ENABLED — see
    /// [`ProfileDefinition::streaming_enabled`]. Carried as a flat
    /// `Option<bool>` so the `napi(object)` projection stays a plain struct,
    /// mirroring the compaction-threshold override fields above.
    pub streaming: Option<bool>,
    /// PROV-142: per-profile auto-continue default. The value encodes both the
    /// on/off state and the budget: `None` (key absent on disk) or `Some(0)`
    /// means auto-continue OFF (session starts with `continue_enabled = false`,
    /// today's behavior); `Some(n)` with `n >= 1` means ON with budget `n`
    /// (session starts as if the user had run `/continue n`). Carried as a
    /// flat `Option<u32>` so the `napi(object)` projection stays a plain
    /// struct, mirroring the `context_window` / `max_output_tokens` fields.
    pub auto_continue: Option<u32>,
    /// PROV-143: per-profile preserve-thinking toggle. `None` (key absent on
    /// disk) or `Some(false)` mean thinking blocks are STRIPPED from the chat
    /// history sent back to the LLM (the default — new profiles are
    /// seeded with the form's disabled value); `Some(true)` preserves the
    /// `AssistantContent::Reasoning` blocks in the outgoing history. Carried
    /// as a flat `Option<bool>` like `streaming` so the `napi(object)`
    /// projection stays a plain struct.
    pub preserve_thinking: Option<bool>,
    /// PROV-144: per-profile Max Images limit for the Read tool's image
    /// budget. `None` (key absent on disk) resolves to the default of 4
    /// images per tool result; `Some(0)` means the profile's model has no
    /// vision (the Read tool fails image reads); `Some(n)` with `n >= 1`
    /// caps the images a single Read tool result may return at `n`.
    /// Carried as a flat `Option<u32>` like `auto_continue` so the
    /// `napi(object)` projection stays a plain struct.
    pub max_images: Option<u32>,
    /// PROV-145: per-profile loop-detection toggle. `None` (key absent on
    /// disk) means the RIG-014 streaming loop detector stays ENABLED
    /// (today's always-on behavior); `Some(false)` disables it for the
    /// profile's sessions; `Some(true)` is the explicit on. Carried as a
    /// flat `Option<bool>` like `streaming` so the `napi(object)`
    /// projection stays a plain struct.
    pub loop_detection_enabled: Option<bool>,
    /// PROV-145: per-profile loop-detector sliding window in words. `None`
    /// (absent) resolves to the RIG-014 default of 160.
    pub loop_detection_window: Option<u32>,
    /// PROV-145: per-profile tail n-gram repeat threshold. `None` (absent)
    /// resolves to the RIG-014 default of 10.
    pub loop_detection_max_repeats: Option<u32>,
    /// PROV-145: per-profile max auto-continue retries after a loop abort
    /// before the agent loop gives up and waits for real user input.
    /// `None` (absent) resolves to the RIG-014 default of 10; `Some(0)` is
    /// the explicit never-retry sentinel.
    pub loop_detection_max_retries: Option<u32>,
}

impl ProfileDefinition {
    /// PROV-139: canonical "is streaming on?" predicate — the single source of
    /// truth for the "absent ⇒ enabled" semantics. Returns `true` when
    /// [`streaming`](Self::streaming) is `None` or `Some(true)`, `false` only
    /// for an explicit `Some(false)`.
    pub fn streaming_enabled(&self) -> bool {
        self.streaming.unwrap_or(true)
    }

    /// PROV-142: canonical "is auto-continue on?" predicate — the single
    /// source of truth for the "0 or absent ⇒ off" semantics. Returns `true`
    /// only when [`auto_continue`](Self::auto_continue) is `Some(n)` with
    /// `n >= 1`; `None` and `Some(0)` both mean off.
    pub fn auto_continue_enabled(&self) -> bool {
        self.auto_continue.is_some_and(|n| n >= 1)
    }

    /// PROV-143: canonical "is preserve-thinking on?" predicate — the single
    /// source of truth for the "absent ⇒ stripped" semantics. Returns `true`
    /// only for an explicit `Some(true)`; `None` and `Some(false)` both mean
    /// thinking blocks are stripped from the outgoing chat history.
    pub fn preserve_thinking_enabled(&self) -> bool {
        self.preserve_thinking.unwrap_or(false)
    }

    /// PROV-144: canonical "effective Max Images limit" predicate — the
    /// single source of truth for the "absent ⇒ default 4" semantics.
    /// Returns the default of 4 when [`max_images`](Self::max_images) is
    /// `None` (key absent on disk, including pre-existing profiles), and the
    /// stored value `n` for `Some(n)` — including the explicit `Some(0)`
    /// no-vision sentinel (the Read tool fails image reads at 0).
    pub fn max_images_limit(&self) -> u32 {
        self.max_images.unwrap_or(4)
    }

    /// PROV-145: canonical "is loop detection on?" predicate — the single
    /// source of truth for the "absent ⇒ enabled" semantics. Returns
    /// `true` when [`loop_detection_enabled`](Self::loop_detection_enabled)
    /// is `None` or `Some(true)` (today's always-on behavior), `false` only
    /// for an explicit `Some(false)`.
    pub fn loop_detection_enabled(&self) -> bool {
        self.loop_detection_enabled.unwrap_or(true)
    }

    /// PROV-145: canonical "effective loop-detector window" predicate — the
    /// stored value, or the RIG-014 default of 160 words when the key is
    /// absent (including pre-existing profiles).
    pub fn loop_detection_window(&self) -> u32 {
        self.loop_detection_window.unwrap_or(160)
    }

    /// PROV-145: canonical "effective tail n-gram repeat threshold"
    /// predicate — the stored value, or the RIG-014 default of 10 when the
    /// key is absent.
    pub fn loop_detection_max_repeats(&self) -> u32 {
        self.loop_detection_max_repeats.unwrap_or(10)
    }

    /// PROV-145: canonical "effective loop-abort retry cap" predicate — the
    /// stored value, or the RIG-014 default of 10 when the key is absent.
    /// An explicit `Some(0)` stays 0 (never auto-retry).
    pub fn loop_detection_max_retries(&self) -> u32 {
        self.loop_detection_max_retries.unwrap_or(10)
    }
}

/// One provider's display metadata plus its set of available models —
/// returned (in a Vec) by `FspecService::list_providers`. Mirrors the
/// TS `NapiProviderModels` shape (rust/napi/src/models/napi_bindings.rs)
/// so the Rust ratatui `ModelSelectorDialog` and the Ink
/// `ModelSelectorView.tsx` consume the same provider/model tree.
///
/// The `key` field is the stable provider identifier (e.g. "openai",
/// "anthropic") passed back through `set_session_model`. The
/// `display_name` is the human-readable label rendered in the
/// provider rows.
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
    /// BUG-325: the base provider id this section belongs to (e.g.
    /// `"openai"` for the `"openai:my-profile"` profile section; equal to
    /// `key` for cloud and custom providers). Clients pass it as the
    /// `provider_id` of the custom-model and profile write RPCs — never the
    /// section `key`.
    pub provider_id: String,
    /// PROV-176: why a section that should list models lists none (the Codex
    /// section when Codex /models cannot be fetched). `None` otherwise.
    /// Always serialized: the RPC wire is bincode.
    #[serde(default)]
    pub unavailable_reason: Option<String>,
}
