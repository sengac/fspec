# Fixing Z.AI Token/Second Calculation - PROV-002

## Problem Statement

The token/second (tok/s) calculation was showing the same count repeatedly and not responding to changes in chunk arrival rate. This made the tok/s display misleading during streaming with Z.AI (and other OpenAI-compatible providers).

## Root Cause Analysis

### 1. Fixed Time Window (3 seconds)
- The tracker used a 3-second rolling window for rate calculation
- When chunk flow changed (e.g., from fast to slow or vice versa), the "old" tokens in the window kept affecting the rate
- This caused the displayed rate to lag behind actual changes

### 2. Fixed Display Throttle (500ms)
- UI updates were throttled to every 500ms
- Combined with the 3-second window, this meant users waited 3.5+ seconds to see rate changes

### 3. Fixed EMA Smoothing (alpha=0.3 or 0.5)
- Exponential Moving Average created inertia
- Even when chunks stopped flowing, the smoothed rate decayed slowly
- This made it look like generation was still happening when it wasn't

## Solution Implemented

### 1. Reduced Time Window to 1 Second
```rust
const TIME_WINDOW: Duration = Duration::from_secs(1);  // was 3
```
- **Why**: Stale tokens exit the window 3x faster
- **Effect**: Rate reflects current chunk flow immediately

### 2. Reduced Display Throttle to 200ms
```rust
const DISPLAY_THROTTLE: Duration = Duration::from_millis(200);  // was 500
```
- **Why**: UI updates faster, users see changes sooner
- **Effect**: Displayed rate stays in sync with actual rate

### 3. Adaptive EMA with Change-Detection
```rust
let adaptive_alpha = match self.smoothed_rate {
    Some(prev) => {
        let rate_delta = (raw_rate - prev).abs();
        let relative_change = rate_delta / (prev.max(1.0));
        let alpha = (relative_change * 2.0).clamp(0.1, 0.9);
        alpha
    }
    None => 1.0,
};
```
- **How it works**:
  - When rate is stable → low alpha (0.1) = smooth display
  - When rate changes rapidly → high alpha (0.9) = immediate response
- **Effect**: No more "stuck" tok/s values when flow changes

### 4. Enhanced Debug Logging
Added detailed logging to trace calculation:
- Chunk arrivals (text length, token count, cumulative)
- Rate calculations (token_delta, time_delta, raw_rate)
- EMA parameters (prev_rate, alpha, new_rate)
- Sample window size

```rust
info!(
    "[PROV-002] Tok/s calc: samples={}, token_delta={:?}, time_delta={:?}ms, raw_rate={:.2}, prev_rate={:?}, adaptive_alpha={:.2}, new_rate={:.2}, cumulative_tokens={}",
    ...
);
```

### 5. Support for OpenAI-Compatible Providers
```rust
// For providers without streaming usage events (Z.AI)
let display_output = if turn_usage.output_tokens > 0 {
    // Anthropic/Gemini: use actual tokens
    turn_cumulative_output + turn_usage.output_tokens as u64
} else {
    // Z.AI/OpenAI: estimate from text chunks
    turn_cumulative_output + tok_per_sec_tracker.estimated_output_tokens() as u64
};
```

## Testing the Fix

### Enable Debug Logging
```bash
RUST_LOG=codelet_cli::interactive::stream_loop=debug codelet chat --provider zai
```

### What to Look For
1. **Chunk arrivals**:
   ```
   [PROV-002] Chunk received: text_len=45, estimated_tokens=12, cumulative=120
   ```

2. **Rate calculations**:
   ```
   [PROV-002] Tok/s calc: samples=3, token_delta=28, time_delta=234ms, raw_rate=119.66, prev_rate="85.23", adaptive_alpha=0.80, new_rate=107.28, cumulative_tokens=156
   ```

3. **Streaming emissions**:
   ```
   [PROV-002] Streaming emit: raw_input=1250, cache_read=0, cache_create=0, total_input=1250, output=156, tok/s=107.28
   ```

### Expected Behavior
- tok/s updates **every 200ms** (not 500ms)
- Rate responds within **1 second** of chunk flow changes (not 3+ seconds)
- Large rate changes have high alpha (0.7-0.9) → quick adaptation
- Stable rates have low alpha (0.1-0.3) → smooth display
- No more "stuck" tok/s when chunks stop flowing

## Files Modified

1. `codelet/cli/src/interactive/stream_loop.rs`
   - TokPerSecTracker constants (time window, throttle)
   - Adaptive EMA calculation
   - Enhanced logging (trace, debug, info)
   - OpenAI-compatible provider support

2. `codelet/patches/rig-core/src/providers/openai/completion/streaming.rs`
   - Fixed usage extraction to use completion_tokens directly
   - Better cache token detection

## Performance Impact

- **Memory**: No change (same Vec-based sample storage)
- **CPU**: Minimal overhead (simple arithmetic for alpha calculation)
- **Network**: No change
- **UI**: Faster updates (200ms vs 500ms throttle)

## Future Improvements

1. **Configurable constants**: Allow per-provider tuning via env vars
2. **Sample-based adaptive window**: Grow/shrink window based on variance
3. **Historical baselines**: Track typical rates per provider/model
4. **Chunk arrival prediction**: Anticipate rate changes before they happen

## References

- PROV-002: OpenAI-compatible provider token tracking
- TUI-031: Real-time token/second display
- Issue: "pathetic, it just keeps showing the same tokens per second count again"
