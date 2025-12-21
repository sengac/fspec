# Tokens Per Second - Real-Time Display Implementation

## Research Summary

### Analysis of Major CLI Tools

I analyzed three major AI CLI tools to understand how they handle token rate display:

| Tool | Real-time tok/s? | What they track instead |
|------|-----------------|------------------------|
| **opencode** | No | Time-to-first-byte (TTFB), total tokens, session averages |
| **letta** | No | Time-to-first-token (TTFT) in nanoseconds, completion token counts |
| **gemini-cli** | No | Session-level aggregates, average latency, wall time |

**Key Finding:** None of these major tools implement real-time tokens per second display during streaming.

### Initial Approach (Rejected)

The initial implementation used cumulative average:
```tsx
tokensPerSecond = outputTokens / elapsedSeconds
```

**Problem:** This approach has a fundamental flaw - as `elapsedSeconds` grows, the rate naturally decreases even if token generation is steady. It doesn't reflect the *current* generation speed.

### Chosen Approach: Per-Chunk Delta Calculation

The correct approach calculates instantaneous rate from each TokenUpdate event:

1. Track previous token count and timestamp
2. When new TokenUpdate arrives: `deltaTokens / deltaTime`
3. Collect all rate samples
4. Display the average of all samples

This provides a stable, representative rate that doesn't artificially decrease over time.

## Implementation Design

### State Required

```tsx
// Track previous TokenUpdate for delta calculation
const lastTokenUpdateRef = useRef<{ tokens: number; timestamp: number } | null>(null);

// Collect instantaneous rate samples
const [tokPerSecSamples, setTokPerSecSamples] = useState<number[]>([]);
```

### Rate Calculation (in TokenUpdate handler)

```tsx
} else if (chunk.type === 'TokenUpdate' && chunk.tokens) {
  const now = Date.now();
  const currentOutputTokens = chunk.tokens.outputTokens;

  if (lastTokenUpdateRef.current) {
    const deltaTokens = currentOutputTokens - lastTokenUpdateRef.current.tokens;
    const deltaTime = (now - lastTokenUpdateRef.current.timestamp) / 1000;
    if (deltaTime > 0 && deltaTokens > 0) {
      const instantRate = deltaTokens / deltaTime;
      setTokPerSecSamples(prev => [...prev, instantRate]);
    }
  }

  lastTokenUpdateRef.current = { tokens: currentOutputTokens, timestamp: now };
  setTokenUsage(chunk.tokens);
}
```

### Display Calculation

```tsx
const tokensPerSecond = useMemo(() => {
  if (!isLoading || tokPerSecSamples.length === 0) return null;
  const sum = tokPerSecSamples.reduce((a, b) => a + b, 0);
  return sum / tokPerSecSamples.length;
}, [isLoading, tokPerSecSamples]);
```

### Reset on New Prompt

```tsx
const handleSubmit = useCallback(async () => {
  // ...
  setIsLoading(true);
  lastTokenUpdateRef.current = null;
  setTokPerSecSamples([]);
  // ...
}, [inputValue, isLoading]);
```

### Header Display

```tsx
{isLoading && tokensPerSecond !== null && (
  <Box marginRight={2}>
    <Text color="magenta">{tokensPerSecond.toFixed(1)} tok/s</Text>
  </Box>
)}
```

## Key Differences from Initial Approach

| Aspect | Initial (Cumulative) | Final (Per-Chunk Delta) |
|--------|---------------------|------------------------|
| Formula | `total / elapsed` | `deltaTokens / deltaTime` |
| Behavior | Decreases over time | Stable average |
| Accuracy | Gets stale | Reflects actual rate |
| Display timing | After 0.5s delay | After 2+ TokenUpdates |
| State | streamingStartTime | lastTokenUpdateRef, samples array |

## Why This Approach Works

1. **Each measurement is a snapshot** - Captures the actual rate between updates
2. **Averaging smooths out bursts** - No wild fluctuations
3. **No artificial decrease** - Denominator doesn't keep growing
4. **Requires real data** - Won't show until we have actual rate samples

## Edge Cases Handled

1. **First TokenUpdate**: No rate shown (need 2+ updates for delta)
2. **Zero delta tokens**: Sample not added (avoids 0 tok/s)
3. **Zero delta time**: Sample not added (avoids division by zero)
4. **Streaming ends**: Samples cleared, display hidden
5. **New prompt**: All state reset for fresh calculation

## Testing Strategy

1. Simulate multiple TokenUpdate events with different token counts
2. Verify tok/s appears after 2+ updates (not before)
3. Verify tok/s hidden when streaming ends
4. Verify display format (X.X tok/s)
5. Verify averaging produces stable values

## Files Modified

- `src/tui/components/AgentModal.tsx` - Core implementation
- `src/tui/__tests__/tokens-per-second.test.tsx` - End-to-end tests
- `spec/features/real-time-tokens-per-second-display-in-agent-modal-header.feature` - Gherkin specs
