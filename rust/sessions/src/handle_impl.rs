//! The production `SessionManagerHandle` impl for the extracted
//! `SessionManager`, added by **RPC-042**.
//!
//! Each method delegates to the existing `SessionManager` or to a
//! `BackgroundSession` looked up via `self.get_session(...)`. The
//! per-session methods return safe defaults when the session is not
//! found so callers (the future `fspec` binary in RPC-044, the
//! `FspecService` in `codelet-rpc`, the WebSocket backend, etc.) can
//! treat the handle as totally robust.
//!
//! **Runtime requirement (RPC-070):** the sync→async bridges for
//! `create_session`, `create_isolated_session`, `test_provider_connection`,
//! and the three `/loop` methods invoke
//! `tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(...))`.
//! They MUST be invoked from a **multi-thread** tokio runtime — the
//! `fspec` binary's `#[tokio::main]` defaults to multi-thread and the
//! NAPI side's `#[napi(tokio_main)]` does too. `block_in_place`
//! temporarily detaches the current worker from the multi-thread
//! scheduler so the nested `block_on` does not panic with
//! "Cannot start a runtime from within a runtime" (the bug the
//! pre-RPC-070 code triggered when called from inside a tarpc handler;
//! see `spec/attachments/RPC-070/root-cause-analysis.md`).
//!
//! Calling these bridges from a single-thread tokio runtime panics
//! inside `block_in_place` itself with a clearer message than the old
//! nested-runtime panic. A `debug_assert!` in `loop_block_on` makes
//! the precondition explicit during development.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{Duration, Utc};
use tokio::sync::broadcast;
use tracing;
use uuid::Uuid;

use codelet_rpc_types::{
    ApprovalChoice, BlocklistRuleInfo, CompactionProgress, CompactionResult, FspecResult,
    HitlRequest, HitlResponse, IncomingMessageInput, IsolatedSessionInfo, LogRecord, MergeOutcome,
    MergeStatus, MergeStrategy, ModelEntry, PauseState, ProviderCredentialInfo,
    ProviderCredentialInput, RegisteredLoop, ScheduledJob, SessionChangesSummary, SessionId,
    SessionInfo, SessionModel, SessionStatus, SessionTokens, SessionWorktreeInfo, StreamChunk,
    TestConnectionResult, ThinkingConfig, ThinkingLevel, TokenRestoreState, WorkUnitContext,
};

use crate::conversions::pause_state_to_rpc;
use crate::session_manager::SessionManager;

/// Parse a wire-portable `SessionId` into a UUID. BUG-369: a malformed id
/// is an error, never `Uuid::nil()` standing in for a session.
fn parse_session_uuid(id: &SessionId) -> Result<Uuid, String> {
    Uuid::parse_str(id.value.as_str())
        .map_err(|error| format!("Invalid session ID: {}: {error}", id.value))
}

/// BUG-369: for handle methods with no error channel — a malformed id is
/// logged and treated as no session (`None`), never as `Uuid::nil()`.
fn logged_session_uuid(id: &SessionId) -> Option<Uuid> {
    parse_session_uuid(id)
        .map_err(|error| tracing::error!(%error, "malformed session id"))
        .ok()
}

