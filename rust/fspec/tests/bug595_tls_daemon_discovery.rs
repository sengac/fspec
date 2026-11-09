//! BUG-595: local daemon discovery entries name the scheme and host the
//! daemon serves, and discovery-based clients dial them.
//!
//! Feature: spec/features/local-daemon-discovery-names-the-scheme-and-host.feature

mod common;

#[allow(dead_code)]
#[path = "../src/common.rs"]
mod fspec_common;

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::SystemTime;

use common::{fspec_bin, make_workspace, ChildGuard};
use fspec_common::{DaemonListener, DaemonScheme};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TOKEN: &str = "bug595-test-token";

/// A live process other than this test (so two distinct live pids exist).
fn live_helper_process() -> std::io::Result<Child> {
    Command::new("sleep").arg("30").spawn()
}

fn tls_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tls")
        .join(name)
}

fn entry_json(dir: &Path, pid: u32) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(
        dir.join(format!("{pid}.json")),
    )?)?)
}

fn write_json(path: &Path, value: &serde_json::Value) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn old_entry(port: u16, pid: u32) -> serde_json::Value {
    serde_json::json!({
        "port": port,
        "pid": pid,
        "workspace": "/tmp",
        "started_at": "2026-11-01T00:00:00Z",
        "version": "0.11.0"
    })
}

fn tls_listener(host: &str) -> DaemonListener {
    DaemonListener {
        scheme: DaemonScheme::Wss,
        host: host.to_string(),
    }
}

#[test]
fn a_discovery_entry_records_the_scheme_and_host_the_daemon_serves() -> TestResult {
    // @step Given a TLS daemon on port 3777 whose discovery host is myhost.lan
    let root = tempfile::tempdir()?;
    let dir = root.path().join("daemons");
    let workspace = tempfile::tempdir()?;
    let mut helper = live_helper_process()?;
    let (me, other) = (std::process::id(), helper.id());

    // @step When its discovery entry is written and the live daemons are listed
    let now = SystemTime::now();
    fspec_common::write_daemon_entry(
        &dir,
        3777,
        me,
        workspace.path(),
        now,
        &tls_listener("myhost.lan"),
    )?;
    fspec_common::write_daemon_entry(
        &dir,
        4002,
        other,
        workspace.path(),
        now,
        &DaemonListener::loopback(),
    )?;
    let live = fspec_common::list_live_daemons(&dir)?;
    let (tls_json, plain_json) = (entry_json(&dir, me)?, entry_json(&dir, other)?);
    helper.kill()?;

    // @step Then the entry holds "scheme": "wss" and "host": "myhost.lan"
    assert_eq!(tls_json["scheme"], "wss");
    assert_eq!(tls_json["host"], "myhost.lan");
    // @step And the listed daemon's URL is wss://myhost.lan:3777
    let urls: Vec<String> = live.iter().map(|entry| entry.url()).collect();
    assert!(
        urls.contains(&"wss://myhost.lan:3777".to_string()),
        "{urls:?}"
    );
    // @step And a plain daemon's entry holds "scheme": "ws" and "host": "127.0.0.1" and is listed as ws://127.0.0.1:<port>
    assert_eq!(plain_json["scheme"], "ws");
    assert_eq!(plain_json["host"], "127.0.0.1");
    assert!(
        urls.contains(&"ws://127.0.0.1:4002".to_string()),
        "{urls:?}"
    );
    Ok(())
}

#[test]
fn an_entry_written_before_the_fields_existed_is_a_plain_loopback_daemon() -> TestResult {
    // @step Given a daemons/<pid>.json and a daemon.json for a live process with only port, pid, workspace, started_at and version
    let root = tempfile::tempdir()?;
    let dir = root.path().join("daemons");
    let me = std::process::id();
    write_json(&dir.join(format!("{me}.json")), &old_entry(4401, me))?;
    let pointer = root.path().join("daemon.json");
    write_json(&pointer, &old_entry(4401, me))?;

    // @step When the live daemons are listed and daemon.json is read
    let live = fspec_common::list_live_daemons(&dir)?;
    let handshake = fspec_common::read_and_verify_daemon_json(&pointer)?;

    // @step Then both name ws://127.0.0.1:<port>
    let urls: Vec<String> = live.iter().map(|entry| entry.url()).collect();
    assert_eq!(urls, ["ws://127.0.0.1:4401"]);
    assert_eq!(handshake.url(), "ws://127.0.0.1:4401");
    Ok(())
}

