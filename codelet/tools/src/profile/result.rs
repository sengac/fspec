//! ProfileResult — the shape returned by `AgentManager profile` action
//!
//! Feature: spec/features/agent-manager-profile-action.feature
//! Feature: spec/features/agent-manager-profile-diagnostics.feature (AMGR-018)
//!
//! Every field here is part of the public tool-call contract. Changing any field is a
//! breaking change for AI agent callers and must be reflected in the TypeScript binding
//! in `src/tools/agentManager.ts` and the JSON schema in `src/tools/agentManagerSchema.ts`.

use serde::{Deserialize, Serialize};

/// Top-level result returned by `ProfileSession::run()`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfileResult {
    /// Actual duration in seconds the window was active
    pub duration_secs: u32,
    /// ISO8601 UTC timestamp when the window started
    pub started_at: String,
    /// ISO8601 UTC timestamp when the window closed
    pub ended_at: String,
    /// Process-level metrics captured at window boundaries
    pub process: ProcessReport,
    /// Tokio runtime metrics (degraded to nulls if tokio_unstable is off)
    pub runtime: RuntimeReport,
    /// Scopes sorted by call_count descending, capped at `top_n`
    pub scopes_by_calls: Vec<ScopeReport>,
    /// Scopes sorted by total_self_ms descending, capped at `top_n`
    pub scopes_by_self_ms: Vec<ScopeReport>,
    /// Instrumented channel reports (from TrackedBroadcast/Mpsc/UnboundedMpsc wrappers)
    pub channels: Vec<ChannelReport>,
    /// AMGR-018: samples broken down by OS thread, sorted by sample_count desc.
    #[serde(default)]
    pub samples_by_thread: Vec<ThreadSampleReport>,
    /// AMGR-018: top-N unique call stacks by sample_count.
    #[serde(default)]
    pub hot_stacks: Vec<StackReport>,
    /// AMGR-018: sampling-quality report (debug info presence, CPU cores, etc.)
    #[serde(default)]
    pub sampling: SamplingReport,
}

/// OS-level process metrics captured at window start and end
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessReport {
    /// Operating system PID
    pub pid: u32,
    /// Resident set size in bytes at window start
    pub rss_bytes_start: u64,
    /// Resident set size in bytes at window end
    pub rss_bytes_end: u64,
    /// Total OS threads in the process at window start
    pub total_threads_start: u32,
    /// Total OS threads in the process at window end
    pub total_threads_end: u32,
}

/// Tokio runtime metrics (requires `tokio_unstable` cfg flag)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeReport {
    /// Configured number of worker threads in the runtime
    pub worker_threads: u32,
    /// Alive tokio tasks at window start (None if tokio_unstable is off)
    pub alive_tokio_tasks_start: Option<u64>,
    /// Alive tokio tasks at window end (None if tokio_unstable is off)
    pub alive_tokio_tasks_end: Option<u64>,
}

/// Per-scope metrics aggregated over the profile window
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScopeReport {
    /// Fully-qualified scope label: `concat!(module_path!(), "::", user_label)`
    pub label: String,
    /// Number of times `ProfileScope::enter` was called during the window
    pub call_count: u64,
    /// Sum of self-time in milliseconds (excludes time in nested scopes)
    pub total_self_ms: f64,
    /// Largest single-iteration self-time observed in milliseconds
    pub max_iter_ms: f64,
    /// Derived rate: call_count / duration_secs
    pub calls_per_sec: f64,
    /// Number of guards still in flight when the window closed
    pub currently_executing_at_end: i32,
}

/// Per-channel metrics from `TrackedBroadcast`/`TrackedMpsc`/`TrackedUnboundedMpsc` wrappers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChannelReport {
    /// Stable channel name assigned at construction (e.g. `supervisor_broadcast_<uuid>`)
    pub name: String,
    /// Number of senders alive at window end
    pub sender_count: u32,
    /// Number of receivers subscribed at window end
    pub receiver_count: u32,
    /// Items buffered in the channel at window end (len snapshot)
    pub queued_at_end: u64,
    /// Number of `RecvError::Lagged(n)` events observed across all receivers during the window
    pub lagged_during_window: u64,
}

/// AMGR-018: samples broken down by OS thread.
///
/// One entry per distinct `(thread_name, thread_id)` tuple that produced at
/// least one sample inside the profile window. Sorted by `sample_count`
/// descending so `samples_by_thread[0]` identifies the hottest thread.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ThreadSampleReport {
    /// Thread name (e.g. `"tokio-runtime-worker"`, `"main"`).
    pub thread_name: String,
    /// Opaque OS thread id reported by `pthread_getthreadid_np` / `gettid`.
    pub thread_id: u64,
    /// Number of stack samples attributed to this thread during the window.
    pub sample_count: u64,
    /// Derived: `sample_count * (1000 / sample_freq_hz)` — approximate CPU ms
    /// this thread burned during the window. A fully saturated thread at
    /// 250 Hz sampling over 10 s yields ~10_000 ms.
    pub cpu_ms: u64,
}

/// AMGR-018: one entry of `ProfileResult::hot_stacks`.
///
/// Represents a unique call chain (keyed by the first
/// `attribution::HOT_STACK_MAX_FRAMES` meaningful symbols in the stack)
/// together with its accumulated sample count and the thread that produced
/// the hottest sample on that chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct StackReport {
    /// The resolved frames of the hot call chain, leaf-first, capped at
    /// `HOT_STACK_MAX_FRAMES`.
    pub frames: Vec<StackFrameInfo>,
    /// Name of the OS thread that produced the hottest sample on this stack.
    pub thread_name: String,
    /// Total number of samples attributed to this unique call chain.
    pub sample_count: u64,
}

/// AMGR-018: a single resolved stack frame in a `StackReport`.
///
/// When the binary is stripped or lacks DWARF info, `file` and `line` are
/// `None` and `symbol` may be the nearest exported C symbol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
pub struct StackFrameInfo {
    /// Demangled symbol name.
    pub symbol: String,
    /// Source file path if resolvable from DWARF info.
    pub file: Option<String>,
    /// Source line number if resolvable from DWARF info.
    pub line: Option<u32>,
}

/// AMGR-018: sampling-quality report.
///
/// Lets callers detect stripped binaries (where >90% of samples are
/// unresolved) and compute an intuitive CPU-usage figure without parsing
/// individual sample counts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SamplingReport {
    /// Total number of stack samples captured during the window.
    pub total_samples: u64,
    /// Samples whose attribution frame has `file.is_some()` AND file does not
    /// end with `.c` or `Unknown`. When `< total_samples / 10` the binary is
    /// almost certainly missing debug info.
    pub resolved_rust_samples: u64,
    /// Samples that could not be resolved to a Rust source location.
    pub unresolved_samples: u64,
    /// Derived: `total_samples * (1.0 / sample_freq_hz) / duration_secs`.
    /// For a fully CPU-saturated single thread at 250 Hz over 10 s this
    /// yields ~1.0. Two saturated threads yield ~2.0.
    pub cpu_cores_consumed: f64,
    /// `false` when `resolved_rust_samples < total_samples / 10` — indicates
    /// the binary was built with `strip = "symbols"` and the profile output
    /// is untrustworthy.
    pub debug_info_available: bool,
    /// Human-readable hint surfaced to LLM callers when
    /// `debug_info_available == false`.
    pub hint: Option<String>,
}
