use super::{RecoveryRecord, MAX_PENDING_BYTES, MAX_PENDING_RECORDS};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub(super) struct DiskRecord {
    pub path: PathBuf,
    pub record: RecoveryRecord,
}

pub(super) fn sequence(path: &Path) -> Result<u64, String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.split_once('-'))
        .ok_or("recovery ordering coordinate unavailable")?
        .0
        .parse()
        .map_err(|_| "invalid recovery ordering coordinate".into())
}

pub(super) fn private_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| format!("recovery directory unavailable: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("recovery directory privacy failed: {error}"))?;
    }
    Ok(())
}

pub(super) fn read(path: &Path) -> Result<Vec<DiskRecord>, String> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("recovery journal unavailable: {error}")),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            paths.push(path);
        }
        if paths.len() > MAX_PENDING_RECORDS {
            return Err("recovery capacity is invalid".into());
        }
    }
    paths.sort();
    let mut total = 0;
    paths
        .into_iter()
        .map(|path| {
            let length = fs::metadata(&path)
                .map_err(|error| error.to_string())?
                .len();
            total += usize::try_from(length).map_err(|_| "recovery record too large")?;
            if total > MAX_PENDING_BYTES {
                return Err("recovery byte capacity is invalid".into());
            }
            let encoded = fs::read(&path).map_err(|error| error.to_string())?;
            let record: RecoveryRecord = serde_json::from_slice(&encoded)
                .map_err(|_| "recovery record cannot be decoded")?;
            if record.version != 1 {
                return Err("incompatible recovery record version".into());
            }
            let sequence = sequence(&path)?;
            let expected = format!("{sequence:020}-{}.json", record.id);
            if path.file_name().and_then(|name| name.to_str()) != Some(expected.as_str()) {
                return Err("recovery filename identity conflict".into());
            }
            Ok(DiskRecord { path, record })
        })
        .collect()
}

pub(super) fn promote(path: &Path, record: &RecoveryRecord, sequence: u64) -> Result<(), String> {
    private_directory(path)?;
    let destination = path.join(format!("{sequence:020}-{}.json", record.id));
    let stage = path.join(format!("{}.stage", record.id));
    let operation = || -> Result<(), String> {
        let mut options = OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&stage).map_err(|error| error.to_string())?;
        file.write_all(&serde_json::to_vec(record).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        fs::rename(&stage, destination).map_err(|error| error.to_string())?;
        sync(path)?;
        if let Some(parent) = path.parent() {
            sync(parent)?;
        }
        Ok(())
    };
    let result = operation();
    if result.is_err() {
        let _ = fs::remove_file(stage);
    }
    result
}

pub(super) fn sync(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|file| file.sync_all())
            .map_err(|error| error.to_string())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err("durable recovery directory barriers unsupported".into())
    }
}
