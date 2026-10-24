use super::super::durability::{perform, sync_parent};
use super::{IndexEntry, INDEX_MAGIC, INDEX_VERSION};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

pub(crate) fn save_index(
    index_path: &Path,
    index: &HashMap<Uuid, IndexEntry>,
    data_file_size: u64,
) -> Result<(), String> {
    let parent = index_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to ensure index parent dir {parent:?}: {error}"))?;

    let mut file = tempfile::Builder::new()
        .prefix("messages.idx.")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|error| format!("Failed to create index temp file: {error}"))?;

    perform("index.write", || file.write_all(INDEX_MAGIC))
        .map_err(|error| format!("Failed to write index magic: {error}"))?;
    file.write_all(&INDEX_VERSION.to_le_bytes())
        .map_err(|error| format!("Failed to write index version: {error}"))?;
    file.write_all(&data_file_size.to_le_bytes())
        .map_err(|error| format!("Failed to write data file size: {error}"))?;
    file.write_all(&(index.len() as u32).to_le_bytes())
        .map_err(|error| format!("Failed to write entry count: {error}"))?;

    for (uuid, entry) in index {
        file.write_all(uuid.as_bytes())
            .map_err(|error| format!("Failed to write uuid: {error}"))?;
        file.write_all(&entry.byte_offset.to_le_bytes())
            .map_err(|error| format!("Failed to write byte_offset: {error}"))?;
        file.write_all(&entry.byte_length.to_le_bytes())
            .map_err(|error| format!("Failed to write byte_length: {error}"))?;
    }

    file.flush()
        .map_err(|error| format!("Failed to flush index file: {error}"))?;
    perform("index.sync", || file.as_file().sync_all())
        .map_err(|error| format!("Failed to synchronize index file: {error}"))?;
    perform("index.rename", || {
        file.persist(index_path)
            .map(|_| ())
            .map_err(|error| error.error)
    })
    .map_err(|error| format!("Failed to rename index temp file: {error}"))?;
    perform("index.directory", || sync_parent(index_path))
        .map_err(|error| format!("Index promoted; failed to synchronize directory: {error}"))?;

    Ok(())
}
