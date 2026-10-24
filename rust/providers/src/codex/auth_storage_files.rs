use super::{AuthError, Result};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

pub(super) fn stage(home: &Path, prefix: &str, bytes: &[u8]) -> Result<NamedTempFile> {
    let mut file = tempfile::Builder::new().prefix(prefix).tempfile_in(home)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    Ok(file)
}

fn remove(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn rollback(home: &Path, backup: Option<NamedTempFile>, operation: AuthError) -> AuthError {
    let path = home.join("auth.json");
    match backup {
        Some(backup) => match backup.persist(path) {
            Ok(_) => operation,
            Err(mut failure) => {
                let recovery = Some(failure.file.path().to_owned());
                failure.file.disable_cleanup(true);
                AuthError::Rollback {
                    operation: Box::new(operation),
                    rollback: failure.error,
                    recovery,
                }
            }
        },
        None => match remove(&path) {
            Ok(()) => operation,
            Err(rollback) => AuthError::Rollback {
                operation: Box::new(operation),
                rollback,
                recovery: None,
            },
        },
    }
}

pub(super) fn change(
    home: &Path,
    document: Option<&Value>,
    finish: impl FnOnce() -> Result<()>,
) -> Result<()> {
    fs::create_dir_all(home)?;
    let path = home.join("auth.json");
    let backup = match fs::read(&path) {
        Ok(bytes) => Some(stage(home, ".fspec-auth-backup-", &bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    match document {
        Some(document) => {
            stage(
                home,
                ".fspec-auth-write-",
                &serde_json::to_vec_pretty(document)?,
            )?
            .persist(path)
            .map_err(|error| error.error)?;
        }
        None => remove(&path)?,
    }
    finish().map_err(|error| rollback(home, backup, error))
}
