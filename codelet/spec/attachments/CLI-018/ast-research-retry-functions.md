# AST Research: CLI-018 Retry Logic Functions

## Key Functions in src/agent/compaction.rs

### generate_summary (lines 352-418)
- Implements CLI-018 retry logic with exponential backoff
- Uses RETRY_DELAYS_MS constant [0, 1000, 2000]
- Falls back to FALLBACK_SUMMARY on all retries exhausted

### compact (lines 264-350)
- Main compaction function that calls generate_summary
- Does not fail if LLM fails - relies on generate_summary fallback

### RETRY_DELAYS_MS constant (line 23)
- Defines retry delays: [0, 1000, 2000] milliseconds

