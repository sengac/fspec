//! Reading spec/fspec-hooks.json for the hook-config commands (BUG-611).
//!
//! The file is broken when it is not valid JSON, its top-level value is not
//! an object, its `hooks` value is not an object, or an event's value under
//! `hooks` is not an array. A writer (add-hook, remove-hook) moves a broken
//! file to `spec/fspec-hooks.json.broken-<timestamp>` (never over an existing
//! backup) and starts a fresh config, with a notice saying so; list-hooks and
//! validate-hooks report a broken file and leave it unchanged.

use std::io::Write as _;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use super::config::CONFIG_FILE;
use crate::error::FspecCoreError;

/// spec/fspec-hooks.json as a hook-config writer found it.
pub(crate) enum ForWrite {
    /// No file: the read error, for a writer that needs the file.
    Missing(std::io::Error),
    /// A well-formed config, key order kept.
    Config(Map<String, Value>),
    /// A broken file was backed up: a fresh `{"hooks": {}}` config and the
    /// notice the writer prints.
    BackedUp {
        config: Map<String, Value>,
        notice: String,
    },
}

/// Read spec/fspec-hooks.json for a writer, backing a broken file up to its
/// `.broken-<now>` name first. The caller holds the spec lock and writes the
/// new config afterwards.
pub(crate) fn load_for_write(
    project_root: &Path,
    command: &'static str,
    now: DateTime<Utc>,
) -> Result<ForWrite, FspecCoreError> {
    let path = crate::io::spec_paths::hooks_config_of(project_root);
    let raw = match std::fs::read(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ForWrite::Missing(e)),
        Err(source) => return Err(FspecCoreError::Io { command, source }),
    };
    let reason = match parse(&raw) {
        Ok(config) => return Ok(ForWrite::Config(config)),
        Err(reason) => reason,
    };
    let stamp = now.format("%Y%m%dT%H%M%S%.3fZ");
    let backup_label = format!("{CONFIG_FILE}.broken-{stamp}");
    let backup_path = path.with_file_name(format!("fspec-hooks.json.broken-{stamp}"));
    let mut backup = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup_path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(FspecCoreError::Message(format!(
                "{CONFIG_FILE} could not be parsed ({reason}), and its backup {backup_label} \
                 already exists; nothing was changed"
            )));
        }
        Err(source) => return Err(FspecCoreError::Io { command, source }),
    };
    backup
        .write_all(&raw)
        .and_then(|()| backup.sync_all())
        .map_err(|source| FspecCoreError::Io { command, source })?;
    Ok(ForWrite::BackedUp {
        config: empty_config(),
        notice: format!(
            "{CONFIG_FILE} could not be parsed ({reason}); it was moved to {backup_label} \
             and a new hook configuration was written"
        ),
    })
}

/// Read spec/fspec-hooks.json without changing it: `None` when there is no
/// file, an error naming the problem when it is broken.
pub(crate) fn load_for_read(
    project_root: &Path,
    command: &'static str,
) -> Result<Option<Map<String, Value>>, FspecCoreError> {
    let path = crate::io::spec_paths::hooks_config_of(project_root);
    let raw = match std::fs::read(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(FspecCoreError::Io { command, source }),
    };
    parse(&raw).map(Some).map_err(|reason| {
        FspecCoreError::Message(format!(
            "{CONFIG_FILE} could not be parsed ({reason}); the file was not changed"
        ))
    })
}

/// A fresh config with no hooks.
pub(crate) fn empty_config() -> Map<String, Value> {
    let mut config = Map::new();
    config.insert("hooks".to_string(), Value::Object(Map::new()));
    config
}

/// The config, or why the file is broken.
fn parse(raw: &[u8]) -> Result<Map<String, Value>, String> {
    let value: Value = serde_json::from_slice(raw).map_err(|e| e.to_string())?;
    let Value::Object(config) = value else {
        return Err("the top-level value is not an object".to_string());
    };
    if let Some(hooks) = config.get("hooks") {
        let hooks = hooks
            .as_object()
            .ok_or_else(|| "hooks is not an object".to_string())?;
        if let Some((event, _)) = hooks.iter().find(|(_, entries)| !entries.is_array()) {
            return Err(format!("hooks.{event} is not an array"));
        }
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn each_broken_shape_has_its_reason() {
        assert_eq!(
            parse(b"[]").err().as_deref(),
            Some("the top-level value is not an object")
        );
        assert_eq!(
            parse(br#"{"hooks": 1}"#).err().as_deref(),
            Some("hooks is not an object")
        );
        assert_eq!(
            parse(br#"{"hooks": {"pre-done": [], "post-done": {}}}"#)
                .err()
                .as_deref(),
            Some("hooks.post-done is not an array")
        );
        assert!(parse(br#"{"global": {}}"#).is_ok());
    }
}
