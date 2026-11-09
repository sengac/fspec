//! Lock-free (with respect to the message store) durable receipt files:
//! `<data>/input-receipts/<session>/<submission>.json`, promoted by
//! stage + fsync + rename + directory fsync. A process-local per-session gate
//! serializes read-modify-write settlement.

use super::{InputReceipt, InputReceiptError, INPUT_RECEIPT_VERSION};
use crate::persistence::messages::durability::{perform, sync_directory};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use uuid::Uuid;

static GATES: LazyLock<Mutex<HashMap<Uuid, Arc<Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn gate(session_id: Uuid) -> Arc<Mutex<()>> {
    let mut gates = GATES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    gates.entry(session_id).or_default().clone()
}

pub(super) fn directory(session_id: Uuid) -> Result<PathBuf, InputReceiptError> {
    Ok(codelet_common::get_data_dir()
        .map_err(InputReceiptError::DataDirectory)?
        .join("input-receipts")
        .join(session_id.to_string()))
}

fn decode(bytes: &[u8]) -> Result<InputReceipt, InputReceiptError> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default();
    if version != u64::from(INPUT_RECEIPT_VERSION) {
        let id = value
            .get("id")
            .and_then(|id| serde_json::from_value(id.clone()).ok())
            .unwrap_or_default();
        return Err(InputReceiptError::UnknownVersion {
            id,
            version: u32::try_from(version).unwrap_or(u32::MAX),
        });
    }
    Ok(serde_json::from_value(value)?)
}

pub(super) fn read(session_id: Uuid, id: Uuid) -> Result<Option<InputReceipt>, InputReceiptError> {
    let path = directory(session_id)?.join(format!("{id}.json"));
    match fs::read(&path) {
        Ok(bytes) => {
            let receipt = decode(&bytes)?;
            if receipt.id != id || receipt.session_id != session_id {
                return Err(InputReceiptError::Conflict(id));
            }
            Ok(Some(receipt))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn ids(session_id: Uuid) -> Result<Vec<Uuid>, InputReceiptError> {
    let entries = match fs::read_dir(directory(session_id)?) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut ids = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            if let Some(id) = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| Uuid::parse_str(stem).ok())
            {
                ids.push(id);
            }
        }
    }
    Ok(ids)
}

pub(super) fn list(session_id: Uuid) -> Result<Vec<InputReceipt>, InputReceiptError> {
    let mut receipts = Vec::new();
    for id in ids(session_id)? {
        receipts.extend(read(session_id, id)?);
    }
    receipts.sort_by_key(|receipt| (receipt.submitted_at, receipt.id));
    Ok(receipts)
}

/// Serialized read-modify-write. `change` returns `Some` to promote a new
/// receipt version, or `None` when the durable state already matches.
pub(super) fn update(
    session_id: Uuid,
    id: Uuid,
    change: impl FnOnce(Option<InputReceipt>) -> Result<Option<InputReceipt>, InputReceiptError>,
) -> Result<(), InputReceiptError> {
    let gate = gate(session_id);
    let _owner = gate
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(receipt) = change(read(session_id, id)?)? else {
        return Ok(());
    };
    promote(&receipt)
}

fn promote(receipt: &InputReceipt) -> Result<(), InputReceiptError> {
    let directory = directory(receipt.session_id)?;
    fs::create_dir_all(&directory)?;
    let destination = directory.join(format!("{}.json", receipt.id));
    let stage = directory.join(format!("{}.{}.stage", receipt.id, Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(receipt)?;
    let written = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&stage)?;
        file.write_all(&bytes)?;
        perform("receipt.sync", || file.sync_all())?;
        perform("receipt.rename", || fs::rename(&stage, &destination))
    })();
    if let Err(error) = written {
        if let Err(cleanup) = fs::remove_file(&stage) {
            if cleanup.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(%cleanup, "could not remove unpromoted input receipt stage");
            }
        }
        return Err(error.into());
    }
    perform("receipt.directory", || sync_directory(&directory))?;
    Ok(())
}
