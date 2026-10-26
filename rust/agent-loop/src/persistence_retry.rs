use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use uuid::Uuid;

#[path = "persistence_retry_commit.rs"]
mod archive;
#[path = "persistence_retry_io.rs"]
mod io;
#[path = "persistence_retry_ownership.rs"]
mod ownership;

pub const MAX_PENDING_RECORDS: usize = 128;
pub const MAX_PENDING_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RecoveryRecord {
    pub version: u32,
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub metadata: HashMap<String, Value>,
}

#[derive(Default)]
struct RecoveryOwner {
    admitted: Vec<RecoveryRecord>,
    retained: Vec<RecoveryRecord>,
    journal: Option<codelet_core::persistence::transcript::JournalStore>,
}

static OWNERS: LazyLock<Mutex<HashMap<PathBuf, Arc<Mutex<RecoveryOwner>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct PersistenceSpool {
    session_id: Uuid,
    directory: PathBuf,
    owner: Arc<Mutex<RecoveryOwner>>,
}

impl PersistenceSpool {
    pub fn open(session_id: Uuid) -> Result<Self, String> {
        let spool = Self::configured(session_id)?;
        let mut owner = spool.owner.lock().map_err(|_| "recovery owner poisoned")?;
        if let Some(_lease) = spool.lease(&mut owner)? {
            io::private_directory(&spool.directory).map_err(|_| {
                "persistence recovery required; shutdown unsafe until journal repair"
            })?;
        }
        drop(owner);
        Ok(spool)
    }

