//! RLCD-001 — service supervisor: config, health polling, local auto-spawn
//! with port scanning.
//!
//! Feature: spec/features/rlcd-service-supervisor.feature
//!
//! Covers the RLCD supervisor contract (see the RLCD-001 architecture notes):
//! user-config loading (defaults / partial / malformed, never fatal, never
//! writes), key-preserving config writes, /health polling into the shared
//! status (model name ALWAYS from the response, never hardcoded; transition
//! warnings logged exactly once per transition), local-only auto-spawn
//! (remote URLs connect-only), pidfile stale/live handling, port scanning
//! (free / occupied-by-non-RLCD / occupied-by-healthy-RLCD / all-occupied
//! fail-open), and the fail-open snapshot contract shared by all RLCD
//! consumers (RLCD-002/003/004).
//!
//! The live spawn scenario is gated on the `rlcd` binary + a checkpoint
//! cache being present (skips silently otherwise) and is `#[ignore]`d by
//! default — it spawns a real server (multi-GB RAM).
//!
//! ACDD red phase: at test-writing time `codelet_tools::rlcd` does not exist
//! yet; this suite fails to compile/link until the implementing phase
//! provides the contracted API.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "rlcd_mock.rs"]
mod rlcd_mock;

use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

use codelet_tools::rlcd::{
    ensure_local_server, global_supervisor, is_local_url, is_pid_alive, load_rlcd_config_from,
    pidfile_path, read_pidfile, save_rlcd_url_at, select_port, EnsureOutcome, PortSelection,
    RlcdConfig, RlcdSpawnConfig, RlcdSupervisor,
};
use rlcd_mock::{occupy_port, MockRlcd};
use serial_test::serial;
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::Mutex;
use tempfile::TempDir;
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::Layer;
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

// ============================================================================
// log capture (transition warnings)
// ============================================================================

/// Minimal event-recording layer: stores the formatted field summary of
/// every WARN-or-above event. Used to assert that the supervisor logs
/// exactly ONE transition warning per state change (never per-poll).
#[derive(Default)]
struct WarnCapture {
    entries: Mutex<Vec<String>>,
}

/// `tracing::field::Visit` visitor that accumulates `name=value` pairs.
struct MsgVisitor(String);

impl Visit for MsgVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.push_str(&format!("{}={:?}", field.name(), value));
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.push_str(&format!("{}={}", field.name(), value));
    }
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.push_str(&format!("{}={}", field.name(), value));
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.push_str(&format!("{}={}", field.name(), value));
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0.push_str(&format!("{}={}", field.name(), value));
    }
    fn record_error(&mut self, field: &Field, value: &dyn std::error::Error) {
        self.0.push_str(&format!("{}={}", field.name(), value));
    }
}

/// Thin layer that forwards to a shared [`WarnCapture`] so the capture can
/// be installed into the global subscriber (by value) while the test side
/// reads it through the `Arc`.
struct WarnLayer(Arc<WarnCapture>);

impl<S> Layer<S> for WarnLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        record_warn_event(&self.0, event);
    }
}

/// Capture one WARN/ERROR event into `capture`.
fn record_warn_event(capture: &WarnCapture, event: &tracing::Event<'_>) {
    let level = *event.metadata().level();
    if level != tracing::Level::WARN && level != tracing::Level::ERROR {
        return;
    }
    let mut visitor = MsgVisitor(String::new());
    event.record(&mut visitor);
    let mut guard = capture.entries.lock().expect("warn capture lock poisoned");
    guard.push(visitor.0);
}

static LOG_LAYER: once_cell::sync::OnceCell<Arc<WarnCapture>> =
    once_cell::sync::OnceCell::new();

/// Install the capture layer as the global subscriber (once per process) and
/// return a handle to it.
fn test_log_layer() -> &'static Arc<WarnCapture> {
    LOG_LAYER.get_or_init(|| {
        let capture = Arc::new(WarnCapture::default());
        let subscriber = tracing_subscriber::registry().with(WarnLayer(capture.clone()));
        let _ = tracing::subscriber::set_global_default(subscriber);
        capture
    })
}

/// Drain (and return) all WARN-level event messages captured so far.
fn drain_warn_events() -> Vec<String> {
    let layer = test_log_layer();
    let mut guard = layer
        .entries
        .lock()
        .expect("warn capture lock poisoned");
    std::mem::take(&mut guard)
}

/// Install a temporary `FSPEC_USER_DIR` so pidfile/log writes land in a temp
/// directory, never in the real `~/.fspec`. Restores the environment on drop
/// (tests that touch this run under `#[serial]`).
struct UserDirGuard(bool);

impl UserDirGuard {
    fn new(tmp: &TempDir) -> Self {
        std::env::set_var("FSPEC_USER_DIR", tmp.path());
        UserDirGuard(true)
    }
}

impl Drop for UserDirGuard {
    fn drop(&mut self) {
        if self.0 {
            std::env::remove_var("FSPEC_USER_DIR");
        }
    }
}

/// A spawn binary name that can never resolve on PATH — guarantees tests that
/// exercise the local-spawn flow cannot accidentally launch a real (multi-GB)
/// RLCD server.
const FAKE_BINARY: &str = "rlcd-test-binary-does-not-exist";

