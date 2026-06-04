//! Canonical list of all 162 fspec CLI commands extracted from
//! `src/cli/program.ts` (TypeScript source of truth).
//!
//! Sourced from `spec/attachments/TOOL-019/canonical-commands.json`. Every
//! entry's `work_unit` is set to the placeholder `"RPC-PENDING"` (real RPC-165..RPC-326 IDs resolved per the
//! per-command mapping at spec/attachments/TOOL-019/command-to-rpc-mapping.json).

/// A single entry in the canonical command map.
#[derive(Debug, Clone, Copy)]
pub struct CanonicalCommand {
    /// kebab-case command name as registered by Commander.js in
    /// `src/cli/program.ts`.
    pub name: &'static str,
    /// Path (repo-relative) to the primary TypeScript source file
    /// implementing the command today.
    pub ts_file: &'static str,
    /// Work-unit ID tracking the Rust port of this command. `"RPC-PENDING"`
    /// until the per-command child cards under RPC-003 are created.
    pub work_unit: &'static str,
}

/// Every fspec CLI command registered today. Length MUST equal 162 (asserted
/// by `tests/canonical_list_test.rs`).
pub const CANONICAL_COMMANDS: &[CanonicalCommand] = &[
    CanonicalCommand { name: "add-aggregate", ts_file: "src/commands/add-aggregate.ts", work_unit: "RPC-165" },
    CanonicalCommand { name: "add-aggregate-to-foundation", ts_file: "src/commands/add-aggregate-to-foundation.ts", work_unit: "RPC-166" },
    CanonicalCommand { name: "add-architecture", ts_file: "src/commands/add-architecture.ts", work_unit: "RPC-167" },
    CanonicalCommand { name: "add-architecture-note", ts_file: "src/commands/add-architecture-note.ts", work_unit: "RPC-168" },
    CanonicalCommand { name: "add-assumption", ts_file: "src/commands/add-assumption.ts", work_unit: "RPC-169" },
    CanonicalCommand { name: "add-attachment", ts_file: "src/commands/add-attachment.ts", work_unit: "RPC-170" },
    CanonicalCommand { name: "add-background", ts_file: "src/commands/add-background.ts", work_unit: "RPC-171" },
    CanonicalCommand { name: "add-bounded-context", ts_file: "src/commands/add-bounded-context.ts", work_unit: "RPC-172" },
    CanonicalCommand { name: "add-capability", ts_file: "src/commands/register-add-capability.ts", work_unit: "RPC-173" },
    CanonicalCommand { name: "add-command", ts_file: "src/commands/add-command.ts", work_unit: "RPC-174" },
    CanonicalCommand { name: "add-command-to-foundation", ts_file: "src/commands/add-command-to-foundation.ts", work_unit: "RPC-175" },
    CanonicalCommand { name: "add-dependencies", ts_file: "src/commands/add-dependencies.ts", work_unit: "RPC-176" },
    CanonicalCommand { name: "add-dependency", ts_file: "src/commands/add-dependency.ts", work_unit: "RPC-177" },
    CanonicalCommand { name: "add-diagram", ts_file: "src/commands/add-diagram.ts", work_unit: "RPC-178" },
    CanonicalCommand { name: "add-domain-event", ts_file: "src/commands/add-domain-event.ts", work_unit: "RPC-179" },
    CanonicalCommand { name: "add-domain-event-to-foundation", ts_file: "src/commands/add-domain-event-to-foundation.ts", work_unit: "RPC-180" },
    CanonicalCommand { name: "add-example", ts_file: "src/commands/add-example.ts", work_unit: "RPC-181" },
    CanonicalCommand { name: "add-external-system", ts_file: "src/commands/add-external-system.ts", work_unit: "RPC-182" },
    CanonicalCommand { name: "add-foundation-bounded-context", ts_file: "src/commands/add-foundation-bounded-context.ts", work_unit: "RPC-183" },
    CanonicalCommand { name: "add-hook", ts_file: "src/commands/add-hook.ts", work_unit: "RPC-184" },
    CanonicalCommand { name: "add-hotspot", ts_file: "src/commands/add-hotspot.ts", work_unit: "RPC-185" },
    CanonicalCommand { name: "add-persona", ts_file: "src/commands/register-add-persona.ts", work_unit: "RPC-186" },
    CanonicalCommand { name: "add-policy", ts_file: "src/commands/add-policy.ts", work_unit: "RPC-187" },
    CanonicalCommand { name: "add-question", ts_file: "src/commands/add-question.ts", work_unit: "RPC-188" },
    CanonicalCommand { name: "add-rule", ts_file: "src/commands/add-rule.ts", work_unit: "RPC-189" },
    CanonicalCommand { name: "add-scenario", ts_file: "src/commands/add-scenario.ts", work_unit: "RPC-190" },
    CanonicalCommand { name: "add-schedule", ts_file: "src/commands/schedule/add-schedule.ts", work_unit: "RPC-191" },
    CanonicalCommand { name: "add-step", ts_file: "src/commands/add-step.ts", work_unit: "RPC-192" },
    CanonicalCommand { name: "add-tag-to-feature", ts_file: "src/commands/add-tag-to-feature.ts", work_unit: "RPC-193" },
    CanonicalCommand { name: "add-tag-to-scenario", ts_file: "src/commands/add-tag-to-scenario.ts", work_unit: "RPC-194" },
    CanonicalCommand { name: "add-virtual-hook", ts_file: "src/commands/add-virtual-hook.ts", work_unit: "RPC-195" },
    CanonicalCommand { name: "answer-question", ts_file: "src/commands/answer-question.ts", work_unit: "RPC-196" },
    CanonicalCommand { name: "audit-coverage", ts_file: "src/commands/audit-coverage.ts", work_unit: "RPC-197" },
    CanonicalCommand { name: "auto-advance", ts_file: "src/commands/auto-advance.ts", work_unit: "RPC-198" },
    CanonicalCommand { name: "board", ts_file: "src/commands/display-board.ts", work_unit: "RPC-199" },
    CanonicalCommand { name: "bootstrap", ts_file: "src/commands/bootstrap.ts", work_unit: "RPC-200" },
    CanonicalCommand { name: "check", ts_file: "src/commands/check.ts", work_unit: "RPC-201" },
    CanonicalCommand { name: "checkpoint", ts_file: "src/commands/checkpoint.ts", work_unit: "RPC-202" },
    CanonicalCommand { name: "cleanup-checkpoints", ts_file: "src/commands/cleanup-checkpoints.ts", work_unit: "RPC-203" },
    CanonicalCommand { name: "clear-dependencies", ts_file: "src/commands/clear-dependencies.ts", work_unit: "RPC-204" },
    CanonicalCommand { name: "clear-virtual-hooks", ts_file: "src/commands/clear-virtual-hooks.ts", work_unit: "RPC-205" },
    CanonicalCommand { name: "compact-work-unit", ts_file: "src/commands/compact-work-unit.ts", work_unit: "RPC-206" },
    CanonicalCommand { name: "compare-implementations", ts_file: "src/commands/compare-implementations.ts", work_unit: "RPC-207" },
    CanonicalCommand { name: "configure-tools", ts_file: "src/commands/configure-tools.ts", work_unit: "RPC-208" },
    CanonicalCommand { name: "copy-virtual-hooks", ts_file: "src/commands/copy-virtual-hooks.ts", work_unit: "RPC-209" },
    CanonicalCommand { name: "create-bug", ts_file: "src/commands/create-bug.ts", work_unit: "RPC-210" },
    CanonicalCommand { name: "create-epic", ts_file: "src/commands/create-epic.ts", work_unit: "RPC-211" },
    CanonicalCommand { name: "create-feature", ts_file: "src/commands/create-feature.ts", work_unit: "RPC-212" },
    CanonicalCommand { name: "create-prefix", ts_file: "src/commands/create-prefix.ts", work_unit: "RPC-213" },
    CanonicalCommand { name: "create-story", ts_file: "src/commands/create-story.ts", work_unit: "RPC-214" },
    CanonicalCommand { name: "create-task", ts_file: "src/commands/create-task.ts", work_unit: "RPC-215" },
    CanonicalCommand { name: "delete-diagram", ts_file: "src/commands/delete-diagram.ts", work_unit: "RPC-216" },
    CanonicalCommand { name: "delete-epic", ts_file: "src/commands/delete-epic.ts", work_unit: "RPC-217" },
    CanonicalCommand { name: "delete-features", ts_file: "src/commands/delete-features-by-tag.ts", work_unit: "RPC-218" },
    CanonicalCommand { name: "delete-scenario", ts_file: "src/commands/delete-scenario.ts", work_unit: "RPC-219" },
    CanonicalCommand { name: "delete-scenarios", ts_file: "src/commands/delete-scenarios-by-tag.ts", work_unit: "RPC-220" },
    CanonicalCommand { name: "delete-step", ts_file: "src/commands/delete-step.ts", work_unit: "RPC-221" },
    CanonicalCommand { name: "delete-tag", ts_file: "src/commands/delete-tag.ts", work_unit: "RPC-222" },
    CanonicalCommand { name: "delete-work-unit", ts_file: "src/commands/delete-work-unit.ts", work_unit: "RPC-223" },
    CanonicalCommand { name: "dependencies", ts_file: "src/commands/dependencies.ts", work_unit: "RPC-224" },
    CanonicalCommand { name: "discover-event-storm", ts_file: "src/commands/discover-event-storm.ts", work_unit: "RPC-225" },
    CanonicalCommand { name: "discover-foundation", ts_file: "src/commands/discover-foundation.ts", work_unit: "RPC-226" },
    CanonicalCommand { name: "export-dependencies", ts_file: "src/commands/export-dependencies.ts", work_unit: "RPC-227" },
    CanonicalCommand { name: "export-example-map", ts_file: "src/commands/export-example-map.ts", work_unit: "RPC-228" },
    CanonicalCommand { name: "export-work-units", ts_file: "src/commands/export-work-units.ts", work_unit: "RPC-229" },
    CanonicalCommand { name: "format", ts_file: "src/commands/format.ts", work_unit: "RPC-230" },
    CanonicalCommand { name: "generate-coverage", ts_file: "src/commands/generate-coverage.ts", work_unit: "RPC-231" },
    CanonicalCommand { name: "generate-example-mapping-from-event-storm", ts_file: "src/commands/generate-example-mapping-from-event-storm.ts", work_unit: "RPC-232" },
    CanonicalCommand { name: "generate-foundation-md", ts_file: "src/commands/generate-foundation-md.ts", work_unit: "RPC-233" },
    CanonicalCommand { name: "generate-scenarios", ts_file: "src/commands/generate-scenarios.ts", work_unit: "RPC-234" },
    CanonicalCommand { name: "generate-summary-report", ts_file: "src/commands/generate-summary-report.ts", work_unit: "RPC-235" },
    CanonicalCommand { name: "generate-tags-md", ts_file: "src/commands/generate-tags-md.ts", work_unit: "RPC-236" },
    CanonicalCommand { name: "get-scenarios", ts_file: "src/commands/get-scenarios.ts", work_unit: "RPC-237" },
    CanonicalCommand { name: "import-example-map", ts_file: "src/commands/import-example-map.ts", work_unit: "RPC-238" },
    CanonicalCommand { name: "init", ts_file: "src/commands/init.ts", work_unit: "RPC-239" },
    CanonicalCommand { name: "link-coverage", ts_file: "src/commands/link-coverage.ts", work_unit: "RPC-240" },
    CanonicalCommand { name: "list-attachments", ts_file: "src/commands/list-attachments.ts", work_unit: "RPC-241" },
    CanonicalCommand { name: "list-checkpoints", ts_file: "src/commands/list-checkpoints.ts", work_unit: "RPC-242" },
    CanonicalCommand { name: "list-epics", ts_file: "src/commands/list-epics.ts", work_unit: "RPC-243" },
    CanonicalCommand { name: "list-feature-tags", ts_file: "src/commands/list-feature-tags.ts", work_unit: "RPC-244" },
    CanonicalCommand { name: "list-features", ts_file: "src/commands/list-features.ts", work_unit: "RPC-245" },
    CanonicalCommand { name: "list-foundation-sections", ts_file: "src/commands/list-foundation-sections.ts", work_unit: "RPC-246" },
    CanonicalCommand { name: "list-hooks", ts_file: "src/commands/list-hooks.ts", work_unit: "RPC-247" },
    CanonicalCommand { name: "list-prefixes", ts_file: "src/commands/list-prefixes.ts", work_unit: "RPC-248" },
    CanonicalCommand { name: "list-scenario-tags", ts_file: "src/commands/list-scenario-tags.ts", work_unit: "RPC-249" },
    CanonicalCommand { name: "list-schedules", ts_file: "src/commands/schedule/list-schedules.ts", work_unit: "RPC-250" },
    CanonicalCommand { name: "list-tags", ts_file: "src/commands/list-tags.ts", work_unit: "RPC-251" },
    CanonicalCommand { name: "list-virtual-hooks", ts_file: "src/commands/list-virtual-hooks.ts", work_unit: "RPC-252" },
    CanonicalCommand { name: "list-work-units", ts_file: "src/commands/list-work-units.ts", work_unit: "RPC-253" },
    CanonicalCommand { name: "pause-schedule", ts_file: "src/commands/schedule/pause-schedule.ts", work_unit: "RPC-254" },
    CanonicalCommand { name: "prioritize-work-unit", ts_file: "src/commands/prioritize-work-unit.ts", work_unit: "RPC-255" },
    CanonicalCommand { name: "query-bottlenecks", ts_file: "src/commands/query-bottlenecks.ts", work_unit: "RPC-256" },
    CanonicalCommand { name: "query-dependency-stats", ts_file: "src/commands/query-dependency-stats.ts", work_unit: "RPC-257" },
    CanonicalCommand { name: "query-estimate-accuracy", ts_file: "src/commands/query-estimate-accuracy.ts", work_unit: "RPC-258" },
    CanonicalCommand { name: "query-estimation-guide", ts_file: "src/commands/query-estimation-guide.ts", work_unit: "RPC-259" },
    CanonicalCommand { name: "query-example-mapping-stats", ts_file: "src/commands/query-example-mapping-stats.ts", work_unit: "RPC-260" },
    CanonicalCommand { name: "query-metrics", ts_file: "src/commands/query-metrics.ts", work_unit: "RPC-261" },
    CanonicalCommand { name: "query-orphans", ts_file: "src/commands/query-orphans.ts", work_unit: "RPC-262" },
    CanonicalCommand { name: "query-work-units", ts_file: "src/commands/query-work-units.ts", work_unit: "RPC-263" },
    CanonicalCommand { name: "record-iteration", ts_file: "src/commands/record-iteration.ts", work_unit: "RPC-264" },
    CanonicalCommand { name: "register-tag", ts_file: "src/commands/register-tag.ts", work_unit: "RPC-265" },
    CanonicalCommand { name: "remove-aggregate-from-foundation", ts_file: "src/commands/remove-aggregate-from-foundation.ts", work_unit: "RPC-266" },
    CanonicalCommand { name: "remove-architecture-note", ts_file: "src/commands/remove-architecture-note.ts", work_unit: "RPC-267" },
    CanonicalCommand { name: "remove-attachment", ts_file: "src/commands/remove-attachment.ts", work_unit: "RPC-268" },
    CanonicalCommand { name: "remove-capability", ts_file: "src/commands/register-remove-capability.ts", work_unit: "RPC-269" },
    CanonicalCommand { name: "remove-command-from-foundation", ts_file: "src/commands/remove-command-from-foundation.ts", work_unit: "RPC-270" },
    CanonicalCommand { name: "remove-dependency", ts_file: "src/commands/remove-dependency.ts", work_unit: "RPC-271" },
    CanonicalCommand { name: "remove-domain-event-from-foundation", ts_file: "src/commands/remove-domain-event-from-foundation.ts", work_unit: "RPC-272" },
    CanonicalCommand { name: "remove-example", ts_file: "src/commands/remove-example.ts", work_unit: "RPC-273" },
    CanonicalCommand { name: "remove-foundation-bounded-context", ts_file: "src/commands/remove-foundation-bounded-context.ts", work_unit: "RPC-274" },
    CanonicalCommand { name: "remove-hook", ts_file: "src/commands/remove-hook.ts", work_unit: "RPC-275" },
    CanonicalCommand { name: "remove-init-files", ts_file: "src/commands/remove-init-files.ts", work_unit: "RPC-276" },
    CanonicalCommand { name: "remove-persona", ts_file: "src/commands/register-remove-persona.ts", work_unit: "RPC-277" },
    CanonicalCommand { name: "remove-question", ts_file: "src/commands/remove-question.ts", work_unit: "RPC-278" },
    CanonicalCommand { name: "remove-rule", ts_file: "src/commands/remove-rule.ts", work_unit: "RPC-279" },
    CanonicalCommand { name: "remove-schedule", ts_file: "src/commands/schedule/remove-schedule.ts", work_unit: "RPC-280" },
    CanonicalCommand { name: "remove-tag-from-feature", ts_file: "src/commands/remove-tag-from-feature.ts", work_unit: "RPC-281" },
    CanonicalCommand { name: "remove-tag-from-scenario", ts_file: "src/commands/remove-tag-from-scenario.ts", work_unit: "RPC-282" },
    CanonicalCommand { name: "remove-virtual-hook", ts_file: "src/commands/remove-virtual-hook.ts", work_unit: "RPC-283" },
    CanonicalCommand { name: "repair-work-units", ts_file: "src/commands/repair-work-units.ts", work_unit: "RPC-284" },
    CanonicalCommand { name: "report-bug-to-github", ts_file: "src/commands/report-bug-to-github.ts", work_unit: "RPC-285" },
    CanonicalCommand { name: "research", ts_file: "src/commands/research.ts", work_unit: "RPC-286" },
    CanonicalCommand { name: "restore-architecture-note", ts_file: "src/commands/restore-architecture-note.ts", work_unit: "RPC-287" },
    CanonicalCommand { name: "restore-checkpoint", ts_file: "src/commands/restore-checkpoint.ts", work_unit: "RPC-288" },
    CanonicalCommand { name: "restore-example", ts_file: "src/commands/restore-example.ts", work_unit: "RPC-289" },
    CanonicalCommand { name: "restore-question", ts_file: "src/commands/restore-question.ts", work_unit: "RPC-290" },
    CanonicalCommand { name: "restore-rule", ts_file: "src/commands/restore-rule.ts", work_unit: "RPC-291" },
    CanonicalCommand { name: "resume-schedule", ts_file: "src/commands/schedule/pause-schedule.ts", work_unit: "RPC-292" },
    CanonicalCommand { name: "retag", ts_file: "src/commands/retag.ts", work_unit: "RPC-293" },
    CanonicalCommand { name: "reverse", ts_file: "src/commands/reverse.ts", work_unit: "RPC-294" },
    CanonicalCommand { name: "review", ts_file: "src/commands/review.ts", work_unit: "RPC-295" },
    CanonicalCommand { name: "search-implementation", ts_file: "src/commands/search-implementation.ts", work_unit: "RPC-296" },
    CanonicalCommand { name: "search-scenarios", ts_file: "src/commands/search-scenarios.ts", work_unit: "RPC-297" },
    CanonicalCommand { name: "set-user-story", ts_file: "src/commands/set-user-story.ts", work_unit: "RPC-298" },
    CanonicalCommand { name: "show-acceptance-criteria", ts_file: "src/commands/show-acceptance-criteria.ts", work_unit: "RPC-299" },
    CanonicalCommand { name: "show-coverage", ts_file: "src/commands/show-coverage.ts", work_unit: "RPC-300" },
    CanonicalCommand { name: "show-deleted", ts_file: "src/commands/show-deleted.ts", work_unit: "RPC-301" },
    CanonicalCommand { name: "show-epic", ts_file: "src/commands/show-epic.ts", work_unit: "RPC-302" },
    CanonicalCommand { name: "show-event-storm", ts_file: "src/commands/show-event-storm.ts", work_unit: "RPC-303" },
    CanonicalCommand { name: "show-feature", ts_file: "src/commands/show-feature.ts", work_unit: "RPC-304" },
    CanonicalCommand { name: "show-foundation", ts_file: "src/commands/show-foundation.ts", work_unit: "RPC-305" },
    CanonicalCommand { name: "show-foundation-event-storm", ts_file: "src/commands/show-foundation-event-storm.ts", work_unit: "RPC-306" },
    CanonicalCommand { name: "show-test-patterns", ts_file: "src/commands/show-test-patterns.ts", work_unit: "RPC-307" },
    CanonicalCommand { name: "show-work-unit", ts_file: "src/commands/show-work-unit.ts", work_unit: "RPC-308" },
    CanonicalCommand { name: "suggest-dependencies", ts_file: "src/commands/suggest-dependencies.ts", work_unit: "RPC-309" },
    CanonicalCommand { name: "tag-stats", ts_file: "src/commands/tag-stats.ts", work_unit: "RPC-310" },
    CanonicalCommand { name: "unlink-coverage", ts_file: "src/commands/unlink-coverage.ts", work_unit: "RPC-311" },
    CanonicalCommand { name: "update-foundation", ts_file: "src/commands/update-foundation.ts", work_unit: "RPC-312" },
    CanonicalCommand { name: "update-prefix", ts_file: "src/commands/update-prefix.ts", work_unit: "RPC-313" },
    CanonicalCommand { name: "update-scenario", ts_file: "src/commands/update-scenario.ts", work_unit: "RPC-314" },
    CanonicalCommand { name: "update-step", ts_file: "src/commands/update-step.ts", work_unit: "RPC-315" },
    CanonicalCommand { name: "update-tag", ts_file: "src/commands/update-tag.ts", work_unit: "RPC-316" },
    CanonicalCommand { name: "update-work-unit", ts_file: "src/commands/update-work-unit.ts", work_unit: "RPC-317" },
    CanonicalCommand { name: "update-work-unit-estimate", ts_file: "src/commands/update-work-unit-estimate.ts", work_unit: "RPC-318" },
    CanonicalCommand { name: "update-work-unit-status", ts_file: "src/commands/update-work-unit-status.ts", work_unit: "RPC-319" },
    CanonicalCommand { name: "validate", ts_file: "src/commands/validate.ts", work_unit: "RPC-320" },
    CanonicalCommand { name: "validate-foundation-schema", ts_file: "src/commands/validate-foundation-schema.ts", work_unit: "RPC-321" },
    CanonicalCommand { name: "validate-hooks", ts_file: "src/commands/validate-hooks.ts", work_unit: "RPC-322" },
    CanonicalCommand { name: "validate-spec-alignment", ts_file: "src/commands/validate-spec-alignment.ts", work_unit: "RPC-323" },
    CanonicalCommand { name: "validate-tags", ts_file: "src/commands/validate-tags.ts", work_unit: "RPC-324" },
    CanonicalCommand { name: "validate-work-units", ts_file: "src/commands/validate-work-units.ts", work_unit: "RPC-325" },
    CanonicalCommand { name: "workflow-automation", ts_file: "src/commands/workflow-automation.ts", work_unit: "RPC-326" },
];

/// Look up a command by kebab-case name. Returns `None` for unknown names.
pub fn lookup(name: &str) -> Option<&'static CanonicalCommand> {
    CANONICAL_COMMANDS.iter().find(|c| c.name == name)
}

/// Commands that have a real Rust implementation. This set grows
/// monotonically as RPC-XXX child cards land. Tests that assert phase-1 stub
/// invariants (e.g. `every_canonical_command_has_a_module_or_is_stubbed`)
/// MUST consult this list to know which commands are exempt from the stub
/// shape — keeping the source of truth in one place.
pub const PORTED_COMMANDS: &[&str] = &[
    "list-work-units",  // RPC-253
    "list-prefixes",    // RPC-248
    "list-epics",       // RPC-243
    "list-tags",        // RPC-251
    "list-features",    // RPC-245
    "list-attachments", // RPC-241
    "list-hooks",       // RPC-247
];

/// True when the named command has a real Rust port (i.e. NOT a stub).
pub fn is_ported(name: &str) -> bool {
    PORTED_COMMANDS.contains(&name)
}
