//! Binary extraction + atomic replacement for the update engine (UPD-002).
//!
//! The downloaded archive (unix `tar.gz` / Windows `zip`) is extracted to a
//! second temp file, then renamed into place. On unix the rename is atomic
//! over the running inode; on Windows `self-replace` handles the locked
//! `.exe` by scheduling the rename after exit. The new binary only lands in
//! place AFTER the checksum has passed (rule [4]).

use std::path::Path;
use tracing::debug;

use super::UpdateError;

/// Extract the `fspec` binary from the archive at `archive` into a temp file
/// in the install directory. Returns the temp file path.
///
/// The archive extension selects the extractor: `.tar.gz` → tar, `.zip` →
/// zip. The binary entry is named `fspec` (or `fspec.exe` on Windows).
pub async fn extract_binary(
    archive: &Path,
    install_path: &Path,
) -> Result<std::path::PathBuf, UpdateError> {
    let install_dir = install_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let extracted = install_dir.join(format!(".fspec-update-{}.bin", install_path.file_name().and_then(|n| n.to_str()).unwrap_or("fspec")));

    let is_zip = archive.extension().and_then(|e| e.to_str()) == Some("zip");
    if is_zip {
        extract_zip(archive, &extracted)?;
    } else {
        extract_targz(archive, &extracted)?;
    }
    debug!(?extracted, "update engine: binary extracted");
    Ok(extracted)
}

/// Replace `install_path` with the binary at `new_binary`.
///
/// Unix: `std::fs::rename` (atomic over the running inode).
/// Windows: `self_replace::self_replace` (the running `.exe` is locked; the
/// rename is scheduled to take effect after the process exits).
pub fn replace_binary(new_binary: &Path, install_path: &Path) -> Result<(), UpdateError> {
    #[cfg(windows)]
    {
        self_replace::self_replace(new_binary)
            .map_err(|e| UpdateError::ReplaceFailed(e.to_string()))?;
        // On Windows the rename is deferred until process exit; the install
        // path is unchanged on disk until then.
        let _ = install_path;
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        std::fs::rename(new_binary, install_path)
            .map_err(|e| UpdateError::ReplaceFailed(e.to_string()))?;
        Ok(())
    }
}

/// Extract the `fspec` entry from a `tar.gz` archive.
fn extract_targz(archive: &Path, out: &Path) -> Result<(), UpdateError> {
    let file = std::fs::File::open(archive)
        .map_err(|e| UpdateError::ReplaceFailed(format!("open archive: {e}")))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(gz);
    for entry in tar
        .entries()
        .map_err(|e| UpdateError::ReplaceFailed(format!("tar entries: {e}")))?
    {
        let mut entry =
            entry.map_err(|e| UpdateError::ReplaceFailed(format!("tar entry: {e}")))?;
        let name = entry.path().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let file_name = Path::new(&name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if file_name == "fspec" || file_name == "fspec.exe" {
            let mut out_file = std::fs::File::create(out)
                .map_err(|e| UpdateError::ReplaceFailed(format!("create temp binary: {e}")))?;
            std::io::copy(&mut entry, &mut out_file)
                .map_err(|e| UpdateError::ReplaceFailed(format!("extract binary: {e}")))?;
            return Ok(());
        }
    }
    Err(UpdateError::ReplaceFailed(
        "no fspec binary entry found in archive".into(),
    ))
}

/// Extract the `fspec` entry from a `zip` archive.
fn extract_zip(archive: &Path, out: &Path) -> Result<(), UpdateError> {
    let file = std::fs::File::open(archive)
        .map_err(|e| UpdateError::ReplaceFailed(format!("open archive: {e}")))?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|e| UpdateError::ReplaceFailed(format!("zip open: {e}")))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| UpdateError::ReplaceFailed(format!("zip entry: {e}")))?;
        let file_name = entry
            .enclosed_name()
            .map(|p| p.to_string_lossy().into_owned())
            .and_then(|s| {
                Path::new(&s)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(str::to_string)
            })
            .unwrap_or_default();
        if file_name == "fspec" || file_name == "fspec.exe" {
            let mut out_file = std::fs::File::create(out)
                .map_err(|e| UpdateError::ReplaceFailed(format!("create temp binary: {e}")))?;
            std::io::copy(&mut entry, &mut out_file)
                .map_err(|e| UpdateError::ReplaceFailed(format!("extract binary: {e}")))?;
            return Ok(());
        }
    }
    Err(UpdateError::ReplaceFailed(
        "no fspec binary entry found in archive".into(),
    ))
}
