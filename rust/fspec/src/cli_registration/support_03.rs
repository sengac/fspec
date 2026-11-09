/// Pre-clap inspection of argv to handle `fspec <list-*> --help` / `-h`
/// without going through clap's auto-generated help block. Returns
/// `Some(exit_code)` when the request was handled (caller must exit
/// immediately); returns `None` to let clap take over.
///
/// Per RPC-247 strict byte-parity: the printed text is whatever
/// `codelet_fspec_core::help::format_command_help(&CONFIG)` produces for
/// the matched subcommand. The TS reference `node dist/index.js <cmd> --help`
/// piped to non-TTY is the contract.
fn intercept_ts_help() -> Option<u8> {
    let args: Vec<String> = std::env::args().collect();
    // Need at least: program, subcommand, --help/-h
    if args.len() < 3 {
        return None;
    }
    let sub = args[1].as_str();
    let wants_help = args[2..].iter().any(|a| a == "--help" || a == "-h");
    if !wants_help {
        return None;
    }
    let text = ts_help_text(sub)?;
    print!("{text}");
    Some(0)
}

/// The help text `fspec <sub> --help` prints, or `None` to let clap answer.
///
/// CLI-016: also the canon the help fixtures under
/// `rust/fspec/tests/fixtures/help` follow. Configured commands print
/// `format_command_help(config)` plus the newline `println!` appends (the TS
/// `console.log` double-newline tail); the five commands without a config
/// print their static text.
fn ts_help_text(sub: &str) -> Option<String> {
    use codelet_fspec_core::help::format_command_help;

    match sub {
        "board" => Some(BOARD_HELP.to_string()),
        "delete-scenarios" => Some(DELETE_SCENARIOS_HELP.to_string()),
        "delete-features" => Some(DELETE_FEATURES_HELP.to_string()),
        "register-tag" => Some(REGISTER_TAG_HELP.to_string()),
        "list-foundation-sections" => Some(LIST_FOUNDATION_SECTIONS_HELP.to_string()),
        _ => ts_help_config(sub).map(|config| format!("{}\n", format_command_help(config))),
    }
}