#[test]
fn an_entry_with_an_unknown_scheme_or_an_empty_host_is_unreadable() -> TestResult {
    // @step Given discovery entries for live processes with "scheme": "https" and with "host": ""
    let root = tempfile::tempdir()?;
    let dir = root.path().join("daemons");
    let mut helper = live_helper_process()?;
    let (me, other) = (std::process::id(), helper.id());
    let mut https = old_entry(4501, me);
    https["scheme"] = "https".into();
    https["host"] = "127.0.0.1".into();
    let mut empty_host = old_entry(4502, other);
    empty_host["scheme"] = "ws".into();
    empty_host["host"] = "".into();
    write_json(&dir.join(format!("{me}.json")), &https)?;
    write_json(&dir.join(format!("{other}.json")), &empty_host)?;
    let (https_pointer, empty_pointer) = (root.path().join("a.json"), root.path().join("b.json"));
    write_json(&https_pointer, &https)?;
    let mut empty_for_me = empty_host.clone();
    empty_for_me["pid"] = me.into();
    write_json(&empty_pointer, &empty_for_me)?;

    // @step When the live daemons are listed and daemon.json is read
    let live = fspec_common::list_live_daemons(&dir)?;
    let https_read = fspec_common::read_and_verify_daemon_json(&https_pointer);
    let empty_read = fspec_common::read_and_verify_daemon_json(&empty_pointer);
    helper.kill()?;

    // @step Then the listing deletes both entries
    assert!(live.is_empty(), "{live:?}");
    assert!(!dir.join(format!("{me}.json")).exists());
    assert!(!dir.join(format!("{other}.json")).exists());
    // @step And reading daemon.json fails naming the invalid field
    let https_error = format!("{:#}", https_read.err().ok_or("https entry was read")?);
    assert!(
        https_error.contains("scheme") && https_error.contains("https"),
        "{https_error}"
    );
    let empty_error = format!("{:#}", empty_read.err().ok_or("empty-host entry was read")?);
    assert!(empty_error.contains("host"), "{empty_error}");
    Ok(())
}

#[test]
fn the_discovery_host_follows_the_bind_tls_and_tls_name() -> TestResult {
    // @step Given the binds 127.0.0.1:0, localhost:0, [::1]:0, 0.0.0.0:0, :::0 and 192.168.1.5:3777
    let listener = |bind: &str, tls: bool, name: Option<&str>| {
        fspec_common::discovery_listener(bind, tls, name).map(|l| (l.scheme, l.host))
    };
    let plain = DaemonScheme::Ws;
    let tls = DaemonScheme::Wss;

    // @step When each is turned into a discovery scheme and host, plain and with TLS
    // @step Then a plain daemon is ws with the bind host, and 127.0.0.1 for 0.0.0.0 and ::
    assert_eq!(
        listener("127.0.0.1:0", false, None)?,
        (plain, "127.0.0.1".to_string())
    );
    assert_eq!(
        listener("localhost:0", false, None)?,
        (plain, "localhost".to_string())
    );
    assert_eq!(
        listener("[::1]:0", false, None)?,
        (plain, "::1".to_string())
    );
    assert_eq!(
        listener("0.0.0.0:0", false, None)?,
        (plain, "127.0.0.1".to_string())
    );
    assert_eq!(
        listener(":::0", false, None)?,
        (plain, "127.0.0.1".to_string())
    );
    assert_eq!(
        listener("[::]:0", false, None)?,
        (plain, "127.0.0.1".to_string())
    );
    let ipv6 = DaemonListener {
        scheme: plain,
        host: "::1".to_string(),
    };
    assert_eq!(ipv6.url(4600), "ws://[::1]:4600");

    // @step And a TLS daemon is wss with the bind host, and with --tls-name myhost.lan its host is myhost.lan
    assert_eq!(
        listener("192.168.1.5:3777", true, None)?,
        (tls, "192.168.1.5".to_string())
    );
    assert_eq!(
        listener("127.0.0.1:0", true, None)?,
        (tls, "127.0.0.1".to_string())
    );
    assert_eq!(
        listener("0.0.0.0:3777", true, Some("myhost.lan"))?,
        (tls, "myhost.lan".to_string())
    );
    assert_eq!(
        listener("192.168.1.5:3777", true, Some("myhost.lan"))?,
        (tls, "myhost.lan".to_string())
    );

    // @step And a TLS daemon on 0.0.0.0 or :: without --tls-name is refused with a message naming --tls-name
    for bind in ["0.0.0.0:3777", ":::3777", "[::]:3777"] {
        let refused = listener(bind, true, None)
            .err()
            .ok_or(format!("{bind} was accepted"))?;
        assert!(refused.to_string().contains("--tls-name"), "{refused}");
    }
    Ok(())
}

