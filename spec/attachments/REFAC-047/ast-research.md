# REFAC-047 AST research

Anchors in the original file (grep over top-level items):
- `:34` MatchedFeature/MatchedScenario, `:48` validate_test_step_docstrings, `:196` matching_features,
  `:258` is_work_unit_tag, `:268` step-comment section, `:472` similarity section, `:709` unit tests.

Cross-section references: validate -> matching_features, validate_steps, format_validation_error;
match_step -> hybrid_similarity; unit tests -> extract_step_comments, match_step, validate_steps, is_work_unit_tag.
`grep -l update_work_unit_status/step_docstrings.rs spec/features/*.coverage` -> none.
