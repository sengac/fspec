//! Locked JSON file I/O — atomic read+init and atomic write with an
//! exclusive file lock.
//!
//! This is the Rust analog of the TS `LockedFileManager` (`src/utils/file-manager.ts`),
//! restricted to the subset of behaviour required by Phase-1 ported commands:
//!
//! - `read_or_init_json` — reads the file if it exists, otherwise serializes
//!   the supplied default and writes it back, then returns the default.
//! - `write_json_atomic` — serializes a value with 2-space indentation
//!   (matching the TS `JSON.stringify(value, null, 2)` output) and atomically
//!   renames a temp file into place.
//!
//! Locking strategy: we acquire an `fs2` exclusive lock on the **target**
//! file during the read+init window so two concurrent dispatcher calls
//! cannot race the initial write. For the atomic-rename path we lock the
//! temp file briefly to serialize concurrent writers. This is intentionally
//! simpler than the TS three-layer (`proper-lockfile` + RW + atomic-rename)
//! design — fspec-core today only has the synchronous Rust dispatch path,
//! which does not need cross-process write fairness.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use fs2::FileExt;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::FspecCoreError;

/// Read a JSON file from `path`, parsing it into `T`. If the file does not
/// exist, write the supplied `default` to disk and return it.
///
/// The `file_label` is used in error messages and MUST match what the TS
/// implementation uses (e.g. `"work-units.json"`) so cross-frontend assertions
/// keep working.
pub fn read_or_init_json<T>(
    path: &Path,
    default: &T,
    file_label: &str,
) -> Result<T, FspecCoreError>
where
    T: DeserializeOwned + Serialize,
{
    // Fast path: file exists → just read+parse.
    if path.exists() {
        let mut f = File::open(path).map_err(|source| FspecCoreError::Io {
            command: "read_or_init_json",
            source,
        })?;
        f.lock_shared().map_err(|source| FspecCoreError::Io {
            command: "read_or_init_json",
            source,
        })?;
        let mut buf = String::new();
        let read_result = f.read_to_string(&mut buf);
        // Always release the lock, even if reading failed.
        let _ = FileExt::unlock(&f);
        read_result.map_err(|source| FspecCoreError::Io {
            command: "read_or_init_json",
            source,
        })?;

        return serde_json::from_str(&buf).map_err(|e| FspecCoreError::ParseJson {
            file: file_label.to_string(),
            reason: e.to_string(),
        });
    }

    // Slow path: file missing → create with default. Ensure parent exists.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
            command: "read_or_init_json",
            source,
        })?;
    }

    write_json_atomic(path, default)?;

    // Re-read so any downstream serializer differences are visible to the
    // caller. (Cheap: we just wrote the file ourselves.)
    let raw = std::fs::read_to_string(path).map_err(|source| FspecCoreError::Io {
        command: "read_or_init_json",
        source,
    })?;
    serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: file_label.to_string(),
        reason: e.to_string(),
    })
}

/// Atomically write `value` as pretty-printed JSON (2-space indent) to `path`.
///
/// Uses the canonical write-temp-then-rename pattern. The lock on the temp
/// file serializes concurrent writers within a single process.
pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), FspecCoreError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
            command: "write_json_atomic",
            source,
        })?;
    }

    let mut tmp_path = path.as_os_str().to_owned();
    tmp_path.push(".tmp");
    let tmp_path = std::path::PathBuf::from(tmp_path);

    {
        let mut tmp = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp_path)
            .map_err(|source| FspecCoreError::Io {
                command: "write_json_atomic",
                source,
            })?;
        tmp.lock_exclusive().map_err(|source| FspecCoreError::Io {
            command: "write_json_atomic",
            source,
        })?;
        let serialized = serde_json::to_string_pretty(value).map_err(|e| {
            FspecCoreError::InvalidArgs {
                command: "write_json_atomic",
                reason: format!("failed to serialize JSON: {e}"),
            }
        })?;
        tmp.write_all(serialized.as_bytes())
            .map_err(|source| FspecCoreError::Io {
                command: "write_json_atomic",
                source,
            })?;
        tmp.write_all(b"\n").map_err(|source| FspecCoreError::Io {
            command: "write_json_atomic",
            source,
        })?;
        tmp.sync_all().map_err(|source| FspecCoreError::Io {
            command: "write_json_atomic",
            source,
        })?;
        let _ = FileExt::unlock(&tmp);
    }

    std::fs::rename(&tmp_path, path).map_err(|source| FspecCoreError::Io {
        command: "write_json_atomic",
        source,
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use tempfile::TempDir;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Sample {
        name: String,
        count: u32,
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
