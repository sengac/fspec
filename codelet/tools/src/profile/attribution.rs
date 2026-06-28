//! AMGR-018 — Pure attribution layer for pprof stack samples
//!
//! Feature: spec/features/agent-manager-profile-diagnostics.feature
//!
//! This module contains the **pure** sample-aggregation logic used by
//! `ProfileSession::run_pprof_window`. It is deliberately free of any
//! pprof-rs types so scenarios in `agent-manager-profile-diagnostics.feature`
//! can be unit-tested against synthetic `SampleStack` inputs without
//! needing a real sampling profiler session.
//!
//! ## Design
//!
//! `run_pprof_window` converts every `pprof::Frames` + sample count into one
//! `SampleStack` and feeds a `Vec<SampleStack>` into `attribute_samples`,
//! which performs:
//!
//! 1. **Noise-frame filtering** — walks each stack leaf → root skipping any
//!    frame whose symbol matches `NOISE_FRAME_PREFIXES` and attributes the
//!    sample to the first non-noise frame.
//! 2. **Inlined double-count fix** — when a physical frame contains multiple
//!    inlined symbols, only the outermost is credited.
//! 3. **Focus filter** — drops any stack whose frames do not contain the
//!    caller-supplied focus substring.
//! 4. **Per-thread aggregation** — buckets samples by `thread_name/thread_id`.
//! 5. **Hot-stack aggregation** — groups full call-chains by their first 6
//!    meaningful frames and reports the top-N.
//! 6. **Sampling quality report** — counts resolved (Rust with file+line) vs
//!    unresolved samples, derives `cpu_cores_consumed`, and flips
//!    `debug_info_available` false when <10% of samples are resolved.

use std::collections::HashMap;

use crate::profile::result::{
    SamplingReport, ScopeReport, StackFrameInfo, StackReport, ThreadSampleReport,
};

/// Compile-time noise-frame blocklist.
///
/// A symbol is treated as noise if any entry here is a substring of the
/// demangled symbol name. Order is not significant. Adding a prefix is a
/// single source edit. The match uses `str::contains` for portability across
/// macOS's underscore-prefixed mangling convention.
pub const NOISE_FRAME_PREFIXES: &[&str] = &[
    "libsystem_",
    "__pthread_",
    "_pthread_",
    "__psynch_",
    "_dyld_",
    "_os_",
    "__os_unfair_",
    "_uv_",
    "_uv__",
    "libuv",
    "libgcc",
    "libc",
    "_napi_register_module_v1",
    "napi_register_module_v1",
    "napi_",
    "__tsan",
    "__asan",
];

/// The maximum number of meaningful frames kept in a `StackReport`.
pub const HOT_STACK_MAX_FRAMES: usize = 6;

/// Returns `true` if `symbol` matches any entry in `NOISE_FRAME_PREFIXES`.
#[must_use]
pub fn is_noise_frame(symbol: &str) -> bool {
    NOISE_FRAME_PREFIXES
        .iter()
        .any(|prefix| symbol.contains(prefix))
}

/// A single resolved stack frame. `file` and `line` are `None` when the
/// backtrace unwinder could not resolve DWARF info (e.g. stripped binary).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FrameInfo {
    /// Demangled symbol name. For stripped binaries this may be the nearest
    /// exported C symbol (e.g. `_napi_register_module_v1`).
    pub symbol: String,
    /// Source file path if DWARF info was present.
    pub file: Option<String>,
    /// Source line number if DWARF info was present.
    pub line: Option<u32>,
}

/// A single captured stack sample with optional multiple inlined symbols per
/// physical frame. `frames` is leaf-first (innermost at index 0). Each inner
/// `Vec<FrameInfo>` represents the inlined symbols collapsed into one physical
/// frame — the first element is the outermost inlined symbol.
#[derive(Debug, Clone)]
pub struct SampleStack {
    /// OS thread name (e.g. `"tokio-runtime-worker"`).
    pub thread_name: String,
    /// Opaque OS thread id.
    pub thread_id: u64,
    /// Leaf-first physical frames; each physical frame may expose multiple
    /// inlined symbols.
    pub frames: Vec<Vec<FrameInfo>>,
    /// How many samples landed on this exact stack signature.
    pub count: u64,
}

/// Output of the pure attribution walker. The fields line up 1:1 with the
/// additive fields on `ProfileResult`, plus the two existing `ScopeReport`
/// lists so `ProfileSession::run` keeps its historical contract.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributionOutput {
    /// Existing field: sorted by call_count desc, capped at `top_n`.
    pub scopes_by_calls: Vec<ScopeReport>,
    /// Existing field: sorted by total_self_ms desc, capped at `top_n`.
    pub scopes_by_self_ms: Vec<ScopeReport>,
    /// New field: samples broken down by thread.
    pub samples_by_thread: Vec<ThreadSampleReport>,
    /// New field: top-N unique call stacks by sample_count.
    pub hot_stacks: Vec<StackReport>,
    /// New field: sampling-quality report.
    pub sampling: SamplingReport,
}

