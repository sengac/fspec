//! Provider-profile persistence for built-in and custom providers stored under
//! `~/.fspec/fspec-config.json` (`providers.<id>.profiles.<name>`).
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
use tracing::{error, info};

use codelet_common::fspec_config::update_config_file;

use crate::profile_sections::CompactionThreshold;

/// `<user dir>/fspec-config.json`; an unresolvable user directory is an
/// error (BUG-640: one resolver, no fallback).
fn user_config_path() -> std::io::Result<std::path::PathBuf> {
    codelet_common::user_dir::fspec_user_dir()
        .map(|directory| directory.join("fspec-config.json"))
        .map_err(std::io::Error::other)
}

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
    /// PROV-143: per-profile preserve-thinking toggle. `None` (key absent) or
    /// `Some(false)` mean thinking blocks are STRIPPED from the chat history
    /// sent back to the LLM (the default); `Some(true)` preserves them. Only
    /// `Some(_)` is written to disk (as `"preserveThinking": true|false`);
    /// `None` removes the key.
    pub preserve_thinking: Option<bool>,
    /// PROV-144: per-profile Max Images limit for the Read tool's image
    /// budget. `None` (key absent) resolves to the tool-layer default of 4;
    /// `Some(0)` marks a no-vision profile (the Read tool fails image reads);
    /// `Some(n)` with `n >= 1` caps a single Read result at `n` images. Only
    /// `Some(_)` is written to disk (as `"maxImages": <n>`, including the
    /// explicit `0` no-vision sentinel); `None` removes the key.
    pub max_images: Option<u32>,
    /// PROV-145: per-profile loop-detection toggle. `None` (key absent) keeps
    /// the RIG-014 detector enabled (today's always-on behavior);
    /// `Some(false)` disables it; `Some(true)` is the explicit on. Only
    /// `Some(_)` is written to disk (as `"loopDetectionEnabled": true|false`);
    /// `None` removes the key.
    pub loop_detection_enabled: Option<bool>,
    /// PROV-145: per-profile loop-detector sliding window in words. `None`
    /// (key absent) resolves to the RIG-014 default of 160. Only `Some(_)` is
    /// written to disk (as `"loopDetectionWindow": <n>`); `None` removes the
    /// key.
    pub loop_detection_window: Option<u32>,
    /// PROV-145: per-profile tail n-gram repeat threshold. `None` (key
    /// absent) resolves to the RIG-014 default of 10. Only `Some(_)` is
    /// written to disk (as `"loopDetectionMaxRepeats": <n>`); `None` removes
    /// the key.
    pub loop_detection_max_repeats: Option<u32>,
    /// PROV-145: per-profile max auto-continue retries after a loop abort.
    /// `None` (key absent) resolves to the RIG-014 default of 10; `Some(0)`
    /// is the explicit never-retry sentinel. Only `Some(_)` is written to
    /// disk (as `"loopDetectionMaxRetries": <n>`, including the explicit `0`
    /// sentinel); `None` removes the key.
    pub loop_detection_max_retries: Option<u32>,
}

/// Profiles are supported for every nonempty logical provider identifier.
pub fn profiles_supported(provider_id: &str) -> bool {
    !provider_id.trim().is_empty()
}

/// Shared validation error for an empty logical provider identifier.
pub fn profiles_unsupported_error(provider_id: &str) -> String {
    format!("A nonempty provider identifier is required (got '{provider_id}')")
}

/// Create or update a profile, persisting to `~/.fspec/fspec-config.json`.
///
/// Resolves the config path via the `FSPEC_USER_DIR`/home-dir convention and
/// delegates to [`save_profile_at`]. Built-in and custom providers share the
/// same provider-qualified storage without modifying inherited credentials.
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
    let config_path = user_config_path().inspect_err(|error| {
        error!(profile = %profile_name, %error, "save_profile: no user directory");
    })?;
    info!(path = %config_path.display(), profile = %profile_name, "save_profile: saving profile");
    save_profile_at(&config_path, provider_id, profile_name, def)
}

/// Delete a profile from `~/.fspec/fspec-config.json`.
///
/// Missing file or absent profile is an idempotent no-op that leaves the file
/// byte-identical (TS `deleteProfile` early-return parity).
pub fn delete_profile(provider_id: &str, profile_name: &str) -> std::io::Result<()> {
    let config_path = user_config_path()?;
    info!(path = %config_path.display(), profile = %profile_name, "delete_profile: deleting profile");
    delete_profile_at(&config_path, provider_id, profile_name)
}

