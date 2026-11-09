//! Shared fixtures: seeded work-units service, in-process WebSocket
//! server, and source-reading helpers for source-shape tests.

use super::*;
use codelet_test_helpers::OrFail;

/// Default fixture body written into `<tempdir>/spec/work-units.json` —
/// two `WorkUnitInfo` records, mirrored from
/// `rust/rpc-embedded/tests/embedded_happy_path.rs` so cross-transport
/// assertions in RPC-008 remain comparable to RPC-005's baseline.
pub const SEED_WORK_UNITS_JSON: &str = r#"{"workUnits":{"AUTH-001":{"id":"AUTH-001","title":"User Login","type":"story","status":"done","description":"Sign in with email/password","estimate":5,"epic":"authentication"},"AUTH-002":{"id":"AUTH-002","title":"Password reset","type":"story","status":"implementing","estimate":3,"epic":"authentication"}}}"#;

/// Fixture (1) per Q-FIX-1.
///
/// Build a real tempdir-backed `WorkUnitsWatcher` hosting a real
/// `SharedFspecService`, seeded with the default fixture. The returned
/// [`TempDir`] MUST be kept alive for the lifetime of the test (dropping
/// it removes the underlying spec/work-units.json the watcher tracks).
pub fn temp_service() -> Result<(TempDir, Arc<SharedFspecService>), codelet_test_helpers::TestError>
{
    init_test_data_directory()?;
    let dir = tempfile::tempdir().or_fail("tempdir")?;
    fs::create_dir_all(dir.path().join("spec")).or_fail("mkdir spec/")?;
    fs::write(
        dir.path().join("spec").join("work-units.json"),
        SEED_WORK_UNITS_JSON,
    )
    .or_fail("write seed work-units.json")?;
    let watcher = Arc::new(WorkUnitsWatcher::new(dir.path()).or_fail("WorkUnitsWatcher::new")?);
    // A daemon always has a workspace directory; without one, workspace
    // reads such as the header checkpoint counts are refused.
    let service = Arc::new(SharedFspecService::new(watcher).with_cwd(dir.path().to_path_buf()));
    Ok((dir, service))
}

/// BUG-468: `temp_service` with the stub session manager attached, for
/// App-level tests whose bootstrap lists providers (RPC-451: a service with
/// no session manager refuses, and the App shows that error).
pub fn temp_service_with_stub(
) -> Result<(TempDir, Arc<SharedFspecService>), codelet_test_helpers::TestError> {
    let (dir, _) = temp_service()?;
    let watcher = Arc::new(WorkUnitsWatcher::new(dir.path()).or_fail("WorkUnitsWatcher::new")?);
    let service = Arc::new(
        SharedFspecService::with_session_manager(
            watcher,
            Arc::new(codelet_core::session_manager_handle::StubSessionManagerHandle::new()),
        )
        .with_cwd(dir.path().to_path_buf()),
    );
    Ok((dir, service))
}

/// Point the process-global data directory (session storage, workspace
/// registry) at one temp dir shared by every test in this binary, so a
/// bootstrapped App does not raise the "Data directory not initialized"
/// session-error dialog over the board. Never the real `~/.fspec`.
pub fn init_test_data_directory() -> Result<(), codelet_test_helpers::TestError> {
    static DATA_DIR: std::sync::OnceLock<TempDir> = std::sync::OnceLock::new();
    let dir = match DATA_DIR.get() {
        Some(dir) => dir,
        None => {
            let created = tempfile::tempdir().or_fail("data tempdir")?;
            // A parallel test may have set it first; its dir is the one used.
            if let Err(unused) = DATA_DIR.set(created) {
                drop(unused);
            }
            DATA_DIR.get().or_fail("data dir set above")?
        }
    };
    if codelet_common::get_data_dir().is_err() {
        codelet_test_helpers::isolated_user_directory()?;
        codelet_common::set_data_directory(dir.path().to_path_buf()).or_fail("set data dir")?;
    }
    Ok(())
}

/// Fixture (2) per Q-FIX-1.
///
/// Spawn a real `codelet_rpc_server::bind_and_serve` task bound to
/// `127.0.0.1:0` against the supplied shared service and return the
/// ephemeral [`SocketAddr`] plus the listener's [`JoinHandle`]. Callers
/// MUST keep the join handle alive (typically by binding to `_join`) for
/// the duration of the test — dropping it does not currently abort the
/// task because the join handle returned by `bind_and_serve` is not
/// `abort_on_drop`. Tests that need an explicit shutdown should call
/// `_join.abort()` at the end of the test body.
///
/// The discarded `ServerStats` middle field of the `bind_and_serve`
/// triple is reserved for behavioural assertions in future tests; this
/// fixture keeps the smoke tests focused on transport-agnostic parity.
pub async fn start_ws_server(
    service: Arc<SharedFspecService>,
) -> Result<(SocketAddr, JoinHandle<()>), codelet_test_helpers::TestError> {
    let (addr, _stats, join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .or_fail("bind_and_serve must succeed against 127.0.0.1:0")?;
    Ok((addr, join))
}

/// RPC-011 variant: also returns the `ServerStats` so the test can call
/// `request_shutdown(stats)` to simulate a daemon-side graceful drain
/// (the per-connection tasks send WS Close{going_away} which propagates
/// to the supervisor).
pub async fn start_ws_server_with_stats(
    service: Arc<SharedFspecService>,
) -> Result<
    (SocketAddr, codelet_rpc_server::ServerStats, JoinHandle<()>),
    codelet_test_helpers::TestError,
> {
    Ok(bind_and_serve("127.0.0.1:0", service)
        .await
        .or_fail("bind_and_serve must succeed against 127.0.0.1:0")?)
}

/// Build a `ws://127.0.0.1:<port>/` URL for a given socket address —
/// helper for WS-connect tests so each call site doesn't repeat the
/// scheme + path concatenation.
pub fn ws_url(addr: SocketAddr) -> Result<url::Url, codelet_test_helpers::TestError> {
    Ok(url::Url::parse(&format!("ws://{addr}/")).or_fail("ws://<addr>/ is always a valid URL")?)
}

/// Resolve a path relative to the codelet workspace root.
///
/// `CARGO_MANIFEST_DIR` resolves to `rust/fspec-tui/` so the workspace
/// root is exactly one level up. Mirrors
/// `rust/rpc-embedded/tests/source_helpers/mod.rs::workspace_root`
/// — duplicated locally because the existing helper isn't yet a
/// shared dev-dependency-friendly export.
pub fn workspace_root() -> Result<PathBuf, codelet_test_helpers::TestError> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Ok(crate_dir
        .parent()
        .or_fail("fspec-tui crate must have a parent (the codelet workspace root)")?
        .to_path_buf())
}

/// Read a file, failing with its path — for source-shape tests that bail
/// on the first failed read.
pub fn read_to_string_or_fail(path: &Path) -> Result<String, codelet_test_helpers::TestError> {
    fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e).into())
}

/// Recursively collect every `.rs` file beneath `root`. Mirrors
/// `rust/rpc-embedded/tests/source_helpers/mod.rs::collect_rs_files`.
pub fn collect_rs_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out
}

/// Strip both `//` line comments and `/* … */` block comments. Mirrors
/// `rust/rpc-embedded/tests/source_helpers/mod.rs::strip_rust_comments`
/// so the source-shape regressions stay byte-equivalent across crates.
pub fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();
        if b == b'/' && next == Some(b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if b == b'/' && next == Some(b'*') {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}
