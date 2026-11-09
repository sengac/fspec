// REFAC-020: dispatch arms, stage 2 of 6 (whole-item composition).
macro_rules! route_dispatch_02 {
    ($command:expr, $workspace:expr, $forward:ident; $($collected:tt)*) => {
        route_dispatch_03! { $command, $workspace, $forward; $($collected)*
            Some(Mode::RegisterTag {
                tag,
                category,
                description,
            }) => $forward!(
                register_tag::run,
                register_tag::CliArgs {
                    tag,
                    category,
                    description
                }
            ),
            Some(Mode::UpdatePrefix {
                prefix,
                description,
            }) => $forward!(
                update_prefix::run,
                update_prefix::CliArgs {
                    prefix,
                    description
                }
            ),
            Some(Mode::UpdateTag {
                tag,
                category,
                description,
            }) => $forward!(
                update_tag::run,
                update_tag::CliArgs {
                    tag,
                    category,
                    description
                }
            ),
            Some(Mode::AddDependencies {
                work_unit_id,
                blocks,
                blocked_by,
                depends_on,
                relates_to,
            }) => $forward!(
                add_dependencies::run,
                add_dependencies::CliArgs {
                    work_unit_id,
                    blocks,
                    blocked_by,
                    depends_on,
                    relates_to
                }
            ),
            Some(Mode::DeleteTag {
                tag,
                force,
                dry_run,
            }) => $forward!(
                delete_tag::run,
                delete_tag::CliArgs {
                    tag,
                    force,
                    dry_run
                }
            ),
            Some(Mode::RemoveDependency {
                work_unit_id,
                depends_on_positional,
                blocks,
                blocked_by,
                depends_on,
                relates_to,
            }) => $forward!(
                remove_dependency::run,
                remove_dependency::CliArgs {
                    work_unit_id,
                    depends_on_positional,
                    blocks,
                    blocked_by,
                    depends_on,
                    relates_to,
                }
            ),
            Some(Mode::ClearDependencies {
                work_unit_id,
                confirm,
            }) => $forward!(
                clear_dependencies::run,
                clear_dependencies::CliArgs {
                    work_unit_id,
                    confirm
                }
            ),
            Some(Mode::AddRule { work_unit_id, rule }) => {
                $forward!(add_rule::run, add_rule::CliArgs { work_unit_id, rule })
            }
            Some(Mode::RemoveRule {
                work_unit_id,
                index,
            }) => $forward!(
                remove_rule::run,
                remove_rule::CliArgs {
                    work_unit_id,
                    index
                }
            ),
            Some(Mode::AddAssumption {
                work_unit_id,
                assumption,
            }) => $forward!(
                add_assumption::run,
                add_assumption::CliArgs {
                    work_unit_id,
                    assumption
                }
            ),
            Some(Mode::AddExample {
                work_unit_id,
                example,
            }) => $forward!(
                add_example::run,
                add_example::CliArgs {
                    work_unit_id,
                    example
                }
            ),
            Some(Mode::RemoveExample {
                work_unit_id,
                index,
            }) => $forward!(
                remove_example::run,
                remove_example::CliArgs {
                    work_unit_id,
                    index
                }
            ),
            Some(Mode::AddQuestion {
                work_unit_id,
                question,
            }) => $forward!(
                add_question::run,
                add_question::CliArgs {
                    work_unit_id,
                    question
                }
            ),
            Some(Mode::RemoveQuestion {
                work_unit_id,
                index,
            }) => $forward!(
                remove_question::run,
                remove_question::CliArgs {
                    work_unit_id,
                    index
                }
            ),
            Some(Mode::AddArchitectureNote { work_unit_id, note }) => $forward!(
                add_architecture_note::run,
                add_architecture_note::CliArgs { work_unit_id, note }
            ),
            Some(Mode::RemoveArchitectureNote {
                work_unit_id,
                index,
            }) => $forward!(
                remove_architecture_note::run,
                remove_architecture_note::CliArgs {
                    work_unit_id,
                    index
                }
            ),
            Some(Mode::SetUserStory {
                work_unit_id,
                role,
                action,
                benefit,
            }) => $forward!(
                set_user_story::run,
                set_user_story::CliArgs {
                    work_unit_id,
                    role,
                    action,
                    benefit
                }
            ),
            // Batch 9 (2026-06-11) — dependency, q&a, tag-feature, tag-scenario, restore-*
            Some(Mode::AddDependency {
                work_unit_id,
                depends_on_positional,
                blocks,
                blocked_by,
                depends_on,
                relates_to,
            }) => $forward!(
                add_dependency::run,
                add_dependency::CliArgs {
                    work_unit_id,
                    depends_on_positional,
                    blocks,
                    blocked_by,
                    depends_on,
                    relates_to,
                }
            ),
            Some(Mode::AnswerQuestion {
                work_unit_id,
                index,
                answer,
                add_to,
            }) => $forward!(
                answer_question::run,
                answer_question::CliArgs {
                    work_unit_id,
                    index,
                    answer,
                    add_to
                }
            ),
            Some(Mode::RestoreExample {
                work_unit_id,
                index,
            }) => $forward!(
                restore_example::run,
                restore_example::CliArgs {
                    work_unit_id,
                    index
                }
            ),
            Some(Mode::RestoreRule {
                work_unit_id,
                index,
            }) => $forward!(
                restore_rule::run,
                restore_rule::CliArgs {
                    work_unit_id,
                    index
                }
            ),
            Some(Mode::RemoveAssumption {
                work_unit_id,
                index,
            }) => $forward!(
                remove_assumption::run,
                remove_assumption::CliArgs {
                    work_unit_id,
                    index
                }
            ),
            Some(Mode::RestoreAssumption {
                work_unit_id,
                index,
            }) => $forward!(
                restore_assumption::run,
                restore_assumption::CliArgs {
                    work_unit_id,
                    index
                }
            ),
            Some(Mode::RestoreQuestion {
                work_unit_id,
                index,
            }) => $forward!(
                restore_question::run,
                restore_question::CliArgs {
                    work_unit_id,
                    index
                }
            ),
            Some(Mode::RestoreArchitectureNote {
                work_unit_id,
                index,
            }) => $forward!(
                restore_architecture_note::run,
                restore_architecture_note::CliArgs {
                    work_unit_id,
                    index
                }
            ),
        }
    };
}
