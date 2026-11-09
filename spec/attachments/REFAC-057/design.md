# REFAC-057 design

BUG-399 must edit rust/agent-loop/tests/prov145_profile_config_applied.rs (374 lines). The Fixtures block
(`DATA_DIR_GUARD`, `MODELS_FIXTURE`, `fresh_background_session`, `NORMAL_WORDS`, `LOOP_PHRASE`,
`loop_stream_words`, `feed_loop_stream`) and the `read_source` helper move unchanged into
`tests/prov145_profile_config_applied_support/mod.rs`; the items the tests use are `pub`. The six tests are
unchanged (test file 265 lines). The module doc now names the feature the coverage actually maps
(per-profile-loop-detection-detector-construction.feature; it named a feature file that does not exist). The six
test mappings are remapped to the new ranges (the old ranges were already offset from the test bodies).
