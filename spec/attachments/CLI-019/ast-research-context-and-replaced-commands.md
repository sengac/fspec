# CLI-019 AST research: fspec context and the three commands it replaces

Searched with `fspec astgrep --lang rust`.

## Findings

- The three commands are each one `pub async fn run(args_json, project_root)` in
  rust/fspec-core/src/commands/{show_acceptance_criteria,get_scenarios,search_scenarios}.rs,
  called only from rust/fspec-core/src/dispatch.rs (run_ported arms) and from their thin
  CLI bridges in rust/fspec/src/. Nothing else in the workspace calls them, so removing them
  only touches registration: commands/mod.rs, dispatch.rs, canonical.rs (CANONICAL_COMMANDS and
  PORTED_COMMANDS), help_dispatch_table.rs, help/configs/mod.rs, rust/fspec/src/main.rs,
  cli_registration/{mode_02,mode_06,mode_07,route_01,route_05,support_03}.rs and
  rust/fspec/tests/cargo_shape/{allowed_files,locked_files}.rs.
- Filters they had: show-acceptance-criteria --tag (AND, feature tags only), --format
  text|markdown|json, --output <file>; get-scenarios --tag (AND over feature+scenario tags),
  --format; search-scenarios --query (case-insensitive substring or --regex) over the
  feature name, description, path, work-unit title and scenario names, --json.
- Reuse: io::gherkin::parse_feature_lenient parses every feature (Rules included);
  io::feature_glob::glob_feature_files lists spec/features/**/*.feature sorted by path;
  io::gherkin_format has the canonical private formatters format_scenario,
  format_background and format_description_block (verbatim descriptions). context reuses
  them through small pub(crate) wrappers added at the end of gherkin_format.rs, so no line
  that other features' coverage maps moves.
- Tag expressions: gherkin-0.16.0's `tagexpr::TagOperation` (FromStr; And/Or/Not/Tag with the
  leading @ stripped) parses cucumber tag expressions; the crate has no evaluator, so context
  evaluates the parsed tree over the scenario's own, Rule and Feature tags (gherkin also strips @
  from those).
- Frozen contracts: rust/fspec/tests/refac020/source.rs has ADDED_COMMANDS / REMOVED_COMMANDS /
  REMOVED_COMMAND_NAMES / REMOVED_MODULES (AUDIT-003 precedent) and ADDED_MODULES; the
  cli_registration sources must stay under 300 lines (refac020 contracts.rs).
- rust/fspec-core/tests/dispatcher_test.rs counts canonical commands with ADDED_AFTER_PORT and
  REMOVED_AFTER_PORT.
- CLI-016's guidance contract (rust/fspec/src/guidance_contract_tests.rs) parses every fspec
  mention in the bootstrap doc, init docs, help configs and non-test string literals with the
  real CLI, and checks every help fixture equals its config, so any leftover mention of a
  removed command fails a test.

## Raw astgrep output

## `commands::show_acceptance_criteria::run($$$)`
```
rust/fspec-core/src/dispatch.rs:340:17:commands::show_acceptance_criteria::run(args_json, project_root)
```
## `commands::get_scenarios::run($$$)`
```
rust/fspec-core/src/dispatch.rs:561:32:commands::get_scenarios::run(args_json, project_root)
```
## `commands::search_scenarios::run($$$)`
```
rust/fspec-core/src/dispatch.rs:590:35:commands::search_scenarios::run(args_json, project_root)
```
## `fn format_scenario($$$) { $$$ }`
```
rust/fspec-core/src/io/gherkin_format.rs:429:1:fn format_scenario(
```
## `fn format_background($$$) { $$$ }`
```
rust/fspec-core/src/io/gherkin_format.rs:397:1:fn format_background(
```
## `fn format_description_block($$$) { $$$ }`
```
rust/fspec-core/src/io/gherkin_format.rs:258:1:fn format_description_block(
```
## `pub fn glob_feature_files($$$) -> $R { $$$ }`
```
rust/fspec-core/src/io/feature_glob.rs:33:1:pub fn glob_feature_files(project_root: &Path) -> Result<Vec<String>, FspecCoreError> {
```
## `pub fn parse_feature_lenient($$$) -> $R { $$$ }`
```
rust/fspec-core/src/io/gherkin.rs:107:1:pub fn parse_feature_lenient(content: &str) -> Result<Feature, ParseError> {
```
## `Some(Mode::$V { $$$ }) => $F` in route_05.rs / route_01.rs
```
rust/fspec/src/show_acceptance_criteria.rs:46:11:show_acceptance_criteria::run

rust/fspec/src/get_scenarios.rs:49:11:get_scenarios::run

rust/fspec/src/search_scenarios.rs:39:11:search_scenarios::run

```
