# UPD-002: In-Place Self-Update (`/update`) — Implementation Guide

> **Status**: Research complete (2026-08-20). This document is the
> authoritative implementation plan for UPD-002. Depends on UPD-001
> (release assets must exist with the exact names in §2).

## 1. Goal

Add an in-place self-update to fspec:

1. **TUI**: typing `/update` in the terminal checks the latest GitHub
   release, downloads the binary for the current platform, verifies it, and
   replaces the running binary. On the next `fspec` launch the new version
   is live.
2. **CLI**: `fspec update` subcommand (same engine, headless output) so the
   update path works without the TUI.
3. **`fspec version`** (or the existing version surface) reports the
   installed version so users can verify.

## 2. Research Findings (WebSearch, 2026-08-20)

### 2.1 Crate choice: `self_update` (jaemk)

- **~638k downloads/month, 240 dependent crates, MIT** — the de-facto
  standard for Rust CLI self-updates.
- **GitHub backend is a default feature** (`github`), exactly our backend.
- **In-place replacement** is handled internally via the `self_replace`
  crate: on Unix it stages the new binary and swaps it over the running
  inode (safe while the process is executing); on Windows it uses the
  documented "write to temp + rename after exit / move-file" hacks that
  `self_replace` exists for. **No manual `exec` shenanigans needed.**
- **Checksum verification**: with the `checksums` feature, the downloaded
  asset is verified against the SHA-256 digest GitHub publishes per release
  asset (from the API response) — free integrity checking, no separate
  `SHA256SUMS` file required.
- **HTTP/TLS**: default `reqwest` + `rustls` (pure-Rust TLS — no OpenSSL
  system dependency, which matters for cross-platform release builds).
- **Async**: `async` feature adds `*_async` methods (tokio-only, requires
  reqwest) — use these since the TUI/CLI are tokio apps.
- **Unattended mode is NOT the default**: `update()` prints a status block
  and **blocks on an interactive yes/no prompt**. In the TUI there is no
  stdin available (the terminal is in raw mode), so we MUST set
  `.no_confirm(true)` and `.show_output(false)` (we render our own
  feedback into the TUI scrollback / CLI stdout).
- **Current version**: `.current_version(cargo_crate_version!())` — the
  crate compares against the running binary's version and reports
  `UpdateStatus::UpToDate` when nothing newer exists.

### 2.2 Required crate features

```toml
# rust/Cargo.toml [workspace.dependencies]
self_update = { version = "0.44", default-features = false,
                features = ["github", "reqwest", "rustls",
                            "archive-tar", "archive-zip",
                            "compression-tar-gz", "compression-zip-deflate",
                            "checksums", "async"] }
```

Feature rationale:
- `github` — our backend (default, but explicit since we disabled defaults)
- `reqwest` + `rustls` — HTTP + pure-Rust TLS
- `archive-tar` + `compression-tar-gz` — unix artifacts
- `archive-zip` + `compression-zip-deflate` — Windows artifacts
- `checksums` — verify against GitHub's published per-asset SHA-256
- `async` — `update_async()` for the tokio runtime

### 2.3 Asset matching contract (depends on UPD-001)

`self_update`'s GitHub backend picks the release asset whose name matches
`bin_name` + platform/target suffix. With `.bin_name("fspec")` the crate
looks for assets like `fspec-<target>.<ext>` in the latest release. This is
**exactly** the naming convention UPD-001's workflow produces:

```
fspec-x86_64-pc-windows-msvc.zip
fspec-aarch64-pc-windows-msvc.zip
fspec-x86_64-unknown-linux-gnu.tar.gz
fspec-aarch64-unknown-linux-gnu.tar.gz
fspec-aarch64-apple-darwin.tar.gz
```

**Verification step (do this first in the testing phase)**: run
`curl -s https://api.github.com/repos/sengac/fspec/releases/latest |
jq '.assets[].name'` and confirm the names match. If the crate's suffix
matching has drifted (it is version-sensitive), fall back to the
**manual path** in §5.3 — the design is resilient to this.

