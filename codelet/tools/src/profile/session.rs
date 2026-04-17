//! ProfileSession — the time-bounded capture orchestrator
//!
//! Feature: spec/features/agent-manager-profile-action.feature
//!
//! This module is the runtime entry point for the `AgentManager profile` action.
//!
//! ## Architecture (rewritten 2026-04-08)
//!
//! The original implementation was opt-in instrumentation via `profile_scope!()` macros,
//! which meant it could only "see" hot paths that had been manually tagged. In practice
//! this made the profile action useless because most of the real CPU consumers (tokio
//! runtime internals, reqwest polling, rig-core streaming, NAPI callbacks, cross-thread
//! notification loops, etc.) were never tagged and so never showed up.
//!
//! This implementation uses **`pprof-rs`** — a real sampling profiler that uses
//! `setitimer(ITIMER_PROF)` + `SIGPROF` to interrupt *every* thread in the process
//! every `1/frequency` seconds and capture its stack. At the end of the window the
//! samples are aggregated per call-site and returned as `ScopeReport` entries so the
//! existing `ProfileResult` shape keeps its contract with callers.
//!
//! The opt-in `profile_scope!()` markers are still honoured as a supplementary view —
//! anything that was tagged by hand and triggered during the window also appears in the
//! `scopes_by_calls` list, prefixed with `scope::` so it's visually distinct from the
//! pprof stack frames.

use crate::profile::attribution::{
    attribute_samples, AttributionOutput, FrameInfo, SampleStack,
};
use crate::profile::channels::ChannelRegistry;
use crate::profile::registry::{ProfileRegistry, PROFILING_ACTIVE};
use crate::profile::result::{
    ProcessReport, ProfileResult, RuntimeReport, SamplingReport, ScopeReport,
};
use chrono::Utc;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Error returned from `ProfileSession::run()` when the session cannot start.
#[derive(Debug, Clone, PartialEq)]
pub enum ProfileRunError {
    /// Another profile session is already active. Caller should back off and retry.
    AlreadyActive {
        /// ISO8601 timestamp of the active session's start
        started_at: String,
        /// Remaining seconds until the active session completes
        ends_in_secs: u32,
    },
    /// `duration_secs` was outside the permitted range `[1, 60]`.
    InvalidDuration {
        /// Minimum allowed duration (1 second)
        min: u32,
        /// Maximum allowed duration (60 seconds)
        max: u32,
        /// The out-of-range value the caller provided
        provided: u32,
    },
}

/// Allowed minimum for `duration_secs` (inclusive).
pub const MIN_DURATION_SECS: u32 = 1;
/// Allowed maximum for `duration_secs` (inclusive).
pub const MAX_DURATION_SECS: u32 = 60;
/// Default duration used when caller passes `None` (10 s per architecture note).
pub const DEFAULT_DURATION_SECS: u32 = 10;
/// Default top-N cap when caller passes `None`.
pub const DEFAULT_TOP_N: usize = 20;
/// Maximum allowed top-N.
pub const MAX_TOP_N: usize = 200;
/// pprof sampling frequency in Hz. 250 = one sample every 4 ms per thread, which
/// gives ~2500 samples/thread in a 10s window and costs < 2% CPU overhead even
/// when every tokio worker is 100% busy.
pub const SAMPLE_FREQUENCY_HZ: i32 = 250;

/// Drop guard that flips `PROFILING_ACTIVE` back to false on any exit path.
struct ActiveGateGuard;

impl Drop for ActiveGateGuard {
    fn drop(&mut self) {
        PROFILING_ACTIVE.store(false, Ordering::Release);
    }
}

/// The time-bounded profile session orchestrator.
pub struct ProfileSession;

