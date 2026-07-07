//! Static kebab-command-name → `&'static CommandHelpConfig` lookup table.
//!
//! Mirrors every module registered in [`crate::help::configs`] (see
//! `help/configs/mod.rs`). Kept separate from [`crate::help_dispatch`] so the
//! large explicit match stays isolated and both files remain under 300 LoC.
//!
//! Commands that intentionally ship NO `CommandHelpConfig` (`register-tag`,
//! `board`, `delete-features`, `delete-scenarios`, `list-foundation-sections`)
//! are absent here on purpose — the caller degrades them gracefully via the
//! canonical-command check in [`crate::help_dispatch`].
//!
//! Feature: spec/features/fspec-tool-help-dispatch.feature (RPC-414)

use crate::help::CommandHelpConfig;

/// Resolve a kebab-case command name to its help configuration, if one exists.
pub(crate) fn config_for(name: &str) -> Option<&'static CommandHelpConfig> {
    match name {
        "add-aggregate"                            => Some(&crate::help::configs::add_aggregate::CONFIG),
        "add-aggregate-to-foundation"              => Some(&crate::help::configs::add_aggregate_to_foundation::CONFIG),
        "add-architecture"                         => Some(&crate::help::configs::add_architecture::CONFIG),
        "add-architecture-note"                    => Some(&crate::help::configs::add_architecture_note::CONFIG),
        "add-assumption"                           => Some(&crate::help::configs::add_assumption::CONFIG),
        "add-attachment"                           => Some(&crate::help::configs::add_attachment::CONFIG),
        "add-background"                           => Some(&crate::help::configs::add_background::CONFIG),
        "add-bounded-context"                      => Some(&crate::help::configs::add_bounded_context::CONFIG),
        "add-capability"                           => Some(&crate::help::configs::add_capability::CONFIG),
        "add-command"                              => Some(&crate::help::configs::add_command::CONFIG),
        "add-command-to-foundation"                => Some(&crate::help::configs::add_command_to_foundation::CONFIG),
        "add-dependencies"                         => Some(&crate::help::configs::add_dependencies::CONFIG),
        "add-dependency"                           => Some(&crate::help::configs::add_dependency::CONFIG),
        "add-diagram"                              => Some(&crate::help::configs::add_diagram::CONFIG),
        "add-domain-event"                         => Some(&crate::help::configs::add_domain_event::CONFIG),
        "add-domain-event-to-foundation"           => Some(&crate::help::configs::add_domain_event_to_foundation::CONFIG),
        "add-example"                              => Some(&crate::help::configs::add_example::CONFIG),
        "add-external-system"                      => Some(&crate::help::configs::add_external_system::CONFIG),
        "add-foundation-bounded-context"           => Some(&crate::help::configs::add_foundation_bounded_context::CONFIG),
        "add-hook"                                 => Some(&crate::help::configs::add_hook::CONFIG),
        "add-hotspot"                              => Some(&crate::help::configs::add_hotspot::CONFIG),
        "add-persona"                              => Some(&crate::help::configs::add_persona::CONFIG),
        "add-policy"                               => Some(&crate::help::configs::add_policy::CONFIG),
        "add-question"                             => Some(&crate::help::configs::add_question::CONFIG),
        "add-rule"                                 => Some(&crate::help::configs::add_rule::CONFIG),
        "add-scenario"                             => Some(&crate::help::configs::add_scenario::CONFIG),
        "add-schedule"                             => Some(&crate::help::configs::add_schedule::CONFIG),
        "add-step"                                 => Some(&crate::help::configs::add_step::CONFIG),
        "add-tag-to-feature"                       => Some(&crate::help::configs::add_tag_to_feature::CONFIG),
        "add-tag-to-scenario"                      => Some(&crate::help::configs::add_tag_to_scenario::CONFIG),
        "add-virtual-hook"                         => Some(&crate::help::configs::add_virtual_hook::CONFIG),
        "answer-question"                          => Some(&crate::help::configs::answer_question::CONFIG),
        "audit-coverage"                           => Some(&crate::help::configs::audit_coverage::CONFIG),
        "auto-advance"                             => Some(&crate::help::configs::auto_advance::CONFIG),
        "bootstrap"                                => Some(&crate::help::configs::bootstrap::CONFIG),
        "check"                                    => Some(&crate::help::configs::check::CONFIG),
        "checkpoint"                               => Some(&crate::help::configs::checkpoint::CONFIG),
        "cleanup-checkpoints"                      => Some(&crate::help::configs::cleanup_checkpoints::CONFIG),
        "clear-dependencies"                       => Some(&crate::help::configs::clear_dependencies::CONFIG),
        "clear-virtual-hooks"                      => Some(&crate::help::configs::clear_virtual_hooks::CONFIG),
        "compact-work-unit"                        => Some(&crate::help::configs::compact_work_unit::CONFIG),
        "compare-implementations"                  => Some(&crate::help::configs::compare_implementations::CONFIG),
        "configure-tools"                          => Some(&crate::help::configs::configure_tools::CONFIG),
        "copy-virtual-hooks"                       => Some(&crate::help::configs::copy_virtual_hooks::CONFIG),
        "create-bug"                               => Some(&crate::help::configs::create_bug::CONFIG),
        "create-epic"                              => Some(&crate::help::configs::create_epic::CONFIG),
        "create-feature"                           => Some(&crate::help::configs::create_feature::CONFIG),
        "create-prefix"                            => Some(&crate::help::configs::create_prefix::CONFIG),
        "create-story"                             => Some(&crate::help::configs::create_story::CONFIG),
        "create-task"                              => Some(&crate::help::configs::create_task::CONFIG),
        "delete-diagram"                           => Some(&crate::help::configs::delete_diagram::CONFIG),
        "delete-epic"                              => Some(&crate::help::configs::delete_epic::CONFIG),
        "delete-scenario"                          => Some(&crate::help::configs::delete_scenario::CONFIG),
        "delete-step"                              => Some(&crate::help::configs::delete_step::CONFIG),
        "delete-tag"                               => Some(&crate::help::configs::delete_tag::CONFIG),
        "delete-work-unit"                         => Some(&crate::help::configs::delete_work_unit::CONFIG),
        "dependencies"                             => Some(&crate::help::configs::dependencies::CONFIG),
        "discover-event-storm"                     => Some(&crate::help::configs::discover_event_storm::CONFIG),
        "discover-foundation"                      => Some(&crate::help::configs::discover_foundation::CONFIG),
        "export-dependencies"                      => Some(&crate::help::configs::export_dependencies::CONFIG),
        "export-example-map"                       => Some(&crate::help::configs::export_example_map::CONFIG),
        "export-work-units"                        => Some(&crate::help::configs::export_work_units::CONFIG),
        "format"                                   => Some(&crate::help::configs::format::CONFIG),
        "generate-coverage"                        => Some(&crate::help::configs::generate_coverage::CONFIG),
        "generate-example-mapping-from-event-storm" => Some(&crate::help::configs::generate_example_mapping_from_event_storm::CONFIG),
        "generate-foundation-md"                   => Some(&crate::help::configs::generate_foundation_md::CONFIG),
        "generate-scenarios"                       => Some(&crate::help::configs::generate_scenarios::CONFIG),
        "generate-summary-report"                  => Some(&crate::help::configs::generate_summary_report::CONFIG),
        "generate-tags-md"                         => Some(&crate::help::configs::generate_tags_md::CONFIG),
        "get-scenarios"                            => Some(&crate::help::configs::get_scenarios::CONFIG),
        "import-example-map"                       => Some(&crate::help::configs::import_example_map::CONFIG),
        "init"                                     => Some(&crate::help::configs::init::CONFIG),
        "link-coverage"                            => Some(&crate::help::configs::link_coverage::CONFIG),
        "list-attachments"                         => Some(&crate::help::configs::list_attachments::CONFIG),
        "list-checkpoints"                         => Some(&crate::help::configs::list_checkpoints::CONFIG),
        "list-epics"                               => Some(&crate::help::configs::list_epics::CONFIG),
        "list-features"                            => Some(&crate::help::configs::list_features::CONFIG),
        "list-feature-tags"                        => Some(&crate::help::configs::list_feature_tags::CONFIG),
        "list-hooks"                               => Some(&crate::help::configs::list_hooks::CONFIG),
        "list-prefixes"                            => Some(&crate::help::configs::list_prefixes::CONFIG),
        "list-scenario-tags"                       => Some(&crate::help::configs::list_scenario_tags::CONFIG),
        "list-schedules"                           => Some(&crate::help::configs::list_schedules::CONFIG),
        "list-tags"                                => Some(&crate::help::configs::list_tags::CONFIG),
        "list-virtual-hooks"                       => Some(&crate::help::configs::list_virtual_hooks::CONFIG),
        "list-work-units"                          => Some(&crate::help::configs::list_work_units::CONFIG),
        "pause-schedule"                           => Some(&crate::help::configs::pause_schedule::CONFIG),
        "prioritize-work-unit"                     => Some(&crate::help::configs::prioritize_work_unit::CONFIG),
        "query-bottlenecks"                        => Some(&crate::help::configs::query_bottlenecks::CONFIG),
        "query-dependency-stats"                   => Some(&crate::help::configs::query_dependency_stats::CONFIG),
        "query-estimate-accuracy"                  => Some(&crate::help::configs::query_estimate_accuracy::CONFIG),
        "query-estimation-guide"                   => Some(&crate::help::configs::query_estimation_guide::CONFIG),
        "query-example-mapping-stats"              => Some(&crate::help::configs::query_example_mapping_stats::CONFIG),
        "query-metrics"                            => Some(&crate::help::configs::query_metrics::CONFIG),
        "query-orphans"                            => Some(&crate::help::configs::query_orphans::CONFIG),
        "query-work-units"                         => Some(&crate::help::configs::query_work_units::CONFIG),
        "record-iteration"                         => Some(&crate::help::configs::record_iteration::CONFIG),
        "remove-aggregate-from-foundation"         => Some(&crate::help::configs::remove_aggregate_from_foundation::CONFIG),
        "remove-architecture-note"                 => Some(&crate::help::configs::remove_architecture_note::CONFIG),
        "remove-attachment"                        => Some(&crate::help::configs::remove_attachment::CONFIG),
        "remove-capability"                        => Some(&crate::help::configs::remove_capability::CONFIG),
        "remove-command-from-foundation"           => Some(&crate::help::configs::remove_command_from_foundation::CONFIG),
        "remove-dependency"                        => Some(&crate::help::configs::remove_dependency::CONFIG),
        "remove-domain-event-from-foundation"      => Some(&crate::help::configs::remove_domain_event_from_foundation::CONFIG),
        "remove-example"                           => Some(&crate::help::configs::remove_example::CONFIG),
        "remove-foundation-bounded-context"        => Some(&crate::help::configs::remove_foundation_bounded_context::CONFIG),
        "remove-hook"                              => Some(&crate::help::configs::remove_hook::CONFIG),
        "remove-init-files"                        => Some(&crate::help::configs::remove_init_files::CONFIG),
        "remove-persona"                           => Some(&crate::help::configs::remove_persona::CONFIG),
        "remove-question"                          => Some(&crate::help::configs::remove_question::CONFIG),
        "remove-rule"                              => Some(&crate::help::configs::remove_rule::CONFIG),
        "remove-schedule"                          => Some(&crate::help::configs::remove_schedule::CONFIG),
        "remove-tag-from-feature"                  => Some(&crate::help::configs::remove_tag_from_feature::CONFIG),
        "remove-tag-from-scenario"                 => Some(&crate::help::configs::remove_tag_from_scenario::CONFIG),
        "remove-virtual-hook"                      => Some(&crate::help::configs::remove_virtual_hook::CONFIG),
        "repair-work-units"                        => Some(&crate::help::configs::repair_work_units::CONFIG),
        "report-bug-to-github"                     => Some(&crate::help::configs::report_bug_to_github::CONFIG),
        "research"                                 => Some(&crate::help::configs::research::CONFIG),
        "restore-architecture-note"                => Some(&crate::help::configs::restore_architecture_note::CONFIG),
        "restore-checkpoint"                       => Some(&crate::help::configs::restore_checkpoint::CONFIG),
        "restore-example"                          => Some(&crate::help::configs::restore_example::CONFIG),
        "restore-question"                         => Some(&crate::help::configs::restore_question::CONFIG),
        "restore-rule"                             => Some(&crate::help::configs::restore_rule::CONFIG),
        "resume-schedule"                          => Some(&crate::help::configs::resume_schedule::CONFIG),
        "retag"                                    => Some(&crate::help::configs::retag::CONFIG),
        "reverse"                                  => Some(&crate::help::configs::reverse::CONFIG),
        "search-implementation"                    => Some(&crate::help::configs::search_implementation::CONFIG),
        "search-scenarios"                         => Some(&crate::help::configs::search_scenarios::CONFIG),
        "set-user-story"                           => Some(&crate::help::configs::set_user_story::CONFIG),
        "show-acceptance-criteria"                 => Some(&crate::help::configs::show_acceptance_criteria::CONFIG),
        "show-coverage"                            => Some(&crate::help::configs::show_coverage::CONFIG),
        "show-deleted"                             => Some(&crate::help::configs::show_deleted::CONFIG),
        "show-epic"                                => Some(&crate::help::configs::show_epic::CONFIG),
        "show-event-storm"                         => Some(&crate::help::configs::show_event_storm::CONFIG),
        "show-feature"                             => Some(&crate::help::configs::show_feature::CONFIG),
        "show-foundation"                          => Some(&crate::help::configs::show_foundation::CONFIG),
        "show-foundation-event-storm"              => Some(&crate::help::configs::show_foundation_event_storm::CONFIG),
        "show-test-patterns"                       => Some(&crate::help::configs::show_test_patterns::CONFIG),
        "show-work-unit"                           => Some(&crate::help::configs::show_work_unit::CONFIG),
        "suggest-dependencies"                     => Some(&crate::help::configs::suggest_dependencies::CONFIG),
        "tag-stats"                                => Some(&crate::help::configs::tag_stats::CONFIG),
        "unlink-coverage"                          => Some(&crate::help::configs::unlink_coverage::CONFIG),
        "update-foundation"                        => Some(&crate::help::configs::update_foundation::CONFIG),
        "update-prefix"                            => Some(&crate::help::configs::update_prefix::CONFIG),
        "update-scenario"                          => Some(&crate::help::configs::update_scenario::CONFIG),
        "update-step"                              => Some(&crate::help::configs::update_step::CONFIG),
        "update-tag"                               => Some(&crate::help::configs::update_tag::CONFIG),
        "update-work-unit"                         => Some(&crate::help::configs::update_work_unit::CONFIG),
        "update-work-unit-estimate"                => Some(&crate::help::configs::update_work_unit_estimate::CONFIG),
        "update-work-unit-status"                  => Some(&crate::help::configs::update_work_unit_status::CONFIG),
        "validate"                                 => Some(&crate::help::configs::validate::CONFIG),
        "validate-foundation-schema"               => Some(&crate::help::configs::validate_foundation_schema::CONFIG),
        "validate-hooks"                           => Some(&crate::help::configs::validate_hooks::CONFIG),
        "validate-spec-alignment"                  => Some(&crate::help::configs::validate_spec_alignment::CONFIG),
        "validate-tags"                            => Some(&crate::help::configs::validate_tags::CONFIG),
        "validate-work-units"                      => Some(&crate::help::configs::validate_work_units::CONFIG),
        "workflow-automation"                      => Some(&crate::help::configs::workflow_automation::CONFIG),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    //! Drift guard (RPC-414 review Observation 4).
    //!
    //! Keeps [`config_for`] 1:1 with the modules registered in
    //! `help/configs/mod.rs`. If someone adds a `pub mod xyz;` (a real CONFIG)
    //! but forgets the matching table arm here, `config_for("xyz")` silently
    //! returns `None` and that command would degrade to the "no detailed help"
    //! branch instead of rendering its doc. This test fails loudly in that case.
    //!
    //! The expected set is derived at test time by reading `help/configs/mod.rs`
    //! (the source of truth) rather than a hand-maintained const list — the same
    //! runtime-source-read convention used by `tests/dispatcher_test.rs`. A
    //! table arm that references a non-existent module would fail to compile, so
    //! together these guarantee a bijection.
    #![allow(clippy::unwrap_used, clippy::panic)]

    use std::fs;
    use std::path::Path;

    /// Commands that intentionally ship NO `CommandHelpConfig` — their TS
    /// reference has no custom `-help.ts`, so they must degrade gracefully
    /// (real canonical command, `config_for` → `None`).
    const KNOWN_NO_CONFIG: &[&str] = &[
        "register-tag",
        "board",
        "delete-features",
        "delete-scenarios",
        "list-foundation-sections",
    ];

    /// Read `help/configs/mod.rs` and return every registered module name as a
    /// kebab-case command name (`pub mod add_rule;` → `"add-rule"`).
    fn configs_with_config() -> Vec<String> {
        let mod_rs = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("help")
            .join("configs")
            .join("mod.rs");
        let src = fs::read_to_string(&mod_rs)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", mod_rs.display()));

        src.lines()
            .filter_map(|line| {
                let line = line.trim();
                let rest = line.strip_prefix("pub mod ")?;
                let name = rest.strip_suffix(';')?;
                Some(name.replace('_', "-"))
            })
            .collect()
    }

    #[test]
    fn config_table_is_one_to_one_with_help_configs_mod() {
        let expected = configs_with_config();
        assert!(
            !expected.is_empty(),
            "parsed zero `pub mod` entries from help/configs/mod.rs — parser drift"
        );

        let mut missing: Vec<String> = Vec::new();
        for name in &expected {
            if super::config_for(name).is_none() {
                missing.push(name.clone());
            }
        }
        assert!(
            missing.is_empty(),
            "{} command(s) registered in help/configs/mod.rs but missing from config_for table \
             (they would silently degrade to 'no detailed help'): {missing:?}",
            missing.len()
        );

        // Count parity: every module maps to a Some, and (by compile-time
        // reference) every table arm maps to a real module — a bijection.
        let some_count = expected
            .iter()
            .filter(|n| super::config_for(n).is_some())
            .count();
        assert_eq!(
            some_count,
            expected.len(),
            "config_for Some-count ({some_count}) must equal help/configs/mod.rs module count ({})",
            expected.len()
        );
    }

    #[test]
    fn known_no_config_commands_degrade_gracefully() {
        for name in KNOWN_NO_CONFIG {
            // No CONFIG → the table must not resolve it.
            assert!(
                super::config_for(name).is_none(),
                "'{name}' is documented as a no-CONFIG command but config_for returned Some"
            );
            // ...yet it IS a real canonical command, so help routing degrades
            // it gracefully rather than reporting UnknownCommand.
            assert!(
                crate::canonical::lookup(name).is_some(),
                "'{name}' should be a real canonical command that degrades gracefully"
            );
        }
    }
}
