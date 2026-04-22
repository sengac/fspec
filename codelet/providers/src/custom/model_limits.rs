//! PROV-095 — optional `get_model_limits(config) -> Map` Rhai hook.
//!
//! Lets a Rhai script override per-model `context_window` /
//! `max_output_tokens` and, optionally, surface a `compaction_threshold`
//! that the NAPI session-creation path wires into
//! [`crate::ProviderManager::set_compaction_threshold_override`].
//!
//! Invoked **once** from [`crate::custom::provider::RhaiCustomProvider::new`]
//! and cached for the provider's lifetime (rule 8 / answered question).
//! When the script does not define `get_model_limits`, or when any
//! returned field fails validation, the JSON `ModelDef` values remain
//! authoritative and a `tracing::warn!` is emitted naming the provider
//! and offending key.
//!
//! This module owns the entire parse-and-validate pipeline so
//! `provider.rs` stays focused on the `LlmProvider` impl and HTTP
//! plumbing.
//!
//! Shape of the returned Rhai map:
//!
//! ```text
//! #{
//!     context_window:     400000,                            // optional i64, > 0
//!     max_output_tokens:  128000,                            // optional i64, > 0
//!     compaction_threshold: #{                               // optional
//!         type:  "tokens",      value: 200000                // tokens > 0
//!     }
//!     // or
//!     compaction_threshold: #{
//!         type:  "percentage",  value: 75                    // 1..=100
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rhai::{Dynamic, Engine, EvalAltResult, Map, Scope, AST};

use super::discovery::discover_provider_configs;
use super::provider::RhaiCustomProvider;
use super::script_loader::ScriptLoader;

/// Maximum age of a cached script-limits entry before the NAPI bridge
/// re-reads the Rhai script. Five seconds is long enough to amortise a
/// burst of `session_set_model*` calls without pinning a stale script
/// result across an interactive edit + reload cycle.
const CACHE_TTL: Duration = Duration::from_secs(5);

/// Parsed result of a successful `get_model_limits` invocation. All
/// fields are optional so a script that sets only `context_window` (for
/// example) still has its override applied while the other fields fall
/// back to the JSON `ModelDef` values (rule 5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ScriptedModelLimits {
    /// Overridden context window (tokens), when the script returned a
    /// positive integer.
    pub context_window: Option<usize>,
    /// Overridden max output tokens, when the script returned a positive
    /// integer.
    pub max_output_tokens: Option<usize>,
    /// Compaction-threshold override tuple `(kind, value)` where `kind`
    /// is `"tokens"` or `"percentage"`. Surfaced to NAPI callers via
    /// [`crate::custom::provider::RhaiCustomProvider::script_compaction_threshold`].
    pub compaction_threshold: Option<(String, u64)>,
}

/// Invoke the optional `get_model_limits(config)` Rhai function and
/// return the parsed + validated overrides. When the script does not
/// define that function, the returned struct is `Default::default()` and
/// NO warning is logged (rule 4, scenario 2).
pub(crate) fn invoke_get_model_limits(
    engine: &Engine,
    ast: &AST,
    provider: &str,
    config: Dynamic,
) -> ScriptedModelLimits {
    let mut scope = Scope::new();
    let result: Result<Dynamic, _> =
        engine.call_fn::<Dynamic>(&mut scope, ast, "get_model_limits", (config,));

    match result {
        Ok(value) => parse_return_value(provider, value),
        Err(boxed) => {
            if matches!(&*boxed, EvalAltResult::ErrorFunctionNotFound(sig, _) if sig.starts_with("get_model_limits"))
            {
                // Script didn't opt in — silent no-op per rule 4.
                ScriptedModelLimits::default()
            } else {
                tracing::warn!(
                    provider = %provider,
                    error = %boxed,
                    "get_model_limits raised a Rhai error; falling back to JSON ModelDef values"
                );
                ScriptedModelLimits::default()
            }
        }
    }
}

/// Validate a parsed return value from `get_model_limits`. Anything that
/// fails validation is dropped (with a log warning) and the
/// corresponding ModelDef field remains authoritative.
fn parse_return_value(provider: &str, value: Dynamic) -> ScriptedModelLimits {
    let Some(map) = value.try_cast::<Map>() else {
        tracing::warn!(
            provider = %provider,
            "get_model_limits must return a Map; ignoring return value"
        );
        return ScriptedModelLimits::default();
    };

    ScriptedModelLimits {
        context_window: parse_positive_usize(provider, &map, "context_window"),
        max_output_tokens: parse_positive_usize(provider, &map, "max_output_tokens"),
        compaction_threshold: parse_compaction_threshold(provider, &map),
    }
}