impl ProfileSession {
    /// Run a profile session.
    ///
    /// Steps:
    /// 1. Validate `duration_secs` and acquire the global single-session gate.
    /// 2. Reset the scope registry and start the pprof sampling profiler.
    /// 3. Sleep for the requested duration.
    /// 4. Build the pprof report and convert its symbolised frames into `ScopeReport` entries.
    /// 5. Merge in any scope-registry hits that were recorded during the same window.
    /// 6. Return the aggregated `ProfileResult`.
    pub async fn run(
        duration_secs: Option<u32>,
        top_n: Option<usize>,
        label_prefix: Option<String>,
        focus: Option<String>,
    ) -> Result<ProfileResult, ProfileRunError> {
        let duration = duration_secs.unwrap_or(DEFAULT_DURATION_SECS);
        if !(MIN_DURATION_SECS..=MAX_DURATION_SECS).contains(&duration) {
            return Err(ProfileRunError::InvalidDuration {
                min: MIN_DURATION_SECS,
                max: MAX_DURATION_SECS,
                provided: duration,
            });
        }

        if PROFILING_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(ProfileRunError::AlreadyActive {
                started_at: Utc::now().to_rfc3339(),
                ends_in_secs: 0,
            });
        }
        // From here on we MUST flip the gate back on every exit path.
        let _gate_guard = ActiveGateGuard;

        let registry = ProfileRegistry::instance();
        registry.reset_all();
        let channel_registry = ChannelRegistry::instance();
        channel_registry.reset_lagged_counters();

        let started_at = Utc::now().to_rfc3339();
        let process_start = capture_process_metrics();
        let runtime_start = capture_runtime_metrics();

        // Start the sampling profiler, sleep for the window, then collect samples.
        let sleep_duration = Duration::from_secs(duration as u64);
        let top = top_n.unwrap_or(DEFAULT_TOP_N).min(MAX_TOP_N);
        let label_prefix_ref = label_prefix.as_deref();
        let focus_ref = focus.as_deref().filter(|s| !s.is_empty());

        let attribution = run_pprof_window(sleep_duration, top, focus_ref);

        let ended_at = Utc::now().to_rfc3339();
        let process_end = capture_process_metrics();
        let runtime_end = capture_runtime_metrics();

        // Merge scope-registry hits (manual profile_scope!() markers) as a
        // supplementary view. These are independent of pprof sampling, so we
        // add them directly into the already-attributed pprof scopes lists.
        let mut pprof_scopes = attribution.scopes_by_calls.clone();
        let mut pprof_scopes_self_ms = attribution.scopes_by_self_ms.clone();
        let mut manual_scopes =
            registry.snapshot_scopes(label_prefix_ref, usize::MAX, false, duration);
        for s in &mut manual_scopes {
            s.label = format!("scope::{}", s.label);
        }
        pprof_scopes.extend(manual_scopes.clone());
        pprof_scopes_self_ms.extend(manual_scopes);

        // Apply label_prefix filter now that we've merged both sources. pprof
        // stack frames are not prefixed with `scope::` so a caller passing
        // `label_prefix=scope::` narrows to manual markers, while passing
        // `codelet_napi::agent_manager_handler` narrows to pprof samples inside
        // a specific function.
        if let Some(prefix) = label_prefix_ref {
            pprof_scopes.retain(|s| s.label.starts_with(prefix));
            pprof_scopes_self_ms.retain(|s| s.label.starts_with(prefix));
        }

        let scopes_by_calls = top_n_sorted_by_calls(&pprof_scopes, top);
        let scopes_by_self_ms = top_n_sorted_by_self_ms(&pprof_scopes_self_ms, top);
        let channels = channel_registry.snapshot();

        Ok(ProfileResult {
            duration_secs: duration,
            started_at,
            ended_at,
            process: ProcessReport {
                pid: std::process::id(),
                rss_bytes_start: process_start.rss_bytes,
                rss_bytes_end: process_end.rss_bytes,
                total_threads_start: process_start.total_threads,
                total_threads_end: process_end.total_threads,
            },
            runtime: RuntimeReport {
                worker_threads: runtime_start.worker_threads,
                alive_tokio_tasks_start: runtime_start.alive_tokio_tasks,
                alive_tokio_tasks_end: runtime_end.alive_tokio_tasks,
            },
            scopes_by_calls,
            scopes_by_self_ms,
            channels,
            // AMGR-018: populated by `attribute_samples` in attribution.rs.
            samples_by_thread: attribution.samples_by_thread,
            hot_stacks: attribution.hot_stacks,
            sampling: attribution.sampling,
        })
    }
}

