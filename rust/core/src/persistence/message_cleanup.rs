use super::{init_message_store, SessionManifest, MESSAGE_STORE};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::path::Path;
use uuid::Uuid;

/// Remove unreferenced messages only when no session writer can be committing a reference.
pub fn cleanup_orphaned_messages() -> Result<usize, String> {
    init_message_store()?;
    let mut messages = MESSAGE_STORE.lock().map_err(|error| error.to_string())?;
    messages
        .as_mut()
        .ok_or("Message store not initialized")?
        .cleanup_with_references(|| {
            let sessions_dir = codelet_common::get_data_dir()?.join("sessions");
            let Some(leases) = idle_session_leases(&sessions_dir)? else {
                return Ok(None);
            };
            let referenced = committed_references(&sessions_dir)?;
            Ok(Some((referenced, leases)))
        })
}

fn idle_session_leases(directory: &Path) -> Result<Option<Vec<File>>, String> {
    let mut leases = Vec::new();
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Failed to read session leases: {error}")),
    };
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.extension().is_none_or(|extension| extension != "lock") {
            continue;
        }
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("Failed to open session lease {path:?}: {error}"))?;
        match lease.try_lock() {
            Ok(()) => leases.push(lease),
            Err(TryLockError::WouldBlock) => return Ok(None),
            Err(error) => return Err(format!("Failed to probe session lease {path:?}: {error}")),
        }
    }
    Ok(Some(leases))
}

fn committed_references(directory: &Path) -> Result<HashSet<Uuid>, String> {
    let mut referenced = HashSet::new();
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("Failed to read committed sessions: {error}"))?;
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let bytes = fs::read(&path)
            .map_err(|error| format!("Failed to read committed session {path:?}: {error}"))?;
        let session: SessionManifest = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Failed to parse committed session {path:?}: {error}"))?;
        referenced.extend(
            session
                .messages
                .into_iter()
                .map(|message| message.message_id),
        );
    }
    Ok(referenced)
}
