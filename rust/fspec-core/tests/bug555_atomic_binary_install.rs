//! BUG-555 — installing fspec while fspec is running.
//!
//! On Linux, `cp` onto a running executable fails with "Text file busy".
//! scripts/build-install.sh and scripts/install.sh copy the binary to a
//! temporary file in the install directory and rename it over the target;
//! the update engine renames its extracted binary over the install path.
//! Each test runs a real process from the installed file (a copy of bash in
//! a loop) and installs over it.
//!
//! Feature: spec/features/atomic-binary-install.feature

#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::Arc;

use codelet_test_helpers::deadlock_guard_blocking;
use std::io::{BufRead, BufReader};

use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};

use codelet_fspec_core::update::{current_target, UpdateConfig, UpdateOutcome};

const RESTART_NOTE: &str = "running fspec processes keep the old version until restarted";
const NEW_BUILD: &str = "#!/bin/sh\necho \"fspec 9.9.9 (new build)\"\n";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize repo root")
}

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).expect("write executable");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod executable");
}

/// A running process whose executable is `path`: a copy of bash looping on
/// `sleep` (the loop keeps bash from exec'ing into sleep). Killed on drop.
struct Running(Child);

impl Running {
    fn start(path: &Path) -> Self {
        let bash = Command::new("sh")
            .args(["-c", "command -v bash"])
            .output()
            .expect("locate bash");
        let bash = PathBuf::from(String::from_utf8_lossy(&bash.stdout).trim());
        fs::copy(fs::canonicalize(&bash).expect("resolve bash"), path).expect("copy bash");
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod copy");
        // TEST-042: no polling. Another test thread forking while it holds a
        // write handle can make exec fail with ETXTBSY for a moment, so the
        // start is retried (under the deadlock guard); the copy prints a line
        // once it runs, and that line is what the test waits on.
        let program = path.to_path_buf();
        let mut child = deadlock_guard_blocking("starting the copied bash", move || loop {
            match Command::new(&program)
                .args(["-c", "echo started; while :; do sleep 1; done"])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(child) => break Ok(child),
                Err(e) if e.raw_os_error() == Some(26) => std::thread::yield_now(),
                Err(e) => break Err(e),
            }
        })
        .expect("the copied bash starts")
        .unwrap_or_else(|e| panic!("start {}: {e}", path.display()));
        let stdout = child.stdout.take().expect("piped stdout");
        let mut line = String::new();
        BufReader::new(stdout)
            .read_line(&mut line)
            .expect("read the copied bash's start line");
        assert_eq!(
            line.trim(),
            "started",
            "the copied bash runs {}",
            path.display()
        );
        Running(child)
    }

    fn is_running(&mut self) -> bool {
        self.0.try_wait().expect("poll running process").is_none()
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn combined(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// A scratch checkout holding scripts/build-install.sh and an empty rust/,
/// plus a `cargo` stub that "builds" NEW_BUILD into rust/target/<profile>/fspec.
fn build_install_sandbox(tmp: &Path) -> (PathBuf, PathBuf) {
    let repo = tmp.join("repo");
    fs::create_dir_all(repo.join("scripts")).expect("mkdir scripts");
    fs::create_dir_all(repo.join("rust")).expect("mkdir rust");
    fs::copy(
        repo_root().join("scripts/build-install.sh"),
        repo.join("scripts/build-install.sh"),
    )
    .expect("copy build-install.sh");
    let stubs = tmp.join("stubs");
    fs::create_dir_all(&stubs).expect("mkdir stubs");
    let cargo = format!(
        "#!/bin/sh\nprofile=\"\"; prev=\"\"\nfor a in \"$@\"; do\n  if [ \"$prev\" = \"--profile\" ]; then profile=\"$a\"; fi\n  prev=\"$a\"\ndone\nmkdir -p \"target/$profile\"\ncat > \"target/$profile/fspec\" <<'BIN'\n{NEW_BUILD}BIN\nchmod +x \"target/$profile/fspec\"\n"
    );
    write_executable(&stubs.join("cargo"), &cargo);
    (repo.join("scripts/build-install.sh"), stubs)
}

fn run_build_install(script: &Path, stubs: &Path, install_dir: &Path) -> Output {
    Command::new("bash")
        .arg(script)
        .env(
            "PATH",
            format!(
                "{}:{}",
                stubs.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("INSTALL_DIR", install_dir)
        .output()
        .expect("run build-install.sh")
}

#[test]
fn build_install_sh_installs_over_a_running_fspec() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (script, stubs) = build_install_sandbox(tmp.path());
    let install_dir = tmp.path().join("bin");
    fs::create_dir_all(&install_dir).expect("mkdir install dir");
    let installed = install_dir.join("fspec");

    // @step Given fspec is running from the install directory
    let mut running = Running::start(&installed);

    // @step When the developer runs scripts/build-install.sh
    let out = run_build_install(&script, &stubs, &install_dir);
    let text = combined(&out);

    // @step Then the script exits 0 and the install directory holds the new build
    assert!(out.status.success(), "build-install.sh failed:\n{text}");
    assert_eq!(
        fs::read_to_string(&installed).expect("read installed"),
        NEW_BUILD
    );

    // @step And the running fspec process keeps running
    assert!(running.is_running(), "the running fspec was stopped");

    // @step And the script says running fspec processes keep the old version until restarted
    assert!(
        text.to_lowercase().contains(RESTART_NOTE),
        "missing restart note:\n{text}"
    );
}

/// curl stub serving the GitHub API, checksums.txt and the release archive.
const CURL_STUB: &str = r##"#!/bin/sh
out=""; prev=""
for a in "$@"; do
  if [ "$prev" = "-o" ]; then out="$a"; fi
  prev="$a"
done
emit() { if [ -n "$out" ]; then printf '%s\n' "$1" > "$out"; else printf '%s\n' "$1"; fi; }
for a in "$@"; do
  case "$a" in
    *api.github.com*) emit '[{"tag_name":"v9.9.9"}]'; exit 0 ;;
    *checksums.txt*) emit "$(sha256sum "$FAKE_ARCHIVE" | awk '{print $1}')  $ASSET_NAME"; exit 0 ;;
    *releases/download*)
      if [ -n "$out" ]; then cp "$FAKE_ARCHIVE" "$out"; else cat "$FAKE_ARCHIVE"; fi
      exit 0 ;;
  esac