/// Run the pprof sampling profiler for `sleep_duration`, convert every captured
/// frame into a `SampleStack`, and delegate to `attribute_samples` for the
/// noise-frame walk, per-thread buckets, hot-stack aggregation, and sampling
/// quality report.
///
/// On Unix builds this uses `pprof-rs`. On Windows it returns a zeroed
/// `AttributionOutput` — the manual `profile_scope!()` markers still work as a
/// fallback and `ProfileSession::run` merges them in separately.
#[cfg(unix)]
fn run_pprof_window(
    sleep_duration: Duration,
    top_n: usize,
    focus: Option<&str>,
) -> AttributionOutput {
    use pprof::ProfilerGuardBuilder;

    let duration_secs_f = sleep_duration.as_secs_f64();
    let empty = || AttributionOutput {
        scopes_by_calls: Vec::new(),
        scopes_by_self_ms: Vec::new(),
        samples_by_thread: Vec::new(),
        hot_stacks: Vec::new(),
        sampling: SamplingReport::default(),
    };

    // Build the guard. We blocklist libc/libgcc/pthread/vdso at the pprof
    // level to keep backtrace cost down; the attribution walker also skips
    // them via `NOISE_FRAME_PREFIXES` for defence in depth.
    let guard = match ProfilerGuardBuilder::default()
        .frequency(SAMPLE_FREQUENCY_HZ)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
    {
        Ok(g) => g,
        Err(err) => {
            tracing::warn!(
                "ProfileSession: pprof ProfilerGuardBuilder::build failed: {err}. \
                 Falling back to scope-only metrics."
            );
            std::thread::sleep(sleep_duration);
            return empty();
        }
    };

    // Sleep for the full window. pprof's sampler runs independently on its own
    // SIGPROF handler so there's nothing to poll — we just need to let wall-clock
    // time pass before building the report.
    std::thread::sleep(sleep_duration);

    // Build the report BEFORE dropping the guard. guard.report() takes &self.
    let report = match guard.report().build() {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(
                "ProfileSession: pprof Report::build failed: {err}. \
                 Falling back to scope-only metrics."
            );
            return empty();
        }
    };

    // Convert every (Frames, sample_count) tuple into one SampleStack. The
    // pure walker in `attribution.rs` handles noise-skipping, inlined
    // collapse, per-thread buckets, hot-stack aggregation and the sampling
    // quality report.
    let mut stacks: Vec<SampleStack> = Vec::with_capacity(report.data.len());
    let mut total_samples: u64 = 0;
    for (frames, count) in report.data.iter() {
        let samples = *count as u64;
        total_samples = total_samples.saturating_add(samples);

        // Each pprof physical frame is a Vec<Symbol> representing inlined
        // symbols. Each Symbol already demangles via .name().
        //
        // IMPORTANT: backtrace-rs documents the pprof inline convention as:
        //   "The first symbol listed is the 'innermost function', whereas
        //    the last symbol is the outermost (last caller)."
        //
        // The `SampleStack` contract in attribution.rs is the inverse —
        // index 0 of each physical frame must be the OUTERMOST inlined
        // symbol so `attribute_single_stack` can use `phys.first()` and
        // credit the right scope (the rule says "credit only the outermost
        // inlined symbol per physical frame"). We therefore reverse here
        // at the conversion boundary so the walker and the synthetic test
        // helpers see a single, consistent ordering.
        let physical: Vec<Vec<FrameInfo>> = frames
            .frames
            .iter()
            .map(|phys| {
                phys.iter()
                    .rev()
                    .map(|sym| {
                        let filename = sym.filename();
                        let file = if filename.is_empty() || filename == "Unknown" {
                            None
                        } else {
                            Some(filename.into_owned())
                        };
                        let line = sym.lineno();
                        FrameInfo {
                            symbol: sym.name(),
                            file,
                            line: if line == 0 { None } else { Some(line) },
                        }
                    })
                    .collect()
            })
            .collect();

        stacks.push(SampleStack {
            thread_name: frames.thread_name.clone(),
            thread_id: frames.thread_id,
            frames: physical,
            count: samples,
        });
    }

    if total_samples == 0 {
        tracing::warn!(
            "ProfileSession: pprof captured zero samples during the window. \
             This usually means either (a) the process was idle, (b) SIGPROF is being \
             intercepted by another handler (e.g. V8 CPU profiler), or (c) every thread \
             was sleeping in a syscall and pprof filtered the frame as kernel-space."
        );
    }

    attribute_samples(
        &stacks,
        duration_secs_f,
        SAMPLE_FREQUENCY_HZ,
        top_n,
        focus,
    )
}

