# KGRAPH-006: Graph Merge & Upsert — Implementation Notes

## JSONL Generation

Nanograph ingests data via JSONL format. The merge logic converts
extracted entities into JSONL entries:

### Node entries
```jsonl
{"type":"Concept","data":{"slug":"jwt-auth","name":"JWT Authentication","category":"pattern","summary":"Token-based auth...","mentionCount":1,"firstSeen":"2026-03-19T...","lastSeen":"2026-03-19T...","confidence":"high"}}
{"type":"Session","data":{"slug":"a6bdbefb-...","projectPath":"/Users/...","startedAt":"2026-03-19T...","lastIndexedAt":"2026-03-19T...","turnCount":42,"indexedTurnCount":42}}
```

### Edge entries
```jsonl
{"edge":"Mentions","from":"a6bdbefb:15","to":"jwt-auth","data":{"confidence":"high","extractedAt":"2026-03-19T..."}}
{"edge":"RelatesTo","from":"jwt-auth","to":"session-management","data":{"strength":0.85,"relationType":"uses","firstSeen":"2026-03-19T...","lastSeen":"2026-03-19T...","coOccurrenceCount":1}}
```

## Merge Mode

All loads use nanograph's **merge** mode with `@key` property:
- If node with same `slug` exists → update properties
- If node doesn't exist → insert
- This makes re-indexing idempotent

### Merge semantics for specific fields

| Field | Merge behavior |
|-------|---------------|
| `mentionCount` | Increment (not overwrite) |
| `firstSeen` | Keep earliest |
| `lastSeen` | Keep latest |
| `summary` | Overwrite with latest extraction |
| `confidence` | Promote (low→medium→high, never demote) |
| `coOccurrenceCount` | Increment |
| `strength` | Weighted average with new extraction |

**Note:** Nanograph's built-in merge only does full-property overwrite.
For increment/min/max semantics, we need to **read-before-write**:
1. Query existing node by slug
2. Compute merged values in Rust
3. Write the merged node via update mutation

## Watermark Tracking

```json
// ~/.fspec/graph/index-state.json
{
  "lastRunAt": "2026-03-19T02:00:00Z",
  "schemaVersion": "1",
  "sessions": {
    "a6bdbefb-902d-4f98-b539-8cbee91ec831": {
      "lastIndexedTurn": 42,
      "lastIndexedAt": "2026-03-19T01:58:00Z"
    }
  }
}
```

Written atomically (temp file + rename) after each successful batch.

## Co-occurrence Tracking

RelatesTo edges track how often two concepts appear together:

```
For each turn:
  concepts_in_turn = [extracted concepts]
  for each pair (c1, c2) in concepts_in_turn:
    upsert RelatesTo edge:
      coOccurrenceCount += 1
      strength = recalculate based on co-occurrence frequency
```

Strength formula:
```
strength = min(1.0, log2(coOccurrenceCount + 1) / 10.0)
```
This gives: 1 co-occurrence → 0.1, 10 → 0.35, 100 → 0.67, 1000 → 1.0
