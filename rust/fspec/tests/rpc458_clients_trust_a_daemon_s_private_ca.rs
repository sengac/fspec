//! RPC-458 — fspec's clients trust a daemon's private-CA certificate
//! (FSPEC_DAEMON_CA, and a saved daemon's own CA), with no
//! skip-verification mode.
//!
//! Feature: spec/features/clients-trust-a-daemon-s-private-ca.feature
//!
//! Each test serves the RPC-456 test certificate (for localhost and
//! 127.0.0.1, signed by tests/fixtures/tls/ca.pem) from an in-process TLS
//! listener and runs the fspec binary against it with HOME and
//! XDG_RUNTIME_DIR in a temp dir.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve_tls;
use codelet_rpc_server::tls::load_tls_acceptor;
use common::{fspec_bin, make_workspace};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TOKEN: &str = "rpc458-test-token";

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tls")
        .join(name)
}

/// A TLS daemon listener on `host` serving the test certificate, with the
/// token armed. Keep the returned workspace alive while it is used.
async fn tls_daemon(host: &str) -> Result<(tempfile::TempDir, u16), Box<dyn std::error::Error>> {
    let (workspace, _path) = make_workspace(&[("TLS-458", "private ca", "backlog")]);
    let service = Arc::new(
        SharedFspecService::new(Arc::new(WorkUnitsWatcher::new(workspace.path())?))
            .with_cwd(workspace.path().to_path_buf()),
    );
    service.set_auth_token(TOKEN.to_owned());
    let acceptor = load_tls_acceptor(&fixture("server.pem"), &fixture("server.key"))?;
    let (address, _stats, _join) =
        bind_and_serve_tls(&format!("{host}:0"), service, acceptor).await?;
    Ok((workspace, address.port()))
}

