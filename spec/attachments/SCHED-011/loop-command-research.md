# /loop Command Research — Claude Code Analysis & fspec Feasibility

## 1. What /loop Does in Claude Code

`/loop` is a slash command (introduced in v2.1.71, stabilized in v2.1.72) that schedules a prompt to run on a recurring interval. It is **syntactic sugar** over Claude Code's internal cron scheduling system (`CronCreate`, `CronDelete`, `CronList` tools).

### Syntax

```
/loop [interval] <prompt or slash command>
```

### Interval Parsing Rules (Priority Order)

1. **Leading token**: If the first token matches `^\d+[smhd]$` (e.g., `5m`, `2h`), that's the interval; rest is the prompt.
2. **Trailing "every" clause**: If the input ends with `every <N><unit>` (e.g., `every 20m`, `every 5 minutes`), extract that as interval.
3. **Default**: No interval detected → defaults to **10 minutes**, entire input = prompt.

### Supported Units

| Unit | Example | Notes |
|------|---------|-------|
| `s` | `45s` | Rounds up to nearest minute (cron has 1-min granularity) |
| `m` | `5m` | Direct minutes; `90m` rounds to nearest clean value |
| `h` | `2h` | Hours |
| `d` | `1d` | Days |

### Interval → Cron Conversion

| Pattern | Cron | Notes |
|---------|------|-------|
| Nm (N≤59) | `*/N * * * *` | every N minutes |
| Nm (N≥60) | `0 */H * * *` | round to hours |
| Nh (N≤23) | `0 */N * * *` | every N hours |
| Nd | `0 0 */N * *` | every N days |
| Ns | `ceil(N/60)m` → then minutes rule | rounds up to minutes |

### Key Behaviors

- **Session-scoped**: Tasks live in the current process, gone when you exit
- **3-day auto-expiry**: Recurring tasks delete themselves after 72 hours
- **Max 50 concurrent tasks** per session
- **Fires between turns**: Scheduled prompts fire when the agent is idle, never mid-response
- **Jitter**: Recurring tasks fire up to 10% of period late (capped at 15 min); one-shot tasks fire up to 90s early. Jitter is deterministic per task ID.
- **No catch-up for missed fires**: If a task's time passes while agent is busy, it fires once when idle
- **Local timezone**: All times interpreted in user's local timezone

### Architecture (Claude Code)

```
User types: /loop 5m check deploys
        │
        ▼
registerLoopSkill.getPromptForCommand()
        │  generates structured LLM prompt with parsing rules
        ▼
LLM parses "5m check deploys" → interval=5m, prompt="check deploys"
        │  converts 5m → cron "*/5 * * * *"
        ▼
LLM calls CronCreate tool
        │  { cron: "*/5 * * * *", prompt: "check deploys", recurring: true }
        ▼
CronCreate.call()
        │  stores job in memory
        │  sets scheduledTasksEnabled = true
        ▼
CronScheduler tick (every 1s)
        │  checks if current time ≥ next fire time (with jitter)
        │  waits for REPL idle
        ▼
onFire("check deploys") → enqueues prompt into REPL
```

**Key insight**: In Claude Code, `/loop` doesn't actually parse the interval itself — it generates a prompt that instructs the LLM to parse the interval and call `CronCreate`. The LLM is the parser.

### Common Use Patterns

1. **Polling deployments**: `/loop 5m check if staging returns 200`
2. **Babysitting PRs**: `/loop 30m /review-pr 1234`
3. **Morning summaries via MCP**: `/loop 1d summarize Slack messages`
4. **Build monitoring**: `/loop 10m check CI build status`
5. **One-shot reminders**: Natural language like "remind me in 45 minutes to push" (schedules single-fire cron task)
6. **Chaining with skills**: `/loop 20m /my-custom-skill`

---

## 2. How fspec's Scheduling System Differs

fspec's SCHED-001 parent story already covers **much more** than Claude Code's ephemeral cron system:

| Aspect | Claude Code /loop | fspec SCHED-001 |
|--------|------------------|-----------------|
| **Persistence** | In-memory only, session-scoped | Persisted in `spec/schedules.json` |
| **Job types** | Prompt injection only | Agent sessions (full subordinate) + shell commands |
| **Session model** | Fires into the same REPL | Spawns separate subordinate sessions via AgentManager |
| **Overlap policy** | No concept | Skip or queue policies |
| **Session limits** | 50 cron tasks | MAX_SESSIONS (10) for agent jobs |
| **Catch-up** | None | Single most-recent missed trigger on restart |
| **Timezone** | Implicit (local) | Explicit per-schedule timezone |
| **Notifications** | None | Bridge notifications (Telegram) on failure/completion |
| **Blocklist** | N/A | Fail-fast on blocked tools |
| **Expiry** | 3-day auto-expiry | No auto-expiry (persistent) |

