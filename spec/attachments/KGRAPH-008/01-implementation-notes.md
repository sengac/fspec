# KGRAPH-008: Scheduled Indexing via Skills File — Notes

## Skills File Loading

The skills file lives at `spec/skills/graph-indexing.md`. It's a markdown
document with embedded JSON config blocks. The loader:

1. Reads the markdown file
2. Extracts fenced `json` code blocks
3. Parses and validates each block against the expected schema
4. Merges with defaults for any missing fields

## Schedule Registration

On fspec startup (or when the skills file is first detected), register
the indexing schedule:

```
Schedule(action='add',
  name='graph-index',
  cron=<from skills file>,
  timezone=<from skills file>,
  job_type='agent',
  role='You are a knowledge graph indexer...',
  prompt='Index unindexed session messages...',
  overlap_policy='skip'
)
```

The scheduled agent job:
1. Reads `~/.fspec/graph/index-state.json` for watermarks
2. Uses `SessionSearch(action='recent')` to find sessions
3. For each session with unindexed turns:
   a. `SessionSearch(action='show', session_id=X, start_turn=N)` to get new turns
   b. Run structural extractors
   c. Run LLM extraction (if enabled in skills)
   d. Merge results into graph
   e. Update watermark
4. Reports summary

## Incremental Indexing Flow

```
read index-state.json
  ↓
for each session in SessionSearch(recent):
  if session.updated_at > watermark[session.id].lastIndexedAt:
    fetch turns from watermark to latest
    extract entities
    upsert to graph
    update watermark
  ↓
write index-state.json (atomic)
```

## Skills File Validation

Required fields with defaults:

| Field | Default | Description |
|-------|---------|-------------|
| frequency | `*/15 * * * *` | Cron expression |
| timezone | `UTC` | IANA timezone |
| batchSize | `10` | Turns per LLM batch |
| maxConcurrent | `3` | Parallel LLM calls |
| extraction.mode | `hybrid` | `structural`, `llm`, or `hybrid` |
| extraction.llmExtraction.enabled | `true` | Enable LLM extraction |
| retention.turnNodes.maxAgeDays | `90` | Turn node retention |

## Scope Filtering

The skills file `scope` section controls what gets indexed:

- `projects: ["*"]` — all projects (default)
- `excludeSessions: [...]` — skip specific session UUIDs
- `minTurnLength: 50` — skip very short turns
- `skipSystemMessages: true` — don't index system prompts
