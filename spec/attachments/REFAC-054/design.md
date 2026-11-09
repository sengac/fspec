# REFAC-054 design

BUG-399 must edit rust/sessions/tests/prov123_active_selection_updates_default.rs (302 lines). The file's only
non-test item besides imports is the `DATA_DIR_GUARD` static; it moves into the existing sibling module
`tests/prov123_support/mod.rs` (as `pub static`), and the test file imports it. The scenarios and their bodies do not
change, so the PROV-123 feature keeps one test file. Test mappings in
active-session-model-selection-updates-default.feature.coverage shift up by the removed lines and are remapped.