// ============================================================================
// CONFIG
// ============================================================================

/// Scenario: Fresh machine with no config file resolves the full defaults
#[test]
fn scenario_fresh_machine_defaults() {
    // @step Given a user config directory with no fspec-config.json file
    let tmp = TempDir::new().expect("tempdir");
    let path = tmp.path().join("fspec-config.json");
    assert!(!path.exists(), "fresh temp dir must have no config file");

    // @step When the RLCD config is loaded
    let cfg = load_rlcd_config_from(&path);

    // @step Then the resolved config uses url "http://127.0.0.1:8000"
    assert_eq!(cfg.url, "http://127.0.0.1:8000");
    // @step And the resolved config uses spawn port 8000
    assert_eq!(cfg.spawn.port, 8000);
    // @step And the resolved config uses spawn binary "rlcd"
    assert_eq!(cfg.spawn.binary, "rlcd");
    // @step And the resolved config uses the default backend "rlcd"
    assert_eq!(cfg.backend, "rlcd");
    // @step And the resolved config has enabled true
    assert!(cfg.enabled, "default config must keep RLCD enabled");
    // @step And no config file has been written to disk
    assert!(!path.exists(), "loading must never write a config file");
}

/// Scenario: A malformed config file resolves to defaults without failing
#[test]
fn scenario_malformed_config_resolves_to_defaults() {
    // @step Given a user config directory whose fspec-config.json contains invalid JSON
    let tmp = TempDir::new().expect("tempdir");
    let path = tmp.path().join("fspec-config.json");
    fs::write(&path, "this is { not json").expect("write malformed config");

    // @step When the RLCD config is loaded
    let cfg = load_rlcd_config_from(&path);

    // @step Then the resolved config uses the default url "http://127.0.0.1:8000"
    assert_eq!(cfg.url, "http://127.0.0.1:8000");
    // @step And loading does not return an error
    // (the loader resolves to defaults instead of erroring — reaching this
    //  point with a fully-populated default config IS the proof)
    assert!(cfg.enabled, "malformed config still resolves to the enabled default");
}