## 3. Architecture

```
fspec binary (codelet-fspec)
├── src/update.rs            ← NEW: shared update engine (CLI + TUI)
│     pub async fn check_latest() -> Result<ReleaseInfo>
│     pub async fn perform_update() -> Result<UpdateOutcome>
├── src/update_cmd.rs        ← NEW: `fspec update` clap subcommand
└── (TUI) codelet-fspec-tui
    ├── src/app/update_parser.rs   ← NEW: `/update` slash parse
    ├── src/app/slash_parser.rs    ← EXTEND: route /update
    └── src/app/dispatch_slash_update.rs ← NEW: handler (progress → exec)
```

**Shared engine** lives in `codelet-fspec` (the binary crate) — but the TUI
runs inside the same binary (combined mode), so the TUI can call it through
an RPC or a direct function. **Preferred**: expose the engine as a small
`codelet-fspec-core::update` module (pure logic, no clap) so BOTH the
binary's `update_cmd.rs` and the TUI's dispatch handler call the same
functions. Keep each file < 300 lines (workspace standard).

### 3.1 Engine API sketch

```rust
// codelet-fspec-core/src/update/mod.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UpdateError {
    #[error("no network / GitHub API unreachable: {0}")]
    Network(String),
    #[error("no release asset found for target {0}")]
    NoAssetForTarget(String),
    #[error("checksum mismatch for asset {0}")]
    ChecksumMismatch(String),
    #[error("failed to replace binary: {0}")]
    ReplaceFailed(String),
}

#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub tag: String,        // e.g. "v0.10.0"
    pub version: String,    // e.g. "0.10.0" (tag with leading v stripped)
    pub is_newer: bool,
    pub asset_name: Option<String>,
}

/// Detect the current target triple at runtime (cfg!-based).
pub fn current_target() -> &'static str {
    if cfg!(target_os = "windows") {
        if cfg!(target_arch = "aarch64") { "aarch64-pc-windows-msvc" }
        else { "x86_64-pc-windows-msvc" }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") { "aarch64-apple-darwin" }
        else { "x86_64-apple-darwin" }
    } else {
        if cfg!(target_arch = "aarch64") { "aarch64-unknown-linux-gnu" }
        else { "x86_64-unknown-linux-gnu" }
    }
}

pub async fn check_latest() -> Result<ReleaseInfo, UpdateError> { /* ... */ }
pub async fn perform_update(progress: Option<impl Fn(&str)>) -> Result<UpdateOutcome, UpdateError> { /* ... */ }
```

Error handling follows workspace standards: `thiserror` derive, no
`unwrap()`, `tracing` for logs, `?` propagation.

### 3.2 `self_update` call sketch (happy path)

```rust
use self_update::backends::github::Update;

let status = Update::configure()
    .repo_owner("sengac")
    .repo_name("fspec")
    .bin_name("fspec")
    .current_version(env!("CARGO_PKG_VERSION"))
    .no_confirm(true)          // TUI/CLI have no interactive stdin
    .show_output(false)        // we render our own feedback
    .show_download_progress(false)
    .build()?
    .update()                  // or .update_async().await with `async` feature
    .map_err(UpdateError::from)?;

match status {
    self_update::Status::UpToDate => /* report "already latest" */,
    self_update::Status::Updated(v) => /* report "updated to {v}" */,
}
```

**Windows note**: on Windows `self_replace` cannot overwrite a running
`.exe` in place; the crate's standard pattern is to install the new binary
and the update takes effect on the **next launch** (the running process
keeps the old inode). This matches the user's requirement ("install it via
update in place when the app restarts"). Communicate this in the UX copy:
"Updated to v0.10.0 — restart fspec to use it."

