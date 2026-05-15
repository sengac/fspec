//! Work Units File Watcher — NAPI compatibility shim (RPC-006).
//!
//! After RPC-006 the cross-platform `notify`-based watcher logic lives
//! in `codelet/core/src/work_units.rs`. This module is now a thin shim
//! that wraps the lifted [`codelet_core::work_units::WorkUnitsWatcher`]
//! and forwards each broadcast event into the existing
//! [`ThreadsafeFunction`] callback so the TypeScript side keeps working
//! unchanged.
//!
//! Exported NAPI surface (preserved verbatim):
//!   - `startWorkUnitsWatcher(projectRoot, callback)`
//!   - `stopWorkUnitsWatcher()`
//!   - `getAllWorkUnits()`, `getWorkUnit(id)`, `getWorkUnitStatus(id)`
//!   - `isWorkUnitsWatcherActive()`

use std::path::Path;
use std::sync::{Arc, RwLock};

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc_types::WorkUnitInfo;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use tracing::{debug, warn};

use crate::types::StreamChunk;

/// Globals: watcher handle (owns the notify debouncer) and the latest
/// snapshot cache used by the synchronous getters.
struct ShimState {
    watcher: Option<WorkUnitsWatcher>,
    snapshot: Vec<WorkUnitInfo>,
    is_watching: bool,
}

impl ShimState {
    fn new() -> Self {
        Self {
            watcher: None,
            snapshot: Vec::new(),
            is_watching: false,
        }
    }
}

lazy_static::lazy_static! {
    static ref SHIM_STATE: Arc<RwLock<ShimState>> = Arc::new(RwLock::new(ShimState::new()));
}

#[napi]
pub fn start_work_units_watcher(
    project_root: String,
    #[napi(ts_arg_type = "(chunk: import('./index').StreamChunk) => void")]
    callback: ThreadsafeFunction<StreamChunk>,
) -> Result<()> {
    let workspace = Path::new(&project_root).to_path_buf();
    let watcher = WorkUnitsWatcher::new(&workspace).map_err(|e| {
        Error::from_reason(format!("create WorkUnitsWatcher: {e}"))
    })?;

    // Subscribe BEFORE we capture the initial snapshot so we don't miss
    // any push that lands between the snapshot read and the subscribe
    // call.
    let mut rx = watcher.subscribe();
    let initial = watcher.snapshot();

    {
        let mut guard = SHIM_STATE.write().map_err(|e| {
            Error::from_reason(format!("acquire shim-state lock: {e}"))
        })?;
        guard.watcher = Some(watcher);
        guard.snapshot = initial.clone();
        guard.is_watching = true;
    }

    // Fire the initial chunk synchronously so the TS side observes it
    // before any potential subsequent file mutation (matches legacy
    // RPC-005 behaviour).
    callback.call(
        Ok(StreamChunk::work_units_update(initial)),
        ThreadsafeFunctionCallMode::NonBlocking,
    );

    // Drive the broadcast receiver from a dedicated std::thread using
    // `broadcast::Receiver::blocking_recv` so the shim does NOT need a
    // Tokio runtime context (NAPI synchronous functions are called
    // outside of any Tokio context, which is why an earlier draft that
    // used `tokio::spawn` panicked with "no reactor running"). The
    // shim's lazy_static state holds the watcher alive, so the
    // receiver stays open until stop_work_units_watcher() is called.
    std::thread::Builder::new()
        .name("napi-work-units-watcher".to_string())
        .spawn(move || {
            use tokio::sync::broadcast::error::RecvError;
            loop {
                match rx.blocking_recv() {
                    Ok(snapshot) => {
                        if let Ok(mut guard) = SHIM_STATE.write() {
                            guard.snapshot = snapshot.clone();
                        }
                        callback.call(
                            Ok(StreamChunk::work_units_update(snapshot)),
                            ThreadsafeFunctionCallMode::NonBlocking,
                        );
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        debug!("napi shim watcher lagged; skipped={skipped}");
                        continue;
                    }
                    Err(RecvError::Closed) => {
                        debug!("napi shim watcher channel closed");
                        break;
                    }
                }
            }
        })
        .map_err(|e| {
            Error::from_reason(format!("spawn napi-work-units-watcher thread: {e}"))
        })?;

    debug!("Work units watcher (post-RPC-006 shim) started successfully");
    Ok(())
}

