//! PROV-100 — Load custom OpenAI profiles from `fspec-config.json`.
//!
//! Feature: spec/features/openai-profiles-from-config.feature
//!
//! TS stores OpenAI custom profiles under `providers.openai.profiles`
//! (name -> `{ baseUrl, ... }`) in `~/.fspec/fspec-config.json`, deep-merged
//! with the project `<cwd>/spec/fspec-config.json` (project overrides user),
//! and reads them via `loadProviderProfiles('openai')`. The Rust port had no
//! reader for this store and the dispatch always passed `&[]`, so OpenAI only
//! ever showed "+ Add Profile".
//!
//! Display string: `"{name} → {baseUrl}"`, or just `"{name}"` when the
//! profile has no (or empty) `baseUrl`. Missing files, empty content, or
//! malformed JSON degrade gracefully to an empty list (mirrors TS
//! `loadConfig`). Profiles are sorted by name for deterministic output (a
//! DELIBERATE divergence from the TS insertion-order behaviour).

use std::collections::BTreeMap;
use std::path::Path;

use codelet_rpc_types::ProfileDefinition;
use serde_json::Value;
use tracing::debug;

/// Read `providers.openai.profiles` from a single config file (if present
/// and parseable) into the supplied name -> baseUrl map. Later callers
/// overwrite earlier entries, giving project-over-user precedence. Missing
/// files / empty content / malformed JSON are silently ignored.
fn merge_profiles_from_file(path: &Path, out: &mut BTreeMap<String, Option<String>>) {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            debug!(path = %path.display(), error = %e, "profiles_config: failed to read config file");
            return;
        }
    };
    if raw.trim().is_empty() {
        return;
    }
    let value: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            debug!(path = %path.display(), error = %e, "profiles_config: malformed JSON in config file");
            return;
        }
    };
    debug!(path = %path.display(), "profiles_config: read config file");
    let profiles = value
        .get("providers")
        .and_then(|p| p.get("openai"))
        .and_then(|o| o.get("profiles"))
        .and_then(Value::as_object);
    let profiles = match profiles {
        Some(p) => p,
        None => return,
    };
    for (name, cfg) in profiles {
        let base_url = cfg
            .get("baseUrl")
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|s| !s.trim().is_empty());
        out.insert(name.clone(), base_url);
    }
}

/// Format one profile entry as its display string.
fn display_string(name: &str, base_url: Option<&String>) -> String {
    match base_url {
        Some(url) => format!("{name} → {url}"),
        None => name.to_string(),
    }
}

/// Pure, path-injectable loader. Reads `<user_config_dir>/fspec-config.json`
/// and `<project_root>/spec/fspec-config.json`, merges
/// `providers.openai.profiles` (project overrides user by name), and returns
/// the display strings sorted by profile name.
pub fn load_openai_profiles_from(user_config_dir: &Path, project_root: &Path) -> Vec<String> {
    // BTreeMap keeps the merge deterministic and sorted by name.
    let mut merged: BTreeMap<String, Option<String>> = BTreeMap::new();
    // User config first, then project so project entries overwrite.
    merge_profiles_from_file(&user_config_dir.join("fspec-config.json"), &mut merged);
    merge_profiles_from_file(
        &project_root.join("spec").join("fspec-config.json"),
        &mut merged,
    );
    merged
        .iter()
        .map(|(name, base_url)| display_string(name, base_url.as_ref()))
        .collect()
}