**Unix note**: on Unix the replacement is live for the next launch too
(current process keeps running on the old inode). Same UX copy.

### 3.3 Restart / exec

The user asked for "update in place when the app restarts via exec or
something". **Do NOT call `exec`/`std::process::exit` + re-exec from inside
the TUI** — it would kill active sessions mid-stream and the TUI cannot
meaningfully resume. The correct semantics (matching rustup, self_update
ecosystem convention):

1. `/update` performs the binary swap (staged by `self_replace`).
2. TUI prints: `✓ Updated fspec to v0.10.0. Restart fspec to activate.`
3. User quits (`/quit` or Ctrl+C) and relaunches → new binary.

**Optional stretch** (record as a question, not in v1): a `--restart`
flag on `fspec update` that, after a successful swap, re-execs itself
(`std::process::Command::new(std::env::current_exe())...spawn()` + exit)
for the **CLI** (non-TUI) invocation only. TUI never auto-restarts.

## 4. TUI Wiring (`/update`)

Follow the exact pattern of `/continue` (CONT-002) — the newest, cleanest
slash-command precedent in the codebase:

1. **Parser** — `rust/fspec-tui/src/app/update_parser.rs`:
   - `/update` (bare) → `UpdateSubcommand::CheckAndUpdate`
   - `/update check` → `UpdateSubcommand::CheckOnly` (report latest version
     without downloading)
   - `/update <other>` → `UpdateSubcommand::Invalid(arg)`
2. **Route** — extend `slash_parser.rs::parse_slash_command`:
   ```rust
   if trimmed == "/update" || trimmed.starts_with("/update ") {
       return SlashCommandParse::UpdateSubcommand(parse_update_command(trimmed));
   }
   ```
   Add the `UpdateSubcommand(UpdateSubcommand)` variant to the enum.
3. **Dispatch** — `rust/fspec-tui/src/app/dispatch_slash_update.rs`:
   - Emit a scrollback line: `[update] checking for latest release…`
   - Spawn the async update on the tokio runtime (the TUI is already
     tokio-driven; get the handle via `tokio::runtime::Handle::current()` —
     workspace rule, never build a new runtime).
   - On success: `[update] ✓ fspec v0.10.0 installed. Restart fspec to activate.`
   - On `UpToDate`: `[update] ✓ fspec is up to date (v0.10.0).`
   - On error: `[update] ✗ {err}` (map `UpdateError` to human strings).
   - **Do not block the UI thread**: the download runs as a task; the TUI
     keeps rendering. Report completion when the future resolves.
4. **Help** — add `/update` to the help dialog content
   (`components/help_content.rs`) and to `help_dialog_content` tests.

## 5. CLI Wiring (`fspec update`)

1. Add `mod update_cmd;` + `Update` subcommand to the clap enum in
   `rust/fspec/src/main.rs` (follow `board.rs`/`status.rs` shape).
2. `update_cmd.rs` calls the same `codelet-fspec-core::update` engine and
   prints human-readable output with `tracing`/`eprintln`-free stdout
   (CLI output goes to stdout; the CLI already uses direct prints for
   command output — match existing subcommand style).
3. Flags:
   - `fspec update` — check + install
   - `fspec update --check` — check only (exit 0 if current, exit 1 if
     newer available — scriptable)
   - `fspec update --version <semver>` — install a specific release (uses
     `get_release_version` / tag URL path)

## 5.3 Fallback: manual download path (if §2.3 verification fails)

