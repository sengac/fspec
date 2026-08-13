//! Shared helpers for the rpc-server integration tests.
//!
//! Cargo integration tests live in separate test binaries; this module is
//! pulled in via `mod common;` from each test file to avoid duplicating
//! WebSocket connect/retry boilerplate.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;

/// Connect a tokio-tungstenite WebSocket client to `127.0.0.1:<port>`,
/// retrying every 20ms until the listener accepts. Used by integration
/// tests that race against a server task that has just been spawned and
/// may not yet have moved past `TcpListener::bind` to `accept`.
pub async fn connect_with_retry(
    port: u16,
) -> WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>> {
    let url = Url::parse(&format!("ws://127.0.0.1:{port}")).unwrap();
    loop {
        match connect_async(url.as_str()).await {
            Ok((s, _)) => return s,
            Err(_) => tokio::time::sleep(Duration::from_millis(20)).await,
        }
    }
}

/// Build a temp workspace directory whose `spec/work-units.json` contains
/// the supplied work units in the schema understood by the watcher
/// (`workUnits` map keyed by id).
///
/// Returns the `TempDir` (kept alive by the caller for RAII cleanup) and
/// the absolute path to `spec/work-units.json` so the test can mutate it
/// to trigger watcher events.
pub fn make_workspace(units: &[(&str, &str, &str)]) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let spec_dir = dir.path().join("spec");
    fs::create_dir_all(&spec_dir).unwrap();
    let path = spec_dir.join("work-units.json");
    write_workspace(&path, units);
    (dir, path)
}

/// Overwrite the work-units.json file at `path` with the supplied units.
pub fn write_workspace(path: &Path, units: &[(&str, &str, &str)]) {
    let mut entries = String::new();
    for (i, (id, title, status)) in units.iter().enumerate() {
        if i > 0 {
            entries.push(',');
        }
        entries.push_str(&format!(
            r#""{id}":{{"id":"{id}","title":"{title}","type":"story","status":"{status}"}}"#,
        ));
    }
    let json = format!(r#"{{"workUnits":{{{entries}}}}}"#);
    fs::write(path, json).unwrap();
}

/// RAII child-process kill on drop so a panicking test never leaves an
/// orphan rpc-server bound to a random port.
pub struct ChildGuard(pub Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Spawn the `codelet-rpc-server` binary against the supplied workspace
/// directory and read its ephemeral port off stdout. The binary now
/// requires a `--workspace <path>` argument (RPC-006).
pub fn spawn_rpc_server_with_workspace(workspace: &Path) -> (ChildGuard, u16) {
    let bin = env!("CARGO_BIN_EXE_codelet-rpc-server");
    let mut child = Command::new(bin)
        .arg("--workspace")
        .arg(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn codelet-rpc-server binary");

    let stdout = child.stdout.take().expect("child stdout missing");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("failed to read port from rpc-server stdout");
    let port: u16 = line
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("rpc-server stdout was not a port: {line:?} ({e})"));
    (ChildGuard(child), port)
}
