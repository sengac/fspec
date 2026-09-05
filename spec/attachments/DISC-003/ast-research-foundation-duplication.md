# AST Research: foundation-discovery duplication + trailer touchpoints (DISC-003)

## 1. Duplicated field-scan/reminder code (target of the dedup rule)

`grep`-verified (AstGrep fn-declaration scan of `rust/fspec-core/src/commands/`) — the same
six symbols are copy-pasted verbatim in BOTH files:

| Symbol | update_foundation.rs | discover_foundation.rs |
|---|---|---|
| `agent_supports_meta_cognition` | line 341 | line 895 |
| `is_known_agent` | line 369 | line 921 |
| `scan_draft_for_next_field*` | `scan_draft_for_next_field_reminder` (401) | `scan_draft_for_next_field` (713) |
| `extract_detected_value` | line 476 | line 765 |
| `field_reminder_body` | line 486 | line 792 |
| (wrapper) | — | `generate_field_reminder` (774), `wrap_in_system_reminder` (662) |
| `TOTAL_FIELDS` | (inline `fields.len()`) | `const TOTAL_FIELDS: usize = 8` (line 707) |

`wrap_in_system_reminder` is also independently duplicated in `bootstrap.rs:217`,
`discover_event_storm.rs:50`, and `show_work_unit.rs:561` — those are out of scope for
this card (only the discover/update pair must be unified per the card text).

## 2. Envelope shape per command (trailer insertion points)

| Command | Success envelope today | `nextSteps` insertion |
|---|---|---|
| update-foundation (draft) | `{success, message, systemReminder?}` | add `nextSteps` |
| add-capability | `{success, fileName, name, description, removedCount}` | add `nextSteps` |
| remove-capability | `{success, fileName, name, removedCount?}` | add `nextSteps` |
| add-persona | `{success, fileName, removedPlaceholders, name, description, goals}` | add `nextSteps` |
| remove-persona | `{success, fileName, name, removedPlaceholders?}` | add `nextSteps` |
| add/remove-foundation-bounded-context | `{success, message}` | add `nextSteps` |
| add/remove-aggregate-to-foundation | `{success, message}` | add `nextSteps` |
| add/remove-domain-event-to-foundation | `{success, message}` | add `nextSteps` |
| add/remove-command-to-foundation | `{success, message}` | add `nextSteps` |

## 3. Event-storm item types (for trailer counting)

- `bounded_context` (add_foundation_bounded_context.rs:116)
- `aggregate` (add_aggregate_to_foundation.rs:180)
- `event` (add_domain_event_to_foundation.rs:132)
- `command` (add_command_to_foundation.rs:129)

All items live under `foundation.json → eventStorm → items[]`, soft-deleted via
`deleted: true`. show-foundation-event-storm already filters `deleted` before context/type
filtering (lines 99–138).

## 4. Dispatch wiring for the NEW foundation-status command

- `rust/fspec-core/src/dispatch.rs` `run_ported` — add arm `"foundation-status"`.
- `rust/fspec-core/src/canonical.rs` `PORTED_COMMANDS` (line 847) — add entry. NOT added to
  `CANONICAL_COMMANDS` (the 162-count invariant in tests/dispatcher_test.rs:209-247 only
  iterates CANONICAL_COMMANDS + the TS attachment; the mapping attachment has exactly 162
  entries and is untouched).
- `rust/fspec-core/src/help_dispatch_table.rs` `config_for` — add entry.
- `rust/fspec/src/main.rs` — clap variant + `forward!` arm + `intercept_ts_help` arm
  (line 4188+).
- `rust/fspec-core/src/commands/foundation_status.rs` — NEW (lives under commands/ but is
  NOT a canonical command; the dispatcher_test module-file invariant iterates
  CANONICAL_COMMANDS only, so no stub-shape constraint applies).
- CLI bridge: `rust/fspec/src/foundation_status.rs` + `mod` registration.

## 5. show-foundation auto-draft constraint

Existing dispatcher tests that MUST stay green (rust/fspec-core + rust/fspec):
- `cli_show_foundation.rs::scenario_empty_workspace_auto_creates_foundation_json` — no
  draft → auto-create final → data == 'Project Name'.
- `cli_show_foundation.rs::scenario_draft_true_reads_draft_file_instead_of_foundation` —
  explicit draft=true unchanged.
- `cli_show_foundation.rs::scenario_cli_default_render_prints_project_section` — final
  file only (no draft) → `=== PROJECT ===` (byte-identical).
- `cli_show_foundation.rs::scenario_cli_positional_section_emits_raw_string_in_text_format`
  — stdout exactly 'fspec\n' (no draft in that fixture → banner must not appear).
- byte-exact `--help` fixture `rust/fspec/tests/fixtures/help/show-foundation.txt` — the
  help CONFIG must NOT change for this card (clap accepts `--final`; the help doc is
  generated from the help CONFIG, not clap; the TS-parity fixture pins the config).

## 6. Guidance-doc test constraints (rust/tools/src/fspec_workflow_guidance.rs)

In-module `#[cfg(test)]` suite (ends ~line 1589) asserts:
- zero occurrences of `"_"` positional-arg JSON patterns,
- named keys like `workUnitId` present,
- specific arg-key presences (workUnitId, filePath, feature…).

The Phase-0 FOUNDATION section (lines 80–148) is NOT covered by any existing assertion on
its arg names — safe to rewrite, but must not introduce `"_": [` patterns and must keep
the general doc assertions passing.
