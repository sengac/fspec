//! Shared helpers for the codelet-fspec integration tests.
//!
//! Each `tests/*.rs` integration-test binary pulls this in via
//! `mod common;`. Mirrors the codelet/rpc-server/tests/common helpers
//! pattern (see codelet/rpc-server/tests/common/mod.rs) so the
//! `spawn_fspec_daemon` helper here proves the port-line contract is
//! verbatim across both binaries (rule [14], scenario "A new
//! spawn_fspec_daemon helper proves the port-line contract is verbatim").
//!
//! These helpers do not encode any scenario steps themselves — they are
//! plumbing referenced by the @step blocks in the test files.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use tempfile::TempDir;

/// Path to the built `fspec` binary supplied by cargo at compile time.
pub fn fspec_bin() -> &'static str {
    env!("CARGO_BIN_EXE_fspec")
}

/// Project root containing `codelet/`. Walks up from the test binary's
/// `CARGO_MANIFEST_DIR` (which is `<root>/codelet/fspec`) twice.
pub fn project_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent() // codelet/
        .and_then(|p| p.parent()) // <root>/
        .map(PathBuf::from)
        .expect("project root walk-up")
}

/// `codelet/` directory (the cargo workspace root).
pub fn codelet_root() -> PathBuf {
    project_root().join("codelet")
}

/// `codelet/fspec/` directory.
pub fn fspec_crate_root() -> PathBuf {
    codelet_root().join("fspec")
}

/// Build a temp workspace whose `spec/work-units.json` carries the
/// supplied work units. Returns the TempDir (RAII cleanup) and the path
/// to `spec/work-units.json`.
pub fn make_workspace(units: &[(&str, &str, &str)]) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let spec_dir = dir.path().join("spec");
    fs::create_dir_all(&spec_dir).expect("mkdir spec");
    let path = spec_dir.join("work-units.json");
    let mut entries = String::new();
    for (i, (id, title, status)) in units.iter().enumerate() {
        if i > 0 {
            entries.push(',');
        }
        entries.push_str(&format!(
            r#""{id}":{{"id":"{id}","title":"{title}","type":"story","status":"{status}"}}"#
        ));
    }
    let json = format!(r#"{{"workUnits":{{{entries}}}}}"#);
    fs::write(&path, json).expect("write work-units.json");
    (dir, path)
}

/// RAII guard so panicking tests don't orphan a child bound to a port.
pub struct ChildGuard(pub Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Spawn `fspec daemon --workspace <ws>` and read the bare-integer port
/// line off STDOUT. Mirrors the
/// `codelet/rpc-server/tests/common/mod.rs::spawn_rpc_server_with_workspace`
/// pattern verbatim (rule [14] / scenario "A new spawn_fspec_daemon
/// helper proves the port-line contract is verbatim").
pub fn spawn_fspec_daemon(workspace: &Path) -> (ChildGuard, u16) {
    let mut child = Command::new(fspec_bin())
        .arg("daemon")
        .arg("--workspace")
        .arg(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec daemon");
    let stdout = child.stdout.take().expect("child stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("read port line from fspec daemon stdout");
    let port: u16 = line
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("fspec daemon stdout was not a port: {line:?} ({e})"));
    (ChildGuard(child), port)
}

/// Spawn `fspec --workspace <ws>` (combined mode) and read the
/// `PORT=<n>` banner off STDERR (per rule [4] combined emits the port
/// on stderr, not stdout, because stdout is the alt-screen TUI canvas).
pub fn spawn_fspec_combined(workspace: &Path) -> (ChildGuard, u16) {
    let mut child = Command::new(fspec_bin())
        .arg("--workspace")
        .arg(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec (combined)");
    let stderr = child.stderr.take().expect("child stderr");
    let mut reader = BufReader::new(stderr);
    let port = scan_for_port_equals(&mut reader);
    (ChildGuard(child), port)
}

/// Read lines off a BufReader until a `PORT=<n>` line appears; return n.
/// Times out after 5 seconds (panics).
pub fn scan_for_port_equals<R: BufRead>(reader: &mut R) -> u16 {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut line = String::new();
    loop {
        if Instant::now() > deadline {
            panic!("timed out waiting for `PORT=<n>` line on stderr");
        }
        line.clear();
        let n = reader.read_line(&mut line).expect("read_line stderr");
        if n == 0 {
            panic!("stderr closed before `PORT=<n>` line");
        }
        if let Some(rest) = line.trim().strip_prefix("PORT=") {
            return rest
                .parse()
                .unwrap_or_else(|e| panic!("PORT= line not a u16: {line:?} ({e})"));
        }
    }
}

/// Drain whatever is currently buffered on a stream and return it as a
/// `Vec<u8>`. Used by tests that need to assert "stdout contains no
/// occurrence of PORT=" — they sleep briefly, then snapshot the buffer.
pub fn drain_now<R: Read>(reader: &mut R, max_bytes: usize) -> Vec<u8> {
    let mut buf = vec![0u8; max_bytes];
    let n = reader.read(&mut buf).unwrap_or(0);
    buf.truncate(n);
    buf
}

/// Strip `//` line comments and `/* … */` block comments from a Rust
/// source body so source-shape assertions (e.g. "no file contains
/// `tokio::runtime::Builder`") aren't fooled by mentions inside
/// rustdoc / explanatory comments. Used by the source-shape regression
/// scenarios in cargo_shape.rs / combined_smoke.rs / daemon_mode.rs /
/// client_mode.rs (rule [16] — single source of truth for the helper).
pub fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    chars.next();
                    for ch in chars.by_ref() {
                        if ch == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    let mut prev = ' ';
                    for ch in chars.by_ref() {
                        if prev == '*' && ch == '/' {
                            break;
                        }
                        prev = ch;
                    }
                    continue;
                }
                _ => {}
            }
        }
        out.push(c);
    }
    out
}
