//! RLCD-001 — supervisor: health polling, shared status, fail-open snapshot.
//!
//! Feature: spec/features/rlcd-service-supervisor.feature
//!
//! The supervisor owns the shared `RlcdStatus` snapshot that every RLCD
//! consumer (RLCD-002 Decision tool, RLCD-003 workflow gate, RLCD-004
//! security layer) reads. Health polling runs in the foreground on demand
//! (`ensure_ready` with a caller-supplied budget / `refresh` for a single
//! poll); the background 10s poller is wired at the process level in
//! RLCD-002 (the first real consumer) so that test suites control all
//! timing. Warnings are logged on state transitions ONLY (down->up,
//! up->down, spawn-failure) — never per-poll.
//!
//! URL re-anchoring (2026-09-24 regression): a spawn/reuse can land on a
//! port DIFFERENT from the configured url (scan-skip, or a server started
//! out-of-band with a pidfile pointing at it). A successful
//! [`crate::rlcd::discover`] hit or a fresh spawn re-anchors the status
//! (`base_url` + `model`) AND persists `rlcd.url` key-preservingly into the
//! user config, so every future fspec process — and the next consumer in
//! this one — connects to the server that is actually serving.

pub use crate::rlcd::health::health_check;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;
use tracing::{debug, info, warn};

use crate::rlcd::config::{save_rlcd_url_at, user_dir, RlcdConfig};
use crate::rlcd::discover::discover_healthy_server_for_pid;
use crate::rlcd::error::RlcdError;
use crate::rlcd::pidfile::{is_pid_alive, pidfile_path, read_pidfile, spawn_url_of};
use crate::rlcd::spawn::{ensure_local_server, EnsureOutcome};

/// Default foreground poll interval used by [`RlcdSupervisor::ensure_ready`].
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Per-request transport timeout for /health probes.
const HEALTH_TIMEOUT: Duration = Duration::from_secs(2);

/// Shared status snapshot (one per supervisor; the global supervisor's copy
/// is the consumer-facing RLCD_STATUS).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RlcdStatus {
    /// Whether the last poll saw the server ready.
    pub reachable: bool,
    /// The endpoint being tracked.
    pub base_url: String,
    /// Model name reported by the server (None when never ready).
    pub model: Option<String>,
    /// Last poll time (UTC, human-readable).
    pub last_check: String,
    /// Spawn/ensure notes from the most recent local-server attempt.
    pub spawn_note: Option<String>,
}

/// A supervisor instance: config + the shared status snapshot it maintains.
pub struct RlcdSupervisor {
    config: RlcdConfig,
    status: Mutex<RlcdStatus>,
    /// Set once a re-anchor (spawn/reuse/discovery) has persisted the
    /// discovered/spawned url into the user config — the write is
    /// idempotent (same url), so every supervisor process does it at most
    /// once per discovered endpoint.
    persisted: Mutex<bool>,
}

impl RlcdSupervisor {
    /// Create a supervisor tracking `config`.
    #[must_use]
    pub fn new(config: RlcdConfig) -> Self {
        Self {
            status: Mutex::new(RlcdStatus {
                reachable: false,
                base_url: config.url.clone(),
                model: None,
                last_check: String::new(),
                spawn_note: None,
            }),
            config,
            persisted: Mutex::new(false),
        }
    }

    /// The configured URL.
    #[must_use]
    pub fn config(&self) -> &RlcdConfig {
        &self.config
    }

    /// The effective endpoint: the configured url while it is the known
    /// target, the re-anchored url after a spawn/reuse/discovery hit.
    /// `None` before the first successful poll (the endpoint is unknown;
    /// consumers fail open and must not issue requests).
    #[must_use]
    pub fn effective_base_url(&self) -> Option<String> {
        let status = self.status.lock().ok()?;
        if status.reachable {
            Some(status.base_url.clone())
        } else {
            None
        }
    }

    /// Whether the last poll reported the server ready.
    #[must_use]
    pub fn reachable(&self) -> bool {
        self.status.lock().is_ok_and(|s| s.reachable)
    }

    /// The model name captured from the most recent ready /health response.
    #[must_use]
    pub fn model(&self) -> Option<String> {
        self.status.lock().ok().and_then(|s| s.model.clone())
    }

    /// Structured, non-empty reason for the current unreachable state
    /// (consumers embed it in their structured errors / warn logs).
    #[must_use]
    pub fn unreachable_reason(&self) -> Option<String> {
        let status = self.status.lock().ok()?;
        if status.reachable {
            return None;
        }
        Some(format!(
            "RLCD unreachable at {} — decision engine unavailable; consumers fail open (last check: {})",
            status.base_url,
            if status.last_check.is_empty() { "never" } else { &status.last_check }
        ))
    }

    /// Single /health poll; updates the status and logs the transition
    /// (if any) exactly once per transition.
    pub async fn refresh(&self) {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let Some((url, previous)) = self
            .status
            .lock()
            .ok()
            .map(|s| (s.base_url.clone(), s.reachable))
        else {
            return;
        };
        let outcome = health_check(&url, HEALTH_TIMEOUT).await;
        let guard = self.status.lock();
        let Ok(mut status) = guard else {
            return;
        };
        match outcome {
            Ok(model) => {
                if !previous {
                    info!(
                        url = %url,
                        model = %model,
                        "rlcd up: RLCD server ready — consumers may use the engine"
                    );
                }
                status.reachable = true;
                status.model = Some(model);
                status.last_check = now;
            }
            Err(e) => {
                if previous {
                    warn!(
                        url = %url,
                        error = %e,
                        "RLCD down: health check failed — decision engine unreachable; consumers fail open (regex blocklist + stage permissions stay in force)"
                    );
                }
                status.reachable = false;
                status.last_check = now;
            }
        }
    }

