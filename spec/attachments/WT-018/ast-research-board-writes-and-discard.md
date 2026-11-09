# AST research: WT-018 board writes and the discard flow

Run 2026-11-08 at HEAD 7c3eb1799 with fspec astgrep.

## Board writers

`fspec astgrep --pattern 'write_json_atomic(&$P, $D)' --lang rust --path rust/fspec-core/src/commands` (85 matches; the work-units.json ones are the writes WT-018 routes through one board writer):

```
rust/fspec-core/src/commands/remove_example.rs:229:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_domain_event_to_foundation.rs:161:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/generate_coverage/selected.rs:45:13:write_json_atomic(&target.coverage_path, next)
rust/fspec-core/src/commands/auto_advance.rs:215:5:write_json_atomic(&work_units_path, &data)
rust/fspec-core/src/commands/create_epic.rs:195:5:write_json_atomic(&path, &Value::Object(top_level))
rust/fspec-core/src/commands/add_assumption.rs:93:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_dependencies.rs:141:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/create_bug.rs:250:5:write_json_atomic(&wu_path, &Value::Object(top))
rust/fspec-core/src/commands/create_bug.rs:269:9:write_json_atomic(&epics_path, &Value::Object(epics_top))
rust/fspec-core/src/commands/add_scenario/coverage_entry.rs:51:5:write_json_atomic(&path, &coverage)
rust/fspec-core/src/commands/add_command.rs:200:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/remove_hook.rs:111:5:write_json_atomic(&path, &Value::Object(config))
rust/fspec-core/src/commands/record_iteration.rs:135:5:write_json_atomic(&work_units_path, &data)
rust/fspec-core/src/commands/add_dependency.rs:107:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/set_user_story.rs:132:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/delete_tag.rs:173:5:write_json_atomic(&tags_json_path, &tags_data)
rust/fspec-core/src/commands/restore_question.rs:171:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/configure_tools.rs:139:5:write_json_atomic(&config_path, &config)
rust/fspec-core/src/commands/add_hotspot.rs:200:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_diagram.rs:148:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_hook.rs:136:5:write_json_atomic(&path, &Value::Object(config))
rust/fspec-core/src/commands/add_domain_event.rs:227:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_foundation_bounded_context.rs:140:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_command_to_foundation.rs:153:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/update_work_unit_estimate.rs:193:5:write_json_atomic(&wu_path, &Value::Object(top))
rust/fspec-core/src/commands/delete_work_unit.rs:189:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_architecture_note.rs:154:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/remove_command_from_foundation.rs:147:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/resume_schedule.rs:127:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_schedule.rs:200:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/clear_virtual_hooks.rs:131:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/remove_rule.rs:217:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/restore_architecture_note.rs:178:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/repair_work_units.rs:265:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_bounded_context.rs:231:5:write_json_atomic(&path, &root)
rust/fspec-core/src/commands/register_tag.rs:169:5:write_json_atomic(&tags_json_path, &tags_data)
rust/fspec-core/src/commands/workflow_automation.rs:191:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/workflow_automation.rs:273:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/compact_work_unit.rs:172:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/update_work_unit.rs:319:9:write_json_atomic(&epics_path, &Value::Object(epics_top))
rust/fspec-core/src/commands/update_work_unit.rs:324:5:write_json_atomic(&wu_path, &Value::Object(wu_top))
rust/fspec-core/src/commands/prioritize_work_unit.rs:249:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/delete_epic.rs:89:9:write_json_atomic(&epics_path, &Value::Object(epics_top.clone()))
rust/fspec-core/src/commands/delete_epic.rs:116:5:write_json_atomic(&epics_path, &Value::Object(epics_top))
rust/fspec-core/src/commands/update_work_unit_status.rs:214:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/update_prefix.rs:161:5:write_json_atomic(&spec_path, &data)
rust/fspec-core/src/commands/remove_foundation_bounded_context.rs:165:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_aggregate_to_foundation.rs:232:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/remove_schedule.rs:97:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/discover_foundation.rs:549:5:write_json_atomic(&wu_path, &Value::Object(top))
rust/fspec-core/src/commands/restore_example.rs:232:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_rule.rs:151:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/clear_dependencies.rs:141:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_example.rs:150:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_attachment.rs:291:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/create_task.rs:250:5:write_json_atomic(&wu_path, &Value::Object(top))
rust/fspec-core/src/commands/create_task.rs:269:9:write_json_atomic(&epics_path, &Value::Object(epics_top))
rust/fspec-core/src/commands/add_virtual_hook.rs:160:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/remove_domain_event_from_foundation.rs:150:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/remove_dependency.rs:154:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/remove_virtual_hook.rs:154:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/remove_assumption.rs:110:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/remove_aggregate_from_foundation.rs:202:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/restore_assumption.rs:110:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/pause_schedule.rs:126:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/answer_question.rs:254:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_external_system.rs:232:5:write_json_atomic(&path, &root)
rust/fspec-core/src/commands/remove_architecture_note.rs:193:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/copy_virtual_hooks.rs:188:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/remove_question.rs:224:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/create_story.rs:268:5:write_json_atomic(&wu_path, &Value::Object(top))
rust/fspec-core/src/commands/create_story.rs:287:9:write_json_atomic(&epics_path, &Value::Object(epics_top))
rust/fspec-core/src/commands/audit/fix.rs:123:13:write_json_atomic(&path, &coverage)
rust/fspec-core/src/commands/restore_rule.rs:229:5:write_json_atomic(&path, data)
rust/fspec-core/src/commands/restore_rule.rs:332:5:write_json_atomic(&path, data)
rust/fspec-core/src/commands/update_tag.rs:198:5:write_json_atomic(&tags_json_path, &tags_data)
rust/fspec-core/src/commands/add_policy.rs:228:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/create_prefix.rs:124:5:write_json_atomic(&spec_path, &data)
rust/fspec-core/src/commands/delete_diagram.rs:102:5:write_json_atomic(&foundation_path, &data)
rust/fspec-core/src/commands/remove_attachment.rs:185:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_aggregate.rs:212:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/add_question.rs:164:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/generate_example_mapping_from_event_storm.rs:266:5:write_json_atomic(&path, &data)
rust/fspec-core/src/commands/import_example_map.rs:124:5:write_json_atomic(&path, &data)

```

## Status change

`fspec astgrep --pattern 'apply_status_change($$$A)' --lang rust --path rust/fspec-core/src`:

```
rust/fspec-core/src/commands/update_work_unit_status.rs:192:5:apply_status_change(
```

## Discard flow

`fspec astgrep --pattern '$R.discard_session_worktree($$$A)' --lang rust --path rust`:

```
rust/rpc/src/mcp_dispatch/group_34.rs:71:9:client
rust/sessions/tests/wt003_worktree_rpc_repo_resolution.rs:196:5:handle
rust/fspec-tui/src/app/dispatch_merge_worktree.rs:156:30:backend.discard_session_worktree(sid_for_task.clone())
rust/fspec-tui/tests/rpc057_cross_transport_parity.rs:132:5:embedded
rust/fspec-tui/tests/rpc057_cross_transport_parity.rs:138:5:websocket

```

The TUI's Discard (rust/fspec-tui/src/app/dispatch_merge_worktree.rs:156) calls discard_session_worktree directly; the daemon's handle (rust/sessions/src/handle_impl/handle_impl_8.rs) runs codelet_git::discard_session. Nothing reads the board on discard today.