fn daemon_command(home: &Path, workspace: &Path) -> common::IsolatedCommand {
    let mut command = common::IsolatedCommand::new(fspec_bin());
    command
        .arg("--workspace")
        .arg(workspace)
        .arg("daemon")
        .args(["--bind", "0.0.0.0:0", "--tls-cert"])
        .arg(tls_fixture("server.pem"))
        .arg("--tls-key")
        .arg(tls_fixture("server.key"))
        .env("HOME", home)
        .env("FSPEC_USER_DIR", home.join(".fspec"))
        .env("XDG_RUNTIME_DIR", home)
        .env("FSPEC_DAEMON_TOKEN", TOKEN)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

#[test]
fn a_tls_daemon_on_an_unspecified_bind_without_tls_name_refuses_to_start() -> TestResult {
    // @step Given a certificate, its key and a token
    let home = tempfile::tempdir()?;
    let (workspace, _path) = make_workspace(&[("TLS-595", "refused", "backlog")]);

    // @step When "fspec daemon --bind 0.0.0.0:0 --tls-cert --tls-key" runs without --tls-name
    let output = daemon_command(home.path(), workspace.path()).output()?;

    // @step Then it exits with an error naming --tls-name
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--tls-name"), "{stderr}");
    Ok(())
}

#[test]
fn fspec_status_without_connect_dials_the_tls_daemons_scheme_and_host() -> TestResult {
    // @step Given "fspec daemon --bind 0.0.0.0:0 --tls-cert --tls-key --tls-name localhost" running with a token
    let home = tempfile::tempdir()?;
    let (workspace, _path) = make_workspace(&[("TLS-595", "status", "backlog")]);
    let mut child = daemon_command(home.path(), workspace.path())
        .args(["--tls-name", "localhost"])
        .spawn()?;
    let stdout = child.stdout.take().ok_or("no daemon stdout")?;
    let mut guard = ChildGuard(child);
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line)?;
    let port: u16 = line.trim().parse()?;
    // The discovery entry is written after the port is printed.
    let entry = home.path().join("fspec/daemon.json");
    while !entry.exists() {
        if let Some(exit) = guard.0.try_wait()? {
            return Err(format!("the daemon exited ({exit}) before writing daemon.json").into());
        }
        std::thread::yield_now();
    }

    // @step When "fspec status" runs without --connect
    let status = Command::new(fspec_bin())
        .arg("status")
        .env("HOME", home.path())
        .env("FSPEC_USER_DIR", home.path().join(".fspec"))
        .env("XDG_RUNTIME_DIR", home.path())
        .env("FSPEC_DAEMON_TOKEN", TOKEN)
        .output()?;

    // @step Then it dials wss://localhost:<port>
    // (The test CA is not a public root, so the TLS handshake itself fails;
    // the URL in the failure is the one discovery produced.)
    let stderr = String::from_utf8_lossy(&status.stderr);
    assert!(
        stderr.contains(&format!("wss://localhost:{port}")),
        "{stderr}"
    );
    Ok(())
}
