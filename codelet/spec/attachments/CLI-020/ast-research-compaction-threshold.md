# AST Research: CLI-020 Autocompact Buffer

## Key Functions in src/cli/compaction_threshold.rs

### Constants
- AUTOCOMPACT_BUFFER = 50_000 tokens (line 26)
- COMPACTION_THRESHOLD_RATIO = 0.9 (line 29)

### calculate_compaction_threshold(context_window: u64) -> u64
- Calculates: (context_window - buffer) * ratio
- Uses saturating_sub for edge cases

