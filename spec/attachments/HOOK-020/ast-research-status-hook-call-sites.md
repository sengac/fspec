# AST research for HOOK-020 (global status hooks)

## run_virtual_hooks call sites
No matches found

## run_event_hooks definition and callers
rust/fspec-core/src/command_hooks/phases.rs:116:5:run_event_hooks(project_root, &event, work_unit_id, phase)


## load_event_hooks callers
rust/fspec-core/src/command_hooks/phases.rs:77:18:load_event_hooks(project_root, event)


## hook_matches callers
rust/fspec-core/src/command_hooks/phases.rs:89:41:hook_matches(h, work_unit.as_ref())


## run_virtual_hooks (path-qualified call)
rust/fspec-core/src/commands/update_work_unit_status.rs:172:23:crate::command_hooks::run_virtual_hooks(


## HookWorkUnit::load callers
rust/fspec-core/src/command_hooks/phases.rs:84:13:HookWorkUnit::load(project_root, id)


Findings: the only status-move hook call is GATE 8 in update_work_unit_status.rs (run_virtual_hooks); global hooks run only through run_event_hooks from with_command_hooks (command events). load_event_hooks already loads any pre-/post- key, so a pre-<status> key is parsed but never run.
