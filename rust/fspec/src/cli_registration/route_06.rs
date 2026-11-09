// REFAC-020: dispatch arms, stage 6 of 6 (whole-item composition).
macro_rules! route_dispatch_06 {
    ($command:expr, $workspace:expr, $forward:ident; $($collected:tt)*) => {
        route_dispatch_finish! { $command, $workspace, $forward; $($collected)*
            Some(Mode::ValidateSpecAlignment { work_unit_id, fix }) => $forward!(
                validate_spec_alignment::run,
                validate_spec_alignment::CliArgs { work_unit_id, fix }
            ),
            Some(Mode::RemoveInitFiles {
                keep_config,
                no_keep_config,
            }) => $forward!(
                remove_init_files::run,
                remove_init_files::CliArgs {
                    keep_config: if no_keep_config {
                        Some(false)
                    } else if keep_config {
                        Some(true)
                    } else {
                        None
                    }
                }
            ),
            Some(Mode::AutoAdvance { dry_run }) => {
                $forward!(auto_advance::run, auto_advance::CliArgs { dry_run })
            }
            Some(Mode::WorkflowAutomation {
                action,
                work_unit_id,
                event,
                from_state,
            }) => $forward!(
                workflow_automation::run,
                workflow_automation::CliArgs {
                    action,
                    work_unit_id,
                    event,
                    from_state
                }
            ),
            Some(Mode::Checkpoint {
                work_unit_id,
                checkpoint_name,
            }) => $forward!(
                checkpoint::run,
                checkpoint::CliArgs {
                    work_unit_id,
                    checkpoint_name
                }
            ),
            Some(Mode::CleanupCheckpoints {
                work_unit_id,
                keep_last,
            }) => $forward!(
                cleanup_checkpoints::run,
                cleanup_checkpoints::CliArgs {
                    work_unit_id,
                    keep_last
                }
            ),
            Some(Mode::RestoreCheckpoint {
                work_unit_id,
                checkpoint_name,
            }) => $forward!(
                restore_checkpoint::run,
                restore_checkpoint::CliArgs {
                    work_unit_id,
                    checkpoint_name
                }
            ),
            Some(Mode::Reverse {
                strategy,
                r#continue,
                status,
                reset,
                complete,
                dry_run,
            }) => $forward!(
                reverse::run,
                reverse::CliArgs {
                    strategy,
                    r#continue,
                    status,
                    reset,
                    complete,
                    dry_run
                }
            ),
            Some(Mode::UpdateWorkUnitStatus {
                work_unit_id,
                status,
                blocked_reason,
                reason,
                skip_temporal_validation,
            }) => $forward!(
                update_work_unit_status::run,
                update_work_unit_status::CliArgs {
                    work_unit_id,
                    status,
                    blocked_reason,
                    reason,
                    skip_temporal_validation,
                }
            ),
            Some(Mode::DiscoverFoundation {
                output,
                finalize,
                draft_path,
                auto_generate_md,
                force,
            }) => $forward!(
                discover_foundation::run,
                discover_foundation::CliArgs {
                    finalize,
                    output,
                    draft_path,
                    auto_generate_md,
                    force,
                }
            ),
            Some(Mode::FoundationStatus { json }) => {
                $forward!(foundation_status::run, foundation_status::CliArgs { json })
            }
            Some(Mode::ValidateConfig {}) => {
                $forward!(validate_config::run, validate_config::CliArgs {})
            }
            Some(Mode::GenerateScenarios {
                work_unit_id,
                feature,
                ignore_possible_duplicates,
            }) => {
                $forward!(
                    generate_scenarios::run,
                    generate_scenarios::CliArgs {
                        work_unit_id,
                        feature,
                        ignore_possible_duplicates
                    }
                )
            }
            Some(Mode::Init { agents }) => $forward!(init::run, init::CliArgs { agents }),
            Some(Mode::Research { tool, work_unit }) => {
                $forward!(research::run, research::CliArgs { tool, work_unit })
            }
            Some(Mode::AstGrep {
                pattern,
                lang,
                path,
            }) => $forward!(
                astgrep::run,
                astgrep::CliArgs {
                    pattern,
                    lang,
                    path
                }
            ),
            Some(Mode::Bootstrap {}) => $forward!(bootstrap::run, bootstrap::CliArgs {}),
            Some(Mode::ReportBugToGitHub {
                project_root,
                bug_description,
                expected_behavior,
                actual_behavior,
                interactive,
            }) => $forward!(
                report_bug_to_github::run,
                report_bug_to_github::CliArgs {
                    project_root,
                    bug_description,
                    expected_behavior,
                    actual_behavior,
                    interactive,
                }
            ),
            Some(Mode::Review { work_unit_id }) => {
                $forward!(review::run, review::CliArgs { work_unit_id })
            }
            Some(Mode::UpdateFeatureText {
                feature,
                find,
                replace,
            }) => $forward!(
                update_feature_text::run,
                update_feature_text::CliArgs {
                    feature,
                    find,
                    replace
                }
            ),
            // AUDIT-003, COV-077, AUDIT-004: the one deterministic drift
            // check and its sessions.
            Some(Mode::Audit {
                area,
                feature,
                work_unit,
                json,
                fix,
                changed,
                start,
                status,
                r#continue,
                reset,
                complete,
            }) => $forward!(
                audit::run,
                audit::CliArgs {
                    area,
                    feature,
                    work_unit,
                    json,
                    fix,
                    changed,
                    start,
                    status,
                    r#continue,
                    reset,
                    complete
                }
            ),
            Some(Mode::Daemons { action }) => daemons_cmd::run(action),
            Some(Mode::Workspaces { action }) => workspaces_cmd::run(action),
        }
    };
}
