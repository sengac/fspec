// REFAC-020: dispatch arms, stage 5 of 6 (whole-item composition).
macro_rules! route_dispatch_05 {
    ($command:expr, $workspace:expr, $forward:ident; $($collected:tt)*) => {
        route_dispatch_06! { $command, $workspace, $forward; $($collected)*
            Some(Mode::RemoveDomainEventFromFoundation {
                context_name,
                event_name,
            }) => $forward!(
                remove_domain_event_from_foundation::run,
                remove_domain_event_from_foundation::CliArgs {
                    context_name,
                    event_name
                }
            ),
            Some(Mode::Dependencies {
                work_unit_id,
                graph,
            }) => $forward!(
                dependencies::run,
                dependencies::CliArgs {
                    work_unit_id,
                    graph
                }
            ),
            Some(Mode::UpdateFoundation { section, content }) => $forward!(
                update_foundation::run,
                update_foundation::CliArgs { section, content }
            ),
            Some(Mode::ConfigureTools {
                test_command,
                quality_commands,
                reconfigure,
            }) => $forward!(
                configure_tools::run,
                configure_tools::CliArgs {
                    test_command,
                    quality_commands,
                    reconfigure
                }
            ),
            // Batch 15 (2026-06-14) — feature-file (.feature) mutation commands
            Some(Mode::CreateFeature { name }) => {
                $forward!(create_feature::run, create_feature::CliArgs { name })
            }
            Some(Mode::AddScenario {
                file,
                scenario_name,
            }) => $forward!(
                add_scenario::run,
                add_scenario::CliArgs {
                    feature: file,
                    scenario: scenario_name
                }
            ),
            Some(Mode::AddStep {
                file,
                scenario,
                r#type,
                text,
            }) => $forward!(
                add_step::run,
                add_step::CliArgs {
                    feature: file,
                    scenario,
                    step_type: r#type,
                    text
                }
            ),
            Some(Mode::AddBackground { feature, text }) => $forward!(
                add_background::run,
                add_background::CliArgs { feature, text }
            ),
            Some(Mode::AddArchitecture { file, notes }) => $forward!(
                add_architecture::run,
                add_architecture::CliArgs {
                    feature: file,
                    text: notes
                }
            ),
            Some(Mode::DeleteScenario { file, scenario }) => $forward!(
                delete_scenario::run,
                delete_scenario::CliArgs {
                    feature: file,
                    scenario
                }
            ),
            Some(Mode::DeleteStep {
                file,
                scenario,
                step,
            }) => $forward!(
                delete_step::run,
                delete_step::CliArgs {
                    feature: file,
                    scenario,
                    step
                }
            ),
            Some(Mode::DeleteFeatures { tag, dry_run }) => $forward!(
                delete_features::run,
                delete_features::CliArgs { tags: tag, dry_run }
            ),
            Some(Mode::UpdateScenario {
                file,
                old_name,
                new_name,
            }) => $forward!(
                update_scenario::run,
                update_scenario::CliArgs {
                    file,
                    old_name,
                    new_name
                }
            ),
            Some(Mode::UpdateStep {
                feature,
                scenario,
                current_step,
                text,
                keyword,
            }) => $forward!(
                update_step::run,
                update_step::CliArgs {
                    feature,
                    scenario,
                    current_step,
                    text,
                    keyword
                }
            ),
            // Batch 16 (2026-06-14) — validation + search + coverage + generator/retag
            Some(Mode::ValidateTags {
                file,
                verbose,
                summary,
            }) => $forward!(
                validate_tags::run,
                validate_tags::CliArgs {
                    file,
                    verbose,
                    summary
                }
            ),
            Some(Mode::ValidateWorkUnits {}) => {
                $forward!(validate_work_units::run, validate_work_units::CliArgs {})
            }
            Some(Mode::ValidateHooks {}) => $forward!(validate_hooks::run, validate_hooks::CliArgs {}),
            Some(Mode::ValidateFoundationSchema { draft }) => $forward!(
                validate_foundation_schema::run,
                validate_foundation_schema::CliArgs { draft }
            ),
            Some(Mode::Validate { file, verbose }) => {
                $forward!(validate::run, validate::CliArgs { file, verbose })
            }
            Some(Mode::SearchImplementation {
                function,
                show_work_units,
                json,
            }) => $forward!(
                search_implementation::run,
                search_implementation::CliArgs {
                    function,
                    show_work_units,
                    json
                }
            ),
            Some(Mode::UnlinkCoverage {
                feature_name,
                scenario,
                test_file,
                impl_file,
                all,
                test_mapping_index,
                expected_scenario_hash,
            }) => $forward!(
                unlink_coverage::run,
                unlink_coverage::CliArgs {
                    feature_name,
                    scenario,
                    test_file,
                    impl_file,
                    all,
                    test_mapping_index,
                    expected_scenario_hash
                }
            ),
            Some(Mode::GenerateTagsMd { output }) => {
                $forward!(generate_tags_md::run, generate_tags_md::CliArgs { output })
            }
            Some(Mode::Retag { from, to, dry_run }) => {
                $forward!(retag::run, retag::CliArgs { from, to, dry_run })
            }
            // Batch 17 (2026-06-15) — coverage/board/check/format/compare/import/report
            Some(Mode::Board { format, limit }) => {
                $forward!(board::run, board::CliArgs { format, limit })
            }
            Some(Mode::CompareImplementations {
                tag,
                show_coverage,
                json,
            }) => $forward!(
                compare_implementations::run,
                compare_implementations::CliArgs {
                    tag,
                    show_coverage,
                    json
                }
            ),
            Some(Mode::DeleteScenarios { tags, dry_run }) => $forward!(
                delete_scenarios::run,
                delete_scenarios::CliArgs { tags, dry_run }
            ),
            Some(Mode::Format { file }) => $forward!(format::run, format::CliArgs { file }),
            Some(Mode::GenerateCoverage {
                dry_run,
                feature_name,
            }) => $forward!(
                generate_coverage::run,
                generate_coverage::CliArgs {
                    dry_run,
                    feature_name
                }
            ),
            Some(Mode::LinkCoverage {
                feature_name,
                scenario,
                test_file,
                test_lines,
                impl_file,
                impl_lines,
                skip_validation,
                skip_step_validation,
                test_mapping_index,
                expected_scenario_hash,
                replace,
            }) => $forward!(
                link_coverage::run,
                link_coverage::CliArgs {
                    feature_name,
                    scenario,
                    test_file,
                    test_lines,
                    impl_file,
                    impl_lines,
                    skip_validation,
                    skip_step_validation,
                    test_mapping_index,
                    expected_scenario_hash,
                    replace,
                }
            ),
            Some(Mode::GenerateSummaryReport { format, output }) => $forward!(
                generate_summary_report::run,
                generate_summary_report::CliArgs { format, output }
            ),
            Some(Mode::ImportExampleMap { work_unit_id, file }) => $forward!(
                import_example_map::run,
                import_example_map::CliArgs { work_unit_id, file }
            ),
            // Batch 18 (2026-06-16) — event-storm/analysis/work-unit-status
            Some(Mode::DiscoverEventStorm { work_unit_id }) => $forward!(
                discover_event_storm::run,
                discover_event_storm::CliArgs { work_unit_id }
            ),
            Some(Mode::GenerateExampleMappingFromEventStorm { work_unit_id }) => $forward!(
                generate_example_mapping_from_event_storm::run,
                generate_example_mapping_from_event_storm::CliArgs { work_unit_id }
            ),
            Some(Mode::SuggestDependencies { output }) => $forward!(
                suggest_dependencies::run,
                suggest_dependencies::CliArgs { output }
            ),
        }
    };
}
