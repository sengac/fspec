# AMGR-016: Stalled Subordinate Agent Investigation

## Incident Summary

During a SESS-015 investigation session on 2026-04-06, 5 subordinate agents were spawned with DeepSearch research tasks. 4 of 5 completed successfully. Agent `179f28e7` became permanently stuck in `running` state.

## Timeline

| Time | Event |
|------|-------|
| ~00:28 | Agent `179f28e7` spawned with role "senior Rust systems engineer" |
| ~00:28 | Task dispatched: research fix approaches for SESS-015 (5 DeepSearch sub-questions) |
| ~01:04 | Agent's last visible turn (turn 10): a DeepSearch result was returned (role=user) |
| ~01:04+ | **Agent never produced an assistant response after the DeepSearch result** |
| 01:15 | `await_idle` with 600s timeout returned `timed_out` for this agent |
| 01:19 | `get_status` still shows `running`, `pending_messages: 0` |
| 01:20 | Agent closed manually |

## Evidence

### Turn 10 (last visible turn)
- **Role**: `user` (this is how DeepSearch results are injected — as a user message)
- **Content**: Begins with "Now I have the complete picture. Here's the comprehensive answer:" followed by detailed analysis of `set_bridge_session_context` call sites
- **Timestamp**: `2026-04-06T01:04:44.122646Z`

### Turn 11+
- **Does not exist**. `SessionSearch` with `start_turn=11` returns empty `messages: []`
- The agent received the DeepSearch result but never generated a response

### Agent status at time of investigation
```json
{
  "session_id": "179f28e7-9b79-455f-bcfa-1feb45732769",
  "status": "running",
  "pending_messages": 0,
  "subordinate_ids": []
}
```

## Comparison with Successful Agents

All 5 agents had identical roles, similar task complexity, and used DeepSearch. The 4 that succeeded:

| Agent | DeepSearches | Turns | Completed |
|-------|-------------|-------|-----------|
| `078db8b3` | 3+ | 43 | ✅ (took ~33 min) |
| `4c85572f` | 2+ | 5 | ✅ (took ~18 min) |
| `930638d6` | 3+ | 11 | ✅ (took ~17 min) |
| `869a119c` | 2+ | 7 | ✅ (took ~30 min) |
| `e71023e1` | 3+ | 9 | ✅ (took ~26 min) |
| **`179f28e7`** | **3+** | **10** | **❌ stuck** |

## Root Cause Hypotheses

### H1: LLM API error swallowed silently (most likely)
The DeepSearch result was injected as turn 10. The agent loop then called the LLM API to generate a response. If the API returned an error (rate limit, timeout, context too large), the error may have been swallowed without transitioning the agent to `idle` or `errored`.

### H2: Context window overflow
Agent `179f28e7` was asked 5 complex sub-questions requiring multiple DeepSearches. By turn 10, the accumulated context (role + task + multiple DeepSearch results) may have exceeded the model's context window, causing a silent failure.

### H3: Deadlock in generation pipeline
A concurrency issue (e.g., holding a lock while awaiting generation) could prevent the agent from ever completing.

## What Should Happen Instead

1. **Generation timeout**: If no tokens are received from the LLM within N seconds (e.g., 120s) after a tool result is injected, abort the turn and mark the agent as errored
2. **Error propagation**: If the LLM API returns an error, the agent should transition to `idle` with an error state, not stay in `running`
3. **Status granularity**: `get_status` should distinguish:
   - `running` — actively generating tokens
   - `waiting_for_tool` — waiting for a DeepSearch/tool to return
   - `stalled` — no tokens received for >N seconds
   - `errored` — LLM API returned an error
4. **`await_idle` improvements**: Should be able to return `errored` status, not just `idle` or `timed_out`
5. **Heartbeat/watchdog**: A background task that monitors time-since-last-token for all running agents and auto-aborts stalled ones
