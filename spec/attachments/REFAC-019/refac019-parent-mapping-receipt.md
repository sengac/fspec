# REFAC-019 fresh continuation: parent-owned corrections

All mutations below used native Fspec link-coverage/unlink-coverage. Existing
Done statuses, failed session histories, frozen baselines and BUG-283 were not
modified. No validation skips were used.

## RPC018

Four moved tests now point to their actual definitions in
rust/fspec-tui/tests/source_shape_rpc018/surface_invariants.rs:
Action variants 2-12, workspace export 15-29, model export 32-54, view isolation
57-90. Each replacement was linked before unlinking its original root-file row.

Action implementation provenance retains exactly 262 original references across
components/mod.rs:1-109,124-139; action_runtime.rs:5-111; action_board.rs:5-34.
Workspace and view-isolation implementation arrays remain unchanged.

The previous missing session_manager.rs rejection was investigated independently.
The scenario explicitly permits a sibling file. The actual documented NAPI
get_model_info item is session_bindings.rs:3998-4015, re-exported by lib.rs:147.
Its unchanged test and actual implementation were linked with native validation.
No production edits, missing-path stand-ins, assertion weakening or skipped
validation were used. This is an explicit semantic correction of the historical
missing-file mapping, not a claim of identical old-line provenance.

Three implementation mappings invalidated by extraction were relocated using
the workers' exact provenance plans. FspecService old lib.rs:1-566 becomes
lib.rs:1-62; service_declaration/finish.rs:3-8; workspace.rs:5-171;
providers_sessions.rs:5-224; supervision_credentials.rs:5-113. Only original
blank separators 236 and 457 disappear.
Backend old mod.rs:1-141 becomes mod.rs:1-65,79-87 and backend/core.rs:5-70;
only original blank separator 66 disappears.
Embedded old embedded.rs:1-135 becomes embedded.rs:1-68,82-83 and
embedded/sessions.rs:5-68; only original blank separator 69 disappears.

## RPC027 footer

The rejected footer mapping exposed a real missing centering proof. Replaced the
old speculative comment with the exact required step and assertions comparing
actual left/right rendered border padding, including terminal Unicode width and
one-column rounding tolerance. No production rendering changed.

Scoped command:
`CARGO_PROFILE_CI_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 cargo test --offline --profile ci-test -p codelet-fspec-tui --test rpc027_dialog_parity_bcd`

Receipt /tmp/refac019-footer-proof.log: 11 passed, 0 failed.
Native link then unlink succeeded for footer test thinking_level.rs:77-108 and
the unchanged implementation thinking_level_dialog.rs:1-208. Remaining RPC027
corrections are owned by worker e73f2e9e and have a separate final receipt.