/// PROV-136: rename a profile (or in-place update when the name is unchanged),
/// persisting to `~/.fspec/fspec-config.json`.
///
/// Resolves the config path via the `FSPEC_USER_DIR`/home-dir convention and
/// delegates to [`rename_profile_at`].
pub fn rename_profile(
    provider_id: &str,
    old_name: &str,
    new_name: &str,
    def: &ProfileDef,
) -> std::io::Result<()> {
    if !profiles_supported(provider_id) {
        return Ok(());
    }
    let config_path = user_config_path().inspect_err(|error| {
        error!(old = %old_name, new = %new_name, %error, "rename_profile: no user directory");
    })?;
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
pub(crate) fn merge_profile(profile: &mut Map<String, Value>, def: &ProfileDef) {
    set_or_remove(
        profile,
        "baseUrl",
        (!def.base_url.is_empty()).then(|| Value::String(def.base_url.clone())),
    );
    set_or_remove(
        profile,
        "apiKey",
        (!def.api_key.is_empty()).then(|| Value::String(def.api_key.clone())),
    );
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
    // PROV-143: write the preserve-thinking toggle when set (including the
    // explicit `false`); `None` removes the key so an absent
    // `preserveThinking` continues to mean stripped (the default).
    set_or_remove(
        profile,
        "preserveThinking",
        def.preserve_thinking.map(Value::from),
    );
    // PROV-144: write the per-profile Max Images limit when set (including the
    // explicit `0` no-vision sentinel); `None` removes the key so an absent
    // `maxImages` continues to resolve to the tool-layer default of 4.
    set_or_remove(profile, "maxImages", def.max_images.map(Value::from));
    // PROV-145: write the loop-detection toggle when set (including the
    // explicit `false`); `None` removes the key so an absent
    // `loopDetectionEnabled` continues to mean enabled (today's always-on
    // behavior).
    set_or_remove(
        profile,
        "loopDetectionEnabled",
        def.loop_detection_enabled.map(Value::from),
    );
    // PROV-145: write the loop-detector window when set; `None` removes the
    // key so an absent `loopDetectionWindow` continues to resolve to the
    // RIG-014 default of 160.
    set_or_remove(
        profile,
        "loopDetectionWindow",
        def.loop_detection_window.map(Value::from),
    );
    // PROV-145: write the tail n-gram repeat threshold when set; `None`
    // removes the key so an absent `loopDetectionMaxRepeats` continues to
    // resolve to the RIG-014 default of 10.
    set_or_remove(
        profile,
        "loopDetectionMaxRepeats",
        def.loop_detection_max_repeats.map(Value::from),
    );
    // PROV-145: write the loop-abort retry cap when set (including the
    // explicit `0` never-retry sentinel); `None` removes the key so an
    // absent `loopDetectionMaxRetries` continues to resolve to the RIG-014
    // default of 10.
    set_or_remove(
        profile,
        "loopDetectionMaxRetries",
        def.loop_detection_max_retries.map(Value::from),
    );
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
    // BUG-461: a config that cannot be read is never replaced. BUG-640: one
    // locked read-modify-write.
    let result = update_config_file(config_path, |root| {
        let storage_id = crate::provider_settings::provider_storage_id(root, provider_id);
        let Some(root_obj) = root.as_object_mut() else {
            return Ok(());
        };
        let Some(providers) = ensure_object(root_obj, "providers") else {
            return Ok(());
        };
        let Some(provider) = ensure_object(providers, &storage_id) else {
            return Ok(());
        };
        let Some(profiles) = ensure_object(provider, "profiles") else {
            return Ok(());
        };
        let Some(profile) = ensure_object(profiles, profile_name) else {
            return Ok(());
        };
        merge_profile(profile, def);
        Ok::<_, std::io::Error>(())
    });
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
    // BUG-461: a config that cannot be read is an error, not "nothing to
    // delete"; nothing removed leaves the file as it was.
    update_config_file(config_path, |root| {
        let storage_id = crate::provider_settings::provider_storage_id(root, provider_id);
        if let Some(profiles) = root
            .get_mut("providers")
            .and_then(|p| p.get_mut(&storage_id))
            .and_then(|p| p.get_mut("profiles"))
            .and_then(Value::as_object_mut)
        {
            profiles.remove(profile_name);
        }
        Ok::<_, std::io::Error>(())
    })
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
    // BUG-461: a config that cannot be read is never replaced. BUG-640: one
    // locked read-modify-write.
    update_config_file(config_path, |root| {
        let storage_id = crate::provider_settings::provider_storage_id(root, provider_id);
        let Some(root_obj) = root.as_object_mut() else {
            return Ok(());
        };
        let Some(providers) = ensure_object(root_obj, "providers") else {
            return Ok(());
        };
        let Some(provider) = ensure_object(providers, &storage_id) else {
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
        Ok(())
    })
}

#[cfg(test)]
#[path = "profile_persistence_tests.rs"]
mod tests;
