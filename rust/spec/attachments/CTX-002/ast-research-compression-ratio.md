# AST Research: Compression Ratio Fix

## Target File: core/src/compaction.rs

### Key Functions Identified:

1. **compact** (line 264)
   - Main compaction entry point
   - Contains compression ratio check that fails with anyhow::bail!

2. **meets_threshold** (line 189)
   - Checks if compression ratio meets minimum

### Bug Location:
Lines 332-337 in compact():
```rust
if !metrics.meets_threshold(self.min_compression_ratio) {
    anyhow::bail!(
        "Compaction did not meet minimum compression ratio: {:.1}% < {:.1}%",
        compression_ratio * 100.0,
        self.min_compression_ratio * 100.0
    );
}
```

### Fix Required:
Change to log WARNING instead of FAIL with anyhow::bail!. Session should continue with suboptimal compaction.

```rust
if !metrics.meets_threshold(self.min_compression_ratio) {
    tracing::warn!(
        "Compression ratio {:.1}% below {:.1}% threshold",
        compression_ratio * 100.0,
        self.min_compression_ratio * 100.0
    );
}
// Continue with compaction result regardless
```