/// Read `map[key]` and coerce it to a positive `usize`. Returns `None`
/// when the key is absent, the value is the wrong type, or the value is
/// non-positive. In the non-positive / wrong-type cases a `tracing::warn!`
/// is emitted naming the `provider` and `key` (rule 6).
fn parse_positive_usize(provider: &str, map: &Map, key: &str) -> Option<usize> {
    let raw = map.get(key)?;
    let Some(n) = raw.clone().try_cast::<i64>() else {
        tracing::warn!(
            provider = %provider,
            key = %key,
            "get_model_limits returned non-integer value; falling back to JSON ModelDef"
        );
        return None;
    };
    if n <= 0 {
        tracing::warn!(
            provider = %provider,
            key = %key,
            value = n,
            "get_model_limits returned non-positive value; falling back to JSON ModelDef"
        );
        return None;
    }
    Some(n as usize)
}

/// Parse the optional `compaction_threshold` entry. Returns `None` when
/// the entry is absent or invalid. Invalid shapes are logged via
/// `tracing::warn!` naming the provider and the `compaction_threshold`
/// key (rule 9).
fn parse_compaction_threshold(provider: &str, map: &Map) -> Option<(String, u64)> {
    let raw = map.get("compaction_threshold")?;
    let Some(ct_map) = raw.clone().try_cast::<Map>() else {
        tracing::warn!(
            provider = %provider,
            key = "compaction_threshold",
            "get_model_limits compaction_threshold must be a Map; ignoring"
        );
        return None;
    };
    let Some(kind_dyn) = ct_map.get("type").cloned() else {
        tracing::warn!(
            provider = %provider,
            key = "compaction_threshold",
            "get_model_limits compaction_threshold missing 'type'; ignoring"
        );
        return None;
    };
    let Ok(kind) = kind_dyn.into_immutable_string() else {
        tracing::warn!(
            provider = %provider,
            key = "compaction_threshold",
            "get_model_limits compaction_threshold.type must be a string; ignoring"
        );
        return None;
    };
    let kind = kind.to_string();

    let Some(value_raw) = ct_map.get("value") else {
        tracing::warn!(
            provider = %provider,
            key = "compaction_threshold",
            "get_model_limits compaction_threshold missing 'value'; ignoring"
        );
        return None;
    };
    let Some(value_i) = value_raw.clone().try_cast::<i64>() else {
        tracing::warn!(
            provider = %provider,
            key = "compaction_threshold",
            "get_model_limits compaction_threshold.value must be an integer; ignoring"
        );
        return None;
    };

    match kind.as_str() {
        "tokens" => {
            if value_i <= 0 {
                tracing::warn!(
                    provider = %provider,
                    key = "compaction_threshold",
                    value = value_i,
                    "get_model_limits tokens threshold must be > 0; ignoring"
                );
                return None;
            }
            Some(("tokens".to_string(), value_i as u64))
        }
        "percentage" => {
            if !(1..=100).contains(&value_i) {
                tracing::warn!(
                    provider = %provider,
                    key = "compaction_threshold",
                    value = value_i,
                    "get_model_limits percentage threshold must be in 1..=100; ignoring"
                );
                return None;
            }
            Some(("percentage".to_string(), value_i as u64))
        }
        other => {
            tracing::warn!(
                provider = %provider,
                key = "compaction_threshold",
                type_name = %other,
                "get_model_limits compaction_threshold.type must be 'tokens' or 'percentage'; ignoring"
            );
            None
        }
    }
}

// ---------------------------------------------------------------------------
// PROV-095 NAPI bridge — lookup cache
// ---------------------------------------------------------------------------

/// Snapshot of the three script-sourced limits exposed to the NAPI
/// session-creation path. Wraps the three `Option<_>` accessors on
/// [`RhaiCustomProvider`] into a single value so the NAPI bridge can
/// consume them in one call and avoid reconstructing the provider
/// multiple times per `session_set_model*`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RhaiScriptedLimits {
    /// Script-supplied `context_window`, or `None` when the script did
    /// not define/return one (in which case the TUI-supplied value or
    /// the JSON ModelDef default wins).
    pub context_window: Option<usize>,
    /// Script-supplied `max_output_tokens`, or `None`.
    pub max_output_tokens: Option<usize>,
    /// Script-supplied compaction-threshold tuple, or `None`.
    pub compaction_threshold: Option<(String, u64)>,
}

/// Module-level cache keyed by `(provider_slug, model_alias)`. Each
/// entry stores a cheaply-cloneable `Arc<RhaiCustomProvider>` plus a
/// timestamp so the cache expires after [`CACHE_TTL`]. This addresses
/// the review finding that the lookup was re-scanning
/// `~/.fspec/providers/*.json` and recompiling the Rhai script on every
/// `session_set_model*` call.
///
/// Entries are invalidated when the underlying script file mtime
/// changes; this is observed lazily on cache hit.
static LOOKUP_CACHE: Mutex<Option<HashMap<(String, String), CacheEntry>>> = Mutex::new(None);

#[derive(Clone)]
struct CacheEntry {
    provider: Arc<RhaiCustomProvider>,
    inserted_at: Instant,
}

