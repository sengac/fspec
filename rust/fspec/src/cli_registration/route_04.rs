// REFAC-020: dispatch arms, stage 4 of 6 (whole-item composition).
macro_rules! route_dispatch_04 {
    ($command:expr, $workspace:expr, $forward:ident; $($collected:tt)*) => {
        route_dispatch_05! { $command, $workspace, $forward; $($collected)*
            Some(Mode::CreateStory {
                prefix,
                title,
                description,
                epic,
                parent,
            }) => $forward!(
                create_story::run,
                create_story::CliArgs {
                    prefix,
                    title,
                    description,
                    epic,
                    parent
                }
            ),
            Some(Mode::CreateBug {
                prefix,
                title,
                description,
                epic,
                parent,
            }) => $forward!(
                create_bug::run,
                create_bug::CliArgs {
                    prefix,
                    title,
                    description,
                    epic,
                    parent
                }
            ),
            Some(Mode::CreateTask {
                prefix,
                title,
                description,
                epic,
                parent,
            }) => $forward!(
                create_task::run,
                create_task::CliArgs {
                    prefix,
                    title,
                    description,
                    epic,
                    parent
                }
            ),
            Some(Mode::UpdateWorkUnit {
                work_unit_id,
                title,
                description,
                epic,
                parent,
            }) => $forward!(
                update_work_unit::run,
                update_work_unit::CliArgs {
                    work_unit_id,
                    title,
                    description,
                    epic,
                    parent
                }
            ),
            Some(Mode::UpdateWorkUnitEstimate {
                work_unit_id,
                estimate,
            }) => $forward!(
                update_work_unit_estimate::run,
                update_work_unit_estimate::CliArgs {
                    work_unit_id,
                    points: estimate
                }
            ),
            Some(Mode::DeleteWorkUnit {
                work_unit_id,
                force,
                skip_confirmation,
                cascade_dependencies,
            }) => $forward!(
                delete_work_unit::run,
                delete_work_unit::CliArgs {
                    work_unit_id,
                    force,
                    skip_confirmation,
                    cascade_dependencies
                }
            ),
            Some(Mode::CompactWorkUnit { work_unit_id }) => $forward!(
                compact_work_unit::run,
                compact_work_unit::CliArgs { work_unit_id }
            ),
            Some(Mode::PrioritizeWorkUnit {
                work_unit_id,
                position,
                before,
                after,
            }) => $forward!(
                prioritize_work_unit::run,
                prioritize_work_unit::CliArgs {
                    work_unit_id,
                    position,
                    before,
                    after
                }
            ),
            Some(Mode::RepairWorkUnits { dry_run }) => $forward!(
                repair_work_units::run,
                repair_work_units::CliArgs { dry_run }
            ),
            Some(Mode::RecordIteration { name, start, end }) => $forward!(
                record_iteration::run,
                record_iteration::CliArgs { name, start, end }
            ),
            Some(Mode::ExportWorkUnits {
                format,
                output,
                status,
            }) => $forward!(
                export_work_units::run,
                export_work_units::CliArgs {
                    format,
                    output,
                    status
                }
            ),
            Some(Mode::ExportExampleMap { work_unit_id, file }) => $forward!(
                export_example_map::run,
                export_example_map::CliArgs { work_unit_id, file }
            ),
            Some(Mode::ExportDependencies { format, output }) => $forward!(
                export_dependencies::run,
                export_dependencies::CliArgs { format, output }
            ),
            // Batch 13 (2026-06-12) — foundation mutation commands
            Some(Mode::AddCapability { name, description }) => $forward!(
                add_capability::run,
                add_capability::CliArgs { name, description }
            ),
            Some(Mode::RemoveCapability { name }) => {
                $forward!(remove_capability::run, remove_capability::CliArgs { name })
            }
            Some(Mode::AddPersona {
                name,
                description,
                goal,
            }) => $forward!(
                add_persona::run,
                add_persona::CliArgs {
                    name,
                    description,
                    goals: goal
                }
            ),
            Some(Mode::RemovePersona { name }) => {
                $forward!(remove_persona::run, remove_persona::CliArgs { name })
            }
            Some(Mode::AddFoundationBoundedContext { text }) => $forward!(
                add_foundation_bounded_context::run,
                add_foundation_bounded_context::CliArgs { text }
            ),
            Some(Mode::RemoveFoundationBoundedContext {
                context_name,
                cascade,
            }) => $forward!(
                remove_foundation_bounded_context::run,
                remove_foundation_bounded_context::CliArgs {
                    context_name,
                    cascade
                }
            ),
            Some(Mode::AddAggregateToFoundation {
                context_name,
                aggregate_name,
                description,
            }) => $forward!(
                add_aggregate_to_foundation::run,
                add_aggregate_to_foundation::CliArgs {
                    context_name,
                    aggregate_name,
                    description
                }
            ),
            Some(Mode::RemoveAggregateFromFoundation {
                context_name,
                aggregate_name,
            }) => $forward!(
                remove_aggregate_from_foundation::run,
                remove_aggregate_from_foundation::CliArgs {
                    context_name,
                    aggregate_name
                }
            ),
            Some(Mode::AddCommandToFoundation {
                context_name,
                command_name,
                description,
            }) => $forward!(
                add_command_to_foundation::run,
                add_command_to_foundation::CliArgs {
                    context_name,
                    command_name,
                    description
                }
            ),
            Some(Mode::RemoveCommandFromFoundation {
                context_name,
                command_name,
            }) => $forward!(
                remove_command_from_foundation::run,
                remove_command_from_foundation::CliArgs {
                    context_name,
                    command_name
                }
            ),
            Some(Mode::GenerateFoundationMd { output }) => $forward!(
                generate_foundation_md::run,
                generate_foundation_md::CliArgs { output }
            ),
            Some(Mode::AddSchedule {
                name,
                cron,
                timezone,
                r#type,
                role,
                prompt,
                command,
                overlap,
            }) => $forward!(
                add_schedule::run,
                add_schedule::CliArgs {
                    name,
                    cron,
                    timezone,
                    job_type: r#type,
                    role,
                    prompt,
                    command,
                    overlap,
                }
            ),
            Some(Mode::RemoveSchedule { name }) => {
                $forward!(remove_schedule::run, remove_schedule::CliArgs { name })
            }
            Some(Mode::PauseSchedule { name }) => {
                $forward!(pause_schedule::run, pause_schedule::CliArgs { name })
            }
            Some(Mode::ResumeSchedule { name }) => {
                $forward!(resume_schedule::run, resume_schedule::CliArgs { name })
            }
            Some(Mode::AddDomainEventToFoundation {
                context_name,
                event_name,
                description,
            }) => $forward!(
                add_domain_event_to_foundation::run,
                add_domain_event_to_foundation::CliArgs {
                    context_name,
                    event_name,
                    description
                }
            ),
        }
    };
}