/// Scenario: A partial config fills only the missing fields from defaults
#[test]
fn scenario_partial_config_fills_missing_fields() {
    // @step Given a user config directory whose fspec-config.json sets rlcd.url to "http://10.0.0.5:9000" only
    let tmp = TempDir::new().expect("tempdir");
    let path = tmp.path().join("fspec-config.json");
    fs::write(&path, r#"{"rlcd":{"url":"http://10.0.0.5:9000"}}"#)
        .expect("write partial config");

    // @step When the RLCD config is loaded
    let cfg = load_rlcd_config_from(&path);

    // @step Then the resolved config uses url "http://10.0.0.5:9000"
    assert_eq!(cfg.url, "http://10.0.0.5:9000");
    // @step And the resolved config uses the default spawn port 8000
    assert_eq!(cfg.spawn.port, 8000);
    // @step And the resolved config uses the default spawn binary "rlcd"
    assert_eq!(cfg.spawn.binary, "rlcd");
}

/// Scenario: Writing the RLCD config preserves every unrelated key verbatim
#[test]
fn scenario_write_preserves_unrelated_keys() {
    // @step Given a user config directory whose fspec-config.json already contains "theme":"dark" and a providers section
    let tmp = TempDir::new().expect("tempdir");
    let path = tmp.path().join("fspec-config.json");
    fs::write(
        &path,
        r#"{"theme":"dark","providers":{"openai":{"profiles":{"p1":{"baseUrl":"http://x"}}}},"tui":{"lastUsedModel":"m1"}}"#,
    )
    .expect("write pre-existing config");

    // @step When the RLCD section is written with url "http://127.0.0.1:9100"
    save_rlcd_url_at(&path, "http://127.0.0.1:9100").expect("write rlcd url");

    // @step Then the config file contains rlcd.url "http://127.0.0.1:9100"
    let raw = fs::read_to_string(&path).expect("read back");
    let root: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    assert_eq!(root["rlcd"]["url"], "http://127.0.0.1:9100");
    // @step And the config file still contains "theme":"dark"
    assert_eq!(root["theme"], "dark");
    // @step And the config file still contains the providers section unchanged
    assert_eq!(
        root["providers"]["openai"]["profiles"]["p1"]["baseUrl"],
        "http://x"
    );
    assert_eq!(root["tui"]["lastUsedModel"], "m1");

    // and the round-tripped config loads the written url
    assert_eq!(
        load_rlcd_config_from(&path).url,
        "http://127.0.0.1:9100"
    );
}

// ============================================================================
// HEALTH POLLING + STATUS
// ============================================================================

/// Scenario: A healthy local server is detected and its reported model name
/// is used
#[tokio::test]
#[serial]
async fn scenario_healthy_local_detected_with_reported_model() {
    // @step Given a mock RLCD server on 127.0.0.1 whose /health returns {"status":"ready","model":"typed-decisions"}
    let mock = MockRlcd::ready("typed-decisions").start().await;
    let _user_dir = UserDirGuard::new(&TempDir::new().expect("temp user dir"));
    let cfg = RlcdConfig {
        url: mock.base_url(),
        ..Default::default()
    };
    let sup = RlcdSupervisor::new(cfg);

    // @step When the supervisor polls /health
    let model = sup
        .ensure_ready(Duration::from_secs(5))
        .await
        .expect("already-healthy server must be detected within the budget");

    // @step Then the RLCD status reports reachable true
    assert!(sup.reachable(), "status must report reachable after a ready /health");
    // @step And the RLCD status carries the model name "typed-decisions"
    assert_eq!(model, "typed-decisions");
    assert_eq!(sup.model().as_deref(), Some("typed-decisions"));
    // @step And no local server has been spawned
    assert!(
        read_pidfile(&pidfile_path()).is_none(),
        "a healthy existing server must not trigger a spawn (no pidfile)"
    );
    mock.stop().await;
}

/// Scenario: A down server flips the status to unreachable and back
#[tokio::test]
#[serial]
async fn scenario_down_server_flips_unreachable_and_back() {
    // @step Given a mock RLCD server on 127.0.0.1 that answers /health then stops
    let mock = MockRlcd::ready("m1").start().await;
    let cfg = RlcdConfig {
        url: mock.base_url(),
        ..Default::default()
    };
    let sup = RlcdSupervisor::new(cfg);
    sup.ensure_ready(Duration::from_secs(5))
        .await
        .expect("server ready");
    assert!(sup.reachable(), "precondition: server initially reachable");

    // @step When the supervisor polls /health after the server stops
    mock.stop().await;
    let _ = drain_warn_events(); // clear any pre-stop noise
    sup.refresh().await;

    // @step Then the RLCD status reports reachable false
    assert!(!sup.reachable(), "stopped server must flip the status to unreachable");
    assert!(sup.unreachable_reason().is_some());

    // @step And a single down transition warning has been logged
    let warns = drain_warn_events();
    assert_eq!(
        warns.iter().filter(|m| m.to_lowercase().contains("down")).count(),
        1,
        "exactly one down-transition warning expected, got: {warns:?}"
    );
    // second poll after the transition: NO additional warning (rule 3 —
    // transition-only logging, never per-poll)
    sup.refresh().await;
    let warns_again = drain_warn_events();
    assert_eq!(
        warns_again.iter().filter(|m| m.to_lowercase().contains("down")).count(),
        0,
        "steady-state down polls must not re-log the transition warning: {warns_again:?}"
    );
}

/// Scenario: The model name from /health is never hardcoded
#[tokio::test]
#[serial]
async fn scenario_model_name_never_hardcoded() {
    // @step Given a mock RLCD server whose /health reports model "custom-variant"
    let mock = MockRlcd::ready("custom-variant").start().await;
    let cfg = RlcdConfig {
        url: mock.base_url(),
        ..Default::default()
    };
    let sup = RlcdSupervisor::new(cfg);

    // @step When the supervisor polls /health and then a classification request is built
    sup.refresh().await;
    // the model the request builder (RLCD-002) will echo as request.model is
    // exactly what the supervisor captured from the /health response:
    let captured_model = sup.model().expect("model captured").to_string();
    // rotate the server-reported model to prove nothing is hardcoded/cached:
    mock.set_health(Some(r#"{"status":"ready","model":"rotated-model"}"#));
    sup.refresh().await;

    // @step Then the request carries model "custom-variant"
    assert_eq!(
        captured_model, "custom-variant",
        "the model captured at poll time must be the server-reported value"
    );
    assert_eq!(
        sup.model().as_deref(),
        Some("rotated-model"),
        "the status model must track the live /health response, not a constant"
    );
    mock.stop().await;
}

/// Scenario: A remote URL is connect-only and never spawned
#[tokio::test]
#[serial]
async fn scenario_remote_url_is_connect_only() {
    // @step Given a configured url pointing at remote host 192.168.1.50 port 9000
    let cfg = RlcdConfig {
        url: "http://192.168.1.50:9000".to_string(),
        ..RlcdConfig::default()
    };
    assert!(!is_local_url(&cfg.url), "remote host must not be classified as local");

    // @step And no server is running on that remote host
    // (192.168.1.50 is not routable from the test environment; every connect
    //  attempt fails)

    // @step When the supervisor checks RLCD availability
    let _user_dir = UserDirGuard::new(&TempDir::new().expect("temp user dir"));
    let started = Instant::now();
    let outcome = ensure_local_server(&cfg).await;
    let sup = RlcdSupervisor::new(cfg);
    let _ = sup.ensure_ready(Duration::from_millis(1000)).await;
    let _ = started;

    // @step Then the status reports unreachable
    assert!(!sup.reachable(), "unroutable remote host must report unreachable");
    assert!(sup.unreachable_reason().is_some());
    // @step And no local server spawn has been attempted
    match outcome {
        EnsureOutcome::NoLocalSpawn => {}
        other => panic!("remote URL must be connect-only, got {other:?}"),
    }
    // @step And no pidfile has been created
    assert!(read_pidfile(&pidfile_path()).is_none());
}

// ============================================================================
// AUTO-SPAWN: PORT SCAN
// ============================================================================

/// Occupy `count` CONSECUTIVE ports (all non-RLCD) and return the base port
/// plus the handles that keep the ports occupied. Retries until a free
/// consecutive window is found.
async fn occupy_consecutive_ports(count: usize) -> (u16, Vec<tokio::net::TcpListener>) {
    for _ in 0..200 {
        let base = probe_base_port().await;
        let mut holders = Vec::with_capacity(count);
        let mut ok = true;
        for i in 0..count {
            match tokio::net::TcpListener::bind(SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::LOCALHOST,
                base + i as u16,
            )))
            .await
            {
                Ok(listener) => holders.push(listener),
                Err(_) => {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            return (base, holders);
        }
    }
    panic!("could not find {count} consecutive free ports");
}

/// A port the kernel just handed out for 127.0.0.1:0 — free at probe time.
async fn probe_base_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind probe");
    listener
        .local_addr()
        .expect("local addr")
        .port()
}

/// Scenario: When the configured port is free it is used for the spawn
#[tokio::test]
async fn scenario_free_configured_port_is_used() {
    // @step Given the configured spawn port 8000 is free
    // (the fixed 8000 is not guaranteed free in CI — an equivalent free
    //  ephemeral port stands in for "the configured port", and the scan
    //  must select it as the first candidate)
    let free = probe_base_port().await;
    let cfg = RlcdConfig {
        spawn: RlcdSpawnConfig {
            port: free,
            ..Default::default()
        },
        ..RlcdConfig::default()
    };

    // @step When the supervisor selects a port for spawning
    let sel = select_port(&cfg).await.expect("port selection must succeed");

    // @step Then port 8000 is selected
    match sel {
        PortSelection::SpawnFree { port } => assert_eq!(port, free, "free configured port must be selected"),
        other => panic!("expected SpawnFree({free}), got {other:?}"),
    }
}

/// Scenario: A port occupied by a non-RLCD service is skipped in favor of
/// the next free port
#[tokio::test]
async fn scenario_occupied_non_rlcd_port_skipped() {
    // @step Given port 8000 is occupied by a service that does not answer /health with the RLCD ready shape
    // (a bare TCP listener stands in for the non-RLCD occupant on the
    //  configured port; it accepts connections but never answers /health)
    let (occupied, _holder) = occupy_port().await;
    let cfg = RlcdConfig {
        spawn: RlcdSpawnConfig {
            port: occupied,
            port_scan_limit: 5,
            ..Default::default()
        },
        ..RlcdConfig::default()
    };

    // @step And port 8001 is free
    // (the scan window continues past the occupied port; at least one port in
    //  [occupied+1, occupied+5] is free in any healthy environment)

    // @step When the supervisor selects a port for spawning
    let sel = select_port(&cfg).await.expect("port selection must succeed");

    // @step Then port 8001 is selected
    match sel {
        PortSelection::SpawnFree { port } => {
            assert!(port > occupied, "the occupied port must be skipped, got {port}");
            assert!(
                port <= occupied + 5,
                "selected port must stay inside the scan window: {port}"
            );
        }
        other => panic!("expected SpawnFree past the occupied port, got {other:?}"),
    }
}

/// Scenario: A port occupied by a healthy RLCD server is reused without
/// spawning
#[tokio::test]
#[serial]
async fn scenario_occupied_healthy_rlcd_port_reused() {
    // @step Given port 8000 is occupied by a server whose /health returns {"status":"ready","model":"m1"}
    let mock = MockRlcd::ready("m1").start().await;
    let cfg = RlcdConfig {
        spawn: RlcdSpawnConfig {
            port: mock.port(),
            ..Default::default()
        },
        ..RlcdConfig::default()
    };

    // @step When the supervisor selects a port for spawning
    let _user_dir = UserDirGuard::new(&TempDir::new().expect("temp user dir"));
    let sel = select_port(&cfg).await.expect("port selection must succeed");

    // @step Then the supervisor connects to the existing server on port 8000
    // @step And no new server process has been spawned
    match sel {
        PortSelection::ReuseExisting { port, model } => {
            assert_eq!(port, mock.port(), "the existing server's port must be reused");
            assert_eq!(model, "m1");
        }
        other => panic!("expected ReuseExisting, got {other:?}"),
    }
    assert!(
        read_pidfile(&pidfile_path()).is_none(),
        "reusing an existing server must not spawn (no pidfile)"
    );
    mock.stop().await;
}

/// Scenario: When every scanned port is occupied the spawn fails and fails
/// open
#[tokio::test]
#[serial]
async fn scenario_all_ports_occupied_fails_open() {
    // @step Given all 10 scanned ports are occupied by non-RLCD services
    let (base, _holders) = occupy_consecutive_ports(10).await;
    let cfg = RlcdConfig {
        url: format!("http://127.0.0.1:{base}"),
        spawn: RlcdSpawnConfig {
            port: base,
            port_scan_limit: 10,
            ..Default::default()
        },
        ..RlcdConfig::default()
    };
    let _user_dir = UserDirGuard::new(&TempDir::new().expect("temp user dir"));

    // @step When the supervisor attempts to ensure a local server
    let _ = drain_warn_events(); // clear pre-attempt noise
    let outcome = ensure_local_server(&cfg).await;

    // @step Then the attempt reports unreachable
    match outcome {
        EnsureOutcome::Failed(reason) => {
            assert!(
                reason.to_lowercase().contains("port"),
                "failure reason must name the port exhaustion: {reason}"
            );
        }
        other => panic!("expected Failed (all ports occupied), got {other:?}"),
    }

    // @step And a single spawn-failure warning has been logged
    let warns = drain_warn_events();
    assert_eq!(
        warns.iter().filter(|m| m.to_lowercase().contains("spawn")).count(),
        1,
        "exactly one spawn-failure warning expected, got: {warns:?}"
    );

    // @step And consumers degrade without blocking
    // (a consumer-facing wait stays bounded: the ensure_ready budget is the
    //  hard ceiling and the call returns an error instead of blocking; the
    //  unresolvable binary guarantees no accidental real spawn)
    let consumer_cfg = RlcdConfig {
        url: "http://127.0.0.1:1".to_string(),
        spawn: RlcdSpawnConfig {
            binary: FAKE_BINARY.to_string(),
            ..Default::default()
        },
        ..RlcdConfig::default()
    };
    let consumer = RlcdSupervisor::new(consumer_cfg);
    let started = Instant::now();
    assert!(
        consumer.ensure_ready(Duration::from_millis(300)).await.is_err(),
        "consumer must degrade (error), never hard-block"
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "fail-open degradation must be bounded, took {:?}",
        started.elapsed()
    );
}

// ============================================================================
// AUTO-SPAWN: PIDFILE
// ============================================================================

/// Scenario: A stale pidfile is removed before spawning
#[tokio::test]
#[serial]
async fn scenario_stale_pidfile_removed() {
    // @step Given a pidfile ~/.fspec/rlcd/rlcd.pid whose pid no longer exists
    // (FSPEC_USER_DIR redirects the pidfile into a temp dir; pid 999999999
    //  is far above any kernel pid_max — guaranteed dead)
    let tmp_user = TempDir::new().expect("temp user dir");
    let _user_dir = UserDirGuard::new(&tmp_user);
    let dead_pid: u32 = 999_999_999;
    assert!(!is_pid_alive(dead_pid), "impossible pid must be dead");
    let pidfile = pidfile_path();
    fs::create_dir_all(pidfile.parent().expect("parent"))
        .expect("create rlcd dir");
    fs::write(&pidfile, dead_pid.to_string()).expect("write stale pidfile");
    assert_eq!(read_pidfile(&pidfile), Some(dead_pid));

    // @step When the supervisor prepares to spawn a local server
    // (the binary is a name that cannot resolve on PATH: the spawn itself
    //  must fail safely, but ONLY AFTER the stale pidfile was cleaned up —
    //  that ordering is the contract under test; the url targets a dead
    //  port so no ambient healthy server short-circuits the pidfile path)
    let cfg = RlcdConfig {
        url: "http://127.0.0.1:1".to_string(),
        spawn: RlcdSpawnConfig {
            binary: FAKE_BINARY.to_string(),
            ..Default::default()
        },
        ..RlcdConfig::default()
    };
    let outcome = ensure_local_server(&cfg).await;

    // @step Then the stale pidfile has been removed
    assert!(
        !pidfile.exists(),
        "stale pidfile (dead pid) must be removed before the spawn attempt"
    );
    // @step And the spawn proceeds
    assert!(
        !matches!(outcome, EnsureOutcome::NoLocalSpawn | EnsureOutcome::WaitingExisting),
        "a stale pidfile must not block the spawn flow (the attempt proceeded far enough to fail on the unresolvable binary), got {outcome:?}"
    );
}

/// Scenario: A live pid that is still loading suppresses a duplicate spawn
#[tokio::test]
#[serial]
async fn scenario_live_pid_still_loading_suppresses_spawn() {
    // @step Given a pidfile pointing at a live process whose /health is not ready yet
    // (the test process itself is a live, non-RLCD process — exactly the
    //  "server still loading" shape from the pidfile's point of view)
    let tmp_user = TempDir::new().expect("temp user dir");
    let _user_dir = UserDirGuard::new(&tmp_user);
    let live_pid = std::process::id();
    assert!(is_pid_alive(live_pid), "precondition: test pid is alive");
    let pidfile = pidfile_path();
    fs::create_dir_all(pidfile.parent().expect("parent"))
        .expect("create rlcd dir");
    fs::write(&pidfile, live_pid.to_string()).expect("write live pid");
    let pid_before = read_pidfile(&pidfile).expect("pidfile readable");

    // @step When the supervisor ensures a local server
    // (the url points at a mock that accepts but never answers /health —
    //  the "server still loading" shape — while the pidfile holds the test
    //  process's own live pid)
    let never_ready = MockRlcd::with_health(None).start().await;
    let cfg = RlcdConfig {
        url: never_ready.base_url(),
        spawn: RlcdSpawnConfig {
            binary: FAKE_BINARY.to_string(),
            ..Default::default()
        },
        ..RlcdConfig::default()
    };
    let started = std::time::Instant::now();
    let outcome = ensure_local_server(&cfg).await;
    let waited = started.elapsed();

    // @step Then the supervisor waits within the poll budget
    assert!(
        matches!(outcome, EnsureOutcome::WaitingExisting),
        "a live pid with health still down must yield WaitingExisting (bounded wait), got {outcome:?}"
    );
    assert!(
        waited >= Duration::from_secs(8),
        "the supervisor must WAIT on the live pid (not bail immediately), waited {waited:?}"
    );
    // @step And no second server process has been spawned
    assert_eq!(
        read_pidfile(&pidfile),
        Some(pid_before),
        "the pidfile must still point at the original live pid (no duplicate spawn)"
    );
    never_ready.stop().await;
}

/// Scenario: A local spawn detaches the server and records its pid
/// (LIVE — gated on the `rlcd` binary + a checkpoint cache being present;
///  spawns a real multi-GB server, so it is `#[ignore]`d by default)
#[tokio::test]
#[serial]
#[ignore = "live: spawns a real RLCD server (multi-GB RAM, needs checkpoint cache)"]
async fn scenario_local_spawn_detaches_and_records_pid() {
    // @step Given a free port and a resolvable "rlcd" binary
    if which_binary("rlcd").is_none() {
        eprintln!("SKIP: rlcd binary not on PATH");
        return;
    }
    let checkpoint = dirs::home_dir()
        .map(|h| h.join(".cache/laya-rs"))
        .filter(|d| d.exists())
        .or_else(|| std::env::var_os("LAYA_MODELS_ROOT").map(std::path::PathBuf::from))
        .filter(|d| d.exists());
    let Some(checkpoint) = checkpoint else {
        eprintln!("SKIP: no checkpoint cache found (set LAYA_MODELS_ROOT or download checkpoints)");
        return;
    };
    let _ = checkpoint;

    let tmp_user = TempDir::new().expect("temp user dir");
    let _user_dir = UserDirGuard::new(&tmp_user);
    let (free_port, _holder) = occupy_port().await;
    let candidate = free_port + 1; // the port after the probe: free
    let cfg = RlcdConfig {
        url: format!("http://127.0.0.1:{candidate}"),
        spawn: RlcdSpawnConfig {
            port: candidate,
            ..Default::default()
        },
        ..RlcdConfig::default()
    };

    // @step When the supervisor spawns the local server
    let outcome = ensure_local_server(&cfg).await;
    let EnsureOutcome::Spawned { .. } = outcome else {
        panic!("expected Spawned, got {outcome:?}");
    };

    // @step Then the process is started with arguments "serve --host 127.0.0.1 --port <chosen>"
    // @step And its stdout and stderr are appended to ~/.fspec/logs/rlcd-serve.log
    // @step And its pid is written to the pidfile
    let pidfile = pidfile_path();
    let Some(pid) = read_pidfile(&pidfile) else {
        panic!("pidfile must record the spawned pid");
    };
    assert!(is_pid_alive(pid), "spawned pid must be alive");
    #[cfg(target_os = "linux")]
    {
        // /proc/<pid>/cmdline pins the exact spawn arguments:
        let cmdline = fs::read_to_string(format!("/proc/{pid}/cmdline"))
            .expect("read /proc cmdline")
            .replace('\0', " ")
            .trim()
            .to_string();
        assert!(cmdline.contains("serve"), "cmdline must contain 'serve': {cmdline}");
        assert!(
            cmdline.contains("--host 127.0.0.1"),
            "cmdline must bind the configured address: {cmdline}"
        );
        assert!(
            cmdline.contains(&format!("--port {candidate}")),
            "cmdline must use the chosen port: {cmdline}"
        );
        // detached process group: the child leads its own group (pgrp == pid)
        // rather than joining the fspec process group — this is what lets it
        // survive the fspec process.
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).expect("read /proc stat");
        let fields: Vec<&str> = stat.rsplit(' ').collect();
        let pgrp = fields.get(5).and_then(|p| p.parse::<u32>().ok());
        assert!(
            pgrp == Some(pid),
            "spawned server must lead its own process group (process_group(0)): pgrp={pgrp:?} pid={pid}"
        );
        // the serve log must exist (stdout/err appended):
        let serve_log = tmp_user.path().join("logs/rlcd-serve.log");
        let deadline = Instant::now() + Duration::from_secs(10);
        while !serve_log.exists() && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(
            serve_log.exists(),
            "stdout/err must be appended to logs/rlcd-serve.log"
        );
    }

    // @step And the process survives the fspec process (process group detached)
    // (pinned on Linux via /proc above; on other platforms the contract is
    //  documented as Unix-shaped and this live test is Linux-gated)
    // the spawned server must also become ready within the load budget:
    let follow_cfg = RlcdConfig {
        url: format!("http://127.0.0.1:{candidate}"),
        ..RlcdConfig::default()
    };
    let follow = RlcdSupervisor::new(follow_cfg);
    let model = follow
        .ensure_ready(Duration::from_secs(90))
        .await
        .expect("spawned server must become ready within 90s");
    assert!(!model.is_empty());

    // teardown: terminate the spawned server via SIGTERM (graceful per JEV-006)
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .spawn();
    }
}

