//! Synchronous dispatcher entry point — the standalone fspec Rust binary's
//! `agent_loop` invokes [`dispatch_command`] for every Fspec tool call.
//!
//! Phase 1 (TOOL-019) wiring:
//!
//! 1. Look up `req.command` in [`crate::canonical::CANONICAL_COMMANDS`].
//! 2. Unknown name → [`FspecCoreError::UnknownCommand`] → failure result.
//! 3. Known name → route to the matching `commands::<snake>::run` stub, which
//!    today always returns [`FspecCoreError::NotYetPorted`].
//!
//! The NAPI chunk-callback delegation path (when the Rust dispatcher is
//! invoked from inside the NAPI Node-hosted CLI and should transparently
//! delegate back into TypeScript) is **out of scope** for the scaffolding
//! turn — see the `TODO(TOOL-019)` marker below.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use crate::canonical::lookup;
use crate::commands;
use crate::error::FspecCoreError;

/// Synchronously drive a future that performs no genuine async work.
///
/// Every ported `commands::*::run` function and every Phase 1 stub
/// resolves on the first poll — they touch only `std::fs` / `serde_json`
/// and never `.await` on a tokio resource. We deliberately avoid
/// `tokio::runtime::Runtime::block_on` here because [`dispatch_command`]
/// is invoked from the agent loop's `FspecToolFacadeWrapper::call` path,
/// which is already being polled inside the outer `#[tokio::main]`
/// runtime. Building and entering a fresh tokio runtime from there
/// either panics ("Cannot start a runtime from within a runtime") or
/// dead-locks the worker thread — both manifest to the user as a
/// hung tool call.
///
/// The future is polled once with a no-op waker; if it returns
/// `Pending` the dispatcher returns a structured error rather than
/// hanging, so a future refactor that accidentally introduces a real
/// `.await` is caught loudly instead of silently.
fn poll_sync_future<T, F>(future: F) -> Result<T, FspecCoreError>
where
    F: Future<Output = Result<T, FspecCoreError>>,
{
    let mut future = Box::pin(future);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    match Pin::as_mut(&mut future).poll(&mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => Err(FspecCoreError::InvalidArgs {
            command: "dispatch",
            reason:
                "ported command future returned Pending under sync dispatch — \
                 introduce a real async runtime or make the command sync"
                    .to_string(),
        }),
    }
}

/// Inputs to a single fspec tool dispatch.
#[derive(Debug, Clone)]
pub struct DispatchRequest {
    /// kebab-case command name (e.g. `"add-rule"`).
    pub command: String,
    /// Raw JSON-encoded args object as supplied by the LLM tool call.
    pub args_json: String,
    /// Absolute path to the project root the command should operate against.
    pub project_root: PathBuf,
}

/// Result returned to the agent loop. Always synchronous, always structured —
/// the agent loop must NEVER block waiting for a JS callback.
#[derive(Debug, Clone)]
pub struct DispatchResult {
    pub success: bool,
    /// Command output (typically JSON-encoded) when `success == true`.
    pub data: String,
    /// Human-readable error message when `success == false`.
    pub error: Option<String>,
    /// Optional `<system-reminder>`-style payload the agent loop should
    /// forward to the LLM verbatim. Unused in Phase 1.
    pub system_reminder: Option<String>,
}

impl DispatchResult {
    fn from_error(err: FspecCoreError) -> Self {
        Self {
            success: false,
            data: String::new(),
            error: Some(err.to_string()),
            system_reminder: None,
        }
    }
}

/// Synchronous dispatch entry point.
///
/// Returns a [`DispatchResult`] — never panics, never blocks on external
/// callbacks. The async runtime is used purely to drive the per-command
/// `async fn run` stubs to completion; in Phase 1 every stub resolves
/// immediately with [`FspecCoreError::NotYetPorted`].
pub fn dispatch_command(req: DispatchRequest) -> DispatchResult {
    // TODO(TOOL-019): NAPI chunk-callback delegation path will be wired in
    // the agent_loop integration step — when the NAPI chunk-callback is
    // registered, dispatch should transparently delegate back into
    // TypeScript via the existing chunk-callback protocol instead of
    // serving the stub below.

    let canonical = match lookup(&req.command) {
        Some(c) => c,
        None => {
            return DispatchResult::from_error(FspecCoreError::UnknownCommand {
                command: req.command.clone(),
            });
        }
    };

    // Fast path: every command that has been ported to Rust gets a dedicated
    // arm in `run_ported` and receives the project_root so it can perform
    // real filesystem work. Anything not in the ported set falls through to
    // `run_stub` and returns the canonical NotYetPorted error.
    let result = match run_ported(canonical.name, &req.args_json, &req.project_root) {
        Some(r) => r,
        None => run_stub(canonical.name, &req.args_json),
    };

    match result {
        Ok(data) => DispatchResult {
            success: true,
            data,
            error: None,
            system_reminder: None,
        },
        Err(err) => DispatchResult::from_error(err),
    }
}