If `self_update`'s asset matching does not line up with our asset names
after testing against the live release, implement the manual path (the
crate's own docs recommend this for unusual naming):

1. `GET https://api.github.com/repos/sengac/fspec/releases/latest`
   (reqwest, `User-Agent: fspec/<version>` — GitHub rejects empty UAs).
2. Pick the asset named `fspec-<current_target()>.<ext>` from
   `assets[].browser_download_url`.
3. Stream to a temp file in the same directory as
   `std::env::current_exe()` (rename across filesystems fails).
4. Verify SHA-256 against the API's `assets[].digest` field
   (`sha256:<hex>`).
5. On unix: `std::fs::rename(tmp, exe)` (atomic over running inode).
   On Windows: `self_replace::self_replace(tmp, exe)` (handles the
   locked-.exe case by scheduling the rename).
6. Remove temp file.

This path is ~150 lines and keeps the feature independent of `self_update`
version drift. **Decision rule**: use `self_update` if §2.3 verification
passes in the testing phase; otherwise use the manual path. Both satisfy
the acceptance criteria.

## 6. Testing Plan (ACDD: tests before implementation)

Feature file: `spec/features/in-place-self-update.feature` (UPD-002).
Test file: `rust/fspec-core/tests/` or inline `#[cfg(test)]` modules.

| Scenario | Test approach |
|----------|---------------|
| `/update` parses to UpdateSubcommand variants | Unit test in `update_parser.rs` (mirror `continue_parser.rs` tests) |
| `current_target()` returns correct triple per platform | Unit test with `cfg!` assertions (each platform asserts its own) |
| Check-only reports latest version | Integration: spin a **local mock GitHub API** (tiny axum server on 127.0.0.1:0 serving `/repos/sengac/fspec/releases/latest` JSON) — point the engine at it via a `base_url` override (the crate supports custom GitHub URLs; the manual path trivially does). **Redirect, don't intercept** (workspace test philosophy). |
| Check-only reports up-to-date | Same mock, latest == current version |
| Update downloads + verifies checksum | Mock server serves a tiny fake tar.gz/zip containing a stub `fspec` binary; assert the staged file replaced a temp-dir "installed" binary (engine takes an injectable install path for tests — `current_exe` override) |
| Checksum mismatch aborts | Mock serves asset with wrong digest; assert `UpdateError::ChecksumMismatch` and no replacement |
| No asset for target | Mock release missing the current target's asset; assert `NoAssetForTarget` |
| Network failure | Point at a closed port; assert `UpdateError::Network` |
| `fspec update --check` exit codes | `assert_cmd` against the built binary with `FSPEC_UPDATE_BASE_URL` env override (add env override for testability) |

Use `rust/test-helpers/` for temp dirs. NO `unwrap()` in production code;
test modules may use it (existing pattern: `#![allow(clippy::unwrap_used)]`
in test mods).

## 7. Definition of Done

- [ ] `codelet-fspec-core::update` engine with `check_latest` /
      `perform_update`, `thiserror` error type, no `unwrap`/`panic`
- [ ] `/update` + `/update check` work in the TUI with scrollback feedback
- [ ] `fspec update` / `--check` / `--version` CLI subcommands
- [ ] Checksum verification against GitHub's published digest is ON
- [ ] All §6 scenarios have passing tests
- [ ] End-to-end: on the dev machine, `fspec update` against the real
      `v0.10.0` release swaps the binary; relaunch shows the new version
- [ ] Help dialog lists `/update`
- [ ] `fspec validate` + `fspec validate-tags` pass on the new feature file

## 8. Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| `self_update` asset-name matching drifts between versions | Pin the crate version; §2.3 verification gate; §5.3 manual-path fallback |
| Windows: running .exe locked | `self_replace` handles it (rename-after-exit); UX copy says "restart to activate" |
| TUI has no stdin for confirmation | `.no_confirm(true)` + `.show_output(false)` (documented requirement) |
| Unsigned binaries (Gatekeeper/SmartScreen) | Documented in UPD-001 §5; out of scope for v0.10.0 |
| Download interrupted mid-write | Write to temp file, verify checksum, THEN rename — never partial-replace |
| `reqwest`+`rustls` bloats release binary | Acceptable (a few MB on `release-slim`); alternative is `ureq`+`rustls` (smaller, sync) if size matters — decide in implementing phase |
