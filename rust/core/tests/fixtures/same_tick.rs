//! BUG-494: put a message store into the state a loaded or fast machine
//! produces at random - the saved index and the message history share one
//! modification time (one filesystem timestamp tick), and the completeness
//! proof (messages.verified), when the store wrote one, matches the current
//! generation of both files exactly, as the store writes it after publishing
//! an index. Tests call this so the same-tick path runs on every run instead
//! of depending on timing (shared-message-store-index-coherence, BUG-519).
use std::error::Error;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Share one modification time, an hour in the past, between
/// `messages.jsonl` and `messages.idx` under `data_dir/messages`, then
/// re-prove an existing completeness proof for the resulting generation.
pub fn share_index_tick(data_dir: &Path) -> Result<(), Box<dyn Error>> {
    let messages = data_dir.join("messages");
    let shared = SystemTime::now() - Duration::from_secs(3600);
    for name in ["messages.jsonl", "messages.idx"] {
        std::fs::OpenOptions::new()
            .write(true)
            .open(messages.join(name))?
            .set_modified(shared)?;
    }
    let proof_path = messages.join("messages.verified");
    if !proof_path.exists() {
        return Ok(());
    }
    let mut proof: serde_json::Value = serde_json::from_slice(&std::fs::read(&proof_path)?)?;
    if proof["version"] != 1 {
        return Err(format!("unexpected completeness proof: {proof}").into());
    }
    proof["generation"] = serde_json::json!({
        "messages": stamp(&messages.join("messages.jsonl"))?,
        "index": stamp(&messages.join("messages.idx"))?,
    });
    std::fs::write(proof_path, serde_json::to_vec(&proof)?)?;
    Ok(())
}

fn time(time: SystemTime) -> Result<serde_json::Value, Box<dyn Error>> {
    let since = time.duration_since(UNIX_EPOCH)?;
    Ok(serde_json::json!({
        "secs_since_epoch": since.as_secs(),
        "nanos_since_epoch": since.subsec_nanos(),
    }))
}

fn stamp(path: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let metadata = std::fs::metadata(path)?;
    let mut stamp = serde_json::json!({
        "length": metadata.len(),
        "modified": time(metadata.modified()?)?,
        "created": metadata.created().ok().map(time).transpose()?,
    });
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        stamp["identity"] = serde_json::json!([
            metadata.dev(),
            metadata.ino(),
            metadata.ctime(),
            metadata.ctime_nsec()
        ]);
    }
    Ok(stamp)
}
