// REFAC-020: dispatch arms, stage 3 of 6 (whole-item composition).
macro_rules! route_dispatch_03 {
    ($command:expr, $workspace:expr, $forward:ident; $($collected:tt)*) => {
        route_dispatch_04! { $command, $workspace, $forward; $($collected)*
            Some(Mode::AddTagToFeature {
                file,
                tags,
                validate_registry,
            }) => $forward!(
                add_tag_to_feature::run,
                add_tag_to_feature::CliArgs {
                    file,
                    tags,
                    validate_registry
                }
            ),
            Some(Mode::RemoveTagFromFeature { file, tags }) => $forward!(
                remove_tag_from_feature::run,
                remove_tag_from_feature::CliArgs { file, tags }
            ),
            Some(Mode::AddTagToScenario {
                file,
                scenario_name,
                tags,
                validate_registry,
            }) => $forward!(
                add_tag_to_scenario::run,
                add_tag_to_scenario::CliArgs {
                    file,
                    scenario_name,
                    tags,
                    validate_registry
                }
            ),
            Some(Mode::RemoveTagFromScenario {
                file,
                scenario_name,
                tags,
            }) => $forward!(
                remove_tag_from_scenario::run,
                remove_tag_from_scenario::CliArgs {
                    file,
                    scenario_name,
                    tags
                }
            ),
            Some(Mode::AddAttachment {
                work_unit_id,
                file_path,
                description,
            }) => $forward!(
                add_attachment::run,
                add_attachment::CliArgs {
                    work_unit_id,
                    file_path,
                    description
                }
            ),
            Some(Mode::RemoveAttachment {
                work_unit_id,
                file_name,
                keep_file,
            }) => $forward!(
                remove_attachment::run,
                remove_attachment::CliArgs {
                    work_unit_id,
                    file_name,
                    keep_file
                }
            ),
            Some(Mode::AddVirtualHook {
                work_unit_id,
                event,
                command,
                blocking,
                git_context,
            }) => $forward!(
                add_virtual_hook::run,
                add_virtual_hook::CliArgs {
                    work_unit_id,
                    event,
                    command,
                    blocking,
                    git_context
                }
            ),
            Some(Mode::RemoveVirtualHook {
                work_unit_id,
                hook_name,
            }) => $forward!(
                remove_virtual_hook::run,
                remove_virtual_hook::CliArgs {
                    work_unit_id,
                    hook_name
                }
            ),
            Some(Mode::ClearVirtualHooks { work_unit_id }) => $forward!(
                clear_virtual_hooks::run,
                clear_virtual_hooks::CliArgs { work_unit_id }
            ),
            Some(Mode::CopyVirtualHooks {
                from,
                to,
                hook_name,
            }) => $forward!(
                copy_virtual_hooks::run,
                copy_virtual_hooks::CliArgs {
                    from,
                    to,
                    hook_name
                }
            ),
            Some(Mode::AddHook {
                event,
                name,
                command,
                blocking,
                timeout,
            }) => $forward!(
                add_hook::run,
                add_hook::CliArgs {
                    event,
                    name,
                    command,
                    blocking,
                    timeout
                }
            ),
            Some(Mode::RemoveHook { event, name }) => {
                $forward!(remove_hook::run, remove_hook::CliArgs { event, name })
            }
            Some(Mode::AddDiagram {
                section,
                title,
                code,
            }) => $forward!(
                add_diagram::run,
                add_diagram::CliArgs {
                    section,
                    title,
                    code
                }
            ),
            Some(Mode::DeleteDiagram { section, title }) => $forward!(
                delete_diagram::run,
                delete_diagram::CliArgs { section, title }
            ),
            // Batch 11 (2026-06-12) — Event Storm item-add + create-* commands
            Some(Mode::AddAggregate {
                work_unit_id,
                text,
                responsibilities,
                timestamp,
                bounded_context,
            }) => $forward!(
                add_aggregate::run,
                add_aggregate::CliArgs {
                    work_unit_id,
                    text,
                    responsibilities,
                    timestamp,
                    bounded_context
                }
            ),
            Some(Mode::AddCommand {
                work_unit_id,
                text,
                actor,
                timestamp,
                bounded_context,
            }) => $forward!(
                add_command::run,
                add_command::CliArgs {
                    work_unit_id,
                    text,
                    actor,
                    timestamp,
                    bounded_context
                }
            ),
            Some(Mode::AddDomainEvent {
                work_unit_id,
                text,
                timestamp,
                bounded_context,
            }) => $forward!(
                add_domain_event::run,
                add_domain_event::CliArgs {
                    work_unit_id,
                    text,
                    timestamp,
                    bounded_context
                }
            ),
            Some(Mode::AddHotspot {
                work_unit_id,
                text,
                concern,
                timestamp,
                bounded_context,
            }) => $forward!(
                add_hotspot::run,
                add_hotspot::CliArgs {
                    work_unit_id,
                    text,
                    concern,
                    timestamp,
                    bounded_context
                }
            ),
            Some(Mode::AddBoundedContext {
                work_unit_id,
                text,
                description,
                timestamp,
                context,
            }) => $forward!(
                add_bounded_context::run,
                add_bounded_context::CliArgs {
                    work_unit_id,
                    text,
                    description,
                    timestamp,
                    context
                }
            ),
            Some(Mode::AddExternalSystem {
                work_unit_id,
                text,
                system_type,
                timestamp,
                context,
            }) => $forward!(
                add_external_system::run,
                add_external_system::CliArgs {
                    work_unit_id,
                    text,
                    system_type,
                    timestamp,
                    context
                }
            ),
            Some(Mode::AddPolicy {
                work_unit_id,
                text,
                when,
                then,
                timestamp,
                bounded_context,
            }) => $forward!(
                add_policy::run,
                add_policy::CliArgs {
                    work_unit_id,
                    text,
                    when,
                    then,
                    timestamp,
                    bounded_context
                }
            ),
        }
    };
}