/// Thin wrapper that resolves the real user config dir (`$HOME/.fspec` on
/// Unix, `%USERPROFILE%\.fspec` on Windows, read-only) and the current
/// working directory, then delegates to
/// [`load_openai_profiles_from`]. Returns an empty list when the home
/// directory or the current directory cannot be resolved.
pub fn load_openai_profiles() -> Vec<String> {
    let user_config_dir = match dirs::home_dir() {
        Some(home) => home.join(".fspec"),
        None => return Vec::new(),
    };
    let project_root = match std::env::current_dir() {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    load_openai_profiles_from(&user_config_dir, &project_root)
}

// ─────────────────────────────────────────────────────────────────────────
// PROV-111 — full per-profile ProfileConfig loader.
//
// The display-string loader above is sufficient for the nav-tree row labels,
// but the EditProfile form needs the FULL stored config (baseUrl, apiKey and
// the optional contextWindow / maxOutputTokens / compactionThreshold) to
// prefill its fields. This loader mirrors the display-string loader's read +
// project-over-user merge but yields a `name -> ProfileDefinition` map.
//
// `customModels` and any other unknown sibling keys are intentionally NOT
// part of `ProfileDefinition` — they are preserved by the PROV-108 backend
// read-modify-write, never round-tripped through the form.
// ─────────────────────────────────────────────────────────────────────────

/// Parse one `providers.openai.profiles.<name>` object into a
/// [`ProfileDefinition`]. `baseUrl` / `apiKey` default to the empty string
/// when absent; the numeric fields are read as `u32` (non-integers dropped);
/// `compactionThreshold` is split into its flat wire fields.
fn profile_definition_from_value(cfg: &Value) -> ProfileDefinition {
    let base_url = cfg
        .get("baseUrl")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let api_key = cfg
        .get("apiKey")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let context_window = cfg
        .get("contextWindow")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok());
    let max_output_tokens = cfg
        .get("maxOutputTokens")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok());
    let (compaction_threshold_type, compaction_threshold_value) = cfg
        .get("compactionThreshold")
        .map(parse_compaction_threshold)
        .unwrap_or((None, None));
    // PROV-139: read the optional streaming toggle; an absent key stays `None`
    // (⇒ enabled) so older profiles are unaffected.
    let streaming = cfg.get("streaming").and_then(Value::as_bool);
    // PROV-142: read the optional auto-continue default; an absent key stays
    // `None` (⇒ off) so older profiles are unaffected.
    let auto_continue = cfg
        .get("autoContinue")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok());
    // PROV-143: read the optional preserve-thinking toggle; an absent key
    // stays `None` (⇒ stripped, the default) so older profiles are unaffected.
    let preserve_thinking = cfg.get("preserveThinking").and_then(Value::as_bool);
    // PROV-144: read the optional Max Images limit; an absent key stays `None`
    // (⇒ tool-layer default 4) so older profiles are unaffected. Mirrors the
    // autoContinue `as_u64 → u32` read pattern above.
    let max_images = cfg
        .get("maxImages")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok());
    // PROV-145: read the optional loop-detection toggle; an absent key stays
    // `None` (⇒ enabled, today's always-on behavior) so older profiles are
    // unaffected.
    let loop_detection_enabled = cfg.get("loopDetectionEnabled").and_then(Value::as_bool);
    // PROV-145: read the optional loop-detection numerics; absent keys stay
    // `None` (⇒ the RIG-014 defaults 160 / 10 / 10) so older profiles are
    // unaffected. Mirrors the autoContinue `as_u64 → u32` read pattern.
    let loop_detection_window = cfg
        .get("loopDetectionWindow")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok());
    let loop_detection_max_repeats = cfg
        .get("loopDetectionMaxRepeats")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok());
    let loop_detection_max_retries = cfg
        .get("loopDetectionMaxRetries")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok());
    ProfileDefinition {
        base_url,
        api_key,
        context_window,
        max_output_tokens,
        compaction_threshold_type,
        compaction_threshold_value,
        streaming,
        auto_continue,
        preserve_thinking,
        max_images,
        loop_detection_enabled,
        loop_detection_window,
        loop_detection_max_repeats,
        loop_detection_max_retries,
    }
}

/// Split a stored `compactionThreshold` object into the flat wire fields
/// `(type, value)`. Both must be present and parseable, otherwise `(None,
/// None)`. The on-disk value may be an integer or float (mirrors the
/// tolerant custom-model loader); it is rounded to the nearest `u32`.
fn parse_compaction_threshold(node: &Value) -> (Option<String>, Option<u32>) {
    let threshold_type = node.get("type").and_then(Value::as_str);
    let value = node
        .get("value")
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite() && *v >= 0.0)
        .map(|v| v.round() as u32);
    match (threshold_type, value) {
        (Some(t), Some(v)) => (Some(t.to_string()), Some(v)),
        _ => (None, None),
    }
}

/// Read `providers.openai.profiles` from a single config file into the
/// supplied `name -> ProfileDefinition` map. Later callers overwrite earlier
/// entries (project-over-user precedence). Missing files / empty content /
/// malformed JSON are silently ignored.
fn merge_profile_configs_from_file(path: &Path, out: &mut BTreeMap<String, ProfileDefinition>) {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            debug!(path = %path.display(), error = %e, "profiles_config: failed to read config file");
            return;
        }
    };
    if raw.trim().is_empty() {
        return;
    }
    let value: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            debug!(path = %path.display(), error = %e, "profiles_config: malformed JSON in config file");
            return;
        }
    };
    debug!(path = %path.display(), "profiles_config: read config file");
    let profiles = value
        .get("providers")
        .and_then(|p| p.get("openai"))
        .and_then(|o| o.get("profiles"))
        .and_then(Value::as_object);
    let profiles = match profiles {
        Some(p) => p,
        None => return,
    };
    for (name, cfg) in profiles {
        out.insert(name.clone(), profile_definition_from_value(cfg));
    }
}

/// Pure, path-injectable full-config loader. Reads
/// `<user_config_dir>/fspec-config.json` and
/// `<project_root>/spec/fspec-config.json`, merges
/// `providers.openai.profiles` (project overrides user by name), and returns
/// the full per-profile [`ProfileDefinition`] map keyed by profile name.
pub fn load_openai_profile_configs_from(
    user_config_dir: &Path,
    project_root: &Path,
) -> BTreeMap<String, ProfileDefinition> {
    let mut merged: BTreeMap<String, ProfileDefinition> = BTreeMap::new();
    merge_profile_configs_from_file(&user_config_dir.join("fspec-config.json"), &mut merged);
    merge_profile_configs_from_file(
        &project_root.join("spec").join("fspec-config.json"),
        &mut merged,
    );
    merged
}

/// Thin wrapper resolving the real user config dir (`$HOME/.fspec` on Unix,
/// `%USERPROFILE%\.fspec` on Windows) and the current working directory, then
/// delegating to [`load_openai_profile_configs_from`]. Returns an empty map
/// when the home directory or the current directory cannot be resolved.
pub fn load_openai_profile_configs() -> BTreeMap<String, ProfileDefinition> {
    let user_config_dir = match dirs::home_dir() {
        Some(home) => home.join(".fspec"),
        None => return BTreeMap::new(),
    };
    let project_root = match std::env::current_dir() {
        Ok(p) => p,
        Err(_) => return BTreeMap::new(),
    };
    load_openai_profile_configs_from(&user_config_dir, &project_root)
}