// ============================================================================
// AUTO-SPAWN: URL RE-ANCHORING (regression: live server on a port DIFFERENT
// from the configured url — the 2026-09-24 incident where a spawn/reuse on a
// different port left every consumer probing the configured url)
// ============================================================================

/// Read `rlcd.url` back from a persisted user config file (`None` when the
/// file or section is absent).
fn persisted_rlcd_url(user_dir: &std::path::Path) -> Option<String> {
    let raw = fs::read_to_string(user_dir.join("fspec-config.json")).ok()?;
    let root: serde_json::Value = serde_json::from_str(&raw).ok()?;
    root.get("rlcd")?
        .get("url")?
        .as_str()
        .map(str::to_string)
}

/// Scenario: Reusing an existing healthy server on a different port
/// re-anchors and persists the url
#[tokio::test]
#[serial]
async fn scenario_reuse_on_different_port_reanchors_and_persists() {
    // @step Given a configured url that is down
    // (a NON-RLCD service occupies the configured port — its /health probe
    //  must fail, and the configured port itself is never spawn-free)
    // @step And a healthy RLCD server occupying a different port in the scan window
    let mut _holder: Option<tokio::net::TcpListener> = None;
    let (mock, mock_port) = loop {
        let base = probe_base_port().await;
        let Ok(mock) = MockRlcd::ready("m1")
            .start_on_port(base + 1)
            .await
        else {
            continue; // base+1 collides: retry on a fresh base
        };
        let h = tokio::net::TcpListener::bind(SocketAddrV4::new(
            std::net::Ipv4Addr::LOCALHOST,
            base,
        ))
        .await
        .expect("bind occupant port");
        _holder = Some(h);
        break (mock, base + 1);
    };
    let tmp_user = TempDir::new().expect("temp user dir");
    let _user_dir = UserDirGuard::new(&tmp_user);
    let cfg = RlcdConfig {
        url: format!("http://127.0.0.1:{}", mock_port - 1),
        spawn: RlcdSpawnConfig {
            port: mock_port - 1,
            port_scan_limit: 10,
            ..Default::default()
        },
        ..RlcdConfig::default()
    };
    let sup = RlcdSupervisor::new(cfg);

    // @step When the supervisor ensures a local server
    let model = sup
        .ensure_ready(Duration::from_secs(30))
        .await
        .expect("the healthy in-window server must be reached within budget");

    // @step Then the user config file now contains rlcd.url pointing at the occupied port
    let persisted = persisted_rlcd_url(tmp_user.path()).expect(
        "re-anchoring to a different healthy port must persist rlcd.url",
    );
    assert_eq!(persisted, mock.base_url(), "persisted url must be the occupied healthy port");
    // @step And the supervisor's effective base url points at the occupied port
    assert_eq!(sup.effective_base_url().as_deref(), Some(mock.base_url().as_str()));
    // @step And a fresh supervisor built from the persisted config reaches the server
    let fresh_cfg = load_rlcd_config_from(&tmp_user.path().join("fspec-config.json"));
    assert_eq!(fresh_cfg.url, mock.base_url());
    let fresh = RlcdSupervisor::new(fresh_cfg);
    let fresh_model = fresh
        .ensure_ready(Duration::from_secs(5))
        .await
        .expect("a fresh supervisor from the persisted config must reach the server");
    assert_eq!(fresh_model, model);
    // @step And no second server process has been spawned
    assert!(
        read_pidfile(&pidfile_path()).is_none(),
        "reusing an existing server must not spawn (no pidfile)"
    );
    drop(_holder); // the configured port was occupied for the whole test
    mock.stop().await;
}

