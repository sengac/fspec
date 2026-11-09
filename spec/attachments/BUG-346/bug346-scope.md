# BUG-346 scope addition: repair existing wrong summaries

Measured on 2026-11-02: summary `stats` blocks disagree with their own `scenarios` arrays. Either `totalScenarios` differs from the number of scenarios, or `coveredScenarios` differs from the number with test mappings.
- **fspec:** 28 `spec/features/*.feature.coverage` files.
- **fspec-paseo:** 20 files, for example `fspec-board-workspaces` (5 total, 8 covered, 8 scenarios) and `fspec-daemon-discovery` (7 total, 9 covered, 8 scenarios).

## Fix
1. Fix the code (add-scenario writes the coverage entry, and link-coverage / unlink-coverage recompute every stat from the scenarios array).
2. Add a validation (`fspec audit-coverage`, or `validate`) that reports any coverage file whose stats disagree with its scenarios, so this can't recur silently.
3. Recompute the stats in all 28 fspec files in the same card.

fspec-paseo FSPEC-079 recomputes its 20 files once this lands.