/// Run fspec with `args` in the isolated `home`, FSPEC_DAEMON_CA set to
/// `ca` (unset when `None`) and no FSPEC_DAEMON_TOKEN.
async fn fspec(
    home: &Path,
    ca: Option<&Path>,
    args: &[&str],
) -> Result<Output, Box<dyn std::error::Error>> {
    let mut command = Command::new(fspec_bin());
    command
        .args(args)
        .env("HOME", home)
        .env("XDG_RUNTIME_DIR", home)
        .env_remove("FSPEC_DAEMON_TOKEN")
        .env_remove("FSPEC_DAEMON_CA");
    if let Some(ca) = ca {
        command.env("FSPEC_DAEMON_CA", ca);
    }
    Ok(tokio::task::spawn_blocking(move || command.output()).await??)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fspec_daemon_ca_makes_a_private_ca_daemon_reachable() -> TestResult {
    // @step Given a TLS daemon serving a certificate for localhost signed by the test CA, with a token
    let (_workspace, port) = tls_daemon("127.0.0.1").await?;
    let home = tempfile::tempdir()?;

    // @step When "fspec status --connect wss://localhost:<port>" runs with FSPEC_DAEMON_CA naming the test CA and the token
    let url = format!("wss://localhost:{port}");
    let ca = fixture("ca.pem");
    let output = fspec(
        home.path(),
        Some(&ca),
        &["status", "--connect", &url, "--token", TOKEN],
    )
    .await?;

    // @step Then it reports the daemon alive
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("fspec daemon: alive"),
        "{}",
        stdout(&output)
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_the_ca_the_certificate_is_refused() -> TestResult {
    // @step Given the same TLS daemon
    let (_workspace, port) = tls_daemon("127.0.0.1").await?;
    let home = tempfile::tempdir()?;

    // @step When "fspec status --connect wss://localhost:<port>" runs without FSPEC_DAEMON_CA
    let url = format!("wss://localhost:{port}");
    let output = fspec(
        home.path(),
        None,
        &["status", "--connect", &url, "--token", TOKEN],
    )
    .await?;

    // @step Then it exits with code 1 and names a certificate verification failure
    let stderr = stderr(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("TLS certificate verification failed for localhost"),
        "{stderr}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_ca_file_that_cannot_be_used_is_an_error_before_connecting() -> TestResult {
    // @step Given FSPEC_DAEMON_CA names a missing file, and then a file that holds no PEM certificate
    let (_workspace, port) = tls_daemon("127.0.0.1").await?;
    let home = tempfile::tempdir()?;
    let missing = home.path().join("missing.pem");
    let empty = home.path().join("empty.pem");
    std::fs::write(&empty, "not a certificate\n")?;

    // @step When a client connects to wss://localhost:<port>
    let url = format!("wss://localhost:{port}");
    let args = ["status", "--connect", &url, "--token", TOKEN];
    let missing_output = fspec(home.path(), Some(&missing), &args).await?;
    let empty_output = fspec(home.path(), Some(&empty), &args).await?;

    // @step Then each connect fails naming the file and FSPEC_DAEMON_CA
    let missing_stderr = stderr(&missing_output);
    assert_eq!(missing_output.status.code(), Some(1), "{missing_stderr}");
    assert!(
        missing_stderr.contains(&format!(
            "FSPEC_DAEMON_CA {} cannot be read",
            missing.display()
        )),
        "{missing_stderr}"
    );
    let empty_stderr = stderr(&empty_output);
    assert_eq!(empty_output.status.code(), Some(1), "{empty_stderr}");
    assert!(
        empty_stderr.contains(&format!(
            "FSPEC_DAEMON_CA {} holds no PEM certificate",
            empty.display()
        )),
        "{empty_stderr}"
    );
    for text in [&missing_stderr, &empty_stderr] {
        assert!(!text.contains("verification failed"), "{text}");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_trusted_ca_does_not_relax_host_verification() -> TestResult {
    // @step Given a TLS daemon serving a certificate for localhost signed by the test CA
    let (_workspace, port) = tls_daemon("127.0.0.2").await?;
    let home = tempfile::tempdir()?;

    // @step When a client trusting the test CA dials it as wss://127.0.0.2:<port>
    let url = format!("wss://127.0.0.2:{port}");
    let ca = fixture("ca.pem");
    let output = fspec(
        home.path(),
        Some(&ca),
        &["status", "--connect", &url, "--token", TOKEN],
    )
    .await?;

    // @step Then the connect fails with a certificate verification error
    let stderr = stderr(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("TLS certificate verification failed for 127.0.0.2"),
        "{stderr}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_saved_daemon_carries_its_own_ca() -> TestResult {
    // @step Given "fspec daemons add lab wss://localhost:<port> --token t --ca ca.pem"
    let (_workspace, port) = tls_daemon("127.0.0.1").await?;
    let home = tempfile::tempdir()?;
    // A relative --ca, resolved against the working directory.
    let ca = fixture("ca.pem");
    let ca_dir = ca.parent().ok_or("fixture dir")?.to_path_buf();
    let url = format!("wss://localhost:{port}");
    let mut add = Command::new(fspec_bin());
    add.args([
        "daemons", "add", "lab", &url, "--token", TOKEN, "--ca", "ca.pem",
    ])
    .current_dir(&ca_dir)
    .env("HOME", home.path())
    .env("XDG_RUNTIME_DIR", home.path())
    .env_remove("FSPEC_DAEMON_CA");
    let added = tokio::task::spawn_blocking(move || add.output()).await??;
    assert!(added.status.success(), "{}", stderr(&added));

    // @step When the saved daemon lab is connected to by name without FSPEC_DAEMON_CA
    let output = fspec(home.path(), None, &["status", "--connect", "lab"]).await?;

    // @step Then daemons.json records the absolute path of ca.pem for lab
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        home.path().join(".fspec/daemons.json"),
    )?)?;
    let absolute = std::fs::canonicalize(&ca)?;
    assert_eq!(
        saved["daemons"][0]["ca"].as_str(),
        absolute.to_str(),
        "{saved}"
    );
    // @step And the connection is made trusting ca.pem
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("fspec daemon: alive"),
        "{}",
        stdout(&output)
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saving_a_daemon_with_an_unusable_ca_file_is_refused() -> TestResult {
    // @step Given a file empty.pem that holds no PEM certificate
    let home = tempfile::tempdir()?;
    let empty = home.path().join("empty.pem");
    std::fs::write(&empty, "")?;
    let empty_arg = empty.to_string_lossy().into_owned();
    let ca = fixture("ca.pem");
    let ca_arg = ca.to_string_lossy().into_owned();

    // @step When "fspec daemons add lab wss://localhost:3777 --token t --ca empty.pem" runs, and then the same with a ws:// URL and ca.pem
    let no_certificate = fspec(
        home.path(),
        None,
        &[
            "daemons",
            "add",
            "lab",
            "wss://localhost:3777",
            "--token",
            "t",
            "--ca",
            &empty_arg,
        ],
    )
    .await?;
    let plain = fspec(
        home.path(),
        None,
        &[
            "daemons",
            "add",
            "lab",
            "ws://localhost:3777",
            "--ca",
            &ca_arg,
        ],
    )
    .await?;

    // @step Then each exits with an error naming the problem, empty.pem naming the file
    let no_certificate_stderr = stderr(&no_certificate);
    assert!(!no_certificate.status.success(), "{no_certificate_stderr}");
    assert!(
        no_certificate_stderr.contains(&format!("{} holds no PEM certificate", empty.display())),
        "{no_certificate_stderr}"
    );
    let plain_stderr = stderr(&plain);
    assert!(!plain.status.success(), "{plain_stderr}");
    assert!(
        plain_stderr.contains("--ca applies only to a wss:// daemon"),
        "{plain_stderr}"
    );
    // @step And lab is not saved
    let listed = fspec(home.path(), None, &["daemons", "list"]).await?;
    assert!(listed.status.success(), "{}", stderr(&listed));
    assert!(
        stdout(&listed).contains("No saved daemons"),
        "{}",
        stdout(&listed)
    );
    Ok(())
}
