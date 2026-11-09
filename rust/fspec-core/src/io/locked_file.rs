//! Locked JSON file I/O — atomic read+init and atomic write under the
//! cross-process spec lock.
//!
//! This is the Rust analog of the TS `LockedFileManager` (`src/utils/file-manager.ts`):
//!
//! - `read_or_init_json` — reads the file if it exists, otherwise writes the
//!   supplied default (under the lock, re-checking existence) and returns it.
//! - `write_json_atomic` — serializes a value with 2-space indentation
//!   (matching the TS `JSON.stringify(value, null, 2)` output) and replaces
//!   the file atomically.
//!
//! BUG-477 (spec/features/concurrent-commands-never-corrupt-spec-json-files.feature):
//! every write of a spec state file (a file directly in `spec/`) holds the
//! spec lock ([`codelet_common::spec_lock::SpecLock`], reentrant, so a no-op
//! when the calling command already holds it from before its first read);
//! coverage sidecars keep their own sidecar transaction. Every write goes
//! through [`codelet_common::spec_lock::replace_file_atomically`]: a
//! uniquely named temp file in the same directory, fsync, rename, directory
//! fsync. The previous fixed `<file>.tmp` path was shared by every writer:
//! one writer's `O_TRUNC` open could truncate another's temp file and then
//! write into the inode the other had already renamed over the target,
//! leaving a valid document followed by the tail of a longer one.
//! Readers take no lock — an atomic rename means they see either the old or
//! the new complete file.

use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::FspecCoreError;
use crate::io::spec_lock::spec_write_guard;

/// Read a JSON file from `path`, parsing it into `T`. If the file does not
/// exist, write the supplied `default` to disk and return it.
///
/// The `file_label` is used in error messages and MUST match what the TS
/// implementation uses (e.g. `"work-units.json"`) so cross-frontend assertions
/// keep working.
pub fn read_or_init_json<T>(path: &Path, default: &T, file_label: &str) -> Result<T, FspecCoreError>
where
    T: DeserializeOwned + Serialize,
{
    if let Some(value) = read_existing(path, file_label)? {
        return Ok(value);
    }

    // Slow path: file missing → create with default under the spec lock,
    // re-checking so a concurrent initialiser's (or writer's) file is never
    // replaced by the default.
    let _guard = spec_write_guard(path, "read_or_init_json")?;
    if let Some(value) = read_existing(path, file_label)? {
        return Ok(value);
    }
    write_json_atomic_inner(path, default, false, true)?;

    // Re-read so any downstream serializer differences are visible to the
    // caller. (Cheap: we just wrote the file ourselves.)
    match read_existing(path, file_label)? {
        Some(value) => Ok(value),
        None => Err(FspecCoreError::Io {
            command: "read_or_init_json",
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{} vanished right after it was written", path.display()),
            ),
        }),
    }
}

/// `Ok(None)` when `path` does not exist; the parsed value otherwise.
fn read_existing<T: DeserializeOwned>(
    path: &Path,
    file_label: &str,
) -> Result<Option<T>, FspecCoreError> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(FspecCoreError::Io {
                command: "read_or_init_json",
                source,
            })
        }
    };
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|e| crate::io::json_error::parse_json_diagnostic(file_label, &raw, &e))
}

/// Atomically write `value` as pretty-printed JSON (2-space indent) to `path`.
///
/// Holds the spec lock when `path` is a spec state file and replaces the
/// file atomically (see the module docs).
pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), FspecCoreError> {
    write_json_atomic_inner(path, value, false, false)
}

/// WT-018: the board's own write, used only by
/// [`crate::io::board::write_work_units`].
pub(crate) fn write_board_json<T: Serialize>(path: &Path, value: &T) -> Result<(), FspecCoreError> {
    write_json_atomic_inner(path, value, false, true)
}

/// WT-018: `spec/work-units.json` is written only through
/// [`crate::io::board::write_work_units`], which records the session id of
/// a worktree's writes; every other writer refuses it.
fn refuse_board(path: &Path) -> Result<(), FspecCoreError> {
    let is_board = path
        .file_name()
        .is_some_and(|name| name == "work-units.json")
        && path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|dir| dir == "spec");
    if is_board {
        return Err(FspecCoreError::Io {
            command: "write_json_atomic",
            source: std::io::Error::other(format!(
                "{} is written only through io::board::write_work_units (WT-018)",
                path.display()
            )),
        });
    }
    Ok(())
}