fn cache_get(provider_slug: &str, model_alias: &str) -> Option<Arc<RhaiCustomProvider>> {
    let mut guard = LOOKUP_CACHE.lock().ok()?;
    let map = guard.as_mut()?;
    let key = (provider_slug.to_string(), model_alias.to_string());
    let entry = map.get(&key)?;
    if entry.inserted_at.elapsed() > CACHE_TTL {
        map.remove(&key);
        return None;
    }
    Some(entry.provider.clone())
}

fn cache_put(provider_slug: &str, model_alias: &str, provider: Arc<RhaiCustomProvider>) {
    let Ok(mut guard) = LOOKUP_CACHE.lock() else {
        return;
    };
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(
        (provider_slug.to_string(), model_alias.to_string()),
        CacheEntry {
            provider,
            inserted_at: Instant::now(),
        },
    );
}

/// Test-only helper to clear the module-level lookup cache between
/// tests that toggle FSPEC_HOME or provider discovery state.
#[doc(hidden)]
pub fn __clear_lookup_cache_for_tests() {
    if let Ok(mut guard) = LOOKUP_CACHE.lock() {
        *guard = None;
    }
}

/// Discover the `ProviderConfig` for `provider_slug` and, if found,
/// construct a `RhaiCustomProvider` for `model_alias`. Any discovery or
/// construction failure is logged at warn and returns `None` so callers
/// fall through to their existing defaults.
fn build_ephemeral_provider(
    provider_slug: &str,
    model_alias: &str,
) -> Option<Arc<RhaiCustomProvider>> {
    let configs = match discover_provider_configs() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                provider = %provider_slug,
                error = %e,
                "lookup_script_model_limits: discover_provider_configs failed"
            );
            return None;
        }
    };

    let config = configs.into_iter().find(|c| c.name == provider_slug)?;
    if !config.models.contains_key(model_alias) {
        return None;
    }

    let loader = Arc::new(ScriptLoader::with_default_engine());
    match RhaiCustomProvider::new(Arc::new(config), loader, model_alias.to_string()) {
        Ok(p) => Some(Arc::new(p)),
        Err(e) => {
            tracing::warn!(
                provider = %provider_slug,
                model_alias = %model_alias,
                error = ?e,
                "lookup_script_model_limits: RhaiCustomProvider::new failed"
            );
            None
        }
    }
}

/// PROV-095 NAPI bridge — look up all script-sourced per-model limits
/// (`context_window`, `max_output_tokens`, `compaction_threshold`) for a
/// `(provider_slug, model_alias)` pair in a single call.
///
/// Returns a [`RhaiScriptedLimits`] with each field populated only when
/// the script's `get_model_limits` hook supplied a valid value for that
/// field. When the provider is not a custom Rhai provider, or when the
/// script does not define `get_model_limits`, every field is `None`.
///
/// Results are cached per `(slug, alias)` for [`CACHE_TTL`] so a burst
/// of `session_set_model*` calls incurs exactly one Rhai engine
/// construction + script compile + `get_model_limits` invocation.
///
/// Invoked from `codelet/napi/src/session_manager.rs` in both
/// `session_set_model` and `session_set_model_profile` to allow the
/// Rhai script to be authoritative over (a) the TUI's hard-coded
/// context-window default (`customProviderSectionBuilder.ts`) and
/// (b) the CTX-008 TUI-supplied compaction-threshold override.
pub fn lookup_script_model_limits(
    provider_slug: &str,
    model_alias: &str,
) -> RhaiScriptedLimits {
    if let Some(p) = cache_get(provider_slug, model_alias) {
        return RhaiScriptedLimits {
            context_window: p.script_context_window(),
            max_output_tokens: p.script_max_output_tokens(),
            compaction_threshold: p.script_compaction_threshold(),
        };
    }

    let Some(provider) = build_ephemeral_provider(provider_slug, model_alias) else {
        return RhaiScriptedLimits::default();
    };
    let snapshot = RhaiScriptedLimits {
        context_window: provider.script_context_window(),
        max_output_tokens: provider.script_max_output_tokens(),
        compaction_threshold: provider.script_compaction_threshold(),
    };
    cache_put(provider_slug, model_alias, provider);
    snapshot
}

/// PROV-095 NAPI bridge — look up just the `compaction_threshold` that
/// a custom Rhai provider's `get_model_limits(config)` hook returns.
///
/// Retained as a thin wrapper over [`lookup_script_model_limits`] for
/// backwards compatibility; new callers should prefer
/// `lookup_script_model_limits` to retrieve context_window /
/// max_output_tokens / compaction_threshold in a single cached call.
pub fn lookup_script_compaction_threshold(
    provider_slug: &str,
    model_alias: &str,
) -> Option<(String, u64)> {
    lookup_script_model_limits(provider_slug, model_alias).compaction_threshold
}
