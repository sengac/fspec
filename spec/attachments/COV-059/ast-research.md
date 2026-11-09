# COV-059 AST research

## fspec astgrep --pattern 'codelet_common::file_lock::with_file_lock' --lang rust --path rust/agent-loop/src
rust/agent-loop/src/schedule_handler.rs:18:5:codelet_common::file_lock::with_file_lock

## fspec astgrep --pattern 'super::retain_populated_cloud_sections($$$A)' --lang rust --path rust/sessions/src
rust/sessions/src/profile_sections/tests.rs:122:16:super::retain_populated_cloud_sections(sections)
rust/sessions/src/profile_sections/tests.rs:139:16:super::retain_populated_cloud_sections(sections)
rust/sessions/src/profile_sections/tests.rs:162:16:super::retain_populated_cloud_sections(sections)

## fspec astgrep --pattern 'use codelet_napi::$$$X;' --lang rust --path rust/fspec-tui/src
No matches found

## Findings

- The schedule handler that uses `codelet_common::file_lock::with_file_lock` lives at
  `rust/agent-loop/src/schedule_handler.rs:18` — the rewritten file_lock steps name it.
- The PROV-127 drop-empty filter is exercised by the unit tests of
  `model-selector-cloud-section-assembly` (`rust/sessions/src/profile_sections/tests.rs:122,139,162`),
  so the retired e2e scenario has no unique behaviour left.
- No Rust source imports `codelet_napi::` and `rust/napi` does not exist (`ls rust/napi` fails),
  so the napi scenarios and the `codelet_napi::` needle describe nothing.
