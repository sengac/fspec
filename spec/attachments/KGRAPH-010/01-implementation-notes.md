# KGRAPH-010: Graph Compaction & Schema Migration — Notes

## Turn Node Pruning

Turn nodes are the most numerous and least valuable over time. Prune them:

**Rules:**
- Delete Turn nodes older than `retention.turnNodes.maxAgeDays` (default: 90)
- KEEP Turn nodes that have `Decides` edges (they're decision provenance)
- Cascade: deleting a Turn also deletes its `Mentions`, `Modifies` edges
- Update parent Session node's `indexedTurnCount` after pruning

**Query:**
```
mutation prune_old_turns($cutoff: DateTime) {
    delete $t: Turn
    not { $t decides $_ }
    $t.timestamp < $cutoff
}
```

## Similar Concept Merging

When two Concept nodes have very high textual similarity, merge them:

**Rules:**
- Threshold: configurable `retention.compaction.mergeThreshold` (default: 0.95)
- Keep the concept with higher `mentionCount` as the survivor
- Sum `mentionCount` values
- Keep earliest `firstSeen`, latest `lastSeen`
- Re-point all edges from the merged concept to the survivor
- Delete the merged concept

**Implementation:**
1. Query all Concept pairs with name similarity > threshold
   (use `fuzzy()` with max_edits=1, or compare slugs)
2. For each pair, pick survivor (higher mentionCount)
3. Update survivor properties
4. Re-point edges (requires reading edges, deleting, reinserting)
5. Delete the loser

## Lance Storage Compaction

Nanograph's underlying Lance storage benefits from periodic compaction:

```rust
db.compact(CompactOptions {
    target_rows_per_fragment: Some(1_000_000),
    ..Default::default()
})?;

db.cleanup(CleanupOptions {
    older_than: Some(chrono::Duration::days(7)),
    ..Default::default()
})?;
```

Schedule: weekly via the same skills file cron system.

## Schema Migration

When the bundled `agent-memory.pg` schema evolves between fspec versions:

1. On database open, compare `schema.ir.json` hash with bundled schema hash
2. If different, run `nanograph migrate`
3. Safe changes (new optional properties, new types) auto-apply
4. Breaking changes (removed types, non-nullable additions) blocked with error

**Migration tracking:**
```json
// ~/.fspec/graph/index-state.json
{
  "schemaVersion": "2",
  "lastMigration": "2026-04-01T...",
  "migrationHistory": [
    { "from": "1", "to": "2", "at": "2026-04-01T...", "changes": ["added Vector embedding to Concept"] }
  ]
}
```

## Embedding Addition (Schema v2)

When we add `@embed` support (deferred from v1):

```
// Schema v2 diff:
node Concept {
    // ... existing fields ...
+   embedding: Vector(1536) @embed(summary) @index
}

node Decision {
    // ... existing fields ...
+   embedding: Vector(1536) @embed(rationale) @index
}
```

This is a safe migration (new nullable properties with defaults).
After migration, run `db.embed()` to backfill vectors for existing nodes.
