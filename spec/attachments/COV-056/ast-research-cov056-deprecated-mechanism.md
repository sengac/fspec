# AST Research: COV-056 @deprecated coverage-exemption mechanism

Generated during discovery for COV-056 (specifying phase gate: AST research evidence).

## 1. Target functions (AstGrep / grep, rust/fspec-core)

| Function | Location | Role in COV-056 |
|---|---|---|
| `deprecated_scenario_names(feature_content: &str) -> Option<HashSet<String>>` | `rust/fspec-core/src/types/coverage.rs:184` (16 lines) | New shared helper: parses a feature file via `io::gherkin::parse_feature_lenient`; returns scenario names carrying `@deprecated` (scenario-level tags) and, when the Feature header itself carries `@deprecated`, every scenario name in the file |
| `check_coverage_completeness(...)` | `rust/fspec-core/src/commands/update_work_unit_status.rs:758` (183 lines) | Gate used by done/validating transitions. COV-056 additions: deprecated set computed at 813-824; `uncovered` filter excludes deprecated names at 857-868 (line 868: `.filter(\|n\| !deprecated.contains(*n))`); `without_impl` filter excludes them at 896-937 (line 914) |
| `show_single_feature(...)` | `rust/fspec-core/src/commands/show_coverage.rs:168` | Reads the sibling `.feature` file and builds the deprecated set (lines 211-221); parse failure or missing file -> empty set (no markers) |
| `render_single_markdown(..., deprecated: &HashSet<String>)` | `rust/fspec-core/src/commands/show_coverage.rs:288` | Per-scenario heading loop 341-361: deprecated scenario gets `### <symbol> <name> (<label>) [DEPRECATED]`; live scenarios render the unchanged heading (byte-identical output when untagged) |
| `render_single_json(..., deprecated: &HashSet<String>)` | `rust/fspec-core/src/commands/show_coverage.rs:432` | `EnrichedScenario` struct (lines 420-431) gains `deprecated: Option<bool>` with `#[serde(skip_serializing_if = "Option::is_none")]` so untagged features stay byte-identical; set via `is_dep.then_some(true)` at 448 |

## 2. Tag parsing facts (gherkin crate v0.16.0)

- Scenario tags live on `scenario.tags` (no `@` prefix); scenario keyword is `Scenario` / `Scenario Outline`.
- Feature-level tags live on `feature.tags` from the `Feature:` header line.
- `parse_feature_lenient` (crate `io::gherkin`) is the tolerant parser already used by show-feature etc.; returns None on hard failure -> helper maps that to `None` -> callers use `unwrap_or_default()`.

## 3. Call sites that consume the deprecated set

- `update_work_unit_status.rs:823` — gate: `deprecated = crate::types::coverage::deprecated_scenario_names(&content).unwrap_or_default();`
- `show_coverage.rs:218` — display: same helper, `.and_then(...).unwrap_or_default()`
- `coverage.rs:317,325,333,339` — inline unit tests of the helper (scenario-level, feature-level, no-tag, unparseable)

## 4. Tests (RED first, now GREEN)

- `rust/fspec-core/tests/cov056_deprecated_gate.rs` (246 lines, 4 tests): done-gate exempts scenario-level deprecated; done-gate still blocks uncovered live scenarios; validating gate exempts deprecated from without_impl; feature-level @deprecated deprecates every scenario
- `rust/fspec-core/tests/cov056_deprecated_display.rs` (165 lines, 3 tests): markdown DEPRECATED marker + live heading unmarked + stats still count deprecated; JSON `deprecated: true` flag + coverageStatus preserved + totalScenarios includes deprecated; untagged feature byte-identical (no DEPRECATED substring, unchanged heading shape)

## 5. Behavioral invariants

1. Exemption is additive: deprecated scenarios are removed from `uncovered`/`without_impl` blocking lists only; stats still count them (coverage percent includes them).
2. Display is additive: `[DEPRECATED]` suffix in markdown headings; `deprecated: true` (omitted when false) in JSON entries.
3. Untagged features: zero output change (byte-identity scenario).
4. Parse failures: helper returns None -> empty set -> behavior identical to pre-COV-056.