done
exit 1
"##;

#[test]
fn install_sh_installs_a_release_over_a_running_fspec() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let stubs = tmp.path().join("stubs");
    let release = tmp.path().join("release");
    fs::create_dir_all(&stubs).expect("mkdir stubs");
    fs::create_dir_all(&release).expect("mkdir release");
    write_executable(
        &stubs.join("uname"),
        "#!/bin/sh\ncase \"$1\" in\n  -s) echo Linux ;;\n  -m) echo x86_64 ;;\nesac\n",
    );
    write_executable(&stubs.join("curl"), CURL_STUB);
    write_executable(&release.join("fspec"), NEW_BUILD);
    let asset = "fspec-x86_64-unknown-linux-gnu.tar.gz";
    let archive = tmp.path().join(asset);
    let tar = Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("fspec")
        .current_dir(&release)
        .output()
        .expect("tar");
    assert!(tar.status.success(), "tar failed: {tar:?}");
    let install_dir = tmp.path().join("bin");
    fs::create_dir_all(&install_dir).expect("mkdir install dir");
    let installed = install_dir.join("fspec");

    // @step Given fspec is running from the install directory
    let mut running = Running::start(&installed);

    // @step When the user runs scripts/install.sh
    let out = Command::new("bash")
        .arg(repo_root().join("scripts/install.sh"))
        .env(
            "PATH",
            format!(
                "{}:{}",
                stubs.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("INSTALL_DIR", &install_dir)
        .env("FAKE_ARCHIVE", &archive)
        .env("ASSET_NAME", asset)
        .output()
        .expect("run install.sh");
    let text = combined(&out);

    // @step Then the script exits 0 and the install directory holds the release binary
    assert!(out.status.success(), "install.sh failed:\n{text}");
    assert_eq!(
        fs::read_to_string(&installed).expect("read installed"),
        NEW_BUILD
    );
    assert!(running.is_running(), "the running fspec was stopped");

    // @step And the script says running fspec processes keep the old version until restarted
    assert!(
        text.to_lowercase().contains(RESTART_NOTE),
        "missing restart note:\n{text}"
    );
}

struct Release {
    asset_name: String,
    asset: Vec<u8>,
    digest: String,
}

async fn latest(
    axum::extract::State(r): axum::extract::State<Arc<Release>>,
) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "tag_name": "v9.9.9",
        "assets": [{ "name": r.asset_name, "digest": format!("sha256:{}", r.digest),
                     "browser_download_url": format!("/assets/{}", r.asset_name) }]
    }))
}

