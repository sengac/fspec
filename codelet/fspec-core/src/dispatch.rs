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
<<<<<<< HEAD
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
=======
>>>>>>> parent of 6fa95633 (refactor: more commands refactored)
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
            "add-architecture-note" => commands::add_architecture_note::run(args_json).await,
            "add-assumption" => commands::add_assumption::run(args_json).await,
            "add-attachment" => commands::add_attachment::run(args_json).await,
            "add-background" => commands::add_background::run(args_json).await,
            "add-bounded-context" => commands::add_bounded_context::run(args_json).await,
            "add-capability" => commands::add_capability::run(args_json).await,
            "add-command" => commands::add_command::run(args_json).await,
            "add-command-to-foundation" => commands::add_command_to_foundation::run(args_json).await,
            "add-dependencies" => commands::add_dependencies::run(args_json).await,
            "add-dependency" => commands::add_dependency::run(args_json).await,
            "add-diagram" => commands::add_diagram::run(args_json).await,
            "add-domain-event" => commands::add_domain_event::run(args_json).await,
            "add-domain-event-to-foundation" => commands::add_domain_event_to_foundation::run(args_json).await,
            "add-example" => commands::add_example::run(args_json).await,
            "add-external-system" => commands::add_external_system::run(args_json).await,
            "add-foundation-bounded-context" => commands::add_foundation_bounded_context::run(args_json).await,
            "add-hook" => commands::add_hook::run(args_json).await,
            "add-hotspot" => commands::add_hotspot::run(args_json).await,
            "add-persona" => commands::add_persona::run(args_json).await,
            "add-policy" => commands::add_policy::run(args_json).await,
            "add-question" => commands::add_question::run(args_json).await,
            "add-rule" => commands::add_rule::run(args_json).await,
            "add-scenario" => commands::add_scenario::run(args_json).await,
            "add-schedule" => commands::add_schedule::run(args_json).await,
            "add-step" => commands::add_step::run(args_json).await,
            "add-tag-to-feature" => commands::add_tag_to_feature::run(args_json).await,
            "add-tag-to-scenario" => commands::add_tag_to_scenario::run(args_json).await,
            "add-virtual-hook" => commands::add_virtual_hook::run(args_json).await,
            "answer-question" => commands::answer_question::run(args_json).await,
            "audit-coverage" => commands::audit_coverage::run(args_json).await,
            "auto-advance" => commands::auto_advance::run(args_json).await,
            "board" => commands::board::run(args_json).await,
            "bootstrap" => commands::bootstrap::run(args_json).await,
            "check" => commands::check::run(args_json).await,
            "checkpoint" => commands::checkpoint::run(args_json).await,
            "cleanup-checkpoints" => commands::cleanup_checkpoints::run(args_json).await,
            "clear-dependencies" => commands::clear_dependencies::run(args_json).await,
            "clear-virtual-hooks" => commands::clear_virtual_hooks::run(args_json).await,
            "compact-work-unit" => commands::compact_work_unit::run(args_json).await,
            "compare-implementations" => commands::compare_implementations::run(args_json).await,
            "configure-tools" => commands::configure_tools::run(args_json).await,
            "copy-virtual-hooks" => commands::copy_virtual_hooks::run(args_json).await,
            "create-bug" => commands::create_bug::run(args_json).await,
            "create-epic" => commands::create_epic::run(args_json).await,
            "create-feature" => commands::create_feature::run(args_json).await,
            "create-prefix" => commands::create_prefix::run(args_json).await,
            "create-story" => commands::create_story::run(args_json).await,
            "create-task" => commands::create_task::run(args_json).await,
            "delete-diagram" => commands::delete_diagram::run(args_json).await,
            "delete-epic" => commands::delete_epic::run(args_json).await,
            "delete-features" => commands::delete_features::run(args_json).await,
            "delete-scenario" => commands::delete_scenario::run(args_json).await,
            "delete-scenarios" => commands::delete_scenarios::run(args_json).await,
            "delete-step" => commands::delete_step::run(args_json).await,
            "delete-tag" => commands::delete_tag::run(args_json).await,
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
            "list-attachments" => commands::list_attachments::run(args_json).await,
            "list-checkpoints" => commands::list_checkpoints::run(args_json).await,
<<<<<<< HEAD
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
=======
            "list-epics" => commands::list_epics::run(args_json).await,
            "list-feature-tags" => commands::list_feature_tags::run(args_json).await,
            "list-features" => commands::list_features::run(args_json).await,
            "list-foundation-sections" => commands::list_foundation_sections::run(args_json).await,
            "list-hooks" => commands::list_hooks::run(args_json).await,
            "list-prefixes" => commands::list_prefixes::run(args_json).await,
            "list-scenario-tags" => commands::list_scenario_tags::run(args_json).await,
            "list-schedules" => commands::list_schedules::run(args_json).await,
            "list-tags" => commands::list_tags::run(args_json).await,
            "list-virtual-hooks" => commands::list_virtual_hooks::run(args_json).await,
