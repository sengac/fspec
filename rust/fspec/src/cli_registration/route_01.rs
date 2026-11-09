// REFAC-020: dispatch arms, stage 1 of 6 (whole-item composition).
macro_rules! route_dispatch_01 {
    ($command:expr, $workspace:expr, $forward:ident; $($collected:tt)*) => {
        route_dispatch_02! { $command, $workspace, $forward; $($collected)*
            None => combined::run($workspace).await,
            Some(Mode::Daemon {
                bind,
                pidfile,
                tls_cert,
                tls_key,
                tls_name,
            }) => daemon::run($workspace, bind, pidfile, tls_cert.zip(tls_key), tls_name).await,
            Some(Mode::Client { connect, token, mcp }) => {
                if mcp {
                    client::run_mcp(connect, token).await
                } else {
                    client::run(connect, token).await
                }
            }
            Some(Mode::Status { connect, token }) => status::run(connect, token).await,
            Some(Mode::Update { check }) => update_cmd::run(check).await,
            Some(Mode::ListWorkUnits { flags }) => $forward!(list_work_units::run, *flags),
            Some(Mode::ListPrefixes) => {
                $forward!(list_prefixes::run, list_prefixes::CliArgs::default())
            }
            Some(Mode::ListEpics) => $forward!(list_epics::run, list_epics::CliArgs::default()),
            Some(Mode::ListTags { category }) => {
                $forward!(list_tags::run, list_tags::CliArgs { category })
            }
            Some(Mode::ListFeatures { tag }) => {
                $forward!(list_features::run, list_features::CliArgs { tag })
            }
            Some(Mode::ListAttachments { work_unit_id }) => $forward!(
                list_attachments::run,
                list_attachments::CliArgs { work_unit_id }
            ),
            Some(Mode::ListHooks) => $forward!(list_hooks::run, list_hooks::CliArgs::default()),
            Some(Mode::ListSchedules { json }) => {
                $forward!(list_schedules::run, list_schedules::CliArgs { json })
            }
            Some(Mode::ListFoundationSections { format }) => $forward!(
                list_foundation_sections::run,
                list_foundation_sections::CliArgs { format }
            ),
            Some(Mode::ListFeatureTags {
                file,
                show_categories,
            }) => $forward!(
                list_feature_tags::run,
                list_feature_tags::CliArgs {
                    file,
                    show_categories,
                }
            ),
            Some(Mode::ListScenarioTags {
                file,
                scenario,
                show_categories,
            }) => $forward!(
                list_scenario_tags::run,
                list_scenario_tags::CliArgs {
                    file,
                    scenario,
                    show_categories,
                }
            ),
            Some(Mode::ListVirtualHooks { work_unit_id }) => $forward!(
                list_virtual_hooks::run,
                list_virtual_hooks::CliArgs { work_unit_id }
            ),
            Some(Mode::ListCheckpoints { work_unit_id }) => $forward!(
                list_checkpoints::run,
                list_checkpoints::CliArgs { work_unit_id }
            ),
            Some(Mode::ShowDeleted { work_unit_id }) => {
                $forward!(show_deleted::run, show_deleted::CliArgs { work_unit_id })
            }
            Some(Mode::ShowEpic { epic_id, format }) => {
                $forward!(show_epic::run, show_epic::CliArgs { epic_id, format })
            }
            Some(Mode::ShowFeature {
                feature,
                format,
                output,
            }) => $forward!(
                show_feature::run,
                show_feature::CliArgs {
                    feature,
                    format,
                    output,
                }
            ),
            Some(Mode::TagStats) => $forward!(tag_stats::run, tag_stats::CliArgs::default()),
            Some(Mode::ShowWorkUnit {
                work_unit_id,
                format,
            }) => $forward!(
                show_work_unit::run,
                show_work_unit::CliArgs {
                    work_unit_id,
                    format
                }
            ),
            Some(Mode::QueryDependencyStats { format }) => $forward!(
                query_dependency_stats::run,
                query_dependency_stats::CliArgs { format }
            ),
            Some(Mode::QueryEstimateAccuracy { format }) => $forward!(
                query_estimate_accuracy::run,
                query_estimate_accuracy::CliArgs { format }
            ),
            Some(Mode::QueryMetrics {
                work_unit_id,
                r#type,
                format,
            }) => $forward!(
                query_metrics::run,
                query_metrics::CliArgs {
                    work_unit_id,
                    r#type,
                    format,
                }
            ),
            Some(Mode::QueryWorkUnits {
                status,
                prefix,
                epic,
                r#type,
                tag,
                format,
            }) => $forward!(
                query_work_units::run,
                query_work_units::CliArgs {
                    status,
                    prefix,
                    epic,
                    r#type,
                    tag,
                    format,
                }
            ),
            Some(Mode::QueryBottlenecks { output }) => $forward!(
                query_bottlenecks::run,
                query_bottlenecks::CliArgs { output }
            ),
            Some(Mode::QueryOrphans {
                output,
                exclude_done,
            }) => $forward!(
                query_orphans::run,
                query_orphans::CliArgs {
                    output,
                    exclude_done
                }
            ),
            Some(Mode::QueryEstimationGuide {
                work_unit_id,
                format,
            }) => $forward!(
                query_estimation_guide::run,
                query_estimation_guide::CliArgs {
                    work_unit_id,
                    format
                }
            ),
            Some(Mode::QueryExampleMappingStats { format }) => $forward!(
                query_example_mapping_stats::run,
                query_example_mapping_stats::CliArgs { format }
            ),
            Some(Mode::ShowEventStorm { work_unit_id }) => $forward!(
                show_event_storm::run,
                show_event_storm::CliArgs { work_unit_id }
            ),
            Some(Mode::ShowFoundation {
                section,
                section_flag,
                format,
                output,
                draft,
                r#final: final_flag,
                list_sections,
                line_numbers,
            }) => $forward!(
                show_foundation::run,
                show_foundation::CliArgs {
                    section: section.or(section_flag),
                    format,
                    output,
                    draft,
                    r#final: final_flag,
                    list_sections,
                    line_numbers,
                }
            ),
            Some(Mode::ShowFoundationEventStorm { r#type, context }) => $forward!(
                show_foundation_event_storm::run,
                show_foundation_event_storm::CliArgs { r#type, context }
            ),
            Some(Mode::ShowTestPatterns {
                tag,
                include_coverage,
                json,
            }) => $forward!(
                show_test_patterns::run,
                show_test_patterns::CliArgs {
                    tag,
                    include_coverage,
                    json
                }
            ),
            Some(Mode::Context { flags }) => $forward!(context::run, *flags),
            Some(Mode::ShowCoverage {
                feature_name,
                format,
                output,
                mapping_json,
            }) => $forward!(
                show_coverage::run,
                show_coverage::CliArgs {
                    feature_name,
                    format,
                    output,
                    mapping_json
                }
            ),
            Some(Mode::CreateEpic {
                epic_id,
                title,
                description,
            }) => $forward!(
                create_epic::run,
                create_epic::CliArgs {
                    epic_id,
                    title,
                    description
                }
            ),
            Some(Mode::DeleteEpic { epic_id, force }) => {
                $forward!(delete_epic::run, delete_epic::CliArgs { epic_id, force })
            }
            Some(Mode::CreatePrefix {
                prefix,
                description,
            }) => $forward!(
                create_prefix::run,
                create_prefix::CliArgs {
                    prefix,
                    description
                }
            ),
        }
    };
}
