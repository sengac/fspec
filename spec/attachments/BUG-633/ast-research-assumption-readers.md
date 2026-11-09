# AST research: BUG-633 assumption readers and writers

## fspec astgrep --pattern 'extra.get("assumptions")' --lang rust --path rust/fspec-core/src
```
rust/fspec-core/src/commands/generate_scenarios.rs:454:32:extra.get("assumptions")

```

## fspec astgrep --pattern 'wu.get("assumptions")' --lang rust --path rust/fspec-core/src
```
rust/fspec-core/src/commands/validate_work_units.rs:248:63:wu.get("assumptions")
rust/fspec-core/src/commands/show_work_unit.rs:423:35:wu.get("assumptions")

```

## fspec astgrep --pattern 'card.active_items("assumptions")' --lang rust --path rust/fspec-core/src
```
rust/fspec-core/src/commands/list_work_units/fields.rs:138:13:card.active_items("assumptions")

```

## fspec astgrep --pattern 'append_field(wu, &import_data, "assumptions")' --lang rust --path rust/fspec-core/src
```
rust/fspec-core/src/commands/import_example_map.rs:108:23:append_field(wu, &import_data, "assumptions")

```

## Findings

- Writers: add_assumption.rs (pushes a raw string), answer_question.rs (--add-to assumption pushes a raw string), import_example_map.rs (append_field copies the imported array).
- Readers: show_work_unit.rs (as_str_array; text output numbered 1., 2., ...), list_work_units/card.rs active_items (strings have no id), list_work_units/fields.rs and filter.rs (--fields assumptions, --has-assumptions), validate_work_units.rs (validate_string_array: only strings), generate_scenarios.rs (# ASSUMPTIONS comment block), export_example_map.rs (raw array), query_example_mapping_stats.rs (array length).
- Soft-delete pattern to follow: remove_question.rs / restore_question.rs (stable id, specifying only, idempotent without writing), show_deleted.rs (ARRAYS_IN_ORDER), compact_work_unit.rs (compact_array_field + next*Id reset).
- Command wiring for a post-port command (BUG-507 update-feature-text): canonical.rs (ts_file ""), PORTED_COMMANDS, dispatch.rs run_ported, help configs + help_dispatch_table.rs, fspec CLI bridge + cli_registration mode/route/support, refac020 ADDED_* lists, cargo_shape allowed_files, dispatcher_test ADDED_AFTER_PORT, bootstrap_doc.txt.
- get_work_unit_details (rpc/src/work_unit_details.rs) returns the raw record, so the plugin sees the stored form.
