# BUG-374 AST research

Text and AST search in `rust/fspec-core/src`:
- `delete_scenario.rs:272-290`: computes `total_scenarios`, `covered_scenarios`, `coverage_percent` from the remaining
  scenarios and spreads the old stats object over the rest (testFiles, implFiles, totalLinesCovered unchanged).
- `delete_scenarios.rs:366-376`: `let mut stats = match coverage.get("stats") { ... }` then inserts only
  `totalScenarios`, `coveredScenarios`, `coveragePercent`.
- `types/coverage.rs:134` `pub fn calculate_stats(scenarios: &[CoverageScenario]) -> CoverageStats`: the shared
  recomputation (dedups test and impl files in insertion order), unused by both delete commands.
