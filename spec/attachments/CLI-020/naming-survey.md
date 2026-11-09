# Work-unit naming survey (2026-11-08)

Read-only survey of the CLI-017 era sources, help configs and fixtures. It counts how fspec names a card reference on each surface.

## Variants found
**Help placeholder (ARGUMENTS/USAGE):**
- `<workUnitId>`: 47 pages (show-work-unit, update-work-unit, add-rule, checkpoint, delete-work-unit, ...).
- `<work-unit-id>`: 5 (add-assumption, set-user-story, dependencies, show-event-storm, workflow-automation).
- `<id>`: 3 (add-dependency, update-work-unit-status, update-work-unit-estimate).

**Clap value_name:**
- `WORK_UNIT_ID`: 37 (e.g. mode_03.rs, mode_04.rs, query-metrics --work-unit-id).
- `workUnitId`: 13 (mode_05.rs, mode_08.rs).
- `work-unit-id`: 7 (mode_06.rs:190; mode_08.rs:68,128,141,152,176; mode_09.rs:49).

**Flags taking a card id:**
- `--depends-on`, `--blocks`, `--blocked-by`, `--relates-to`: add-dependency, add-dependencies, remove-dependency, list-work-units.
- `--parent`: create-story, create-bug, create-task, update-work-unit, list-work-units.
- `--work-unit <id>`: audit (mode_07.rs:211), research (mode_08.rs:280).
- `--work-unit-id <id>`: query-metrics (mode_01.rs:249).
- `--before`/`--after`: prioritize-work-unit.
- `--from`/`--to`: copy-virtual-hooks.
- `--id`: list-work-units only (list_work_units.rs:51).

**JSON keys:**
- `id` on a card object: spec/work-units.json, show-work-unit (show_work_unit.rs:161), query-work-units (query_work_units.rs:372), board (board.rs:119), dependencies (dependencies.rs:217), rpc-types (work_units.rs:12).
- `workUnitId` for a reference from another object: 63 core command args, HookContext (executor.rs:23-27), list-checkpoints (:244), list-attachments (:125), query-work-units data rows (:425), rpc-types git_state.rs:64.
- `workUnit`: audit and research args (fspec/src/audit.rs:53, research.rs:79).
- `work_unit_id`: 11 MCP tool arg structs (rpc/src/mcp_dispatch/group_04/05/19/47.rs).

## Inconsistencies that predate CLI-017
1. Placeholder outliers. `<work-unit-id>` appears in add_assumption.rs:10,32, set_user_story.rs:12,105, dependencies.rs:13,99, show_event_storm.rs:15,68 and workflow_automation.rs:22. `<id>` appears in add_dependency.rs:13,103, update_work_unit_status.rs:15,98 and update_work_unit_estimate.rs:14,80.
2. Clap value_name uses three spellings.
3. audit and research use `--work-unit`, while query-metrics uses `--work-unit-id`. All three show `<id>` in help (audit.rs:17, research.rs:22, query_metrics.rs:31).
4. Link-flag placeholders differ: `<id>` in add-dependency, `<ids...>` in add-dependencies (:20-35), `<targetId>` in remove-dependency (:27-42), and the positional `[dependsOnId]`.
5. copy-virtual-hooks shows `<sourceId>`/`<targetId>` in USAGE (copy_virtual_hooks.rs:122) but `<workUnitId>` in OPTIONS, and its clap names are FROM_ID/TO_ID (mode_04.rs:132,135).
6. `--parent <parent>` placeholder in create-story/task/bug and update-work-unit.
7. Args keys `workUnit` (audit, research), `workUnitId` (the others) and `work_unit_id` (MCP).
8. Error-fix hints say `show-work-unit <id>` (add_hotspot.rs:84, add_domain_event.rs:89, discover_event_storm.rs:70,74, delete_work_unit.rs:116 and others).
9. query-work-units output mixes card `id` with `workUnitId` columns (query_work_units.rs:372 vs :425,468).
10. Stored data has both `updatedAt` and `updated` (1115 cards), and one card has `statusHistory` instead of `stateHistory`.
11. The init docs (init_doc_*.md:82) and bootstrap_doc.txt:991 say `fspec show-work-unit <id>`; the init docs mostly use `<work-unit-id>` (8 of 10) and bootstrap uses it 59 of 97 times.
12. `-p` means --prefix in list-work-units and query-work-units, but --parent in create-story/bug/task and update-work-unit.

## CLI-017 names that depart
- `--id`, which should be `--work-unit`.
- Placeholders `<id>`, `<id|any>` and `<ids>` for --parent and the link flags, which should be `<workUnitId>`.
- `doneAt`, `enteredAt:<status>` and `ageInStatus` are new names. auto_advance.rs:211 writes `completedAt`.
- `openQuestions` names a filtered subset of `questions`.
- The `dependencies` field shares its name with a command.
- `groupBy`/`key` are new JSON keys.