    fn configured(session_id: Uuid) -> Result<Self, String> {
        let directory = codelet_common::get_data_dir()?
            .join("persistence-recovery")
            .join(session_id.to_string());
        Ok(Self {
            session_id,
            owner: ownership::owner(&directory)?,
            directory,
        })
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn enqueue(&self, record: &RecoveryRecord) -> Result<(), String> {
        if record.version != 1 {
            return Err("incompatible recovery admission version".into());
        }
        let mut owner = self.owner.lock().map_err(|_| "recovery owner poisoned")?;
        let _lease = self
            .lease(&mut owner)?
            .ok_or("recovery admission refused for deleted session")?;
        let disk = io::read(&self.directory)?;
        if !owner.retained.is_empty() {
            return Err(
                "recovery retained originals require explicit retry; original remains with caller"
                    .into(),
            );
        }
        let mut pending: Vec<_> = disk.iter().map(|item| item.record.clone()).collect();
        merge(&mut pending, &owner.admitted)?;
        if let Some(existing) = pending.iter().find(|item| item.id == record.id) {
            if existing != record {
                return Err("recovery identity conflict".into());
            }
        } else {
            let bytes = pending
                .iter()
                .try_fold(encoded_size(record)?, |total, item| {
                    encoded_size(item).map(|size| total.saturating_add(size))
                })?;
            if pending.len() >= MAX_PENDING_RECORDS || bytes > MAX_PENDING_BYTES {
                return Err(
                    "recovery admission capacity exhausted; original remains with caller".into(),
                );
            }
            owner.admitted.push(record.clone());
        }
        self.promote_memory(&owner.admitted, &disk).map_err(|_| {
            "persistence recovery required; shutdown unsafe until journal repair".into()
        })
    }

    fn retain(&self, record: &RecoveryRecord) -> Result<(), String> {
        let mut owner = self.owner.lock().map_err(|_| "recovery owner poisoned")?;
        let _lease = self
            .lease(&mut owner)?
            .ok_or("recovery admission refused for deleted session")?;
        if let Some(existing) = owner
            .admitted
            .iter()
            .chain(&owner.retained)
            .find(|item| item.id == record.id)
        {
            return if existing == record {
                Ok(())
            } else {
                Err("recovery identity conflict; original remains with caller".into())
            };
        }
        let bytes = owner
            .retained
            .iter()
            .try_fold(encoded_size(record)?, |total, item| {
                encoded_size(item).map(|size| total.saturating_add(size))
            })?;
        if record.version != 1 || owner.retained.len() >= 3 || bytes > MAX_PENDING_BYTES {
            return Err("recovery ownership capacity exhausted; original remains with caller; shutdown unsafe".into());
        }
        owner.retained.push(record.clone());
        Ok(())
    }

    pub fn pending_count(&self) -> Result<usize, String> {
        let mut owner = self.owner.lock().map_err(|_| "recovery owner poisoned")?;
        let Some(_lease) = self.lease(&mut owner)? else {
            return Ok(0);
        };
        let mut pending: Vec<_> = io::read(&self.directory)?
            .into_iter()
            .map(|item| item.record)
            .collect();
        merge(&mut pending, &owner.admitted)?;
        merge(&mut pending, &owner.retained)?;
        Ok(pending.len())
    }

    fn promote_memory(
        &self,
        memory: &[RecoveryRecord],
        disk: &[io::DiskRecord],
    ) -> Result<(), String> {
        let mut sequence = match disk.last() {
            Some(item) => io::sequence(&item.path)?,
            None => 0,
        };
        for record in memory {
            if let Some(existing) = disk.iter().find(|item| item.record.id == record.id) {
                if existing.record != *record {
                    return Err("recovery identity conflict".into());
                }
                io::sync(&self.directory)?;
            } else {
                sequence = sequence
                    .checked_add(1)
                    .ok_or("recovery sequence exhausted")?;
                io::promote(&self.directory, record, sequence)?;
            }
        }
        Ok(())
    }

    pub fn retry(&self) -> Result<(), String> {
        let mut owner = self.owner.lock().map_err(|_| "recovery owner poisoned")?;
        let Some(_lease) = self.lease(&mut owner)? else {
            return Ok(());
        };
        io::private_directory(&self.directory)
            .map_err(|_| "persistence recovery required; shutdown unsafe until journal repair")?;
        for _ in 0..2 {
            let disk = io::read(&self.directory)?;
            self.promote_memory(&owner.admitted, &disk).map_err(|_| {
                "persistence recovery required; shutdown unsafe until journal repair"
            })?;
            for pending in io::read(&self.directory)? {
                archive::commit(self.session_id, &pending.record, &mut owner.journal)?;
                std::fs::remove_file(&pending.path)
                    .map_err(|_| "persistence recovery acknowledgement failed")?;
                io::sync(&self.directory)
                    .map_err(|_| "persistence recovery acknowledgement failed")?;
                owner
                    .admitted
                    .retain(|record| record.id != pending.record.id);
            }
            let retained = std::mem::take(&mut owner.retained);
            owner.admitted.extend(retained);
        }
        Ok(())
    }
}

fn encoded_size(record: &RecoveryRecord) -> Result<usize, String> {
    serde_json::to_vec(record)
        .map(|bytes| bytes.len())
        .map_err(|_| "recovery encoding failed".into())
}

fn merge(pending: &mut Vec<RecoveryRecord>, memory: &[RecoveryRecord]) -> Result<(), String> {
    for record in memory {
        if let Some(existing) = pending.iter().find(|item| item.id == record.id) {
            if existing != record {
                return Err("recovery identity conflict".into());
            }
        } else {
            pending.push(record.clone());
        }
    }
    Ok(())
}

pub(crate) fn submit(session_id: Uuid, record: RecoveryRecord) -> Result<(), String> {
    let spool = PersistenceSpool::configured(session_id)?;
    if record.role == "transcript"
        && record.version == 1
        && encoded_size(&record)? <= MAX_PENDING_BYTES
    {
        let mut owner = spool.owner.lock().map_err(|_| "recovery owner poisoned")?;
        let _lease = spool
            .lease(&mut owner)?
            .ok_or("recovery admission refused for deleted session")?;
        if owner.admitted.is_empty() && owner.retained.is_empty() {
            if let Ok(disk) = io::read(&spool.directory) {
                if disk.is_empty() {
                    if archive::commit(session_id, &record, &mut owner.journal).is_ok() {
                        return Ok(());
                    }
                    owner.admitted.push(record.clone());
                    let _ = spool.promote_memory(&owner.admitted, &disk);
                }
            }
        }
    }
    if spool.enqueue(&record).is_err() {
        spool.retain(&record)?;
        return Err("persistence recovery required; shutdown unsafe until journal repair".into());
    }
    let mut result = Ok(());
    for _ in 0..3 {
        result = spool.retry();
        if result.is_ok() {
            return result;
        }
    }
    result
}
