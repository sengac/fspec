//! Runtime profiling submodule — time-bounded profile session orchestration
//!
//! Feature: spec/features/agent-manager-profile-action.feature
//!
//! Provides the `profile_scope!()` macro, `ProfileRegistry` singleton, `ProfileSession::run()`
//! orchestrator, and `TrackedBroadcast`/`TrackedMpsc`/`TrackedUnboundedMpsc` channel wrappers
//! used by the AgentManager `profile` action (AMGR-017).
//!
//! The core performance property: when `PROFILING_ACTIVE == false` (steady state), every
//! `profile_scope!("label")` call site expands to a single Relaxed atomic load plus a
//! branch-not-taken (~1 ns on aarch64). Counter increments and timing only occur during an
//! active profile session window (default 10 s, range 1–60 s).

pub mod attribution;
pub mod channels;
pub mod registry;
pub mod result;
pub mod scope;
pub mod session;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod diagnostics_tests;

pub use attribution::{
    attribute_samples, is_noise_frame, AttributionOutput, FrameInfo, SampleStack,
    HOT_STACK_MAX_FRAMES, NOISE_FRAME_PREFIXES,
};
pub use registry::{ProfileRegistry, PROFILING_ACTIVE};
pub use result::{
    ChannelReport, ProcessReport, ProfileResult, RuntimeReport, SamplingReport, ScopeReport,
    StackFrameInfo, StackReport, ThreadSampleReport,
};
pub use scope::ProfileScope;
pub use session::{ProfileRunError, ProfileSession};
