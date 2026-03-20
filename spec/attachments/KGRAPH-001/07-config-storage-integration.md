# Skills File, Configuration & Storage Strategy

## Skills File (`spec/skills/graph-indexing.md`)

Operators configure the knowledge graph via a skills file in `spec/skills/`. Skills files are markdown documents — any structured configuration is embedded in fenced code blocks:

````markdown
# Graph Indexing Skill

## Purpose
Periodically index agent session history into the knowledge graph,
extracting concepts, decisions, and relationships.

## Schedule
- **Frequency:** Every 15 minutes (`*/15 * * * *`)
- **Timezone:** UTC

## Indexing Configuration

```json
{
  "batchSize": 10,
  "maxConcurrent": 3,
  "scope": {
    "projects": ["*"],
    "excludeProjects": [],
    "excludeSessions": [],
    "minTurnLength": 50,
    "skipSystemMessages": true,
    "indexAssistantMessages": true,
    "indexUserMessages": true,
    "indexToolCalls": true
  }
}
```

## Extraction

**Mode:** hybrid (structural + LLM)

### Structural Extractors (zero-cost)
- `fspec-tool-calls` — detect work unit changes
- `file-modifications` — detect Write/Edit tool calls
- `error-resolutions` — detect failure→success patterns

### LLM Extraction
- **Model:** claude-sonnet-4-20250514
- **Max tokens per batch:** 4096
- **Categories:** architecture, convention, decision, dependency,
  domain_term, error_class, feature, library, pattern, platform,
  process, technology, tool

## Embeddings
- **Model:** text-embedding-3-small
- **Dimensions:** 1536
- **Batch size:** 100

## Retention
- **Turn nodes:** Prune after 90 days (keep turns linked to decisions)
- **Compaction:** Weekly (Sunday 3am), merge threshold 0.95,
  minimum 2 mentions to keep

## GraphSearch Defaults
- **Default limit:** 20 results
- **Max traversal depth:** 3
- **Max path length:** 5
- **Semantic search:** enabled
- **Fuzzy search:** enabled
````

## Storage Strategy

### Location: Project-Local

```
{project_root}/
├── spec/
│   ├── skills/
│   │   └── graph-indexing.md    # Skills file (markdown)
│   └── ...

## Storage Strategy

### Location: Global `~/.fspec/`

The graph lives in the global fspec data directory, alongside sessions and messages:

```
~/.fspec/
├── messages/               # Content-addressed message store
├── sessions/               # Session manifests
├── blobs/                  # Large content blobs
├── history.jsonl           # Command history
└── graph/
    └── agent-memory.nano/  # Nanograph database
        ├── schema.pg
        ├── schema.ir.json
        ├── graph.manifest.json
        ├── _tx_catalog.jsonl
        ├── _cdc_log.jsonl
        ├── nodes/
        └── edges/
```

**Why global (not project-local)?**
- The knowledge graph indexes ALL session history across ALL projects
- Sessions and messages already live in `~/.fspec/` — the graph is a derived index over them
- A single graph enables cross-project concept discovery
- No gitignore concerns — `~/.fspec/` is already user-local state

## Initialization Flow

```
1. First GraphSearch call OR `fspec graph init`
   ↓
2. Check if ~/.fspec/graph/agent-memory.nano/ exists
   ↓
3. If not: Database.init(path, schemaSource)
   - Write agent-memory.pg schema
   - Compile to SchemaIR
   - Create empty datasets
   ↓
4. If exists: Database.open(path)
   - Load existing graph
   - Check schema version, run migration if needed
```

## Schema Migration Path

Nanograph has built-in migration support:

```
1. Edit schema.pg (add new node/edge types, properties)
2. nanograph migrate → detects safe/confirm/blocked changes
3. Apply safe changes automatically
4. @rename_from("old_name") for property renames
```

This means the agent-memory schema can evolve as we discover new node/edge types to track.

## Integration Points with Existing fspec Architecture

### 1. Message Persistence Hook
When `persist_assistant_message()` or `persist_user_message()` fires in the Rust NAPI layer, optionally trigger lightweight structural extraction immediately (no LLM, just pattern matching for file changes, fspec calls, etc.).

### 2. SessionSearch Bridge
GraphSearch uses SessionSearch internally to fetch turn content for LLM extraction. Same handler registry pattern.

### 3. Compaction DAG Awareness
When compaction creates D0/D1/D2 summaries, the indexer can also create/update Concept nodes from the summary content — these are pre-distilled knowledge.

### 4. Scheduler Integration
Uses the existing `Schedule` engine (30s tick loop, cron evaluation, overlap policies) — no new scheduling infrastructure needed.

### 5. AgentManager Integration
The graph indexing agent job runs as a scheduled subordinate session. It has full access to GraphSearch and SessionSearch tools.

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Storage engine | Nanograph (Lance) | Sub-ms opens, ACID, schema enforcement, vector search |
| Location | Global `~/.fspec/graph/` | Co-located with sessions/messages, cross-project |
| Indexing trigger | Cron (Schedule tool) | Decoupled, non-blocking, configurable |
| Extraction | Hybrid (structural + LLM) | Structural is free/instant; LLM catches nuance |
| Schema | Typed `.pg` | Compile-time type checking, migration support |
| Node.js binding | nanograph-ts (napi-rs) | Already exists, async-safe, Arrow IPC |
| Merge strategy | `@key` upsert | Idempotent re-indexing, safe for retries |
| Embeddings | OpenAI text-embedding-3-small | Configurable, cached, good quality/cost ratio |