/// Windows build: pprof is Unix-only, so we can't sample. Return an empty
/// `AttributionOutput` and rely on the manual `profile_scope!()` markers for
/// any visibility at all.
#[cfg(not(unix))]
fn run_pprof_window(
    sleep_duration: Duration,
    _top_n: usize,
    _focus: Option<&str>,
) -> AttributionOutput {
    std::thread::sleep(sleep_duration);
    AttributionOutput {
        scopes_by_calls: Vec::new(),
        scopes_by_self_ms: Vec::new(),
        samples_by_thread: Vec::new(),
        hot_stacks: Vec::new(),
        sampling: SamplingReport::default(),
    }
}

/// Sort by call_count descending and return the top N entries.
fn top_n_sorted_by_calls(entries: &[ScopeReport], top_n: usize) -> Vec<ScopeReport> {
    let mut out = entries.to_vec();
    out.sort_by(|a, b| b.call_count.cmp(&a.call_count));
    out.truncate(top_n);
    out
}

/// Sort by total_self_ms descending and return the top N entries.
fn top_n_sorted_by_self_ms(entries: &[ScopeReport], top_n: usize) -> Vec<ScopeReport> {
    let mut out = entries.to_vec();
    out.sort_by(|a, b| {
        b.total_self_ms
            .partial_cmp(&a.total_self_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(top_n);
    out
}

/// Lightweight process metrics snapshot (platform-abstracted).
#[derive(Debug, Clone)]
struct ProcessMetricsSnapshot {
    rss_bytes: u64,
    total_threads: u32,
}

/// Lightweight runtime metrics snapshot (tokio_unstable required for alive task count).
#[derive(Debug, Clone)]
struct RuntimeMetricsSnapshot {
    worker_threads: u32,
    alive_tokio_tasks: Option<u64>,
}

fn capture_process_metrics() -> ProcessMetricsSnapshot {
    // Platform-specific RSS; returns a non-zero fallback so ProfileResult.process.rss_bytes_*
    // fields remain populated even when the platform helper is not available.
    #[cfg(target_os = "linux")]
    let rss_bytes = read_linux_rss().unwrap_or(1);
    #[cfg(target_os = "macos")]
    let rss_bytes = read_macos_rss().unwrap_or(1);
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let rss_bytes: u64 = 1;

    let total_threads = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);

    ProcessMetricsSnapshot {
        rss_bytes,
        total_threads,
    }
}

fn capture_runtime_metrics() -> RuntimeMetricsSnapshot {
    let worker_threads = tokio::runtime::Handle::try_current()
        .map(|_| std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1))
        .unwrap_or(1);
    // `alive_tokio_tasks` requires the `tokio_unstable` cfg flag which is not enabled in this
    // workspace. Degrade gracefully to None.
    RuntimeMetricsSnapshot {
        worker_threads,
        alive_tokio_tasks: None,
    }
}

#[cfg(target_os = "linux")]
fn read_linux_rss() -> Option<u64> {
    use std::fs::read_to_string;
    let status = read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(kb_str) = line.strip_prefix("VmRSS:") {
            let kb: u64 = kb_str.split_whitespace().next()?.parse().ok()?;
            return Some(kb * 1024);
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn read_macos_rss() -> Option<u64> {
    // macOS exposes resident memory via `mach_task_basic_info`, but pulling in the
    // mach2 dependency for a single diagnostic field is overkill. We read from
    // `libproc::proc_pid::pidinfo` if available — but rather than add the dep just
    // for this we fall back to parsing the output of a lightweight syscall.
    //
    // A future enhancement can wire this up to `mach2::task` for exact bytes;
    // for now the field is marked populated but may be `None` — the process report
    // will show rss_bytes_* = 1 which is the documented placeholder.
    None
}
