# Indexing Pipeline & Cron Architecture

## Overview

The indexing pipeline keeps the knowledge graph updated by periodically scanning new session messages, extracting concepts/relations/entities, and inserting them into nanograph. It uses fspec's existing `Schedule` tool for cron automation and `SessionSearch` for message retrieval.

## Watermark-Based Incremental Indexing

The graph tracks a **watermark** per session — the timestamp of the last indexed turn. Each indexing run only processes turns newer than the watermark.

```
Session nodes store: lastIndexedAt, indexedTurnCount
→ Only turns after lastIndexedAt are fetched
→ After indexing, lastIndexedAt is updated
```

### Global Watermark File

```json
// {data_dir}/graph/index-state.json
{
  "lastRunAt": "2026-03-19T02:00:00Z",
  "sessionsIndexed": {
    "a6bdbefb-902d-4f98-b539-8cbee91ec831": {
      "lastIndexedTurn": 42,
      "lastIndexedAt": "2026-03-19T01:58:00Z"
    }
  },
  "stats": {
    "totalRuns": 156,
    "totalTurnsIndexed": 4230,
    "lastConceptsAdded": 3,
    "lastEdgesAdded": 12
  }
}
```

## Extraction Pipeline (Per Turn)

```
SessionSearch(action='show', session_id=uuid, start_turn=N, end_turn=M)
    ↓
For each unindexed turn:
    ↓
1. STRUCTURAL EXTRACTION (zero-cost, deterministic)
   - Detect fspec tool calls → WorkUnit nodes, status changes
   - Detect Write/Edit calls → CodeEntity nodes + Modifies edges
   - Detect error→success patterns → Decision nodes
    ↓
2. LLM-BASED EXTRACTION (batched, async)
   - Send turn content to extraction prompt
   - Extract: concepts, decisions, relations, entities
   - Returns structured JSON
    ↓
3. MERGE INTO GRAPH
   - Upsert Concept nodes (merge on slug, update mentionCount/lastSeen)
   - Upsert Decision nodes
   - Upsert CodeEntity nodes
   - Insert Turn + Session nodes (if new)
   - Insert edges (Mentions, Decides, Modifies, RelatesTo)
   - Update co-occurrence counts on RelatesTo edges
    ↓
4. UPDATE WATERMARK
   - Set Session.lastIndexedAt = max turn timestamp
   - Update index-state.json
```

## LLM Extraction Prompt

```
You are a knowledge graph extractor. Given an agent conversation turn, extract:

1. **Concepts**: Named ideas, technologies, patterns, domain terms
   - slug (kebab-case), name, category, summary (one sentence)

2. **Decisions**: Explicit conclusions or choices made
   - slug, title, rationale, domain

3. **Relations**: How concepts relate to each other
   - from_concept, to_concept, relation_type, strength (0.0-1.0)

4. **Code entities**: Files, functions, modules mentioned or modified
   - slug (path::name), entityType, filePath, operation (created/modified/deleted/reviewed)

Return JSON:
{
  "concepts": [{ "slug": "...", "name": "...", "category": "...", "summary": "..." }],
  "decisions": [{ "slug": "...", "title": "...", "rationale": "...", "domain": "..." }],
  "relations": [{ "from": "...", "to": "...", "type": "...", "strength": 0.8 }],
  "code_entities": [{ "slug": "...", "entityType": "...", "filePath": "...", "operation": "..." }]
}

Only extract what is EXPLICITLY present. Set confidence=high for named items, medium for inferred.
Do NOT hallucinate concepts that aren't discussed.
```

## Batching Strategy

- **Batch size**: 5-10 turns per LLM call (to amortize overhead)
- **Max concurrent**: 3 extraction calls (avoid rate limits)
- **Deduplication**: Merge concepts by slug before graph insert
- **Idempotency**: Use `@key` merge mode — re-indexing same turn is safe

## Cron Job Configuration

Using fspec's `Schedule` tool:

```
Schedule(action='add',
  name='graph-index',
  cron='*/15 * * * *',          // Every 15 minutes
  timezone='UTC',
  job_type='agent',
  role='You are a knowledge graph indexer. Use GraphSearch(action_type="index", scope="recent") to index new session messages into the knowledge graph. Report what was indexed.',
  prompt='Index all unindexed session messages into the knowledge graph. Use GraphSearch with action_type="index" and scope="recent".'
)
```

### Alternative: Shell Job (lighter weight)

```
Schedule(action='add',
  name='graph-index',
  cron='*/15 * * * *',
  timezone='UTC',
  job_type='shell',
  command='fspec graph-index --scope=recent --quiet'
)
```

## DeepSearch Integration

### `--update-graph` Flag

When DeepSearch runs with `--update-graph`, it triggers graph indexing as a side effect:

```typescript
interface DeepSearchArgs {
  query: string;
  scope?: string[];
  max_depth?: number;
  update_graph?: boolean;  // NEW: trigger graph indexing on explored sessions
}
```

**Behavior**: After the sub-agent completes its research, if `update_graph=true`, the explored sessions are queued for graph indexing. This piggybacks on the research work the agent already did.

### Graph-Enhanced DeepSearch

DeepSearch can USE the graph to improve search quality:

1. Before spawning the sub-agent, query the graph for related concepts
2. Include graph context in the sub-agent's system prompt
3. The sub-agent can use GraphSearch as one of its tools

```
DeepSearch sub-agent tools (when graph enabled):
  Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch, GraphSearch
```

This gives the sub-agent relational context: "JWT authentication is related to session management which was decided in session X turn 42."

## Graph Compaction & Pruning

### Automatic Compaction
- Run `db.compact()` weekly (Lance storage optimization)
- Merge low-confidence concepts below threshold after 30 days
- Prune Turn nodes older than 90 days (keep Session + Concept links)

### Manual Compaction
```
GraphSearch(action_type='compact', strategy='prune_old_turns', older_than='90d')
GraphSearch(action_type='compact', strategy='merge_similar', min_similarity=0.95)
```