/// Walk every sample stack leaf → root, skip noise frames, credit only the
/// outermost inlined symbol of each physical frame, bucket by thread and by
/// unique call-chain, and produce an `AttributionOutput`.
///
/// # Arguments
/// * `stacks` — synthetic or pprof-derived stacks (leaf-first).
/// * `duration_secs` — actual wall-clock window length (not the sleep arg).
/// * `sample_freq_hz` — the profiler's sampling frequency (used for CPU time).
/// * `top_n` — cap for `scopes_by_calls`, `scopes_by_self_ms`, `hot_stacks`.
/// * `focus` — optional substring filter; drops any stack that does not
///   contain at least one frame whose symbol contains the focus substring.
///
/// # Returns
/// An `AttributionOutput` populated with every derived list.
pub fn attribute_samples(
    stacks: &[SampleStack],
    duration_secs: f64,
    sample_freq_hz: i32,
    top_n: usize,
    focus: Option<&str>,
) -> AttributionOutput {
    // 1. Apply focus filter: drop any stack whose frames do not contain a
    //    frame with a symbol matching the focus substring. Filter is applied
    //    BEFORE any aggregation so every downstream list reflects the
    //    narrowed view.
    let kept: Vec<&SampleStack> = match focus {
        Some(needle) if !needle.is_empty() => stacks
            .iter()
            .filter(|s| {
                s.frames
                    .iter()
                    .any(|phys| phys.iter().any(|f| f.symbol.contains(needle)))
            })
            .collect(),
        _ => stacks.iter().collect(),
    };

    let total_samples: u64 = kept.iter().map(|s| s.count).sum();

    // 2. Per-thread, per-scope, per-hot-stack accumulators.
    let mut per_thread: HashMap<(String, u64), u64> = HashMap::new();
    let mut per_scope: HashMap<String, ScopeAccum> = HashMap::new();
    let mut per_stack: HashMap<Vec<String>, HotStackAccum> = HashMap::new();
    let mut resolved_rust_samples: u64 = 0;

    // 3. Walk each kept stack leaf -> root and attribute the sample to the
    //    first non-noise physical frame's outermost inlined symbol.
    for stack in &kept {
        *per_thread
            .entry((stack.thread_name.clone(), stack.thread_id))
            .or_insert(0) += stack.count;

        let Some(attribution) = attribute_single_stack(stack) else {
            continue;
        };

        if attribution.is_resolved {
            resolved_rust_samples = resolved_rust_samples.saturating_add(stack.count);
        }

        let scope = per_scope
            .entry(attribution.label.clone())
            .or_insert_with(|| ScopeAccum {
                label: attribution.label.clone(),
                call_count: 0,
            });
        scope.call_count = scope.call_count.saturating_add(stack.count);

        let stack_key: Vec<String> = attribution
            .hot_frames
            .iter()
            .map(|f| f.symbol.clone())
            .collect();
        let hot = per_stack.entry(stack_key).or_insert_with(|| HotStackAccum {
            frames: attribution.hot_frames.clone(),
            thread_name: stack.thread_name.clone(),
            sample_count: 0,
            top_thread_samples: 0,
        });
        hot.sample_count = hot.sample_count.saturating_add(stack.count);
        if stack.count > hot.top_thread_samples {
            hot.top_thread_samples = stack.count;
            hot.thread_name = stack.thread_name.clone();
        }
    }

    // 4. Derived time maths.
    let sample_period_ms = if sample_freq_hz > 0 {
        1000.0_f64 / sample_freq_hz as f64
    } else {
        0.0
    };
    let cpu_cores_consumed = if duration_secs > 0.0 && sample_freq_hz > 0 {
        total_samples as f64 / sample_freq_hz as f64 / duration_secs
    } else {
        0.0
    };

    // 5. Build scopes_by_calls / scopes_by_self_ms with the same entry set.
    let scope_entries: Vec<ScopeReport> = per_scope
        .into_values()
        .map(|acc| {
            let total_self_ms = acc.call_count as f64 * sample_period_ms;
            let calls_per_sec = if duration_secs > 0.0 {
                acc.call_count as f64 / duration_secs
            } else {
                0.0
            };
            ScopeReport {
                label: acc.label,
                call_count: acc.call_count,
                total_self_ms,
                max_iter_ms: sample_period_ms,
                calls_per_sec,
                currently_executing_at_end: 0,
            }
        })
        .collect();

    let mut scopes_by_calls = scope_entries.clone();
    scopes_by_calls.sort_by(|a, b| {
        b.call_count
            .cmp(&a.call_count)
            .then_with(|| a.label.cmp(&b.label))
    });
    scopes_by_calls.truncate(top_n);

    let mut scopes_by_self_ms = scope_entries;
    scopes_by_self_ms.sort_by(|a, b| {
        b.total_self_ms
            .partial_cmp(&a.total_self_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.label.cmp(&b.label))
    });
    scopes_by_self_ms.truncate(top_n);

    // 6. Build samples_by_thread sorted by sample_count desc.
    let mut samples_by_thread: Vec<ThreadSampleReport> = per_thread
        .into_iter()
        .map(|((thread_name, thread_id), sample_count)| {
            let cpu_ms = (sample_count as f64 * sample_period_ms) as u64;
            ThreadSampleReport {
                thread_name,
                thread_id,
                sample_count,
                cpu_ms,
            }
        })
        .collect();
    samples_by_thread.sort_by(|a, b| {
        b.sample_count
            .cmp(&a.sample_count)
            .then_with(|| a.thread_name.cmp(&b.thread_name))
    });

    // 7. Build hot_stacks sorted by sample_count desc, capped at top_n.
    let mut hot_stacks: Vec<StackReport> = per_stack
        .into_values()
        .map(|acc| StackReport {
            frames: acc.frames,
            thread_name: acc.thread_name,
            sample_count: acc.sample_count,
        })
        .collect();
    hot_stacks.sort_by(|a, b| {
        b.sample_count.cmp(&a.sample_count).then_with(|| {
            a.frames
                .first()
                .map(|f| f.symbol.as_str())
                .unwrap_or("")
                .cmp(b.frames.first().map(|f| f.symbol.as_str()).unwrap_or(""))
        })
    });
    hot_stacks.truncate(top_n);

    // 8. Build sampling report + debug-info heuristic.
    let unresolved_samples = total_samples.saturating_sub(resolved_rust_samples);
    let debug_info_available =
        total_samples == 0 || resolved_rust_samples.saturating_mul(10) >= total_samples;
    let hint = if debug_info_available {
        None
    } else {
        Some(
            "Fewer than 10% of samples resolved to Rust source locations. \
             Rebuild the native binary with debug info (set [profile.release] \
             debug = 1 and strip = \"none\" in codelet/Cargo.toml) and rerun \
             the profile action to get meaningful attribution."
                .to_string(),
        )
    };

    let sampling = SamplingReport {
        total_samples,
        resolved_rust_samples,
        unresolved_samples,
        cpu_cores_consumed,
        debug_info_available,
        hint,
    };

    AttributionOutput {
        scopes_by_calls,
        scopes_by_self_ms,
        samples_by_thread,
        hot_stacks,
        sampling,
    }
}

