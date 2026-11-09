# CLI-017 AST research: list-work-units filter chain, clap surface and reusable readers

fspec astgrep, 2026-11-07, over the files this card changes or reuses.

## `fn $NAME($$$A) -> $R { $$$B }` in rust/fspec-core/src/commands/list_work_units.rs

```
rust/fspec-core/src/commands/list_work_units.rs:81:1:fn filter_and_summarize(
rust/fspec-core/src/commands/list_work_units.rs:134:1:fn render_text(summaries: &[Value]) -> String {
rust/fspec-core/src/commands/list_work_units.rs:170:5:fn parse_wu(v: Value) -> WorkUnit {
```

## `pub async fn run($$$A) -> $R { $$$B }` in rust/fspec/src/list_work_units.rs

```
rust/fspec/src/list_work_units.rs:59:1:pub async fn run(args: CliArgs) -> Result<u8> {
```

## `fn parse_feature_header($$$A) -> $R { $$$B }` in rust/fspec-core/src/commands/list_features.rs

```
rust/fspec-core/src/commands/list_features.rs:166:1:fn parse_feature_header(content: &str) -> Option<ParsedHeader> {
```

## `pub(super) fn check_unanswered_questions($$$A) -> $R { $$$B }` in rust/fspec-core/src/commands/update_work_unit_status/queries.rs

```
rust/fspec-core/src/commands/update_work_unit_status/queries.rs:124:1:pub(super) fn check_unanswered_questions(
```

## `pub fn type_str(&self) -> &str { $$$B }` in rust/fspec-core/src/types/work_unit.rs

```
rust/fspec-core/src/types/work_unit.rs:645:5:pub fn type_str(&self) -> &str {
```

## Findings

- list_work_units::run parses ListWorkUnitsArgs (status, prefix, epic, type, format), loads the board with ensure_work_units_file (one parse of spec/work-units.json) and runs filter_and_summarize then render_text; the filter chain is a sequence of Vec::retain calls, so new filters are more retain steps over the same parsed board.
- The fspec CLI bridge only rebuilds the JSON args from clap fields (Mode::ListWorkUnits in cli_registration/mode_01.rs, routed in route_01.rs); no logic lives there.
- An open question is one that is neither deleted nor selected (check_unanswered_questions, show_work_unit's projection); list-work-units uses the same definition.
- list_features::parse_feature_header returns the feature-level tags of a feature file without a full Gherkin parse; --tag reuses it.
- WorkUnit::type_str keeps the missing-type-is-story default that RPC-253 scenarios rely on.
- No clock seam exists in fspec-core commands; time-dependent queries need one (run_at with a BoardClock).
