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
use tracing::{debug, error, info, warn};

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
    /// PROV-139: per-profile streaming toggle. `None` (the on-disk default when
    /// the `streaming` key is absent) means streaming is enabled; only
    /// `Some(false)` is written to disk as `"streaming": false`.
    pub streaming: Option<bool>,
    /// PROV-142: per-profile auto-continue default. `None` (key absent) or
    /// `Some(0)` mean OFF; `Some(n)` with `n >= 1` means ON with budget `n`.
    /// Only `Some(_)` is written to disk (as `"autoContinue": <n>`, including
    /// the explicit-off sentinel `0`); `None` removes the key.
    pub auto_continue: Option<u32>,
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
/// Resolves the config path via the `FSPEC_USER_DIR`/home-dir convention and
/// delegates to [`save_profile_at`]. Profiles are only supported for the
/// `openai` provider (see [`profiles_supported`]); a non-openai call is a
/// no-op at this layer — the `Err` is surfaced by the handle / NAPI callers.
///
/// Every config-file read and write is logged (path + outcome) via the shared
/// `tracing` system so a silent failure (e.g. an unresolvable home directory)
/// is visible in `~/.fspec/logs/` instead of vanishing.
pub fn save_profile(
    provider_id: &str,
    profile_name: &str,
    def: &ProfileDef,
) -> std::io::Result<()> {
    if !profiles_supported(provider_id) {
        return Ok(());
    }
    let Some(config_path) = fspec_user_dir().map(|d| d.join("fspec-config.json")) else {
        let msg = "cannot resolve the fspec user directory (~/.fspec): no home directory available";
        error!(profile = %profile_name, "save_profile: {msg}");
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, msg));
    };
    info!(path = %config_path.display(), profile = %profile_name, "save_profile: saving profile");
    save_profile_at(&config_path, provider_id, profile_name, def)
}

/// Delete a profile from `~/.fspec/fspec-config.json`.
///
/// Missing file or absent profile is an idempotent no-op that leaves the file
/// byte-identical (TS `deleteProfile` early-return parity).
pub fn delete_profile(provider_id: &str, profile_name: &str) -> std::io::Result<()> {
    let Some(config_path) = fspec_user_dir().map(|d| d.join("fspec-config.json")) else {
        // Deleting a profile whose config file cannot be located is a benign
        // no-op (there is nothing to delete), so only log at debug level.
        debug!(profile = %profile_name, "delete_profile: no fspec user directory; no-op");
        return Ok(());
    };
    info!(path = %config_path.display(), profile = %profile_name, "delete_profile: deleting profile");
    delete_profile_at(&config_path, provider_id, profile_name)
}

/// PROV-136: rename a profile (or in-place update when the name is unchanged),
/// persisting to `~/.fspec/fspec-config.json`.
///
/// Resolves the config path via the `FSPEC_USER_DIR`/home-dir convention and
/// delegates to [`rename_profile_at`]. A non-openai call is a no-op at this
/// layer (the `Err` is surfaced by the handle / NAPI callers).
pub fn rename_profile(
    provider_id: &str,
    old_name: &str,
    new_name: &str,
    def: &ProfileDef,
) -> std::io::Result<()> {
    if !profiles_supported(provider_id) {
        return Ok(());
    }
    let Some(config_path) = fspec_user_dir().map(|d| d.join("fspec-config.json")) else {
        let msg = "cannot resolve the fspec user directory (~/.fspec): no home directory available";
        error!(old = %old_name, new = %new_name, "rename_profile: {msg}");
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, msg));
    };
    info!(path = %config_path.display(), old = %old_name, new = %new_name, "rename_profile: renaming profile");
    rename_profile_at(&config_path, provider_id, old_name, new_name, def)
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
    // PROV-139: write the streaming toggle only when explicitly set; `None`
    // removes the key so an absent `streaming` continues to mean enabled.
    set_or_remove(profile, "streaming", def.streaming.map(Value::from));
    // PROV-142: write the auto-continue default when set (including the
    // explicit-off sentinel `Some(0)`); `None` removes the key so an absent
    // `autoContinue` continues to mean off (today's behavior).
    set_or_remove(profile, "autoContinue", def.auto_continue.map(Value::from));
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
        // A pre-existing file that failed to parse is a data-loss risk (its
        // contents get replaced), so surface it at warn level.
        _ => {
            if config_path.exists() {
                warn!(
                    path = %config_path.display(),
                    "save_profile: existing config file is missing or malformed; starting from an empty object"
                );
            }
            Value::Object(Map::new())
        }
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
    let result = write_config_value(config_path, &root);
    match &result {
        Ok(()) => {
            info!(path = %config_path.display(), profile = %profile_name, "save_profile: write succeeded");
        }
        Err(e) => {
            error!(path = %config_path.display(), profile = %profile_name, error = %e, "save_profile: write failed");
        }
    }
    result
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

/// Path-injectable core of [`rename_profile`]. Renames `old_name` to `new_name`
/// (or applies an in-place update when they are equal) in a SINGLE
/// read-modify-write so the config never lands in a torn state.
///
/// An in-place update (`old_name == new_name`) delegates to [`save_profile_at`]
/// so the existing merge path preserves `customModels` + sibling keys. A true
/// rename MOVES the stored profile object to the new key (carrying its
/// `customModels` and any unknown keys), applies the connection fields from
/// `def`, and removes the old key. A rename onto an EXISTING different profile
/// is rejected with [`std::io::ErrorKind::AlreadyExists`] and never writes.
pub fn rename_profile_at(
    config_path: &Path,
    provider_id: &str,
    old_name: &str,
    new_name: &str,
    def: &ProfileDef,
) -> std::io::Result<()> {
    if !profiles_supported(provider_id) {
        return Ok(());
    }
    if old_name == new_name {
        return save_profile_at(config_path, provider_id, new_name, def);
    }
    let mut root = match read_config_value(config_path) {
        Some(value @ Value::Object(_)) => value,
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
    // Collision check: never silently overwrite a different existing profile.
    if profiles.contains_key(new_name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("A profile named '{new_name}' already exists"),
        ));
    }
    // MOVE the existing object (preserving customModels + unknown keys), or
    // start fresh if the old profile is somehow absent.
    let mut profile = match profiles.remove(old_name) {
        Some(Value::Object(obj)) => obj,
        _ => Map::new(),
    };
    merge_profile(&mut profile, def);
    profiles.insert(new_name.to_string(), Value::Object(profile));
    write_config_value(config_path, &root)
}

#[cfg(test)]
#[path = "profile_persistence_tests.rs"]
mod tests;