/// Atomically write `value` as pretty-printed JSON (2-space indent) **with a
/// single trailing newline** to `path`.
///
/// This matches the TS foundation-mutation commands that write
/// `JSON.stringify(foundation, null, 2) + '\n'` (e.g. `add-capability`,
/// `remove-capability`, `add-persona`, `remove-persona`). The plain
/// [`write_json_atomic`] omits the trailing newline for byte-exact parity with
/// `JSON.stringify(...)` callers (`work-units.json`, the `FileManager`
/// eventStorm commands, etc.). Use this variant ONLY where the TS source
/// explicitly appends `'\n'`.
pub fn write_json_atomic_trailing_newline<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), FspecCoreError> {
    write_json_atomic_inner(path, value, true, false)
}

fn write_json_atomic_inner<T: Serialize>(
    path: &Path,
    value: &T,
    trailing_newline: bool,
    board_writer: bool,
) -> Result<(), FspecCoreError> {
    if !board_writer {
        refuse_board(path)?;
    }
    let mut serialized =
        serde_json::to_string_pretty(value).map_err(|e| FspecCoreError::InvalidArgs {
            command: "write_json_atomic",
            reason: format!("failed to serialize JSON: {e}"),
        })?;
    // Parity with TS `JSON.stringify(data, null, 2)`+`writeFile`: NO
    // trailing newline by default. Adding a final `\n` would diverge from
    // the byte-exact TS output for every command that writes through this
    // helper (`spec/work-units.json`, `spec/prefixes.json`,
    // `spec/epics.json`, `spec/tags.json`). The `trailing_newline` flag is
    // set ONLY for TS foundation commands that append `'\n'` explicitly.
    if trailing_newline {
        serialized.push('\n');
    }
    replace_bytes(path, serialized.as_bytes())
}

/// Replace `path` with `bytes` (under the spec lock when `path` is a spec
/// state file) through
/// [`codelet_common::spec_lock::replace_file_atomically`]. For spec files
/// that are not pretty-printed through [`write_json_atomic`] (e.g. coverage
/// sidecars serialised by their own formatter).
pub fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), FspecCoreError> {
    refuse_board(path)?;
    replace_bytes(path, bytes)
}

fn replace_bytes(path: &Path, bytes: &[u8]) -> Result<(), FspecCoreError> {
    let parent = crate::io::spec_lock::parent_dir(path, "write_json_atomic")?;
    std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
        command: "write_json_atomic",
        source,
    })?;
    let _guard = spec_write_guard(path, "write_json_atomic")?;
    codelet_common::spec_lock::replace_file_atomically(path, bytes).map_err(|source| {
        FspecCoreError::Io {
            command: "write_json_atomic",
            source,
        }
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;
    use serde::Deserialize;
    use tempfile::TempDir;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Sample {
        name: String,
        count: u32,
    }

    #[test]
    fn the_board_is_refused_with_an_error_naming_its_one_writer() {
        // WT-018: a command that writes spec/work-units.json any other way
        // fails with this exact error instead of writing it unmarked.
        let tmp = TempDir::new().unwrap();
        let board = tmp.path().join("spec").join("work-units.json");
        for result in [
            write_json_atomic(
                &board,
                &Sample {
                    name: "x".into(),
                    count: 1,
                },
            ),
            write_bytes_atomic(&board, b"{}"),
        ] {
            let message = result.unwrap_err().to_string();
            assert!(
                message.contains(&format!(
                    "{} is written only through io::board::write_work_units (WT-018)",
                    board.display()
                )),
                "{message}"
            );
        }
        assert!(!board.exists());
    }

    #[test]
    fn read_or_init_creates_file_with_default_when_missing() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("nested/data.json");
        let default = Sample {
            name: "init".into(),
            count: 0,
        };
        let got: Sample = read_or_init_json(&p, &default, "data.json").unwrap();
        assert_eq!(got, default);
        assert!(p.exists());
        let on_disk: Sample = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(on_disk, default);
    }

    #[test]
    fn read_or_init_returns_existing_content() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("data.json");
        let existing = Sample {
            name: "present".into(),
            count: 7,
        };
        write_json_atomic(&p, &existing).unwrap();
        let default = Sample {
            name: "ignored".into(),
            count: 0,
        };
        let got: Sample = read_or_init_json(&p, &default, "data.json").unwrap();
        assert_eq!(got, existing);
    }

    #[test]
    fn read_or_init_returns_parse_error_for_malformed_json() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("data.json");
        std::fs::write(&p, "{ not json").unwrap();
        let default = Sample {
            name: "ignored".into(),
            count: 0,
        };
        let err = read_or_init_json::<Sample>(&p, &default, "data.json").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Failed to parse data.json"),
            "unexpected error message: {msg}"
        );
    }
}
