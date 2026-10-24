use codelet_core::persistence::reset_stores_for_tests;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use uuid::Uuid;

pub fn cold(root: &Path, session: Uuid, mode: &str) -> Result<ExitStatus, std::io::Error> {
    reset_stores_for_tests();
    Command::new(std::env::current_exe()?)
        .args(["--ignored", "--exact", "retirement_child"])
        .env("BUG250_ROOT", root)
        .env("BUG250_SESSION", session.to_string())
        .env("BUG250_MODE", mode)
        .status()
}

pub fn pending(root: &Path, session: Uuid) -> PathBuf {
    root.join("persistence-recovery").join(session.to_string())
}

pub fn bytes(directory: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, std::io::Error> {
    let mut records = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let encoded = std::fs::read(&path)?;
            records.push((path, encoded));
        }
    }
    records.sort();
    Ok(records)
}
