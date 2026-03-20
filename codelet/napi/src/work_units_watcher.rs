//! Work Units File Watcher
//!
//! Cross-platform file watcher for spec/work-units.json using notify crate.
//! Emits StreamChunk::WorkUnitsUpdate events to TypeScript via callback.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use serde::Deserialize;
use tracing::{debug, error, warn};

use crate::types::{StreamChunk, WorkUnitInfo};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkUnit {
    id: String,
    title: String,
    #[serde(rename = "type", default)]
    work_type: Option<String>,
    status: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    estimate: Option<i32>,
    #[serde(default)]
    epic: Option<String>,
}

impl From<WorkUnit> for WorkUnitInfo {
    fn from(wu: WorkUnit) -> Self {
        let work_type = wu.work_type.unwrap_or_else(|| {
            warn!("Work unit {} is missing 'type' field, defaulting to 'story'", wu.id);
            "story".to_string()
        });
        WorkUnitInfo {
            id: wu.id,
            title: wu.title,
            work_type,
            status: wu.status,
            description: wu.description,
            estimate: wu.estimate,
            epic: wu.epic,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkUnitsFile {
    work_units: HashMap<String, WorkUnit>,
}

struct WatcherState {
    work_units: HashMap<String, WorkUnit>,
    watch_path: PathBuf,
    is_watching: bool,
}

impl WatcherState {
    fn new() -> Self {
        WatcherState {
            work_units: HashMap::new(),
            watch_path: PathBuf::new(),
            is_watching: false,
        }
    }
}

lazy_static::lazy_static! {
    static ref WATCHER_STATE: Arc<RwLock<WatcherState>> = Arc::new(RwLock::new(WatcherState::new()));
    static ref WATCHER_HANDLE: Arc<RwLock<Option<notify_debouncer_mini::Debouncer<RecommendedWatcher>>>> = 
        Arc::new(RwLock::new(None));
}

fn load_work_units(path: &Path) -> Result<HashMap<String, WorkUnit>> {
    if !path.exists() {
        debug!("Work units file does not exist: {:?}", path);
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(path)
        .map_err(|e| Error::from_reason(format!("Failed to read work-units.json: {}", e)))?;

    let file: WorkUnitsFile = serde_json::from_str(&content)
        .map_err(|e| Error::from_reason(format!("Failed to parse work-units.json: {}", e)))?;

    Ok(file.work_units)
}

fn create_update_chunk(units: &HashMap<String, WorkUnit>) -> StreamChunk {
    let work_units: Vec<WorkUnitInfo> = units.values().cloned().map(WorkUnitInfo::from).collect();
    StreamChunk::work_units_update(work_units)
}

#[napi]
pub fn start_work_units_watcher(
    project_root: String,
    #[napi(ts_arg_type = "(chunk: import('./index').StreamChunk) => void")]
    callback: ThreadsafeFunction<StreamChunk>,
) -> Result<()> {
    let work_units_path = Path::new(&project_root).join("spec").join("work-units.json");

    let initial_units = load_work_units(&work_units_path)?;
    
    {
        let mut state = WATCHER_STATE.write().map_err(|e| {
            Error::from_reason(format!("Failed to acquire watcher state lock: {}", e))
        })?;
        state.work_units = initial_units.clone();
        state.watch_path = work_units_path.clone();
        state.is_watching = true;
    }

    let initial_chunk = create_update_chunk(&initial_units);
    callback.call(Ok(initial_chunk), ThreadsafeFunctionCallMode::NonBlocking);

    let watch_path = work_units_path.clone();
    
    let mut debouncer = new_debouncer(
        Duration::from_millis(100),
        move |res: std::result::Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
            match res {
                Ok(events) => {
                    // Only react to events that touch work-units.json itself.
                    // The spec/ directory also contains lock files (.lock dirs)
                    // created by proper-lockfile during reads. Without this filter,
                    // loadData() → lock creation → watcher event → loadData() creates
                    // an infinite feedback loop burning ~20% CPU while idle.
                    let relevant = events.iter().any(|e| {
                        matches!(e.kind, DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous)
                            && e.path.file_name().is_some_and(|name| name == "work-units.json")
                    });
                    
                    if relevant {
                        match load_work_units(&watch_path) {
                            Ok(units) => {
                                if let Ok(mut state) = WATCHER_STATE.write() {
                                    state.work_units = units.clone();
                                }
                                let chunk = create_update_chunk(&units);
                                callback.call(Ok(chunk), ThreadsafeFunctionCallMode::NonBlocking);
                            }
                            Err(e) => {
                                warn!("Failed to reload work units: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("File watcher error: {:?}", e);
                }
            }
        },
    ).map_err(|e| Error::from_reason(format!("Failed to create file watcher: {}", e)))?;

    let spec_dir = work_units_path.parent().unwrap_or(Path::new("."));
    debouncer.watcher().watch(spec_dir, RecursiveMode::NonRecursive)
        .map_err(|e| Error::from_reason(format!("Failed to watch directory: {}", e)))?;

    {
        let mut handle = WATCHER_HANDLE.write().map_err(|e| {
            Error::from_reason(format!("Failed to acquire watcher handle lock: {}", e))
        })?;
        *handle = Some(debouncer);
    }

    debug!("Work units watcher started successfully");
    Ok(())
}

#[napi]
pub fn stop_work_units_watcher() -> Result<()> {
    debug!("Stopping work units watcher");
    
    {
        let mut handle = WATCHER_HANDLE.write().map_err(|e| {
            Error::from_reason(format!("Failed to acquire watcher handle lock: {}", e))
        })?;
        *handle = None;
    }
    
    {
        let mut state = WATCHER_STATE.write().map_err(|e| {
            Error::from_reason(format!("Failed to acquire watcher state lock: {}", e))
        })?;
        state.is_watching = false;
        state.work_units.clear();
    }
    
    debug!("Work units watcher stopped");
    Ok(())
}

#[napi]
pub fn get_work_unit_status(work_unit_id: String) -> Result<Option<String>> {
    let state = WATCHER_STATE.read().map_err(|e| {
        Error::from_reason(format!("Failed to acquire watcher state lock: {}", e))
    })?;
    
    Ok(state.work_units.get(&work_unit_id).map(|wu| wu.status.clone()))
}

#[napi]
pub fn get_work_unit(work_unit_id: String) -> Result<Option<WorkUnitInfo>> {
    let state = WATCHER_STATE.read().map_err(|e| {
        Error::from_reason(format!("Failed to acquire watcher state lock: {}", e))
    })?;
    
    Ok(state.work_units.get(&work_unit_id).cloned().map(WorkUnitInfo::from))
}

#[napi]
pub fn get_all_work_units() -> Result<Vec<WorkUnitInfo>> {
    let state = WATCHER_STATE.read().map_err(|e| {
        Error::from_reason(format!("Failed to acquire watcher state lock: {}", e))
    })?;
    
    Ok(state.work_units.values().cloned().map(WorkUnitInfo::from).collect())
}

#[napi]
pub fn is_work_units_watcher_active() -> Result<bool> {
    let state = WATCHER_STATE.read().map_err(|e| {
        Error::from_reason(format!("Failed to acquire watcher state lock: {}", e))
    })?;
    
    Ok(state.is_watching)
}
