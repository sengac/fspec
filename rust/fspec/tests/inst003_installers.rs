//! UPD-003 — GitHub-release binary installer (install.sh) + source build installer (build-install.sh).
//!
//! Feature: spec/features/github-release-binary-installer-install-sh-source-build-installer-build-install-sh.feature
//!
//! These tests shell out to the installer scripts with stubbed `uname`/`curl`
//! (PATH-manipulated fake binaries) so the platform-detection, release
//! resolution, checksum, and install flows can be exercised without network
//! access or a Rust toolchain.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Repo root (rust/fspec/../.. → repo root).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("canonicalize repo root")
}

fn scripts_dir() -> PathBuf {
    repo_root().join("scripts")
}

/// Build a fake release archive (tar.gz containing a fake `fspec` binary
/// that prints `fspec 9.9.9 (fake)`) and return its path.
fn make_fake_archive(tmp: &Path, name: &str) -> PathBuf {
    let fake_bin = tmp.join("fake-fsbuild-dir");
    fs::create_dir_all(&fake_bin).expect("mkdir fake bin dir");
    let fake_bin_path = fake_bin.join("fspec");
    fs::write(&fake_bin_path, "#!/bin/sh\necho \"fspec 9.9.9 (fake)\"\n").expect("write fake fspec");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&fake_bin_path, fs::Permissions::from_mode(0o755)).expect("chmod fake");
    }
    let archive = tmp.join(name);
    let tar_out = Command::new("tar")
        .args(["-czf", archive.to_str().unwrap(), "fspec"])
        .current_dir(&fake_bin)
        .output()
        .expect("tar");
    assert!(tar_out.status.success(), "tar failed: {:?}", tar_out);
    archive
}

/// curl stub body. `$1` selects the checksums.txt behavior:
///   ok      → serve a correct checksum for $FAKE_ARCHIVE
///   missing → exit 22 (simulated 404, like real `curl -f`)
///   wrong   → serve an all-zero hash
const CURL_STUB_TEMPLATE: &str = r##"#!/bin/sh
out=""
prev=""
for a in "$@"; do
  if [ "$prev" = "-o" ]; then out="$a"; fi
  prev="$a"
done
for a in "$@"; do
  case "$a" in
    *api.github.com*)
      if [ -n "$out" ]; then printf '[{"tag_name":"v9.9.9"}]' > "$out"; else printf '[{"tag_name":"v9.9.9"}]'; fi
      exit 0 ;;
    *checksums.txt*)
      case "$CHECKSUM_MODE" in
        ok)
          hash=$(sha256sum "$FAKE_ARCHIVE" | awk '{print $1}')
          line="$hash  $ASSET_NAME"
          if [ -n "$out" ]; then printf '%s\n' "$line" > "$out"; else printf '%s\n' "$line"; fi
          exit 0 ;;
        missing)
          exit 22 ;;
        wrong)
          line="0000000000000000000000000000000000000000000000000000000000000000  $ASSET_NAME"
          if [ -n "$out" ]; then printf '%s\n' "$line" > "$out"; else printf '%s\n' "$line"; fi
          exit 0 ;;
      esac ;;
    *releases/download*)
      if [ -n "$out" ]; then cp "$FAKE_ARCHIVE" "$out"; else cp "$FAKE_ARCHIVE" /dev/stdout; fi
      exit 0 ;;
  esac
done
exit 1
"##;