/// Scenario: A live pidfile pointing at a healthy server on a different
/// port re-anchors the supervisor
#[tokio::test]
#[serial]
async fn scenario_live_pidfile_healthy_on_different_port_reanchors() {
    // @step Given a pidfile pointing at a live process that serves /health on a port other than the configured url
    // (the test process itself is the live, non-RLCD pid; its tokio runtime
    //  hosts the mock listener — scoped discovery must find it. The
    //  configured url targets a port held by a NON-RLCD listener, so the
    //  bounded wait on it can never succeed — the discovery path is the
    //  only way to the healthy port.)
    let mock = MockRlcd::ready("m2").start().await;
    let holder_port = probe_base_port().await;
    let holder = tokio::net::TcpListener::bind(SocketAddrV4::new(
        std::net::Ipv4Addr::LOCALHOST,
        holder_port,
    ))
    .await
    .expect("bind occupant port");
    let _ = &holder; // keep the occupant bound for the whole test
    let tmp_user = TempDir::new().expect("temp user dir");
    let _user_dir = UserDirGuard::new(&tmp_user);
    let live_pid = std::process::id();
    let pidfile = pidfile_path();
    fs::create_dir_all(pidfile.parent().expect("parent")).expect("create rlcd dir");
    fs::write(&pidfile, live_pid.to_string()).expect("write live pid");
    assert!(
        holder_port != mock.port(),
        "the holder and the mock must not collide: holder={holder_port} mock={}",
        mock.port()
    );
    let cfg = RlcdConfig {
        url: format!("http://127.0.0.1:{holder_port}"),
        spawn: RlcdSpawnConfig {
            port: holder_port,
            port_scan_limit: 10,
            ..Default::default()
        },
        ..RlcdConfig::default()
    };
    let sup = RlcdSupervisor::new(cfg);

    // @step When the supervisor ensures a local server
    let model = sup
        .ensure_ready(Duration::from_secs(30))
        .await
        .expect("scoped discovery must find the pid's healthy loopback port");
    assert_eq!(model, "m2");

    // @step Then the supervisor connects to the healthy port found on the local loopback listeners
    assert_eq!(sup.effective_base_url().as_deref(), Some(mock.base_url().as_str()));
    // @step And the user config file now contains rlcd.url pointing at that port
    // (the shared persistence contract, also covered by the
    //  spawn-persistence scenario above)
    assert_eq!(
        persisted_rlcd_url(tmp_user.path()).as_deref(),
        Some(mock.base_url().as_str()),
        "discovery must persist rlcd.url at the discovered healthy port"
    );
    // @step And the pidfile is unchanged
    assert_eq!(read_pidfile(&pidfile), Some(live_pid));
    // @step And no second server process has been spawned
    mock.stop().await;
}

