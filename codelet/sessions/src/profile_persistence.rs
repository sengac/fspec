//! PROV-108 — provider-profile persistence (save / delete) for local-server
//! `openai` profiles stored under `~/.fspec/fspec-config.json`
//! (`providers.openai.profiles.<name>`).
//!
//! Feature: spec/features/provider-config-profile-persistence.feature
//!
//! Mirrors the TS `profile-management.ts` `saveProfile` / `deleteProfile`
//! contract, with one deliberate hardening over the TS full-object replace:
//! the save is a read-modify-write that PRESERVES the profile's existing
//! `customModels` array (owned by the RPC-347 custom-model path) and every
//! unrelated key (sibling profiles, top-level config). The connection-level
//! fields the form controls (`baseUrl`, `apiKey`, and the optional
//! `contextWindow` / `maxOutputTokens` / `compactionThreshold`) reflect the
//! incoming definition exactly — present fields are written, absent optionals
//! are removed.
//!
//! Lives in its own module (not `profile_sections.rs`, already over the
//! 300-LoC ceiling) and reuses that module's `pub(crate)` filesystem helpers
//! so there is a single read/write code path.

use std::path::Path;

use serde_json::{Map, Value};

use crate::profile_sections::{
    fspec_user_dir, read_config_value, write_config_value, CompactionThreshold,
};

/// On-disk-facing profile definition (connection settings only).
///
/// `customModels` is intentionally absent — it is owned by the RPC-347
/// custom-model write path and preserved verbatim by [`save_profile`].
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileDef {
    pub base_url: String,
    pub api_key: String,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub compaction_threshold: Option<CompactionThreshold>,
}

/// Canonical openai-only guard predicate — the single source of truth for
/// which providers support profile persistence (TS `profile-management.ts`
/// `saveProfile` guard). Every caller (handle override, NAPI binding) and this
/// module's own defense-in-depth checks reference this so the semantics cannot
/// drift across layers.
pub fn profiles_supported(provider_id: &str) -> bool {
    provider_id == "openai"
}

/// Canonical user-facing error message for an unsupported (non-`openai`)
/// profile provider. Referenced by the handle override and the NAPI binding so
/// the wording is identical everywhere.
pub fn profiles_unsupported_error(provider_id: &str) -> String {
    format!("Profiles are only supported for the OpenAI API provider (got '{provider_id}')")
}

/// Create or update a profile, persisting to `~/.fspec/fspec-config.json`.
///
/// Resolves the config path via the `FSPEC_USER_DIR`/`HOME` convention and
/// delegates to [`save_profile_at`]. Profiles are only supported for the
/// `openai` provider (see [`profiles_supported`]); a non-openai call is a
/// no-op at this layer — the `Err` is surfaced by the handle / NAPI callers.
pub fn save_profile(
    provider_id: &str,
    profile_name: &str,
    def: &ProfileDef,
) -> std::io::Result<()> {
    if !profiles_supported(provider_id) {
        return Ok(());
    }
    let Some(config_path) = fspec_user_dir().map(|d| d.join("fspec-config.json")) else {
        return Ok(());
    };
    save_profile_at(&config_path, provider_id, profile_name, def)
}

/// Delete a profile from `~/.fspec/fspec-config.json`.
///
/// Missing file or absent profile is an idempotent no-op that leaves the file
/// byte-identical (TS `deleteProfile` early-return parity).
pub fn delete_profile(provider_id: &str, profile_name: &str) -> std::io::Result<()> {
    let Some(config_path) = fspec_user_dir().map(|d| d.join("fspec-config.json")) else {
        return Ok(());
    };
    delete_profile_at(&config_path, provider_id, profile_name)
}

/// Get-or-create the child object at `key`, normalizing a non-object value to a
/// fresh empty object. Returns `None` only if the borrow cannot be re-acquired,
/// which never happens after the normalization above (panic-free path).
fn ensure_object<'a>(
    parent: &'a mut Map<String, Value>,
    key: &str,
) -> Option<&'a mut Map<String, Value>> {
    let is_object = parent.get(key).map(Value::is_object).unwrap_or(false);
    if !is_object {
        parent.insert(key.to_string(), Value::Object(Map::new()));
    }
    parent.get_mut(key).and_then(Value::as_object_mut)
}

/// Write `key`, or remove it entirely when the value is `None`, so the stored
/// profile reflects the definition exactly.
fn set_or_remove(obj: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    match value {
        Some(v) => {
            obj.insert(key.to_string(), v);
        }
        None => {
            obj.remove(key);
        }
    }
}

/// Apply the connection-level fields onto an existing (or freshly created)
/// profile object, leaving `customModels` and any unknown keys untouched.
fn merge_profile(profile: &mut Map<String, Value>, def: &ProfileDef) {
    profile.insert("baseUrl".to_string(), Value::String(def.base_url.clone()));
    profile.insert("apiKey".to_string(), Value::String(def.api_key.clone()));
    set_or_remove(
        profile,
        "contextWindow",
        def.context_window.map(Value::from),
    );
    set_or_remove(
        profile,
        "maxOutputTokens",
        def.max_output_tokens.map(Value::from),
    );
    let compaction = def.compaction_threshold.as_ref().map(|c| {
        let mut m = Map::new();
        m.insert("type".to_string(), Value::String(c.threshold_type.clone()));
        m.insert("value".to_string(), Value::from(c.value));
        Value::Object(m)
    });
    set_or_remove(profile, "compactionThreshold", compaction);
}

/// Path-injectable core of [`save_profile`]. Whole-file read-modify-write that
/// creates the `providers.<provider>.profiles.<name>` path as needed while
/// preserving every unrelated key.
pub fn save_profile_at(
    config_path: &Path,
    provider_id: &str,
    profile_name: &str,
    def: &ProfileDef,
) -> std::io::Result<()> {
    if !profiles_supported(provider_id) {
        return Ok(());
    }
    let mut root = match read_config_value(config_path) {
        Some(value @ Value::Object(_)) => value,
        // Missing, malformed, or non-object JSON: start from an empty object.
        _ => Value::Object(Map::new()),
    };
    let Some(root_obj) = root.as_object_mut() else {
        return Ok(());
    };
    let Some(providers) = ensure_object(root_obj, "providers") else {
        return Ok(());
    };
    let Some(provider) = ensure_object(providers, provider_id) else {
        return Ok(());
    };
    let Some(profiles) = ensure_object(provider, "profiles") else {
        return Ok(());
    };
    let Some(profile) = ensure_object(profiles, profile_name) else {
        return Ok(());
    };
    merge_profile(profile, def);
    write_config_value(config_path, &root)
}

/// Path-injectable core of [`delete_profile`]. Removes only the named profile,
/// leaving the file untouched (byte-identical) when there is nothing to remove.
pub fn delete_profile_at(
    config_path: &Path,
    provider_id: &str,
    profile_name: &str,
) -> std::io::Result<()> {
    let Some(mut root) = read_config_value(config_path) else {
        return Ok(());
    };
    let Some(profiles) = root
        .get_mut("providers")
        .and_then(|p| p.get_mut(provider_id))
        .and_then(|p| p.get_mut("profiles"))
        .and_then(Value::as_object_mut)
    else {
        return Ok(());
    };
    if profiles.remove(profile_name).is_none() {
        return Ok(());
    }
    write_config_value(config_path, &root)
}

#[cfg(test)]
#[path = "profile_persistence_tests.rs"]
mod tests;