async fn asset(axum::extract::State(r): axum::extract::State<Arc<Release>>) -> Vec<u8> {
    r.asset.clone()
}

#[tokio::test]
async fn the_update_engine_replaces_a_running_fspec() {
    let new_binary = b"#!/bin/sh\necho \"fspec 9.9.9 (release)\"\n";
    let mut targz = Vec::new();
    {
        let mut builder = tar::Builder::new(GzEncoder::new(&mut targz, Compression::fast()));
        let mut header = tar::Header::new_gnu();
        header.set_size(new_binary.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append_data(&mut header, "fspec", &new_binary[..])
            .expect("append fspec");
        builder.into_inner().expect("finish tar");
    }
    let release = Arc::new(Release {
        asset_name: format!("fspec-{}.tar.gz", current_target()),
        digest: hex::encode(Sha256::digest(&targz)),
        asset: targz,
    });
    let app = axum::Router::new()
        .route(
            "/repos/sengac/fspec/releases/latest",
            axum::routing::get(latest),
        )
        .route("/assets/{name}", axum::routing::get(asset))
        .with_state(release);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock");
    let base = format!("http://{}", listener.local_addr().expect("mock addr"));
    tokio::spawn(async move { axum::serve(listener, app).await });
    let tmp = tempfile::tempdir().expect("tempdir");
    let install = tmp.path().join("fspec");

    // @step Given the binary at the install path is running
    let mut running = Running::start(&install);

    // @step When the update engine replaces the install path with a new binary
    let outcome = UpdateConfig {
        base_url: base,
        repo_owner: "sengac".to_string(),
        repo_name: "fspec".to_string(),
        bin_name: "fspec".to_string(),
        current_version: "0.1.0".to_string(),
        target: None,
        install_path: install.clone(),
    }
    .perform_update()
    .await;

    // @step Then the replace succeeds and the install path holds the new binary
    assert!(
        matches!(outcome, Ok(UpdateOutcome::Updated { ref version, restart_required: true }) if version == "9.9.9"),
        "expected Updated(9.9.9) with restart_required, got {outcome:?}"
    );
    assert_eq!(fs::read(&install).expect("read install"), new_binary);
    assert!(running.is_running(), "the running fspec was stopped");
}

#[test]
fn a_failed_install_leaves_the_existing_fspec_untouched() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (script, stubs) = build_install_sandbox(tmp.path());
    let install_dir = tmp.path().join("bin");
    fs::create_dir_all(&install_dir).expect("mkdir install dir");
    let installed = install_dir.join("fspec");
    let old = "#!/bin/sh\necho \"fspec 1.0.0 (old)\"\n";

    // @step Given the install directory holds an fspec binary and is read-only
    write_executable(&installed, old);
    fs::set_permissions(&install_dir, fs::Permissions::from_mode(0o555)).expect("chmod dir");

    // @step When the developer runs scripts/build-install.sh
    let out = run_build_install(&script, &stubs, &install_dir);
    let entries: Vec<String> = fs::read_dir(&install_dir)
        .expect("list install dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    let content = fs::read_to_string(&installed).expect("read installed");
    fs::set_permissions(&install_dir, fs::Permissions::from_mode(0o755)).expect("restore dir");

    // @step Then the script exits non-zero
    assert!(
        !out.status.success(),
        "build-install.sh must fail on a read-only install dir:\n{}",
        combined(&out)
    );

    // @step And the existing fspec is unchanged and no temporary file is left in the install directory
    assert_eq!(content, old);
    assert_eq!(entries, vec!["fspec".to_string()]);
}
