# PROV-169 AST research

## fspec astgrep --pattern 'merge_profile_models($$$A)' --lang rust --path rust/sessions/src
rust/sessions/src/profile_sections/persistence_tests.rs:248:19:merge_profile_models(Vec::new(), &custom, 128_000)
rust/sessions/src/profile_sections.rs:149:26:merge_profile_models(discovered, &profile.custom_models, context_window)


## fspec astgrep --pattern 'ModelEntry { $$$F }' --lang rust --path rust/sessions/src
rust/sessions/src/profile_sections/tests.rs:6:5:ModelEntry {
rust/sessions/src/cloud_models.rs:107:18:ModelEntry {
rust/sessions/src/profile_sections.rs:105:22:ModelEntry {
rust/sessions/src/profile_sections.rs:115:22:ModelEntry {
rust/sessions/src/startup_model_resolution_tests.rs:15:5:ModelEntry {


## fspec astgrep --pattern '$P.context_window.unwrap_or($D)' --lang rust --path rust/sessions/src
rust/sessions/src/profile_sections.rs:121:29:c.context_window.unwrap_or(context_window)
rust/sessions/src/profile_sections.rs:147:34:profile.context_window.unwrap_or(128_000)


## Struct literals found with grep

`ModelEntry {` — sessions/src/cloud_models.rs:107 (catalog, `m.limit.context`), profile_sections.rs:105
and :115 (profile/custom), handle_impl/handle_impl_5.rs:58 and handle_impl_8.rs:47 (custom
provider definitions, `usize` → `u32`), plus test helpers in sessions and fspec-tui. Consumers:
fspec-tui/src/views/model_selector/rows.rs:109,130-135 and crud.rs:50.