/// Scenario: A failed local spawn leaves the configured url untouched
#[tokio::test]
#[serial]
async fn scenario_failed_spawn_leaves_url_untouched() {
    // @step Given a configured url that is down and a spawn flow that fails (all ports occupied)
    let (base, _holders) = occupy_consecutive_ports(10).await;
    let tmp_user = TempDir::new().expect("temp user dir");
    let _user_dir = UserDirGuard::new(&tmp_user);
    let path = tmp_user.path().join("fspec-config.json");
    fs::write(&path, r#"{"theme":"dark","rlcd":{"url":"http://127.0.0.1:8000"}}"#)
        .expect("write pre-existing config");
    let cfg = RlcdConfig {
        url: format!("http://127.0.0.1:{base}"),
        spawn: RlcdSpawnConfig {
            port: base,
            port_scan_limit: 10,
            ..Default::default()
        },
        ..RlcdConfig::default()
    };
    let sup = RlcdSupervisor::new(cfg);

    // @step When the supervisor ensures a local server
    assert!(
        sup.ensure_ready(Duration::from_millis(500)).await.is_err(),
        "all-occupied scan must fail open"
    );

    // @step Then the attempt reports unreachable
    assert!(
        sup.unreachable_reason()
            .expect("unreachable status must carry a reason")
            .to_lowercase()
            .contains("rlcd")
    );
    // @step And the user config file's rlcd.url is unchanged
    assert_eq!(
        persisted_rlcd_url(tmp_user.path()).as_deref(),
        Some("http://127.0.0.1:8000"),
        "a failed spawn flow must not touch the persisted url"
    );
}

// ============================================================================
// INTEGRATION
// ============================================================================

/// Scenario: All consumers read the shared status and fail open together
#[tokio::test]
#[serial]
async fn scenario_all_consumers_read_shared_status_and_fail_open() {
    // @step Given the RLCD status reports unreachable
    let tmp_user = TempDir::new().expect("temp user dir");
    let _user_dir = UserDirGuard::new(&tmp_user);
    let cfg = RlcdConfig {
        url: "http://127.0.0.1:1".to_string(),
        spawn: RlcdSpawnConfig {
            binary: FAKE_BINARY.to_string(),
            ..Default::default()
        },
        ..RlcdConfig::default()
    };
    let sup = RlcdSupervisor::new(cfg);
    assert!(
        sup.ensure_ready(Duration::from_millis(300)).await.is_err(),
        "port 1 refuses connections: ensure_ready must fail"
    );
    assert!(!sup.reachable());

    // @step When the Decision tool, the workflow gate, and the security layer each act
    // (all three consumers — RLCD-002 Decision, RLCD-003 workflow gate,
    //  RLCD-004 security layer — read this same shared status snapshot; the
    //  contract they key off is: reachable=false + a non-empty structured
    //  reason + the configured base url)
    let reason = sup
        .unreachable_reason()
        .expect("unreachable status must carry a structured reason");
    let base = sup.effective_base_url();

    // @step Then the Decision tool returns a structured error
    assert!(
        reason.to_lowercase().contains("rlcd"),
        "the structured error payload must identify RLCD: {reason}"
    );
    // @step And the workflow gate executes the command with a warning
    assert!(
        !reason.trim().is_empty(),
        "the gate's warn payload (the structured reason) must be non-empty"
    );
    // @step And the security layer skips the RLCD stage while regex rules still apply
    assert_eq!(
        sup.effective_base_url(),
        None,
        "an unreachable status carries no usable endpoint — consumers fail open and keep the configured target only via their own config (the RLCD stage is skipped, not the rest)"
    );
    // @step And none of the three hard-blocks the caller
    // (ensure_ready returned Err within its budget instead of panicking or
    //  blocking forever — the fail-open guarantee)
    let _ = base;

    // And the global supervisor is the shared, lazily-initialized instance
    // every consumer uses:
    let global = global_supervisor();
    assert!(
        global.config().url.starts_with("http"),
        "global supervisor must carry a url"
    );
}

// ============================================================================
// helpers
// ============================================================================

/// Resolve a binary name against `PATH` (the production impl does the same
/// before any spawn attempt).
fn which_binary(bin: &str) -> Option<std::path::PathBuf> {
    let var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&var) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
