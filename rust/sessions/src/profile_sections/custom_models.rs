//! RPC-346 custom-model writes on local-server profiles.
//!
//! BUG-325: every write either changes the config or returns an error — a
//! non-`openai` provider id, a missing or unreadable config, a missing
//! profile, or (for edit/delete) a model id the profile does not have are
//! refused instead of being reported as a successful no-op.

use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

use codelet_common::fspec_config::update_config_file;

use super::CustomModelDef;

/// RPC-346: add or update a custom model on a local-server profile.
///
/// When `original_model_id` is `Some`, the entry with that id is updated in
/// place (edit: the dialog's fields are replaced and every other key, such as
/// `thinkingLevels`, is kept, BUG-511); otherwise the definition is appended
/// (add). Mirrors the TS `customModelCrudService.saveCustomModel`. Resolves the config path via the
/// `FSPEC_USER_DIR`/`HOME` convention and delegates to
/// [`save_custom_model_at`].
pub fn save_custom_model(
    provider_id: &str,
    profile_name: &str,
    definition: &CustomModelDef,
    original_model_id: Option<&str>,
) -> std::io::Result<()> {
    save_custom_model_at(
        &config_path()?,
        provider_id,
        profile_name,
        definition,
        original_model_id,
    )
}

/// RPC-346: delete a custom model from a local-server profile by id.
///
/// Mirrors the TS `customModelCrudService.deleteCustomModel`: removes the
/// matching entry and drops the `customModels` key entirely when the array
/// becomes empty.
pub fn delete_custom_model(
    provider_id: &str,
    profile_name: &str,
    model_id: &str,
) -> std::io::Result<()> {
    delete_custom_model_at(&config_path()?, provider_id, profile_name, model_id)
}

fn config_path() -> std::io::Result<PathBuf> {
    codelet_common::user_dir::fspec_user_dir()
        .map(|dir| dir.join("fspec-config.json"))
        .map_err(|error| Error::new(ErrorKind::NotFound, error))
}

/// Borrow the `providers.openai.profiles.<name>` object out of the parsed
/// config. Profiles are only supported for the `openai` provider (parity with
/// the TS `saveProfile` guard).
fn profile_object_mut<'a>(
    root: &'a mut serde_json::Value,
    provider_id: &str,
    profile_name: &str,
) -> std::io::Result<&'a mut serde_json::Map<String, serde_json::Value>> {
    if provider_id != "openai" {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Profiles are only supported for the OpenAI API provider (got '{provider_id}')"
            ),
        ));
    }
    root.get_mut("providers")
        .and_then(|providers| providers.get_mut(provider_id))
        .and_then(|provider| provider.get_mut("profiles"))
        .and_then(|profiles| profiles.get_mut(profile_name))
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("profile '{profile_name}' not found for provider '{provider_id}'"),
            )
        })
}

fn custom_models(profile: &serde_json::Map<String, serde_json::Value>) -> Vec<serde_json::Value> {
    profile
        .get("customModels")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
}

fn has_id(entry: &serde_json::Value, id: &str) -> bool {
    entry.get("id").and_then(|value| value.as_str()) == Some(id)
}

/// BUG-511: the keys of a `customModels` entry that the custom-model dialog
/// edits (the wire `CustomModelDefinition` fields). An edit replaces these
/// and keeps every other key of the stored entry: MODEL-016's
/// `thinkingLevels` declaration and keys fspec does not know, as
/// `merge_profile` keeps a profile's own keys.
const DIALOG_KEYS: [&str; 8] = [
    "id",
    "displayName",
    "facade",
    "contextWindow",
    "maxOutputTokens",
    "compactionThreshold",
    "reasoning",
    "hasVision",
];

/// BUG-511: apply an edit to the stored entry. The dialog's keys come from
/// `definition` (a key it left empty is removed); every other stored key is
/// kept, unless `definition` itself carries it.
fn apply_edit(
    stored: &mut serde_json::Map<String, serde_json::Value>,
    definition: serde_json::Map<String, serde_json::Value>,
) {
    for key in DIALOG_KEYS {
        stored.remove(key);
    }
    stored.extend(definition);
}

fn missing_model(model_id: &str, profile_name: &str) -> Error {
    Error::new(
        ErrorKind::NotFound,
        format!("custom model '{model_id}' not found in profile '{profile_name}'"),
    )
}

/// Path-injectable core of [`save_custom_model`]. Performs a whole-file
/// read-modify-write of `config_path`, mutating only the target profile's
/// `customModels` array so unrelated keys are preserved verbatim. An edit
/// replaces only the dialog's keys of the entry (BUG-511).
pub(super) fn save_custom_model_at(
    config_path: &Path,
    provider_id: &str,
    profile_name: &str,
    definition: &CustomModelDef,
    original_model_id: Option<&str>,
) -> std::io::Result<()> {
    // BUG-325: a missing file has no profile, so it is refused; BUG-640:
    // one locked read-modify-write.
    update_config_file(config_path, |root| {
        let profile = profile_object_mut(root, provider_id, profile_name)?;
        let def_value =
            serde_json::to_value(definition).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
        let mut entries = custom_models(profile);
        match original_model_id {
            Some(original) => {
                // `has_id` matches only an object entry.
                let stored = entries
                    .iter_mut()
                    .find(|entry| has_id(entry, original))
                    .and_then(serde_json::Value::as_object_mut)
                    .ok_or_else(|| missing_model(original, profile_name))?;
                let serde_json::Value::Object(definition) = def_value else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!("custom model '{original}' did not serialize to an object"),
                    ));
                };
                apply_edit(stored, definition);
            }
            None => entries.push(def_value),
        }
        profile.insert(
            "customModels".to_string(),
            serde_json::Value::Array(entries),
        );
        Ok(())
    })
}

/// Path-injectable core of [`delete_custom_model`].
pub(super) fn delete_custom_model_at(
    config_path: &Path,
    provider_id: &str,
    profile_name: &str,
    model_id: &str,
) -> std::io::Result<()> {
    update_config_file(config_path, |root| {
        let profile = profile_object_mut(root, provider_id, profile_name)?;
        let mut entries = custom_models(profile);
        let before = entries.len();
        entries.retain(|entry| !has_id(entry, model_id));
        if entries.len() == before {
            return Err(missing_model(model_id, profile_name));
        }
        if entries.is_empty() {
            profile.remove("customModels");
        } else {
            profile.insert(
                "customModels".to_string(),
                serde_json::Value::Array(entries),
            );
        }
        Ok(())
    })
}
