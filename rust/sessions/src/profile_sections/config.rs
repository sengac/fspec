//! Profile and custom-model configuration types (`~/.fspec/fspec-config.json`).

use codelet_providers::local_reasoning::ThinkingLevelsDeclaration;
use serde::{Deserialize, Serialize};

/// A single local-server profile as stored under
/// `~/.fspec/fspec-config.json` → `providers.openai.profiles.<name>`.
/// Mirrors the TS `ProfileConfig` fields consumed by `loadProfileSections`.
#[derive(Debug, Clone, Deserialize)]
pub struct LocalServerProfile {
    /// Profile name (the map key).
    #[serde(skip)]
    pub name: String,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    /// Custom models declared directly on the profile (MODEL-004). Each
    /// entry's `id` is all the renderer needs; extra fields are ignored.
    #[serde(rename = "customModels", default)]
    pub custom_models: Vec<CustomModelDef>,
    /// MODEL-016: the inference engine the profile's server runs
    /// (`vllm`, `sglang`, `strata`, `llama-cpp`, `ollama`, `lm-studio`,
    /// `tgi`, `tensorrt-llm` or `lmdeploy`). Never inferred.
    pub engine: Option<String>,
    /// MODEL-016: thinking levels every model of the profile takes, unless
    /// its customModels entry declares its own.
    #[serde(rename = "thinkingLevels")]
    pub thinking_levels: Option<ThinkingLevelsDeclaration>,
    #[serde(
        rename = "contextWindow",
        default,
        deserialize_with = "de_opt_u32_lenient"
    )]
    pub context_window: Option<u32>,
    /// PROV-140: per-profile streaming toggle. `None` (absent) means streaming
    /// stays enabled (the provider default); `Some(false)` selects the
    /// non-streaming request path.
    #[serde(rename = "streaming", default)]
    pub streaming: Option<bool>,
    /// PROV-142: per-profile auto-continue default. `None` (absent) or `0`
    /// mean OFF; `Some(n)` with `n >= 1` means ON with budget `n`.
    #[serde(
        rename = "autoContinue",
        default,
        deserialize_with = "de_opt_u32_lenient"
    )]
    pub auto_continue: Option<u32>,
    /// PROV-143: per-profile preserve-thinking toggle. `None` (absent) or
    /// `Some(false)` mean thinking blocks are STRIPPED from the chat history
    /// sent back to the LLM (the default); `Some(true)` preserves them.
    #[serde(rename = "preserveThinking", default)]
    pub preserve_thinking: Option<bool>,
    /// PROV-144: per-profile Max Images limit for the Read tool's image
    /// budget. `None` (absent, including pre-existing profiles) resolves to
    /// the tool-layer default of 4; `Some(0)` is the explicit no-vision
    /// sentinel; `Some(n)` with `n >= 1` caps a single Read result at `n`
    /// images. Lenient u32 deserialization so a TS-written float (e.g. `2.0`)
    /// saturates rather than dropping the whole profile.
    #[serde(rename = "maxImages", default, deserialize_with = "de_opt_u32_lenient")]
    pub max_images: Option<u32>,
    /// PROV-145: per-profile loop-detection toggle. `None` (absent, including
    /// pre-existing profiles) keeps the RIG-014 detector enabled (today's
    /// always-on behavior); `Some(false)` disables it for the profile's
    /// sessions.
    #[serde(rename = "loopDetectionEnabled", default)]
    pub loop_detection_enabled: Option<bool>,
    /// PROV-145: per-profile loop-detector sliding window in words. `None`
    /// (absent) resolves to the RIG-014 default of 160. Lenient u32
    /// deserialization so a TS-written float (e.g. `320.0`) saturates rather
    /// than dropping the whole profile.
    #[serde(
        rename = "loopDetectionWindow",
        default,
        deserialize_with = "de_opt_u32_lenient"
    )]
    pub loop_detection_window: Option<u32>,
    /// PROV-145: per-profile tail n-gram repeat threshold. `None` (absent)
    /// resolves to the RIG-014 default of 10. Lenient u32 deserialization
    /// (float-tolerant), like [`maxImages`](Self::max_images).
    #[serde(
        rename = "loopDetectionMaxRepeats",
        default,
        deserialize_with = "de_opt_u32_lenient"
    )]
    pub loop_detection_max_repeats: Option<u32>,
    /// PROV-145: per-profile max auto-continue retries after a loop abort.
    /// `None` (absent) resolves to the RIG-014 default of 10; `Some(0)` is
    /// the explicit never-retry sentinel. Lenient u32 deserialization, like
    /// [`maxImages`](Self::max_images).
    #[serde(
        rename = "loopDetectionMaxRetries",
        default,
        deserialize_with = "de_opt_u32_lenient"
    )]
    pub loop_detection_max_retries: Option<u32>,
}

