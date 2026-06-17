# RPC-334 — Rust code inventory: where to apply native serde diagnostics

**Attachment for RPC-334.** Enumerates every production site that parses a JSON
**file** and surfaces a user-facing parse error, classified by the change
required. Test-only `from_str` calls and `args_json` deserialization are
excluded (those parse internal RPC payloads, not corrupt-able user files).

Generated from `codelet/fspec-core/src` via grep on `serde_json::from_str` +
`FspecCoreError::ParseJson` / `InvalidArgs`.

---

## Group 0 — Shared funnel (fix here first; covers the most sites)

`io/locked_file.rs::read_or_init_json` is the single read+parse path behind all
the `ensure_*` helpers. Routing **these two `from_str` sites** through the new
formatter upgrades every `ensure_work_units_file` / `ensure_prefixes_file` /
`ensure_tags_file` / `ensure_foundation_file` / `ensure_epics_file` caller for
free (dozens of commands).

| File | Line | Notes |
|------|------|-------|
| `io/locked_file.rs` | 60 | fast-path read (`buf`) — raw input available |
| `io/locked_file.rs` | 82 | re-read after init (`raw`) — raw input available |

`io/ensure.rs` read-only twins (their own `from_str`, raw input available):

| File | Line | File label |
|------|------|-----------|
| `io/ensure.rs` | 250 | `prefixes.json` |
| `io/ensure.rs` | 281 | `work-units.json` (match form) |
| `io/ensure.rs` | 308 | `epics.json` |

## Group 1 — The V8-emulation removal (PRIMARY scope of this card)

These 6 commands fabricate a `Unexpected token in JSON:` prefix that the TS
frontend never emits. **Remove the prefix** and route the raw input through the
shared formatter. (All already hold the file contents as `raw`.)

| File | Line | Current reason expression |
|------|------|---------------------------|
| `commands/auto_advance.rs` | 149 | `wrap_failure(format!("Unexpected token in JSON: {e}"))` |
| `commands/record_iteration.rs` | 104 | `wrap_failure(format!("Unexpected token in JSON: {e}"))` |
| `commands/workflow_automation.rs` | 126 | `invalid_args(format!("Unexpected token in JSON: {e}"))` |
| `commands/query_work_units.rs` | 70 | `wrap_failure(format!("Unexpected token in JSON: {e}"))` |
| `commands/query_metrics.rs` | 146 | `wrap(format!("Unexpected token in JSON: {e}"))` |
| `commands/export_work_units.rs` | 106 | `wrap_failure(format!("Unexpected token in JSON: {e}"))` |

> Note: these use `InvalidArgs`, not `ParseJson`, because the TS commands wrap
> the message in a command-specific outer prefix (`Failed to auto-advance:` /
> `Failed to query metrics:` etc.). Keep those outer prefixes; only the inner
> body changes from the fabricated V8-ish text to the serde snippet.

## Group 2 — Command-local `ParseJson` sites (raw input available)

Each does its own `serde_json::from_str(&raw|&content)` then builds
`FspecCoreError::ParseJson { reason: e.to_string() }`. Swap `e.to_string()` for
the shared formatter helper. The raw variable name is in the last column.

