# CLI-018 AST research (fspec astgrep, 2026-11-08)

## fn feature_tags in list_work_units
```
rust/fspec-core/src/commands/list_work_units/filter.rs:147:1:fn feature_tags(root: &Path) -> Result<TagsByCard, String> {

```
Today's --tag reads only the feature header (list_features::parse_feature_header), so scenario tags are invisible.

## parse_feature_lenient call sites (the shared Gherkin parse)
```
rust/fspec-core/src/commands/generate_coverage/plan.rs:149:19:parse_feature_lenient(&content)
rust/fspec-core/src/commands/generate_coverage/global.rs:19:19:parse_feature_lenient(&feature_content)
rust/fspec-core/src/commands/add_step.rs:111:25:parse_feature_lenient(&content)
rust/fspec-core/src/commands/add_step.rs:217:17:parse_feature_lenient(&new_content)
rust/fspec-core/src/commands/validate.rs:344:11:parse_feature_lenient(content)
rust/fspec-core/src/commands/search_scenarios.rs:120:27:parse_feature_lenient(&content)
rust/fspec-core/src/commands/show_feature.rs:208:25:parse_feature_lenient(&content)
rust/fspec-core/src/commands/delete_scenario.rs:73:25:parse_feature_lenient(&content)
rust/fspec-core/src/commands/delete_scenario.rs:102:21:parse_feature_lenient(&new_content)
rust/fspec-core/src/commands/show_acceptance_criteria.rs:100:27:parse_feature_lenient(&content)
rust/fspec-core/src/commands/remove_tag_from_feature.rs:65:25:parse_feature_lenient(&content)
rust/fspec-core/src/commands/remove_tag_from_feature.rs:104:17:parse_feature_lenient(&new_content)
rust/fspec-core/src/commands/update_step.rs:91:25:parse_feature_lenient(&content)
rust/fspec-core/src/commands/update_step.rs:170:21:parse_feature_lenient(&new_content)
rust/fspec-core/src/commands/generate_scenarios.rs:666:27:parse_feature_lenient(&content)
rust/fspec-core/src/commands/add_tag_to_scenario.rs:119:19:parse_feature_lenient(&content)
rust/fspec-core/src/commands/update_feature_text.rs:59:19:parse_feature_lenient(&content)
rust/fspec-core/src/commands/update_feature_text.rs:109:20:parse_feature_lenient(&updated)
rust/fspec-core/src/commands/retag.rs:156:25:parse_feature_lenient(&new_content)
rust/fspec-core/src/commands/delete_features.rs:85:29:parse_feature_lenient(&content)
rust/fspec-core/src/commands/add_scenario.rs:91:25:parse_feature_lenient(&content)
rust/fspec-core/src/commands/add_scenario.rs:163:17:parse_feature_lenient(&new_content)
rust/fspec-core/src/commands/add_architecture.rs:98:21:parse_feature_lenient(&content)
rust/fspec-core/src/commands/add_architecture.rs:109:21:parse_feature_lenient(&new_content)
rust/fspec-core/src/commands/delete_step.rs:75:25:parse_feature_lenient(&content)
rust/fspec-core/src/commands/delete_step.rs:148:21:parse_feature_lenient(&new_content)
rust/fspec-core/src/commands/format.rs:81:29:parse_feature_lenient(&content)
rust/fspec-core/src/commands/link_coverage/step.rs:191:19:parse_feature_lenient(feature_content)
rust/fspec-core/src/commands/create_feature.rs:206:19:parse_feature_lenient(feature_content)
rust/fspec-core/src/commands/remove_tag_from_scenario.rs:71:19:parse_feature_lenient(&content)
rust/fspec-core/src/commands/get_scenarios.rs:112:27:parse_feature_lenient(&content)
rust/fspec-core/src/commands/delete_scenarios.rs:106:29:parse_feature_lenient(&content)
rust/fspec-core/src/commands/delete_scenarios.rs:182:25:parse_feature_lenient(&new_content)
rust/fspec-core/src/commands/add_tag_to_feature.rs:110:25:parse_feature_lenient(&content)
rust/fspec-core/src/commands/add_tag_to_feature.rs:165:17:parse_feature_lenient(&new_content)
rust/fspec-core/src/commands/validate_tags.rs:211:25:parse_feature_lenient(content)
rust/fspec-core/src/commands/update_scenario.rs:81:25:parse_feature_lenient(&content)
rust/fspec-core/src/commands/update_scenario.rs:128:21:parse_feature_lenient(&new_content)
rust/fspec-core/src/commands/add_background.rs:99:21:parse_feature_lenient(&content)
rust/fspec-core/src/commands/add_background.rs:110:21:parse_feature_lenient(&new_content)
rust/fspec-core/src/commands/list_scenario_tags.rs:163:25:parse_feature_lenient(&content)

```
The scenario tags come from the same parse other commands use (search_scenarios, show_feature, get_scenarios): Feature.scenarios and each Rule's scenarios carry their own tags.
