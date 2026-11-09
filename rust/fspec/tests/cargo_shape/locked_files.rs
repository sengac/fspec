//! The `.rs` files that must exist under `rust/fspec/src/` (RPC-010).

/// The `.rs` files that must exist directly under `rust/fspec/src/`.
///
///
/// RPC-011 rule [24] / architecture note [7] add `status.rs` as a
/// fifth subcommand sibling alongside `client.rs` and `daemon.rs`.
/// RPC-253 (list-work-units CLI port) adds `list_work_units.rs` as a
/// sixth subcommand sibling — the shell-facing bridge that delegates
/// to `fspec_core::commands::list_work_units::run`.
/// RPC-248 (list-prefixes CLI port) adds `list_prefixes.rs` as a
/// seventh subcommand sibling — the shell-facing bridge that
/// delegates to `fspec_core::commands::list_prefixes::run`.
/// RPC-243/251/245/241/247 (list-epics, list-tags, list-features,
/// list-attachments, list-hooks CLI ports) add the 5 corresponding
/// bridges. The lock-list grows from 5 → 6 → 7 → 8 → 13. RPC-246
/// and RPC-250 (list-foundation-sections, list-schedules CLI ports)
/// add 2 more (→15). RPC-244, RPC-249, RPC-252 (list-feature-tags,
/// list-scenario-tags, list-virtual-hooks) add the final 3 →18.
/// Batch 6 (RPC-242/301/302/304/310) adds 5 more (→23): list-checkpoints,
/// show-deleted, show-epic, show-feature, tag-stats.
/// Batch 6B (RPC-308/257/258/261/263) adds 5 more (→28): show-work-unit,
/// query-dependency-stats, query-estimate-accuracy, query-metrics, query-work-units.
/// Batch 6C (RPC-256/259/260/262/299/300/303/305/306/307) adds 10 more (→38):
/// query-bottlenecks, query-estimation-guide, query-example-mapping-stats,
/// query-orphans, show-acceptance-criteria (removed by CLI-019), show-coverage, show-event-storm,
/// show-foundation, show-foundation-event-storm, show-test-patterns.
/// Batch 7 (RPC-211/217/213/313/265/316/222/176/271/204) adds 10 more (→48):
/// create-epic, delete-epic, create-prefix, update-prefix, register-tag,
/// update-tag, delete-tag, add-dependencies, remove-dependency,
/// clear-dependencies.
/// Batch 8 (RPC-189/279/169/181/273/188/278/168/267/298) adds 10 more (→58):
/// add-rule, remove-rule, add-assumption, add-example, remove-example,
/// add-question, remove-question, add-architecture-note,
/// remove-architecture-note, set-user-story.
/// Batch 9 (RPC-177/196/289/291/290/287/193/281/194/282) adds 10 more (→68):
/// add-dependency, answer-question, restore-example, restore-rule,
/// restore-question, restore-architecture-note, add-tag-to-feature,
/// remove-tag-from-feature, add-tag-to-scenario, remove-tag-from-scenario.
/// Batch 10 (RPC-170/268/195/283/205/209/184/275/178/216) adds 10 more (→78):
/// add-attachment, remove-attachment, add-virtual-hook, remove-virtual-hook,
/// clear-virtual-hooks, copy-virtual-hooks, add-hook, remove-hook,
/// add-diagram, delete-diagram.
pub const LOCKED_FILES: &[&str] = &[
    "main.rs",
    "combined.rs",
    "daemon.rs",
    "client.rs",
    "common.rs",
    "status.rs",
    // PROV-097: dotenv-at-startup seam module.
    "startup_env.rs",
    "list_work_units.rs",
    "list_prefixes.rs",
    "list_epics.rs",
    "list_tags.rs",
    "list_features.rs",
    "list_attachments.rs",
    "list_hooks.rs",
    "list_foundation_sections.rs",
    "list_schedules.rs",
    "list_feature_tags.rs",
    "list_scenario_tags.rs",
    "list_virtual_hooks.rs",
    "list_checkpoints.rs",
    "show_deleted.rs",
    "show_epic.rs",
    "show_feature.rs",
    "show_work_unit.rs",
    "tag_stats.rs",
    "query_dependency_stats.rs",
    "query_estimate_accuracy.rs",
    "query_metrics.rs",
    "query_work_units.rs",
    "query_bottlenecks.rs",
    "query_estimation_guide.rs",
    "query_example_mapping_stats.rs",
    "query_orphans.rs",
    "show_coverage.rs",
    "show_event_storm.rs",
    "show_foundation.rs",
    "show_foundation_event_storm.rs",
    "show_test_patterns.rs",
    // Batch 7
    "create_epic.rs",
    "delete_epic.rs",
    "create_prefix.rs",
    "update_prefix.rs",
    "register_tag.rs",
    "update_tag.rs",
    "delete_tag.rs",
    "add_dependencies.rs",
    "remove_dependency.rs",
    "clear_dependencies.rs",
    // Batch 8 (Example Mapping mutation)
    "add_rule.rs",
    "remove_rule.rs",
    "add_assumption.rs",
    "add_example.rs",
    "remove_example.rs",
    "add_question.rs",
    "remove_question.rs",
    "add_architecture_note.rs",
    "remove_architecture_note.rs",
    "set_user_story.rs",
    // Batch 9 (dependencies, q&a, tag-feature, tag-scenario, restore-*)
    "add_dependency.rs",
    "answer_question.rs",
    "restore_example.rs",
    "restore_rule.rs",
    "restore_question.rs",
    "restore_architecture_note.rs",
    "add_tag_to_feature.rs",
    "remove_tag_from_feature.rs",
    "add_tag_to_scenario.rs",
    "remove_tag_from_scenario.rs",
    // Batch 10 (attachments, virtual hooks, hooks, diagrams)
    "add_attachment.rs",
    "remove_attachment.rs",
    "add_virtual_hook.rs",
    "remove_virtual_hook.rs",
    "clear_virtual_hooks.rs",
    "copy_virtual_hooks.rs",
    "add_hook.rs",
    "remove_hook.rs",
    "add_diagram.rs",
    "delete_diagram.rs",
    // Batch 11 (Event Storm item-add + create-*)
    "add_aggregate.rs",
    "add_command.rs",
    "add_domain_event.rs",
    "add_hotspot.rs",
    "add_bounded_context.rs",
    "add_external_system.rs",
    "add_policy.rs",
    "create_story.rs",
    "create_bug.rs",
    "create_task.rs",
    // Batch 12 (work-units.json mutation + export)
    "update_work_unit.rs",
    "update_work_unit_estimate.rs",
    "delete_work_unit.rs",
    "compact_work_unit.rs",
    "prioritize_work_unit.rs",
    "repair_work_units.rs",
    "record_iteration.rs",
    "export_work_units.rs",
    "export_example_map.rs",
    "export_dependencies.rs",
    // Batch 13 (foundation mutation)
    "add_capability.rs",
    "remove_capability.rs",
    "add_persona.rs",
    "remove_persona.rs",
    "add_foundation_bounded_context.rs",
    "remove_foundation_bounded_context.rs",
    "add_aggregate_to_foundation.rs",
    "remove_aggregate_from_foundation.rs",
    "add_command_to_foundation.rs",
    "remove_command_from_foundation.rs",
    // DISC-003: foundation-status CLI bridge (Rust-only extension
    // command; read-only progress report for foundation discovery).
    "foundation_status.rs",
    // CONFIG-009: validate-config CLI bridge (Rust-only extension
    // command; user-scope config key-ownership validation).
    "validate_config.rs",
];