| File | Line | File label | Raw var |
|------|------|-----------|---------|
| `commands/show_work_unit.rs` | 89 | work-units.json | `raw` |
| `commands/validate_work_units.rs` | 74 | work-units.json | `raw` |
| `commands/dependencies.rs` | 99 | work-units.json | `raw` |
| `commands/remove_capability.rs` | 77 | foundation.json | `raw` |
| `commands/add_capability.rs` | 82 | foundation.json | `raw` |
| `commands/add_persona.rs` | 121 | foundation.json | `raw` |
| `commands/remove_persona.rs` | 80 | foundation.json | `raw` |
| `commands/update_foundation.rs` | 131 | foundation.json | `raw` |
| `commands/show_foundation.rs` | 82 | foundation.json | `raw` |
| `commands/show_foundation_event_storm.rs` | 68 | foundation.json | `raw` |
| `commands/add_aggregate.rs` | 94 | work-units.json | `raw` |
| `commands/add_domain_event.rs` | 94 | work-units.json | `raw` |
| `commands/add_command.rs` | 89 | work-units.json | `raw` |
| `commands/add_policy.rs` | 125 | work-units.json | `raw` |
| `commands/add_hotspot.rs` | 97 | work-units.json | `raw` |
| `commands/add_bounded_context.rs` | 138 | work-units.json | `raw` |
| `commands/add_external_system.rs` | 139 | work-units.json | `raw` |
| `commands/discover_event_storm.rs` | 297 | work-units.json | `raw` |
| `commands/generate_example_mapping_from_event_storm.rs` | 106 | work-units.json | `raw` |
| `commands/delete_diagram.rs` | 62 | foundation.json | `raw` |
| `commands/delete_tag.rs` | 91 | tags.json | `raw` |
| `commands/update_tag.rs` | 96 | tags.json | `raw` |
| `commands/generate_tags_md.rs` | 88 | tags.json | `content` |
| `commands/generate_foundation_md.rs` | 90 | foundation.json | `content` |
| `commands/configure_tools.rs` | 97 | fspec-config.json | `raw` |
| `commands/remove_hook.rs` | 89 | fspec-hooks.json | `raw` |
| `commands/add_schedule.rs` | 211 | schedules.json | `raw` |
| `commands/pause_schedule.rs` | 90 | schedules.json | `raw` |
| `commands/resume_schedule.rs` | 91 | schedules.json | `raw` |
| `commands/remove_schedule.rs` | 108 | schedules.json | `raw` |

## Group 3 — `InvalidArgs`-wrapped file parses (raw input available)

Same swap as Group 1 but not V8-prefixed; they embed a command-specific wrapper
and `{e}`. Replace `{e}` body with the formatter output.

| File | Line | Wrapper |
|------|------|---------|
| `commands/generate_summary_report.rs` | 55 | `Failed to generate summary report:` |
| `commands/query_estimate_accuracy.rs` | 234 | (InvalidArgs) |
| `commands/audit_coverage.rs` | 111 | (InvalidArgs) |
| `commands/check.rs` | 118 | (InvalidArgs) |
| `commands/link_coverage.rs` | 106 | coverage file |
| `commands/import_example_map.rs` | 98 | import file |

## Group 4 — `.map_err(|e| e.to_string())` String-returning sites

Return a bare `String` reason (later wrapped by caller). Same body swap.

| File | Line |
|------|------|
| `commands/add_tag_to_scenario.rs` | 324 |
| `commands/add_tag_to_feature.rs` | 284 |
| `commands/validate_spec_alignment.rs` | 88 |
| `commands/compare_implementations.rs` | 65 |

## Group 5 — Deliberately NOT changed (silent / lenient parses)

These intentionally swallow parse errors (`.ok()?`, `match { Err => empty }`,
`if let Ok`) to mirror TS `catch {}` leniency — e.g. `list-hooks`,
`list-schedules`, `read_work_units_or_empty`, coverage-file readers,
`validate-tags`, `validate-hooks`, `search-*`, `create-*` epic-lookup helpers.
**Do not** route these through the formatter; they must keep returning
empty/None on malformed input for behavioural parity.

---

## Recommended implementation order

1. Add `codelet-fspec-json-error` dep to `fspec-core` (crate already created).
2. Add helper `io/json_error.rs::parse_json_diagnostic(file_label, input, &err)`
   returning `FspecCoreError::ParseJson` (and a `*_reason(input, &err) -> String`
   variant for Groups 3–4) that runs the formatter.
3. Wire **Group 0** (funnel) → biggest blast radius, smallest diff.
4. Wire **Group 1** (the V8-prefix removal — the headline of this card).
5. Wire **Groups 2–4**.
6. Decide `ParseJson` Display shape: the multi-line snippet sits awkwardly
   before the trailing `". The file may be corrupted…"` sentence — likely move
   that sentence to a leading line and let the snippet trail. (Design TODO.)
7. Update the substring assertions in the affected command tests (they assert
   on the old serde text / V8-ish prefix).

## Test impact (assert on the old wording — will need updates)

`grep -rn "Unexpected token in JSON\|key must be a string"` across
`fspec-core` + `fspec` CLI integration tests; plus the 6 Group-1 command test
modules and `io/locked_file.rs` test `read_or_init_returns_parse_error_for_malformed_json`.