/// Run install.sh with stubbed uname/curl. Returns (status, combined output).
fn run_install(
    uname_os: &str,
    uname_arch: &str,
    asset_name: &str,
    checksum_mode: &str,
    archive: &Path,
    install_dir: &Path,
    extra_args: &[&str],
) -> (bool, String) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let stub_dir = tmp.path().to_path_buf();

    let uname_stub = format!(
        "#!/bin/sh\ncase \"$1\" in\n  -s) echo {uname_os} ;;\n  -m) echo {uname_arch} ;;\nesac\n"
    );
    fs::write(stub_dir.join("uname"), uname_stub).expect("uname stub");
    fs::write(stub_dir.join("curl"), CURL_STUB_TEMPLATE).expect("curl stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for name in ["uname", "curl"] {
            let p = stub_dir.join(name);
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).expect("chmod stub");
        }
    }

    fs::create_dir_all(install_dir).expect("mkdir install dir");

    let mut cmd = Command::new("bash");
    cmd.arg(scripts_dir().join("install.sh"));
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.env(
        "PATH",
        format!("{}:{}", stub_dir.display(), std::env::var("PATH").unwrap_or_default()),
    );
    cmd.env("INSTALL_DIR", install_dir.to_string_lossy().to_string());
    cmd.env("FAKE_ARCHIVE", archive.to_string_lossy().to_string());
    cmd.env("ASSET_NAME", asset_name);
    cmd.env("CHECKSUM_MODE", checksum_mode);
    let out = cmd.output().expect("run install.sh");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), combined)
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: One-line curl install on Apple Silicon Mac
// ─────────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/github-release-binary-installer-install-sh-source-build-installer-build-install-sh.feature
#[test]
fn scenario_one_line_curl_install_on_apple_silicon_mac() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let install_dir = tmp.path().join("local-bin");
    let archive = make_fake_archive(tmp.path(), "fspec-aarch64-apple-darwin.tar.gz");

    // @step Given a user on an aarch64 macOS host
    let uname_os = "Darwin";
    let uname_arch = "arm64";

    // @step When they run curl -fsSL https://raw.githubusercontent.com/sengac/fspec/main/scripts/install.sh | bash
    let (ok, combined) = run_install(
        uname_os,
        uname_arch,
        "fspec-aarch64-apple-darwin.tar.gz",
        "ok",
        &archive,
        &install_dir,
        &[],
    );
    assert!(ok, "install.sh should succeed. output: {combined}");

    // @step Then the script detects the platform aarch64-apple-darwin
    assert!(
        combined.contains("aarch64-apple-darwin"),
        "should report the detected platform. output: {combined}"
    );

    // @step And it downloads fspec-aarch64-apple-darwin.tar.gz from the latest release with that asset
    assert!(
        combined.contains("fspec-aarch64-apple-darwin.tar.gz"),
        "should reference the downloaded asset. output: {combined}"
    );

    // @step And it verifies the archive against the release checksums.txt
    assert!(
        !combined.to_lowercase().contains("skipped"),
        "checksum verification must not be skipped. output: {combined}"
    );

    // @step And it installs the fspec binary to ~/.local/bin/fspec
    let installed = install_dir.join("fspec");
    assert!(installed.is_file(), "fspec binary should be installed at {installed:?}");

    // @step And it prints the installed version from fspec --version
    assert!(
        combined.contains("9.9.9"),
        "installer should print the version. output: {combined}"
    );

    // @step And no Rust toolchain, cargo, or git is required
    let content = fs::read_to_string(scripts_dir().join("install.sh")).expect("read install.sh");
    assert!(
        !content.contains("cargo build") && !content.contains("git clone"),
        "install.sh must not build from source or clone the repo"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Install on x86_64 Linux
// ─────────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/github-release-binary-installer-install-sh-source-build-installer-build-install-sh.feature
#[test]
fn scenario_install_on_x86_64_linux() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let install_dir = tmp.path().join("local-bin");
    let archive = make_fake_archive(tmp.path(), "fspec-x86_64-unknown-linux-gnu.tar.gz");

    // @step Given a user on an x86_64 Linux host
    let uname_os = "Linux";
    let uname_arch = "x86_64";

    // @step When they run scripts/install.sh
    let (ok, combined) = run_install(
        uname_os,
        uname_arch,
        "fspec-x86_64-unknown-linux-gnu.tar.gz",
        "ok",
        &archive,
        &install_dir,
        &[],
    );
    assert!(ok, "install.sh should succeed. output: {combined}");

    // @step Then the script detects the platform x86_64-unknown-linux-gnu
    assert!(
        combined.contains("x86_64-unknown-linux-gnu"),
        "should report the detected platform. output: {combined}"
    );

    // @step And it downloads fspec-x86_64-unknown-linux-gnu.tar.gz from the latest release
    assert!(
        combined.contains("fspec-x86_64-unknown-linux-gnu.tar.gz"),
        "should reference the downloaded asset. output: {combined}"
    );

    // @step And the installed binary runs without glibc version errors
    let installed = install_dir.join("fspec");
    assert!(installed.is_file(), "fspec binary should be installed");
    let run = Command::new(&installed).arg("--version").output().expect("run fake fspec");
    let ver = String::from_utf8_lossy(&run.stdout);
    assert!(
        run.status.success() && ver.contains("9.9.9"),
        "installed binary should run. output: {ver}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: From-source build via build-install.sh
// ─────────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/github-release-binary-installer-install-sh-source-build-installer-build-install-sh.feature
#[test]
fn scenario_from_source_build_via_build_install_sh() {
    // @step Given a developer with cargo and git installed
    // (static check — the full build is slow and toolchain-dependent)
    assert!(
        Command::new("cargo").arg("--version").output().map(|o| o.status.success()).unwrap_or(false),
        "cargo should be available in this environment"
    );
    assert!(
        Command::new("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false),
        "git should be available in this environment"
    );

    // @step When they run scripts/build-install.sh
    let path = scripts_dir().join("build-install.sh");
    assert!(path.exists(), "build-install.sh must exist");
    let content = fs::read_to_string(&path).expect("read build-install.sh");

    // @step Then it checks the cargo and git prerequisites
    assert!(content.contains("cargo"), "must check for cargo");
    assert!(content.contains("git"), "must check for git");

    // @step And it builds codelet-fspec with the release-slim profile
    assert!(
        content.contains("cargo build") && content.contains("release-slim"),
        "must run cargo build with the release-slim profile"
    );
    assert!(content.contains("codelet-fspec"), "must build the codelet-fspec package");

    // @step And it installs the built binary to ~/.local/bin/fspec
    assert!(
        content.contains("INSTALL_DIR") && content.contains(".local/bin"),
        "must install to INSTALL_DIR (default ~/.local/bin)"
    );
    assert!(
        content.contains("--dir") && content.contains("--profile"),
        "must support --dir and --profile options"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Unsupported architecture fails fast
// ─────────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/github-release-binary-installer-install-sh-source-build-installer-build-install-sh.feature
#[test]
fn scenario_unsupported_architecture_fails_fast() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let install_dir = tmp.path().join("local-bin");
    let archive = make_fake_archive(tmp.path(), "fspec-x86_64-unknown-linux-gnu.tar.gz");

    // @step Given a user on an unsupported Linux architecture
    let uname_os = "Linux";
    let uname_arch = "i686";

    // @step When they run scripts/install.sh
    let (ok, combined) = run_install(
        uname_os,
        uname_arch,
        "fspec-x86_64-unknown-linux-gnu.tar.gz",
        "ok",
        &archive,
        &install_dir,
        &[],
    );

    // @step Then it exits with a non-zero status
    assert!(!ok, "install.sh must fail on unsupported arch. output: {combined}");

    // @step And it prints an error naming the unsupported architecture
    assert!(
        combined.contains("i686") || combined.to_lowercase().contains("unsupported"),
        "error should name the unsupported architecture. output: {combined}"
    );

    // @step And it downloads nothing
    assert!(
        !install_dir.join("fspec").exists(),
        "nothing should be installed"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Missing checksums.txt warns but succeeds
// ─────────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/github-release-binary-installer-install-sh-source-build-installer-build-install-sh.feature
#[test]
fn scenario_missing_checksums_txt_warns_but_succeeds() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let install_dir = tmp.path().join("local-bin");
    let archive = make_fake_archive(tmp.path(), "fspec-x86_64-unknown-linux-gnu.tar.gz");

    // @step Given a release that has no checksums.txt asset
    let checksum_mode = "missing";

    // @step When the installer downloads and installs the archive
    let (ok, combined) = run_install(
        "Linux",
        "x86_64",
        "fspec-x86_64-unknown-linux-gnu.tar.gz",
        checksum_mode,
        &archive,
        &install_dir,
        &[],
    );

    // @step Then it prints a warning that checksum verification was skipped
    assert!(
        combined.to_lowercase().contains("skipped")
            || combined.to_lowercase().contains("not found")
            || combined.to_lowercase().contains("no checksum"),
        "should warn that checksum verification was skipped. output: {combined}"
    );

    // @step And the installation completes successfully
    assert!(ok, "install.sh should succeed. output: {combined}");
    assert!(
        install_dir.join("fspec").is_file(),
        "fspec should be installed"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Checksum mismatch aborts installation
// ─────────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/github-release-binary-installer-install-sh-source-build-installer-build-install-sh.feature
#[test]
fn scenario_checksum_mismatch_aborts_installation() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let install_dir = tmp.path().join("local-bin");
    let archive = make_fake_archive(tmp.path(), "fspec-x86_64-unknown-linux-gnu.tar.gz");

    // @step Given a release whose checksums.txt does not match the downloaded archive
    let checksum_mode = "wrong";

    // @step When the installer verifies the archive
    let (ok, combined) = run_install(
        "Linux",
        "x86_64",
        "fspec-x86_64-unknown-linux-gnu.tar.gz",
        checksum_mode,
        &archive,
        &install_dir,
        &[],
    );

    // @step Then it aborts with a non-zero exit
    assert!(!ok, "install.sh must abort on checksum mismatch. output: {combined}");

    // @step And it prints the expected and actual hashes
    assert!(
        combined.contains("0000000000000000000000000000000000000000000000000000000000000000"),
        "expected hash should be printed. output: {combined}"
    );
    let actual = std::process::Command::new("sha256sum")
        .arg(&archive)
        .output()
        .expect("sha256sum")
        .stdout;
    let actual_stdout = String::from_utf8_lossy(&actual);
    let actual_hash = actual_stdout.split_whitespace().next().unwrap_or("");
    assert!(
        combined.contains(actual_hash),
        "actual hash should be printed. expected to find {actual_hash} in: {combined}"
    );

    // @step And no binary is installed
    assert!(
        !install_dir.join("fspec").exists(),
        "no binary should be installed on checksum mismatch"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Install directory override via --dir
// ─────────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/github-release-binary-installer-install-sh-source-build-installer-build-install-sh.feature
#[test]
fn scenario_install_directory_override_via_dir_flag() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let env_dir = tmp.path().join("env-dir");
    let flag_dir = tmp.path().join("flag-dir");
    let archive = make_fake_archive(tmp.path(), "fspec-x86_64-unknown-linux-gnu.tar.gz");

    // @step Given the user wants to install to /usr/local/bin
    // @step When they run scripts/install.sh --dir /usr/local/bin
    // (the temp flag_dir stands in for /usr/local/bin; INSTALL_DIR points
    // elsewhere to prove --dir takes precedence over the env var)
    let (ok, combined) = run_install(
        "Linux",
        "x86_64",
        "fspec-x86_64-unknown-linux-gnu.tar.gz",
        "missing",
        &archive,
        &env_dir,
        &["--dir", flag_dir.to_string_lossy().as_ref()],
    );
    assert!(ok, "install.sh --dir should succeed. output: {combined}");

    // @step Then the fspec binary is installed to /usr/local/bin/fspec
    assert!(
        flag_dir.join("fspec").is_file(),
        "fspec should be installed to the --dir path (not the INSTALL_DIR path)"
    );
    assert!(
        !env_dir.join("fspec").exists(),
        "fspec must NOT be installed to the INSTALL_DIR path when --dir is given"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Install directory not on PATH prints instructions
// ─────────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/github-release-binary-installer-install-sh-source-build-installer-build-install-sh.feature
#[test]
fn scenario_install_directory_not_on_path_prints_instructions() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let install_dir = tmp.path().join("not-on-path");
    let archive = make_fake_archive(tmp.path(), "fspec-x86_64-unknown-linux-gnu.tar.gz");

    // @step Given the install directory is not on the user PATH
    assert!(!std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .any(|p| p == install_dir.to_string_lossy().as_ref()),
        "install dir must not be on PATH");

    // @step When the installer completes
    let (ok, combined) = run_install(
        "Linux",
        "x86_64",
        "fspec-x86_64-unknown-linux-gnu.tar.gz",
        "missing",
        &archive,
        &install_dir,
        &[],
    );
    assert!(ok, "install.sh should succeed. output: {combined}");

    // @step Then it prints instructions for adding the install directory to the shell config
    assert!(
        combined.contains("PATH") && (combined.contains("export") || combined.contains("shell config")),
        "should print PATH instructions. output: {combined}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: Release pipeline publishes checksums.txt
// ─────────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/github-release-binary-installer-install-sh-source-build-installer-build-install-sh.feature
#[test]
fn scenario_release_pipeline_publishes_checksums_txt() {
    // @step Given a git tag matching v* is pushed
    // (static check of the workflow — the pipeline itself runs in CI)
    let wf = fs::read_to_string(
        repo_root()
            .join(".github")
            .join("workflows")
            .join("release.yml"),
    )
    .expect("read release.yml");

    // @step When the release workflow completes
    // @step Then the GitHub Release contains the 5 binary archives
    for target in [
        "x86_64-pc-windows-msvc",
        "aarch64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        assert!(wf.contains(target), "workflow must build {target}");
    }

    // @step And it contains a checksums.txt asset
    assert!(
        wf.contains("checksums.txt"),
        "release.yml must generate a checksums.txt asset"
    );

    // @step And checksums.txt has one sha256sum-format line per archive (hash, two spaces, filename)
    assert!(
        wf.contains("sha256sum"),
        "release.yml must use sha256sum to generate checksums.txt"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario: README documents the one-line install
// ─────────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/github-release-binary-installer-install-sh-source-build-installer-build-install-sh.feature
#[test]
fn scenario_readme_documents_the_one_line_install() {
    // @step Given the README quick-start section
    let readme = fs::read_to_string(repo_root().join("README.md")).expect("read README.md");

    // @step When it is read
    // @step Then it shows the curl one-line install command for macOS and Linux
    assert!(
        readme.contains("curl") && readme.contains("install.sh"),
        "README must show the curl one-line install command"
    );

    // @step And it shows the PowerShell install command for Windows
    assert!(
        readme.contains("install.ps1") || readme.to_lowercase().contains("powershell"),
        "README must show the PowerShell install for Windows"
    );

    // @step And it references build-install.sh for the from-source build path
    assert!(
        readme.contains("build-install.sh"),
        "README must reference build-install.sh for the from-source path"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper scenario coverage: --help exits zero for both scripts
// ─────────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/github-release-binary-installer-install-sh-source-build-installer-build-install-sh.feature
#[test]
fn scenario_install_sh_help_exits_zero() {
    // @step When they run scripts/install.sh --help (via --help flag)
    let out = Command::new("bash")
        .args(["-c", &format!("{} --help", scripts_dir().join("install.sh").display())])
        .output()
        .expect("run install.sh --help");
    assert!(out.status.success(), "install.sh --help should exit 0");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("--dir") || combined.contains("INSTALL_DIR"),
        "help should document the install dir option: {combined}"
    );
}

/// Feature: spec/features/github-release-binary-installer-install-sh-source-build-installer-build-install-sh.feature
#[test]
fn scenario_build_install_sh_help_exits_zero() {
    // @step When they run scripts/build-install.sh --help (via --help flag)
    let out = Command::new("bash")
        .args(["-c", &format!("{} --help", scripts_dir().join("build-install.sh").display())])
        .output()
        .expect("run build-install.sh --help");
    assert!(out.status.success(), "build-install.sh --help should exit 0");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("--dir") || combined.contains("--profile"),
        "help should document the options: {combined}"
    );
}
