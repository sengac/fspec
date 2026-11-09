# AST research: coverage stats recomputation

- `fspec astgrep --pattern 'fn update_stats($$$A) $$$B' --lang rust --path rust/fspec-core/src/commands`: two local copies, `link_coverage/stats.rs:20` and `unlink_coverage/stats.rs:20`. Both read `stats.totalScenarios` with `.and_then(Value::as_i64).unwrap_or(0)`, which is also a silent default.
- `link_coverage/stats.rs:24`: `total_scenarios_from_arr = scenarios.len()`, used only when the `stats` object is created (line 72).
- `delete_scenario.rs` and `delete_scenarios.rs` recompute `totalScenarios` from the scenarios list, which is why a delete corrected the count while the later add did not.
- `commands/add_scenario.rs` has no reference to `coverage`, so the scenario never reaches the coverage file.
- Look at `types/coverage.rs` (`calculate_stats`) as the single shared recomputation the two local copies should use.