/// The `CommandHelpConfig` behind `fspec <sub> --help`, when it has one.
fn ts_help_config(sub: &str) -> Option<&'static codelet_fspec_core::help::CommandHelpConfig> {
    use codelet_fspec_core::help::configs;

    match sub {
        "list-attachments" => Some(&configs::list_attachments::CONFIG),
        "list-checkpoints" => Some(&configs::list_checkpoints::CONFIG),
        "list-epics" => Some(&configs::list_epics::CONFIG),
        "list-feature-tags" => Some(&configs::list_feature_tags::CONFIG),
        "list-features" => Some(&configs::list_features::CONFIG),
        "list-hooks" => Some(&configs::list_hooks::CONFIG),
        "list-prefixes" => Some(&configs::list_prefixes::CONFIG),
        "list-scenario-tags" => Some(&configs::list_scenario_tags::CONFIG),
        "list-schedules" => Some(&configs::list_schedules::CONFIG),
        "list-tags" => Some(&configs::list_tags::CONFIG),
        "list-virtual-hooks" => Some(&configs::list_virtual_hooks::CONFIG),
        "list-work-units" => Some(&configs::list_work_units::CONFIG),
        "show-deleted" => Some(&configs::show_deleted::CONFIG),
        "show-epic" => Some(&configs::show_epic::CONFIG),
        "show-feature" => Some(&configs::show_feature::CONFIG),
        "show-work-unit" => Some(&configs::show_work_unit::CONFIG),
        "tag-stats" => Some(&configs::tag_stats::CONFIG),
        "query-dependency-stats" => Some(&configs::query_dependency_stats::CONFIG),
        "query-estimate-accuracy" => Some(&configs::query_estimate_accuracy::CONFIG),
        "query-metrics" => Some(&configs::query_metrics::CONFIG),
        "query-work-units" => Some(&configs::query_work_units::CONFIG),
        "query-bottlenecks" => Some(&configs::query_bottlenecks::CONFIG),
        "query-orphans" => Some(&configs::query_orphans::CONFIG),
        "query-estimation-guide" => Some(&configs::query_estimation_guide::CONFIG),
        "query-example-mapping-stats" => Some(&configs::query_example_mapping_stats::CONFIG),
        "show-event-storm" => Some(&configs::show_event_storm::CONFIG),
        "show-foundation" => Some(&configs::show_foundation::CONFIG),
        "foundation-status" => Some(&configs::foundation_status::CONFIG),
        "validate-config" => Some(&configs::validate_config::CONFIG),
        "show-foundation-event-storm" => Some(&configs::show_foundation_event_storm::CONFIG),
        "show-test-patterns" => Some(&configs::show_test_patterns::CONFIG),
        "context" => Some(&configs::context::CONFIG),
        "show-coverage" => Some(&configs::show_coverage::CONFIG),
        "create-epic" => Some(&configs::create_epic::CONFIG),
        "delete-epic" => Some(&configs::delete_epic::CONFIG),
        "create-prefix" => Some(&configs::create_prefix::CONFIG),
        "update-prefix" => Some(&configs::update_prefix::CONFIG),
        "update-tag" => Some(&configs::update_tag::CONFIG),
        "add-dependencies" => Some(&configs::add_dependencies::CONFIG),
        "delete-tag" => Some(&configs::delete_tag::CONFIG),
        "remove-dependency" => Some(&configs::remove_dependency::CONFIG),
        "clear-dependencies" => Some(&configs::clear_dependencies::CONFIG),
        "add-rule" => Some(&configs::add_rule::CONFIG),
        "remove-rule" => Some(&configs::remove_rule::CONFIG),
        "add-assumption" => Some(&configs::add_assumption::CONFIG),
        "add-example" => Some(&configs::add_example::CONFIG),
        "remove-example" => Some(&configs::remove_example::CONFIG),
        "add-question" => Some(&configs::add_question::CONFIG),
        "remove-question" => Some(&configs::remove_question::CONFIG),
        "add-architecture-note" => Some(&configs::add_architecture_note::CONFIG),
        "remove-architecture-note" => Some(&configs::remove_architecture_note::CONFIG),
        "set-user-story" => Some(&configs::set_user_story::CONFIG),
        "add-dependency" => Some(&configs::add_dependency::CONFIG),
        "answer-question" => Some(&configs::answer_question::CONFIG),
        "restore-example" => Some(&configs::restore_example::CONFIG),
        "restore-rule" => Some(&configs::restore_rule::CONFIG),
        "restore-question" => Some(&configs::restore_question::CONFIG),
        "remove-assumption" => Some(&configs::remove_assumption::CONFIG),
        "restore-assumption" => Some(&configs::restore_assumption::CONFIG),
        "restore-architecture-note" => Some(&configs::restore_architecture_note::CONFIG),
        "add-tag-to-feature" => Some(&configs::add_tag_to_feature::CONFIG),
        "remove-tag-from-feature" => Some(&configs::remove_tag_from_feature::CONFIG),
        "add-tag-to-scenario" => Some(&configs::add_tag_to_scenario::CONFIG),
        "remove-tag-from-scenario" => Some(&configs::remove_tag_from_scenario::CONFIG),
        "add-attachment" => Some(&configs::add_attachment::CONFIG),
        "remove-attachment" => Some(&configs::remove_attachment::CONFIG),
        "add-virtual-hook" => Some(&configs::add_virtual_hook::CONFIG),
        "remove-virtual-hook" => Some(&configs::remove_virtual_hook::CONFIG),
        "clear-virtual-hooks" => Some(&configs::clear_virtual_hooks::CONFIG),
        "copy-virtual-hooks" => Some(&configs::copy_virtual_hooks::CONFIG),
        "add-hook" => Some(&configs::add_hook::CONFIG),
        "remove-hook" => Some(&configs::remove_hook::CONFIG),
        "add-diagram" => Some(&configs::add_diagram::CONFIG),
        "delete-diagram" => Some(&configs::delete_diagram::CONFIG),
        "add-aggregate" => Some(&configs::add_aggregate::CONFIG),
        "add-command" => Some(&configs::add_command::CONFIG),
        "add-domain-event" => Some(&configs::add_domain_event::CONFIG),
        "add-hotspot" => Some(&configs::add_hotspot::CONFIG),
        "add-bounded-context" => Some(&configs::add_bounded_context::CONFIG),
        "add-external-system" => Some(&configs::add_external_system::CONFIG),
        "add-policy" => Some(&configs::add_policy::CONFIG),
        "create-story" => Some(&configs::create_story::CONFIG),
        "create-bug" => Some(&configs::create_bug::CONFIG),
        "create-task" => Some(&configs::create_task::CONFIG),
        "update-work-unit" => Some(&configs::update_work_unit::CONFIG),
        "update-work-unit-estimate" => Some(&configs::update_work_unit_estimate::CONFIG),
        "delete-work-unit" => Some(&configs::delete_work_unit::CONFIG),
        "compact-work-unit" => Some(&configs::compact_work_unit::CONFIG),
        "prioritize-work-unit" => Some(&configs::prioritize_work_unit::CONFIG),
        "repair-work-units" => Some(&configs::repair_work_units::CONFIG),
        "record-iteration" => Some(&configs::record_iteration::CONFIG),
        "export-work-units" => Some(&configs::export_work_units::CONFIG),
        "export-example-map" => Some(&configs::export_example_map::CONFIG),
        "export-dependencies" => Some(&configs::export_dependencies::CONFIG),
        "add-capability" => Some(&configs::add_capability::CONFIG),
        "remove-capability" => Some(&configs::remove_capability::CONFIG),
        "add-persona" => Some(&configs::add_persona::CONFIG),
        "remove-persona" => Some(&configs::remove_persona::CONFIG),
        "add-foundation-bounded-context" => Some(&configs::add_foundation_bounded_context::CONFIG),
        "remove-foundation-bounded-context" => {
            Some(&configs::remove_foundation_bounded_context::CONFIG)
        }
        "add-aggregate-to-foundation" => Some(&configs::add_aggregate_to_foundation::CONFIG),
        "remove-aggregate-from-foundation" => {
            Some(&configs::remove_aggregate_from_foundation::CONFIG)
        }
        "add-command-to-foundation" => Some(&configs::add_command_to_foundation::CONFIG),
        "remove-command-from-foundation" => Some(&configs::remove_command_from_foundation::CONFIG),
        "generate-foundation-md" => Some(&configs::generate_foundation_md::CONFIG),
        "add-schedule" => Some(&configs::add_schedule::CONFIG),
        "remove-schedule" => Some(&configs::remove_schedule::CONFIG),
        "pause-schedule" => Some(&configs::pause_schedule::CONFIG),
        "resume-schedule" => Some(&configs::resume_schedule::CONFIG),
        "add-domain-event-to-foundation" => Some(&configs::add_domain_event_to_foundation::CONFIG),
        "remove-domain-event-from-foundation" => {
            Some(&configs::remove_domain_event_from_foundation::CONFIG)
        }
        "dependencies" => Some(&configs::dependencies::CONFIG),
        "update-foundation" => Some(&configs::update_foundation::CONFIG),
        "configure-tools" => Some(&configs::configure_tools::CONFIG),
        "create-feature" => Some(&configs::create_feature::CONFIG),
        "add-scenario" => Some(&configs::add_scenario::CONFIG),
        "add-step" => Some(&configs::add_step::CONFIG),
        "add-background" => Some(&configs::add_background::CONFIG),
        "add-architecture" => Some(&configs::add_architecture::CONFIG),
        "delete-scenario" => Some(&configs::delete_scenario::CONFIG),
        "delete-step" => Some(&configs::delete_step::CONFIG),
        "update-scenario" => Some(&configs::update_scenario::CONFIG),
        "update-step" => Some(&configs::update_step::CONFIG),
        "update-feature-text" => Some(&configs::update_feature_text::CONFIG),
        "validate-tags" => Some(&configs::validate_tags::CONFIG),
        "validate-work-units" => Some(&configs::validate_work_units::CONFIG),
        "validate-hooks" => Some(&configs::validate_hooks::CONFIG),
        "validate-foundation-schema" => Some(&configs::validate_foundation_schema::CONFIG),
        "validate" => Some(&configs::validate::CONFIG),
        "search-implementation" => Some(&configs::search_implementation::CONFIG),
        "unlink-coverage" => Some(&configs::unlink_coverage::CONFIG),
        "generate-tags-md" => Some(&configs::generate_tags_md::CONFIG),
        "retag" => Some(&configs::retag::CONFIG),
        "audit" => Some(&configs::audit::CONFIG),
        "compare-implementations" => Some(&configs::compare_implementations::CONFIG),
        "format" => Some(&configs::format::CONFIG),
        "generate-coverage" => Some(&configs::generate_coverage::CONFIG),
        "link-coverage" => Some(&configs::link_coverage::CONFIG),
        "generate-summary-report" => Some(&configs::generate_summary_report::CONFIG),
        "import-example-map" => Some(&configs::import_example_map::CONFIG),
        "discover-event-storm" => Some(&configs::discover_event_storm::CONFIG),
        "generate-example-mapping-from-event-storm" => {
            Some(&configs::generate_example_mapping_from_event_storm::CONFIG)
        }
        "suggest-dependencies" => Some(&configs::suggest_dependencies::CONFIG),
        "validate-spec-alignment" => Some(&configs::validate_spec_alignment::CONFIG),
        "remove-init-files" => Some(&configs::remove_init_files::CONFIG),
        "auto-advance" => Some(&configs::auto_advance::CONFIG),
        "workflow-automation" => Some(&configs::workflow_automation::CONFIG),
        "checkpoint" => Some(&configs::checkpoint::CONFIG),
        "cleanup-checkpoints" => Some(&configs::cleanup_checkpoints::CONFIG),
        "restore-checkpoint" => Some(&configs::restore_checkpoint::CONFIG),
        "reverse" => Some(&configs::reverse::CONFIG),
        "discover-foundation" => Some(&configs::discover_foundation::CONFIG),
        "update-work-unit-status" => Some(&configs::update_work_unit_status::CONFIG),
        "generate-scenarios" => Some(&configs::generate_scenarios::CONFIG),
        "init" => Some(&configs::init::CONFIG),
        "research" => Some(&configs::research::CONFIG),
        "bootstrap" => Some(&configs::bootstrap::CONFIG),
        "report-bug-to-github" => Some(&configs::report_bug_to_github::CONFIG),
        _ => None,
    }
}