    /// Re-anchor the effective endpoint to a known-healthy RLCD endpoint:
    /// update the status (`base_url` + `model`) and persist `rlcd.url` into
    /// the user config (key-preserving, at most once) so the NEXT fspec
    /// process connects to the server that is actually serving. The
    /// configured url is never overwritten with itself; a failed persist
    /// still re-anchors the in-process status (fail-open).
    async fn reanchor(&self, url: &str, model: &str) {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        {
            let guard = self.status.lock();
            if let Ok(mut status) = guard {
                if !status.reachable {
                    info!(url = %url, model = %model, "rlcd: re-anchored to a healthy endpoint");
                }
                status.reachable = true;
                status.base_url = url.to_string();
                status.model = Some(model.to_string());
                status.last_check = now;
            }
        }
        if url == self.config.url {
            return;
        }
        let mut persisted = self
            .persisted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if *persisted {
            return;
        }
        let Some(dir) = user_dir() else {
            debug!("rlcd: no user dir resolvable — url persist skipped");
            return;
        };
        match save_rlcd_url_at(&dir.join("fspec-config.json"), url) {
            Ok(()) => {
                *persisted = true;
            }
            Err(e) => warn!(url = %url, error = %e, "rlcd: url persist failed (in-memory re-anchor stands)"),
        }
    }

    /// The local-server ensure flow plus re-anchoring (2026-09-24
    /// regression): after `ensure_local_server`, re-anchor when
    /// (a) a fresh spawn/reuse landed on a port other than the configured
    /// url, or (b) the pidfile's live pid turns out to be serving a healthy
    /// loopback port outside the configured url (scoped discovery — never
    /// spawns, never re-anchors on a still-loading server).
    async fn ensure_local_reanchored(&self) -> Option<EnsureOutcome> {
        let outcome = ensure_local_server(&self.config).await;
        let configured = self.config.url.clone();
        match &outcome {
            EnsureOutcome::Spawned { port, .. } => {
                let url = spawn_url_of(&self.config.spawn.bind_address, *port);
                if *port != crate::rlcd::pidfile::url_port(&configured) {
                    if let Ok(model) = health_check(&url, HEALTH_TIMEOUT).await {
                        self.reanchor(&url, &model).await;
                    }
                }
            }
            EnsureOutcome::WaitingExisting => {
                let pidfile = pidfile_path();
                if let Some(pid) = read_pidfile(&pidfile).filter(|p| is_pid_alive(*p)) {
                    if let Some((port, model)) = discover_healthy_server_for_pid(pid).await {
                        let url = spawn_url_of(&self.config.spawn.bind_address, port);
                        if url != configured {
                            self.reanchor(&url, &model).await;
                        }
                    }
                }
            }
            _ => {}
        }
        Some(outcome)
    }

    /// Bounded foreground wait until the server is ready (or the budget
    /// elapses). For local URLs, a down first poll triggers the
    /// auto-spawn flow (bounded; fail-open). Returns the model name on
    /// success.
    pub async fn ensure_ready(&self, budget: Duration) -> Result<String, RlcdError> {
        if !self.config.enabled {
            return Err(RlcdError::Unreachable {
                url: self.config.url.clone(),
                detail: "rlcd disabled in user config".to_string(),
            });
        }
        let deadline = Instant::now() + budget;
        let mut interval = tokio::time::interval(DEFAULT_POLL_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            // First poll: if down and the URL is local, attempt the
            // auto-spawn flow ONCE (it is bounded internally), including
            // the re-anchor to where the server actually serves.
            if !self.reachable() && crate::rlcd::pidfile::is_local_url(&self.config.url) {
                if let Some(outcome) = self.ensure_local_reanchored().await {
                    if let Ok(mut guard) = self.status.lock() {
                        guard.spawn_note = Some(format!("{outcome:?}"));
                    }
                }
            }
            self.refresh().await;
            if self.reachable() {
                if let Some(model) = self.model() {
                    return Ok(model);
                }
            }
            if Instant::now() >= deadline {
                return Err(RlcdError::Unreachable {
                    url: self.config.url.clone(),
                    detail: format!(
                        "still unreachable after {} (consumers fail open)",
                        budget.as_millis()
                    ),
                });
            }
            interval.tick().await;
        }
    }
}

/// The shared process-level supervisor (RLCD_STATUS): lazily initialized
/// from the user config; every consumer reads its snapshot instead of
/// re-polling.
static GLOBAL_SUPERVISOR: Lazy<RlcdSupervisor> =
    Lazy::new(|| RlcdSupervisor::new(crate::rlcd::config::load_rlcd_config()));

/// The shared, lazily-initialized global supervisor.
#[must_use]
pub fn global_supervisor() -> &'static RlcdSupervisor {
    &GLOBAL_SUPERVISOR
}
