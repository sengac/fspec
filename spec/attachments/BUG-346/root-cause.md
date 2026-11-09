# Root cause: coverage stats go stale after add-scenario

Found while working on MCP-007.

## What happened
1. `fspec add-scenario spec/features/mcp-stdio-structured-arguments.feature "<name>"` added the scenario to the feature but not to `mcp-stdio-structured-arguments.feature.coverage`.
2. `fspec link-coverage` then refused the scenario because it was missing from the coverage file, so an entry had to be added by hand.
3. After linking, the stats read `coveredScenarios: 7`, `totalScenarios: 5`, `coveragePercent: 140`.

## Where
- `rust/fspec-core/src/commands/add_scenario.rs`: never touches the `.feature.coverage` file.
- `rust/fspec-core/src/commands/link_coverage/stats.rs` (`update_stats`, about line 88) and `rust/fspec-core/src/commands/unlink_coverage/stats.rs` (about line 88): when a `stats` object already exists, they read `totalScenarios` from it as-is and only fill it from `scenarios.len()` when stats are missing (line 72). The comment says the TypeScript version did the same.

## Fix
- `add-scenario` adds an empty coverage entry (`{"name": ..., "testMappings": []}`) for the new scenario whenever the feature has a coverage file.
- Every coverage write recomputes `totalScenarios` from the scenarios list and derives `coveragePercent` from it.