/// WT-003: the distinct repository roots a no-session-id worktree op
/// (list/prune) must run against: the union of the in-memory sessions'
/// project roots and the project roots recorded in every git-session
/// manifest (`~/.fspec/git-sessions/`). Closed sessions leave their
/// manifest behind, which is the only place their worktree's project
/// root is recorded once the in-memory session is gone.
fn session_worktree_repo_roots(manager: &SessionManager) -> Vec<String> {
    let mut roots: std::collections::BTreeSet<String> =
        manager.all_in_memory_project_roots().into_iter().collect();
    if let Some(dir) = codelet_git::get_sessions_dir() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };
                let Some(id) = name.strip_suffix(".json") else {
                    continue;
                };
                if let Ok(Some(m)) = codelet_git::read_manifest(id) {
                    if !m.project_root.as_os_str().is_empty() {
                        roots.insert(m.project_root.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    roots.into_iter().collect()
}

include!("handle_impl/handle_impl_1.rs");
include!("handle_impl/handle_impl_2.rs");
include!("handle_impl/handle_impl_3.rs");
include!("handle_impl/handle_impl_4.rs");
include!("handle_impl/handle_impl_5.rs");
include!("handle_impl/handle_impl_6.rs");
include!("handle_impl/handle_impl_7.rs");
include!("handle_impl/handle_impl_8.rs");
include!("handle_impl/handle_impl_9.rs");
include!("handle_impl/handle_impl_10.rs");

macro_rules! compose_handle_impl_finish {
    ($($collected:tt)*) => {
/// Production `SessionManagerHandle` impl for the extracted
/// `SessionManager`.
///
/// **Runtime requirement (RPC-070):** the sync→async bridge methods
/// (`create_session`, `create_isolated_session`,
/// `test_provider_connection`, `loop_add`, `loop_cancel`, `loop_list`)
/// wrap their `Handle::current().block_on(...)` calls in
/// `tokio::task::block_in_place(|| ...)`. The trait MUST be invoked
/// from a multi-thread tokio runtime — the `fspec` binary and the
/// napi `tokio_main` thread both satisfy this. Calling from a
/// single-thread runtime panics inside `block_in_place` with a clearer
/// message than the pre-RPC-070 "Cannot start a runtime from within
/// a runtime" nested-runtime panic.
impl codelet_core::SessionManagerHandle for SessionManager {
    $($collected)*
}
    };
}

compose_handle_impl_1! {}

/// RPC-059: shared sync→async bridge for every `/loop` handle method.
/// Centralising the `Handle::current().block_on(...)` call here keeps
/// the impl block's bridge-count small even though three trait methods
/// have to cross the boundary.
///
/// **RPC-070:** the inner `block_on` is wrapped in
/// `tokio::task::block_in_place(...)` so this bridge is legal from
/// inside a tokio worker that is currently driving an async future
/// (e.g. the live tarpc handler at
/// `rust/rpc/src/lib.rs::FspecServiceImpl::create_session`). Without
/// the wrapper, calling `block_on` from inside an executing future
/// panics with "Cannot start a runtime from within a runtime".
/// `block_in_place` temporarily detaches the worker from the
/// multi-thread scheduler — which is why this helper requires a
/// multi-thread runtime (asserted below).
fn loop_block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send,
    F::Output: Send,
{
    debug_assert_eq!(
        tokio::runtime::Handle::current().runtime_flavor(),
        tokio::runtime::RuntimeFlavor::MultiThread,
        "loop_block_on requires a multi-thread tokio runtime — \
         tokio::task::block_in_place panics on a single-thread runtime",
    );
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}

/// RPC-059: convert an internal `LoopEntry` (chrono-typed) into the
/// wire-portable `RegisteredLoop` (RFC-3339 String timestamps).
fn entry_to_wire(entry: &codelet_core::loops::LoopEntry) -> RegisteredLoop {
    RegisteredLoop {
        id: entry.id.clone(),
        session_id: SessionId::new(entry.session_id.to_string()),
        prompt: entry.prompt.clone(),
        interval_seconds: entry.interval_seconds,
        created_at: entry.created_at.to_rfc3339(),
        expires_at: entry.expires_at.to_rfc3339(),
        last_run_at: entry.last_run_at.map(|t| t.to_rfc3339()),
    }
}

/// RPC-059: generate a fresh 8-char lowercase-hex loop id by taking the
/// first 8 chars of a UUID v4's simple representation. Avoids pulling
/// in the `rand` crate since `uuid` is already in the workspace
/// dependency tree.
fn generate_loop_id() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_string()
}

/// Convert a `codelet_tools::blocklist::BlocklistRule` to the wire-portable
/// `BlocklistRuleInfo`, stamping the supplied `source` tag.
fn blocklist_rule_to_wire(
    rule: codelet_tools::blocklist::BlocklistRule,
    source: &str,
) -> BlocklistRuleInfo {
    let action = match rule.action {
        codelet_tools::blocklist::BlocklistAction::Block => "block",
        codelet_tools::blocklist::BlocklistAction::Allow => "allow",
        codelet_tools::blocklist::BlocklistAction::Prompt => "prompt",
    };
    BlocklistRuleInfo {
        id: rule.id,
        pattern: rule.pattern,
        action: action.to_string(),
        reason: rule.reason,
        guidance: rule.guidance,
        source: source.to_string(),
    }
}

/// Build the models.dev `ModelRegistry` once for `list_providers` cloud-model
/// population (RPC-073). Returns `None` on any failure (cold cache offline,
/// corrupt file) so the model selector degrades to empty built-in model lists
/// rather than erroring. Bridges the async registry load from the sync trait
/// method via `block_in_place` (the same pattern `build_local_profile_sections`
/// and `test_provider_connection` use), so it is only safe inside a tokio
/// multi-thread runtime — which `list_providers` always runs within.
fn build_cloud_registry() -> Option<codelet_providers::models::ModelRegistry> {
    // RPC-073: bridge the async registry load from the sync trait method.
    // `block_in_place` + `Handle::block_on` requires a live multi-thread
    // tokio runtime. Outside one (e.g. plain `#[test]` callers exercising
    // the no-panic graceful-degradation contract), `Handle::try_current`
    // returns Err — degrade to `None` rather than panicking.
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return None;
    };
    tokio::task::block_in_place(|| {
        handle.block_on(async {
            match codelet_providers::models::ModelCache::new() {
                Ok(cache) => codelet_providers::models::ModelRegistry::new(&cache)
                    .await
                    .ok(),
                Err(_) => None,
            }
        })
    })
}

/// BUG-207: build a models.dev `ModelRegistry` after FORCING a catalog
/// re-fetch, for `refresh_models_cache`.
///
/// Unlike [`build_cloud_registry`] (which goes through the TTL-aware cache and
/// may serve a stale snapshot), this reaches `ModelCache::refresh` — a
/// synchronous fetch that bypasses both the TTL and the revalidation cooldown.
/// Without it, "refresh" builds a fresh registry from the SAME stale snapshot
/// on disk, which is exactly the reported no-op.
///
/// Returns `None` when no tokio runtime is available or the fetch failed; the
/// caller decides whether to fall back to the on-disk snapshot.
fn force_refresh_cloud_registry() -> Option<codelet_providers::models::ModelRegistry> {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return None;
    };
    tokio::task::block_in_place(|| {
        handle.block_on(async {
            match codelet_providers::models::ModelCache::new() {
                Ok(cache) => codelet_providers::models::ModelRegistry::refreshing(&cache)
                    .await
                    .ok(),
                Err(_) => None,
            }
        })
    })
}