/// Output of `attribute_single_stack`.
struct StackAttribution {
    /// The attributed scope label (outermost inlined symbol of the first
    /// non-noise physical frame walked leaf -> root).
    label: String,
    /// The first `HOT_STACK_MAX_FRAMES` meaningful (non-noise) frames, leaf
    /// first, with each physical frame collapsed to its outermost inlined
    /// symbol.
    hot_frames: Vec<StackFrameInfo>,
    /// True when the attributed frame has a populated source file that is
    /// neither `Unknown` nor ends in `.c`.
    is_resolved: bool,
}

/// Walk one `SampleStack` leaf -> root, skipping noise frames, and collapse
/// each physical frame to its outermost inlined symbol.
fn attribute_single_stack(stack: &SampleStack) -> Option<StackAttribution> {
    let mut meaningful: Vec<StackFrameInfo> = Vec::new();
    for phys in &stack.frames {
        let Some(outer) = phys.first() else {
            continue;
        };
        if is_noise_frame(&outer.symbol) {
            continue;
        }
        meaningful.push(StackFrameInfo {
            symbol: outer.symbol.clone(),
            file: outer.file.clone(),
            line: outer.line,
        });
        if meaningful.len() >= HOT_STACK_MAX_FRAMES {
            break;
        }
    }

    let first = meaningful.first()?.clone();
    let is_resolved = matches!(
        first.file.as_deref(),
        Some(path) if !path.is_empty() && path != "Unknown" && !path.ends_with(".c")
    );

    Some(StackAttribution {
        label: first.symbol,
        hot_frames: meaningful,
        is_resolved,
    })
}

/// Mutable aggregator for `scopes_by_calls` / `scopes_by_self_ms`.
struct ScopeAccum {
    label: String,
    call_count: u64,
}

/// Mutable aggregator for `hot_stacks`.
struct HotStackAccum {
    frames: Vec<StackFrameInfo>,
    thread_name: String,
    sample_count: u64,
    /// Largest single-stack contribution seen — used to pick which thread
    /// name we surface on the aggregated stack report.
    top_thread_samples: u64,
}