/// A custom model declared on a profile (`profile.customModels[]`).
///
/// RPC-346: extended from the read-only `{ id }` shape to the full
/// definition mirroring the TS `CustomModelDefinition`
/// (provider-config.ts:95-112). All fields besides `id` are optional and
/// omitted from the serialized JSON when `None`, keeping existing config
/// files backward-compatible. Wire names are camelCase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomModelDef {
    /// Model ID string sent to the API (required).
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display_name: Option<String>,
    /// Facade override for tool-schema selection
    /// (`openai` | `codex` | `claude` | `gemini` | `zai`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub facade: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "de_opt_u32_lenient"
    )]
    pub context_window: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "de_opt_u32_lenient"
    )]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub compaction_threshold: Option<CompactionThreshold>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reasoning: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub has_vision: Option<bool>,
    /// MODEL-016: the thinking levels this model takes; replaces the
    /// profile-wide declaration for it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_levels: Option<ThinkingLevelsDeclaration>,
}

impl LocalServerProfile {
    /// MODEL-016: the reasoning `model` declares on this profile; `Ok(None)`
    /// when nothing is declared for it (no reasoning control). An error is
    /// this model's only: other models of the profile still resolve.
    pub fn model_reasoning(
        &self,
        model: &str,
    ) -> Result<Option<codelet_providers::local_reasoning::LocalReasoning>, String> {
        let per_model = self
            .custom_models
            .iter()
            .find(|definition| definition.id == model)
            .and_then(|definition| definition.thinking_levels.as_ref());
        codelet_providers::local_reasoning::resolve_local_reasoning(
            &self.name,
            model,
            self.engine.as_deref(),
            self.thinking_levels.as_ref(),
            per_model,
        )
    }
}

/// CTX-008 compaction-threshold override on a custom model. Mirrors the TS
/// `CompactionThresholdConfig` (`{ type: 'tokens' | 'percentage', value }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionThreshold {
    /// `"tokens"` (absolute count) or `"percentage"` (1-100 of context window).
    #[serde(rename = "type")]
    pub threshold_type: String,
    #[serde(deserialize_with = "de_u32_lenient")]
    pub value: u32,
}

/// PROV-175: an optional `u32` profile number. A whole number from 0 to
/// `u32::MAX` is accepted, and an integral float such as `128000.0` is the
/// same number (an explicit rule: JSON writers emit both); a fractional,
/// negative, oversized or non-numeric value is an error naming it.
fn de_opt_u32_lenient<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<serde_json::Value>::deserialize(deserializer)?
        .map(|value| strict_u32(&value).map_err(serde::de::Error::custom))
        .transpose()
}

/// PROV-175: a required `u32` profile number (see [`de_opt_u32_lenient`]).
fn de_u32_lenient<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    strict_u32(&value).map_err(serde::de::Error::custom)
}

/// PROV-175: `value` as a `u32`, or an error naming it.
fn strict_u32(value: &serde_json::Value) -> Result<u32, String> {
    let invalid = || format!("{value} is not a whole number from 0 to {}", u32::MAX);
    let serde_json::Value::Number(number) = value else {
        return Err(invalid());
    };
    if let Some(whole) = number.as_u64() {
        return u32::try_from(whole).map_err(|_| invalid());
    }
    match number.as_f64() {
        Some(float) if float.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&float) => {
            Ok(float as u32)
        }
        _ => Err(invalid()),
    }
}