#[napi]
pub fn stop_work_units_watcher() -> Result<()> {
    debug!("Stopping work units watcher (post-RPC-006 shim)");
    let mut guard = SHIM_STATE.write().map_err(|e| {
        Error::from_reason(format!("acquire shim-state lock: {e}"))
    })?;
    guard.watcher = None;
    guard.is_watching = false;
    guard.snapshot.clear();
    Ok(())
}

#[napi]
pub fn get_work_unit_status(work_unit_id: String) -> Result<Option<String>> {
    let guard = SHIM_STATE.read().map_err(|e| {
        Error::from_reason(format!("acquire shim-state lock: {e}"))
    })?;
    Ok(guard
        .snapshot
        .iter()
        .find(|wu| wu.id == work_unit_id)
        .map(|wu| wu.status.clone()))
}

#[napi]
pub fn get_work_unit(work_unit_id: String) -> Result<Option<WorkUnitInfo>> {
    let guard = SHIM_STATE.read().map_err(|e| {
        Error::from_reason(format!("acquire shim-state lock: {e}"))
    })?;
    Ok(guard
        .snapshot
        .iter()
        .find(|wu| wu.id == work_unit_id)
        .cloned())
}

#[napi]
pub fn get_all_work_units() -> Result<Vec<WorkUnitInfo>> {
    let guard = SHIM_STATE.read().map_err(|e| {
        Error::from_reason(format!("acquire shim-state lock: {e}"))
    })?;
    Ok(guard.snapshot.clone())
}

#[napi]
pub fn is_work_units_watcher_active() -> Result<bool> {
    let guard = SHIM_STATE.read().map_err(|e| {
        Error::from_reason(format!("acquire shim-state lock: {e}"))
    })?;
    Ok(guard.is_watching)
}

/// RPC-017: move the work unit with `id` one position UP in its current
/// `states[<column>]` array in `<cwd>/spec/work-units.json`. Delegates
/// to the shared `codelet_core::work_units_write::move_work_unit`
/// helper so both this NAPI export AND the new
/// `FspecService::move_work_unit_up` RPC method converge on a single
/// inter-process-locked atomic-write code path. No-op at the top
/// boundary. Returns an error when the unit lives in the done column,
/// is unknown, or on I/O / data-integrity failure.
///
/// Additive: the existing TS `fspec prioritize-work-unit` command path
/// continues to use `fileManager.transaction` and is NOT changed by
/// RPC-017. Both paths cooperate through the same proper-lockfile-
/// compatible mkdir lock on `spec/work-units.json.lock`.
///
/// @param cwd - Path to the workspace root (containing `spec/work-units.json`)
/// @param id  - Work unit ID to reorder
#[napi]
pub fn move_work_unit_up(cwd: String, id: String) -> Result<()> {
    codelet_core::work_units_write::move_work_unit(
        std::path::Path::new(&cwd),
        &id,
        codelet_core::work_units_write::Direction::Up,
    )
    .map_err(|e| Error::from_reason(format!("{e:#}")))
}

/// RPC-017: mirror of [`move_work_unit_up`] for the DOWN direction.
///
/// @param cwd - Path to the workspace root (containing `spec/work-units.json`)
/// @param id  - Work unit ID to reorder
#[napi]
pub fn move_work_unit_down(cwd: String, id: String) -> Result<()> {
    codelet_core::work_units_write::move_work_unit(
        std::path::Path::new(&cwd),
        &id,
        codelet_core::work_units_write::Direction::Down,
    )
    .map_err(|e| Error::from_reason(format!("{e:#}")))
}

// `warn!` is referenced indirectly via tracing macros above. Keep the
// import in scope so linters do not flag the unused import in builds
// that compile this file without the napi feature gate.
#[allow(dead_code)]
fn _ensure_warn_in_scope() {
    warn!("placeholder");
}