/// Route ported commands. Returns `Some(result)` when `name` matches a Rust
/// implementation; `None` when the command is still a stub. The set of arms
/// here grows monotonically with each landed RPC-XXX child card.
fn run_ported(
    name: &'static str,
    args_json: &str,
    project_root: &std::path::Path,
) -> Option<Result<String, FspecCoreError>> {
    // Identify ported commands FIRST so unported names skip future construction.
    if !crate::canonical::is_ported(name) {
        return None;
    }

    Some(poll_sync_future(async move {
        match name {
            // RPC-253 — list-work-units
            "list-work-units" => commands::list_work_units::run(args_json, project_root).await,
            // RPC-248 — list-prefixes
            "list-prefixes" => commands::list_prefixes::run(args_json, project_root).await,
            // RPC-243 — list-epics
            "list-epics" => commands::list_epics::run(args_json, project_root).await,
            // RPC-251 — list-tags
            "list-tags" => commands::list_tags::run(args_json, project_root).await,
            // RPC-245 — list-features
            "list-features" => commands::list_features::run(args_json, project_root).await,
            // RPC-241 — list-attachments
            "list-attachments" => commands::list_attachments::run(args_json, project_root).await,
            // RPC-247 — list-hooks
            "list-hooks" => commands::list_hooks::run(args_json, project_root).await,
            // RPC-244 — list-feature-tags
            "list-feature-tags" => {
                commands::list_feature_tags::run(args_json, project_root).await
            }
            // RPC-246 — list-foundation-sections
            "list-foundation-sections" => {
                commands::list_foundation_sections::run(args_json, project_root).await
            }
            // RPC-249 — list-scenario-tags
            "list-scenario-tags" => {
                commands::list_scenario_tags::run(args_json, project_root).await
            }
            // RPC-250 — list-schedules
            "list-schedules" => commands::list_schedules::run(args_json, project_root).await,
            // RPC-252 — list-virtual-hooks
            "list-virtual-hooks" => {
                commands::list_virtual_hooks::run(args_json, project_root).await
            }
            // RPC-242 — list-checkpoints
            "list-checkpoints" => {
                commands::list_checkpoints::run(args_json, project_root).await
            }
            // RPC-301 — show-deleted
            "show-deleted" => commands::show_deleted::run(args_json, project_root).await,
            // RPC-302 — show-epic
            "show-epic" => commands::show_epic::run(args_json, project_root).await,
            // RPC-304 — show-feature
            "show-feature" => commands::show_feature::run(args_json, project_root).await,
            // RPC-310 — tag-stats
            "tag-stats" => commands::tag_stats::run(args_json, project_root).await,
            // RPC-308 — show-work-unit
            "show-work-unit" => commands::show_work_unit::run(args_json, project_root).await,
            // RPC-257 — query-dependency-stats
            "query-dependency-stats" => {
                commands::query_dependency_stats::run(args_json, project_root).await
            }
            // RPC-258 — query-estimate-accuracy
            "query-estimate-accuracy" => {
                commands::query_estimate_accuracy::run(args_json, project_root).await
            }
            // RPC-261 — query-metrics
            "query-metrics" => commands::query_metrics::run(args_json, project_root).await,
            // RPC-263 — query-work-units
            "query-work-units" => commands::query_work_units::run(args_json, project_root).await,
            // Batch 6 (2026-06-09)
            // RPC-256 — query-bottlenecks
            "query-bottlenecks" => {
                commands::query_bottlenecks::run(args_json, project_root).await
            }
            // RPC-262 — query-orphans
            "query-orphans" => commands::query_orphans::run(args_json, project_root).await,
            // RPC-259 — query-estimation-guide
            "query-estimation-guide" => {
                commands::query_estimation_guide::run(args_json, project_root).await
            }
            // RPC-260 — query-example-mapping-stats
            "query-example-mapping-stats" => {
                commands::query_example_mapping_stats::run(args_json, project_root).await
            }
            // RPC-303 — show-event-storm
            "show-event-storm" => commands::show_event_storm::run(args_json, project_root).await,
            // RPC-305 — show-foundation
            "show-foundation" => commands::show_foundation::run(args_json, project_root).await,
            // RPC-306 — show-foundation-event-storm
            "show-foundation-event-storm" => {
                commands::show_foundation_event_storm::run(args_json, project_root).await
            }
            // RPC-307 — show-test-patterns
            "show-test-patterns" => {
                commands::show_test_patterns::run(args_json, project_root).await
            }
            // RPC-299 — show-acceptance-criteria
            "show-acceptance-criteria" => {
                commands::show_acceptance_criteria::run(args_json, project_root).await
            }
            // RPC-300 — show-coverage
            "show-coverage" => commands::show_coverage::run(args_json, project_root).await,
            // Batch 7 (2026-06-10) — mutation commands
            // RPC-211 — create-epic
            "create-epic" => commands::create_epic::run(args_json, project_root).await,
            // RPC-217 — delete-epic
            "delete-epic" => commands::delete_epic::run(args_json, project_root).await,
            // RPC-213 — create-prefix
            "create-prefix" => commands::create_prefix::run(args_json, project_root).await,
            // RPC-265 — register-tag
            "register-tag" => commands::register_tag::run(args_json, project_root).await,
            // RPC-313 — update-prefix
            "update-prefix" => commands::update_prefix::run(args_json, project_root).await,
            // RPC-316 — update-tag
            "update-tag" => commands::update_tag::run(args_json, project_root).await,
            // RPC-222 — delete-tag
            "delete-tag" => commands::delete_tag::run(args_json, project_root).await,
            // RPC-176 — add-dependencies
            "add-dependencies" => {
                commands::add_dependencies::run(args_json, project_root).await
            }
            // RPC-271 — remove-dependency
            "remove-dependency" => {
                commands::remove_dependency::run(args_json, project_root).await
            }
            // RPC-204 — clear-dependencies
            "clear-dependencies" => {
                commands::clear_dependencies::run(args_json, project_root).await
            }
            // Batch 8 (2026-06-11) — Example Mapping mutation commands
            // RPC-189 — add-rule
            "add-rule" => commands::add_rule::run(args_json, project_root).await,
            // RPC-279 — remove-rule
            "remove-rule" => commands::remove_rule::run(args_json, project_root).await,
            // RPC-169 — add-assumption
            "add-assumption" => commands::add_assumption::run(args_json, project_root).await,
            // RPC-181 — add-example
            "add-example" => commands::add_example::run(args_json, project_root).await,
            // RPC-273 — remove-example
            "remove-example" => commands::remove_example::run(args_json, project_root).await,
            // RPC-188 — add-question
            "add-question" => commands::add_question::run(args_json, project_root).await,
            // RPC-278 — remove-question
            "remove-question" => commands::remove_question::run(args_json, project_root).await,
            // RPC-168 — add-architecture-note
            "add-architecture-note" => {
                commands::add_architecture_note::run(args_json, project_root).await
            }
            // RPC-267 — remove-architecture-note
            "remove-architecture-note" => {
                commands::remove_architecture_note::run(args_json, project_root).await
            }
            // RPC-298 — set-user-story
            "set-user-story" => commands::set_user_story::run(args_json, project_root).await,
            // Batch 9 (2026-06-11) — dependency, q&a, tag-feature, tag-scenario, restore-*
            // RPC-177 — add-dependency
            "add-dependency" => commands::add_dependency::run(args_json, project_root).await,
            // RPC-196 — answer-question
            "answer-question" => commands::answer_question::run(args_json, project_root).await,
            // RPC-289 — restore-example
            "restore-example" => commands::restore_example::run(args_json, project_root).await,
            // RPC-291 — restore-rule
            "restore-rule" => commands::restore_rule::run(args_json, project_root).await,
            // RPC-290 — restore-question
            "restore-question" => commands::restore_question::run(args_json, project_root).await,
            // RPC-287 — restore-architecture-note
            "restore-architecture-note" => {
                commands::restore_architecture_note::run(args_json, project_root).await
            }
            // RPC-193 — add-tag-to-feature
            "add-tag-to-feature" => {
                commands::add_tag_to_feature::run(args_json, project_root).await
            }
            // RPC-281 — remove-tag-from-feature
            "remove-tag-from-feature" => {
                commands::remove_tag_from_feature::run(args_json, project_root).await
            }
            // RPC-194 — add-tag-to-scenario
            "add-tag-to-scenario" => {
                commands::add_tag_to_scenario::run(args_json, project_root).await
            }
            // RPC-282 — remove-tag-from-scenario
            "remove-tag-from-scenario" => {
                commands::remove_tag_from_scenario::run(args_json, project_root).await
            }
            // Batch 10 (2026-06-11) — attachments, virtual hooks, hooks, diagrams
            // RPC-170 — add-attachment
            "add-attachment" => commands::add_attachment::run(args_json, project_root).await,
            // RPC-268 — remove-attachment
            "remove-attachment" => {
                commands::remove_attachment::run(args_json, project_root).await
            }
            // RPC-195 — add-virtual-hook
            "add-virtual-hook" => {
                commands::add_virtual_hook::run(args_json, project_root).await
            }
            // RPC-283 — remove-virtual-hook
            "remove-virtual-hook" => {
                commands::remove_virtual_hook::run(args_json, project_root).await
            }
            // RPC-205 — clear-virtual-hooks
            "clear-virtual-hooks" => {
                commands::clear_virtual_hooks::run(args_json, project_root).await
            }
            // RPC-209 — copy-virtual-hooks
            "copy-virtual-hooks" => {
                commands::copy_virtual_hooks::run(args_json, project_root).await
            }
            // RPC-184 — add-hook
            "add-hook" => commands::add_hook::run(args_json, project_root).await,
            // RPC-275 — remove-hook
            "remove-hook" => commands::remove_hook::run(args_json, project_root).await,
            // RPC-178 — add-diagram
            "add-diagram" => commands::add_diagram::run(args_json, project_root).await,
            // RPC-216 — delete-diagram
            "delete-diagram" => commands::delete_diagram::run(args_json, project_root).await,
            // Unreachable: gated by `is_ported` above.
            _ => unreachable!("ported-command match must agree with `is_ported` predicate"),
        }
    }))
}