>>>>>>> parent of 6fa95633 (refactor: more commands refactored)
            // "list-work-units" — ported (RPC-253). Handled by `run_ported`
            // before reaching this match; intentionally absent here.
            "pause-schedule" => commands::pause_schedule::run(args_json).await,
            "prioritize-work-unit" => commands::prioritize_work_unit::run(args_json).await,
            "query-bottlenecks" => commands::query_bottlenecks::run(args_json).await,
            "query-dependency-stats" => commands::query_dependency_stats::run(args_json).await,
            "query-estimate-accuracy" => commands::query_estimate_accuracy::run(args_json).await,
            "query-estimation-guide" => commands::query_estimation_guide::run(args_json).await,
            "query-example-mapping-stats" => commands::query_example_mapping_stats::run(args_json).await,
            "query-metrics" => commands::query_metrics::run(args_json).await,
            "query-orphans" => commands::query_orphans::run(args_json).await,
            "query-work-units" => commands::query_work_units::run(args_json).await,
            "record-iteration" => commands::record_iteration::run(args_json).await,
            "register-tag" => commands::register_tag::run(args_json).await,
            "remove-aggregate-from-foundation" => commands::remove_aggregate_from_foundation::run(args_json).await,
            "remove-architecture-note" => commands::remove_architecture_note::run(args_json).await,
            "remove-attachment" => commands::remove_attachment::run(args_json).await,
            "remove-capability" => commands::remove_capability::run(args_json).await,
            "remove-command-from-foundation" => commands::remove_command_from_foundation::run(args_json).await,
            "remove-dependency" => commands::remove_dependency::run(args_json).await,
            "remove-domain-event-from-foundation" => commands::remove_domain_event_from_foundation::run(args_json).await,
            "remove-example" => commands::remove_example::run(args_json).await,
            "remove-foundation-bounded-context" => commands::remove_foundation_bounded_context::run(args_json).await,
            "remove-hook" => commands::remove_hook::run(args_json).await,
            "remove-init-files" => commands::remove_init_files::run(args_json).await,
            "remove-persona" => commands::remove_persona::run(args_json).await,
            "remove-question" => commands::remove_question::run(args_json).await,
            "remove-rule" => commands::remove_rule::run(args_json).await,
            "remove-schedule" => commands::remove_schedule::run(args_json).await,
            "remove-tag-from-feature" => commands::remove_tag_from_feature::run(args_json).await,
            "remove-tag-from-scenario" => commands::remove_tag_from_scenario::run(args_json).await,
            "remove-virtual-hook" => commands::remove_virtual_hook::run(args_json).await,
            "repair-work-units" => commands::repair_work_units::run(args_json).await,
            "report-bug-to-github" => commands::report_bug_to_github::run(args_json).await,
            "research" => commands::research::run(args_json).await,
            "restore-architecture-note" => commands::restore_architecture_note::run(args_json).await,
            "restore-checkpoint" => commands::restore_checkpoint::run(args_json).await,
            "restore-example" => commands::restore_example::run(args_json).await,
            "restore-question" => commands::restore_question::run(args_json).await,
            "restore-rule" => commands::restore_rule::run(args_json).await,
            "resume-schedule" => commands::resume_schedule::run(args_json).await,
            "retag" => commands::retag::run(args_json).await,
            "reverse" => commands::reverse::run(args_json).await,
            "review" => commands::review::run(args_json).await,
            "search-implementation" => commands::search_implementation::run(args_json).await,
            "search-scenarios" => commands::search_scenarios::run(args_json).await,
            "set-user-story" => commands::set_user_story::run(args_json).await,
            "show-acceptance-criteria" => commands::show_acceptance_criteria::run(args_json).await,
            "show-coverage" => commands::show_coverage::run(args_json).await,
            "show-deleted" => commands::show_deleted::run(args_json).await,
            "show-epic" => commands::show_epic::run(args_json).await,
            "show-event-storm" => commands::show_event_storm::run(args_json).await,
            "show-feature" => commands::show_feature::run(args_json).await,
            "show-foundation" => commands::show_foundation::run(args_json).await,
            "show-foundation-event-storm" => commands::show_foundation_event_storm::run(args_json).await,
            "show-test-patterns" => commands::show_test_patterns::run(args_json).await,
            "show-work-unit" => commands::show_work_unit::run(args_json).await,
            "suggest-dependencies" => commands::suggest_dependencies::run(args_json).await,
            "tag-stats" => commands::tag_stats::run(args_json).await,
            "unlink-coverage" => commands::unlink_coverage::run(args_json).await,
            "update-foundation" => commands::update_foundation::run(args_json).await,
            "update-prefix" => commands::update_prefix::run(args_json).await,
            "update-scenario" => commands::update_scenario::run(args_json).await,
            "update-step" => commands::update_step::run(args_json).await,
            "update-tag" => commands::update_tag::run(args_json).await,
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

