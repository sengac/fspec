use super::durability::{perform, sync_parent};
use super::{compute_hash, MessageStore};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use uuid::Uuid;

mod identity;
mod references;
mod scan;
pub(super) use scan::{parse_record, scan_range};

#[derive(Debug, thiserror::Error)]
pub enum FramingError {
    #[error("message framing I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("corrupt message record at byte {offset}: {reason}")]
    Corrupt { offset: u64, reason: String },
    #[error("committed message {message_id} is corrupt; tail recovery refused")]
    Referenced { message_id: Uuid },
    #[error("committed reference evidence unavailable: {0}")]
    Evidence(String),
    #[error("message store unavailable: {0}")]
    Store(String),
}

fn unterminated_tail(path: &Path) -> Result<Option<(u64, Vec<u8>)>, FramingError> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let length = file.metadata()?.len();
    let mut cursor = length;
    let mut start = 0;
    while cursor > 0 {
        codelet_common::read_work::checkpoint("history.tail")?;
        let block_start = cursor.saturating_sub(8192);
        file.seek(SeekFrom::Start(block_start))?;
        let mut block = vec![0; (cursor - block_start) as usize];
        file.read_exact(&mut block)?;
        if let Some(position) = block.iter().rposition(|byte| *byte == b'\n') {
            start = block_start + position as u64 + 1;
            break;
        }
        cursor = block_start;
    }
    if start == length {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(start))?;
    let mut tail = Vec::new();
    codelet_common::read_work::read_to_end(&mut file, &mut tail, "history.tail.bytes")?;
    Ok(Some((start, tail)))
}

fn quarantine(path: &Path, offset: u64, bytes: &[u8]) -> Result<(), FramingError> {
    let directory = path
        .parent()
        .ok_or_else(|| FramingError::Evidence("message history has no parent directory".into()))?;
    let target = directory.join(format!(
        "messages.tail.{offset}.{}.quarantine",
        compute_hash(bytes)
    ));
    if target.exists() {
        if fs::read(&target)? != bytes {
            return Err(FramingError::Evidence(
                "quarantine evidence mismatch".into(),
            ));
        }
        perform("tail.quarantine.sync", || File::open(&target)?.sync_all())?;
    } else {
        let mut stage = tempfile::Builder::new()
            .prefix("messages.tail.")
            .suffix(".stage")
            .tempfile_in(directory)?;
        stage.write_all(bytes)?;
        perform("tail.quarantine.sync", || stage.as_file().sync_all())?;
        stage
            .persist_noclobber(&target)
            .map_err(|error| FramingError::Io(error.error))?;
    }
    perform("tail.quarantine.directory", || sync_parent(&target))?;
    Ok(())
}

impl MessageStore {
    /// Validate history with typed failures and durably recover eligible uncommitted tails.
    pub fn validate_framing(&self) -> Result<(), FramingError> {
        let _lock = self.lock_file().map_err(FramingError::Store)?;
        self.recover_tail_locked()?;
        if self.messages_file.exists() {
            scan_range(&self.messages_file, 0)?;
        }
        Ok(())
    }

    pub(super) fn recover_tail_locked(&self) -> Result<(), FramingError> {
        let Some((offset, tail)) = unterminated_tail(&self.messages_file)? else {
            return Ok(());
        };
        let (prefix, _) = scan_range(&self.messages_file, 0)?;
        if tail.iter().all(u8::is_ascii_whitespace) || parse_record(&tail, offset).is_ok() {
            let mut file = OpenOptions::new().append(true).open(&self.messages_file)?;
            file.write_all(b"\n")?;
            perform("tail.sync", || file.sync_all())?;
            perform("tail.directory", || sync_parent(&self.messages_file))?;
            return Ok(());
        }

        quarantine(&self.messages_file, offset, &tail)?;
        let data_root = self
            .messages_dir
            .parent()
            .ok_or_else(|| FramingError::Evidence("message history has no data root".into()))?;
        let referenced = references::committed_references(&data_root.join("sessions"))?;
        for message_id in &referenced {
            if !prefix.contains_key(message_id) {
                return Err(FramingError::Referenced {
                    message_id: *message_id,
                });
            }
        }
        if let Some(message_id) =
            identity::before_framing_failure(&tail).filter(|id| referenced.contains(id))
        {
            return Err(FramingError::Referenced { message_id });
        }
        match fs::remove_file(&self.index_file) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        sync_parent(&self.index_file)?;
        let file = OpenOptions::new().write(true).open(&self.messages_file)?;
        perform("tail.truncate", || file.set_len(offset))?;
        perform("tail.sync", || file.sync_all())?;
        perform("tail.directory", || sync_parent(&self.messages_file))?;
        Ok(())
    }
}