### What /loop Would Add to fspec

A `/loop` command in fspec would be a **UX shortcut** — the simplest possible path to schedule a recurring agent prompt. It would:

1. Parse a natural-language interval + prompt (no `--cron`, `--tz`, `--role` flags)
2. Convert to a cron expression
3. Create a **session-scoped, auto-expiring** schedule (unlike the persistent `/schedule add`)
4. Use sensible defaults: overlap=skip, timezone=local, no role, 3-day expiry

This is a **different tier** from `/schedule add` — ephemeral convenience vs. persistent configuration.

---

## 3. fspec Codebase Feasibility Assessment

### What Already Exists

#### Slash Command Infrastructure (Ready)
- `src/tui/slashCommands.ts` — Registry of all slash commands with name, description, and handler references
- `src/tui/hooks/useSlashCommandInput.ts` — Input parsing hook that detects `/` prefix and routes to handlers
- `AgentView.tsx` `handleSubmitWithCommand()` — Dispatch function that maps slash command names to handlers

#### Session/Agent Infrastructure (Ready via AgentManager)
- `AgentManager` tool — Spawn subordinate sessions, send messages, manage lifecycle
- `ChainOfCommand` graph — Tracks parent/child session relationships
- `BackgroundSession` — Full agent loop for non-interactive sessions
- `SessionSearch` — Cross-session history search for finding past job results

#### Schedule Infrastructure (Planned in SCHED-002 through SCHED-010)
- `SCHED-002`: `spec/schedules.json` schema and persistence
- `SCHED-003`: Core scheduler engine (tokio task with interval timer)
- `SCHED-004`: Agent job execution (subordinate session spawning)
- `SCHED-005`: Shell job execution
- `SCHED-008`: `/schedule` slash commands (add, list, pause, resume, remove)
- `SCHED-009`: AI-callable Schedule tool

### What /loop Needs Beyond Existing SCHED Work

| Component | Status | Notes |
|-----------|--------|-------|
| Interval parser (`5m`, `2h`, `every 30m`, etc.) | **New** | Simple regex-based parser, ~50-100 lines |
| Interval → cron converter | **New** | Mapping table from units to cron expressions, ~30-50 lines |
| Session-scoped (non-persistent) schedule variant | **New** | Needs a `sessionScoped: true` flag on schedule entries — not written to `spec/schedules.json` but held in scheduler memory |
| Auto-expiry (3-day TTL) | **New** | Add `expiresAt` field to schedule entries, scheduler checks on tick |
| `/loop` slash command registration | **Trivial** | Add entry to `slashCommands.ts`, handler function |
| `/loop cancel <id>` subcommand | **Trivial** | Maps to schedule removal |
| TUI feedback (confirmation message) | **Trivial** | Display scheduled job ID, cadence, next fire time |

### Implementation Approach Options

**Option A: LLM-as-Parser (Claude Code approach)**
- `/loop` generates a structured prompt that the LLM interprets
- LLM calls the Schedule tool with parsed cron + prompt
- Pro: Handles natural language edge cases ("every other Tuesday")
- Con: Requires active agent turn, adds latency, burns tokens

**Option B: Deterministic Parser (Recommended for fspec)**
- `/loop` has a small TypeScript parser that handles `Ns`, `Nm`, `Nh`, `Nd` and `every N unit` patterns
- Falls through to Schedule tool directly — no LLM involved
- Pro: Instant, zero tokens, predictable
- Con: Can't handle arbitrary natural language (but we don't need that — `/schedule add` handles complex cases)

**Recommendation: Option B.** fspec already has the full `/schedule add` command for complex scheduling. `/loop` should be the fast, zero-overhead shorthand — parse deterministically, create schedule, confirm. No LLM round-trip needed.

### Effort Estimate

| Work | Estimate |
|------|----------|
| Interval parser + cron converter | 2-3 hours |
| Session-scoped schedule variant in scheduler | 1-2 hours |
| Auto-expiry logic | 1 hour |
| Slash command registration + handler | 1 hour |
| TUI confirmation display | 0.5 hours |
| Tests | 2-3 hours |
| **Total** | **~8-10 hours** |

**But**: This depends on SCHED-003 (core scheduler engine) and SCHED-008 (slash command infrastructure) being done first. `/loop` is pure sugar on top of those.

### Risks

1. **Session-scoped vs persistent duality**: Adding a "non-persistent" schedule type means the scheduler needs to handle two lifecycles. This is manageable but adds complexity.
2. **Auto-expiry edge case**: If the user exits and restarts, session-scoped schedules are gone. This is the expected behavior (matches Claude Code) but might confuse users who expect persistence.
3. **Overlap with `/schedule add`**: Need clear documentation on when to use `/loop` vs `/schedule add`.