/// Route a canonical command name to its corresponding stub module. Phase 1
/// only wires `add-rule`; every other known-but-unwired command name falls
/// through to a [`FspecCoreError::NotYetPorted`] with the canonical
/// `"RPC-PENDING"` work-unit placeholder.
fn run_stub(name: &'static str, args_json: &str) -> Result<String, FspecCoreError> {
    // Drive the per-command async stub synchronously. Phase 1 stubs perform
    // no I/O and resolve on the first poll, so a no-op waker is sufficient.
    // We deliberately avoid building a fresh tokio runtime here because this
    // path is reached from inside the agent loop's `#[tokio::main]` runtime
    // and a nested `block_on` either panics or dead-locks the worker.
    poll_sync_future(async move {
        match name {
            "add-aggregate" => commands::add_aggregate::run(args_json).await,
            "add-aggregate-to-foundation" => commands::add_aggregate_to_foundation::run(args_json).await,
            "add-architecture" => commands::add_architecture::run(args_json).await,
            // "add-architecture-note" — ported (RPC-168, Batch 8). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "add-assumption" — ported (RPC-169, Batch 8). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "add-attachment" — ported (RPC-170, Batch 10). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "add-background" => commands::add_background::run(args_json).await,
            "add-bounded-context" => commands::add_bounded_context::run(args_json).await,
            "add-capability" => commands::add_capability::run(args_json).await,
            "add-command" => commands::add_command::run(args_json).await,
            "add-command-to-foundation" => commands::add_command_to_foundation::run(args_json).await,
            // "add-dependencies" — ported (RPC-176, Batch 7). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "add-dependency" — ported (RPC-177, Batch 9). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "add-diagram" — ported (RPC-178, Batch 10). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "add-domain-event" => commands::add_domain_event::run(args_json).await,
            "add-domain-event-to-foundation" => commands::add_domain_event_to_foundation::run(args_json).await,
            // "add-example" — ported (RPC-181, Batch 8). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "add-external-system" => commands::add_external_system::run(args_json).await,
            "add-foundation-bounded-context" => commands::add_foundation_bounded_context::run(args_json).await,
            // "add-hook" — ported (RPC-184, Batch 10). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "add-hotspot" => commands::add_hotspot::run(args_json).await,
            "add-persona" => commands::add_persona::run(args_json).await,
            "add-policy" => commands::add_policy::run(args_json).await,
            // "add-question" — ported (RPC-188, Batch 8). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "add-rule" — ported (RPC-189, Batch 8). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "add-scenario" => commands::add_scenario::run(args_json).await,
            "add-schedule" => commands::add_schedule::run(args_json).await,
            "add-step" => commands::add_step::run(args_json).await,
            // "add-tag-to-feature" — ported (RPC-193, Batch 9). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "add-tag-to-scenario" — ported (RPC-194, Batch 9). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "add-virtual-hook" — ported (RPC-195, Batch 10). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "answer-question" — ported (RPC-196, Batch 9). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "audit-coverage" => commands::audit_coverage::run(args_json).await,
            "auto-advance" => commands::auto_advance::run(args_json).await,
            "board" => commands::board::run(args_json).await,
            "bootstrap" => commands::bootstrap::run(args_json).await,
            "check" => commands::check::run(args_json).await,
            "checkpoint" => commands::checkpoint::run(args_json).await,
            "cleanup-checkpoints" => commands::cleanup_checkpoints::run(args_json).await,
            // "clear-dependencies" — ported (RPC-204, Batch 7). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "clear-virtual-hooks" — ported (RPC-205, Batch 10). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "compact-work-unit" => commands::compact_work_unit::run(args_json).await,
            "compare-implementations" => commands::compare_implementations::run(args_json).await,
            "configure-tools" => commands::configure_tools::run(args_json).await,
            // "copy-virtual-hooks" — ported (RPC-209, Batch 10). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "create-bug" => commands::create_bug::run(args_json).await,
            // "create-epic" — ported (RPC-211, Batch 7). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "create-feature" => commands::create_feature::run(args_json).await,
            // "create-prefix" — ported (RPC-213, Batch 7). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "create-story" => commands::create_story::run(args_json).await,
            "create-task" => commands::create_task::run(args_json).await,
            // "delete-diagram" — ported (RPC-216, Batch 10). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "delete-epic" — ported (RPC-217, Batch 7). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "delete-features" => commands::delete_features::run(args_json).await,
            "delete-scenario" => commands::delete_scenario::run(args_json).await,
            "delete-scenarios" => commands::delete_scenarios::run(args_json).await,
            "delete-step" => commands::delete_step::run(args_json).await,
            // "delete-tag" — ported (RPC-222, Batch 7). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "delete-work-unit" => commands::delete_work_unit::run(args_json).await,
            "dependencies" => commands::dependencies::run(args_json).await,
            "discover-event-storm" => commands::discover_event_storm::run(args_json).await,
            "discover-foundation" => commands::discover_foundation::run(args_json).await,
            "export-dependencies" => commands::export_dependencies::run(args_json).await,
            "export-example-map" => commands::export_example_map::run(args_json).await,
            "export-work-units" => commands::export_work_units::run(args_json).await,
            "format" => commands::format::run(args_json).await,
            "generate-coverage" => commands::generate_coverage::run(args_json).await,
            "generate-example-mapping-from-event-storm" => commands::generate_example_mapping_from_event_storm::run(args_json).await,
            "generate-foundation-md" => commands::generate_foundation_md::run(args_json).await,
            "generate-scenarios" => commands::generate_scenarios::run(args_json).await,
            "generate-summary-report" => commands::generate_summary_report::run(args_json).await,
            "generate-tags-md" => commands::generate_tags_md::run(args_json).await,
            "get-scenarios" => commands::get_scenarios::run(args_json).await,
            "import-example-map" => commands::import_example_map::run(args_json).await,
            "init" => commands::init::run(args_json).await,
            "link-coverage" => commands::link_coverage::run(args_json).await,
            // "list-attachments" — ported (RPC-241). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "list-checkpoints" — ported (RPC-242). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "list-epics" — ported (RPC-243). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "list-feature-tags" — ported (RPC-244). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "list-features" — ported (RPC-245). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "list-foundation-sections" — ported (RPC-246). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "list-hooks" — ported (RPC-247). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "list-prefixes" — ported (RPC-248). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "list-scenario-tags" — ported (RPC-249). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "list-schedules" — ported (RPC-250). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "list-tags" — ported (RPC-251). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "list-virtual-hooks" — ported (RPC-252). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "list-work-units" — ported (RPC-253). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "pause-schedule" => commands::pause_schedule::run(args_json).await,
            "prioritize-work-unit" => commands::prioritize_work_unit::run(args_json).await,
            // "query-bottlenecks" — ported (RPC-256). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "query-dependency-stats" — ported (RPC-257). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "query-estimate-accuracy" — ported (RPC-258). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "query-estimation-guide" — ported (RPC-259). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "query-example-mapping-stats" — ported (RPC-260). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "query-metrics" — ported (RPC-261). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "query-orphans" — ported (RPC-262). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "query-work-units" — ported (RPC-263). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "record-iteration" => commands::record_iteration::run(args_json).await,
            // "register-tag" — ported (RPC-265, Batch 7). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "remove-aggregate-from-foundation" => commands::remove_aggregate_from_foundation::run(args_json).await,
            // "remove-architecture-note" — ported (RPC-267, Batch 8). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "remove-attachment" — ported (RPC-268, Batch 10). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "remove-capability" => commands::remove_capability::run(args_json).await,
            "remove-command-from-foundation" => commands::remove_command_from_foundation::run(args_json).await,
            // "remove-dependency" — ported (RPC-271, Batch 7). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "remove-domain-event-from-foundation" => commands::remove_domain_event_from_foundation::run(args_json).await,
            // "remove-example" — ported (RPC-273, Batch 8). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "remove-foundation-bounded-context" => commands::remove_foundation_bounded_context::run(args_json).await,
            // "remove-hook" — ported (RPC-275, Batch 10). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "remove-init-files" => commands::remove_init_files::run(args_json).await,
            "remove-persona" => commands::remove_persona::run(args_json).await,
            // "remove-question" — ported (RPC-278, Batch 8). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "remove-rule" — ported (RPC-279, Batch 8). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "remove-schedule" => commands::remove_schedule::run(args_json).await,
            // "remove-tag-from-feature" — ported (RPC-281, Batch 9). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "remove-tag-from-scenario" — ported (RPC-282, Batch 9). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "remove-virtual-hook" — ported (RPC-283, Batch 10). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "repair-work-units" => commands::repair_work_units::run(args_json).await,
            "report-bug-to-github" => commands::report_bug_to_github::run(args_json).await,
            "research" => commands::research::run(args_json).await,
            // "restore-architecture-note" — ported (RPC-287, Batch 9). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "restore-checkpoint" => commands::restore_checkpoint::run(args_json).await,
            // "restore-example" — ported (RPC-289, Batch 9). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "restore-question" — ported (RPC-290, Batch 9). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "restore-rule" — ported (RPC-291, Batch 9). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "resume-schedule" => commands::resume_schedule::run(args_json).await,
            "retag" => commands::retag::run(args_json).await,
            "reverse" => commands::reverse::run(args_json).await,
            "review" => commands::review::run(args_json).await,
            "search-implementation" => commands::search_implementation::run(args_json).await,
            "search-scenarios" => commands::search_scenarios::run(args_json).await,
            // "set-user-story" — ported (RPC-298, Batch 8). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "show-acceptance-criteria" — ported (RPC-299). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "show-coverage" — ported (RPC-300). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "show-deleted" — ported (RPC-301). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "show-epic" — ported (RPC-302). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "show-event-storm" — ported (RPC-303). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "show-feature" — ported (RPC-304). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "show-foundation" — ported (RPC-305). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "show-foundation-event-storm" — ported (RPC-306). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "show-test-patterns" — ported (RPC-307). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            // "show-work-unit" — ported (RPC-308). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "suggest-dependencies" => commands::suggest_dependencies::run(args_json).await,
            // "tag-stats" — ported (RPC-310). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "unlink-coverage" => commands::unlink_coverage::run(args_json).await,
            "update-foundation" => commands::update_foundation::run(args_json).await,
            // "update-prefix" — ported (RPC-313, Batch 7). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "update-scenario" => commands::update_scenario::run(args_json).await,
            "update-step" => commands::update_step::run(args_json).await,
            // "update-tag" — ported (RPC-316, Batch 7). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "update-work-unit" => commands::update_work_unit::run(args_json).await,
            "update-work-unit-estimate" => commands::update_work_unit_estimate::run(args_json).await,
            "update-work-unit-status" => commands::update_work_unit_status::run(args_json).await,
            "validate" => commands::validate::run(args_json).await,
            "validate-foundation-schema" => commands::validate_foundation_schema::run(args_json).await,
            "validate-hooks" => commands::validate_hooks::run(args_json).await,
            "validate-spec-alignment" => commands::validate_spec_alignment::run(args_json).await,
            "validate-tags" => commands::validate_tags::run(args_json).await,
            "validate-work-units" => commands::validate_work_units::run(args_json).await,
            "workflow-automation" => commands::workflow_automation::run(args_json).await,
            // Unreachable: canonical lookup already validated the
            // command exists, and every canonical entry has a stub.
            other => Err(FspecCoreError::UnknownCommand { command: other.to_string() }),
        }
    })
}

