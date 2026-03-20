# KGRAPH-007: GraphSearch Query Implementations — Notes

## Compiled Queries

Each GraphSearch action maps to one or more pre-written `.gq` queries.
These are bundled into the binary via `include_str!()` just like the schema.

### search — Concept text search

```
query search_concepts($term: String, $limit: I32) {
    match {
        $c: Concept
        fuzzy($c.name, $term, 2)
    }
    return { $c.slug, $c.name, $c.category, $c.summary,
             $c.mentionCount, $c.lastSeen, $c.confidence }
    order { $c.mentionCount desc }
    limit $limit
}
```

When embeddings are available (KGRAPH-010), add a hybrid query using
`rrf(nearest($c.embedding, $query_vec), bm25($c.summary, $term))`.

### neighbors — Bounded expansion

```
query concept_neighbors($slug: String, $depth: I32) {
    match {
        $c: Concept { slug: $slug }
        $c relatesTo{1,$depth} $neighbor
    }
    return {
        $neighbor.slug, $neighbor.name, $neighbor.category
    }
}
```

Note: nanograph bounded expansion `{1,$depth}` requires a literal, not a
parameter. We may need to generate the query string dynamically for
variable depth, or have 3 pre-compiled variants (depth 1, 2, 3).

### path — Shortest path

```
query find_path($from: String, $to: String) {
    match {
        $src: Concept { slug: $from }
        $src relatesTo{1,5} $dst
        $dst: Concept { slug: $to }
    }
    return { $src.slug, $dst.slug }
}
```

Nanograph bounded expansion finds ALL paths up to length 5. We post-filter
in Rust to find the shortest. If no path exists, return empty.

### related — Direct RelatesTo edges

```
query related_concepts($slug: String) {
    match {
        $c: Concept { slug: $slug }
        $c relatesTo $other
    }
    return {
        $other.slug, $other.name, $other.category,
        $c relatesTo $other as rel
    }
}
```

Filter by `min_strength` in Rust post-processing (or via filter clause
on the edge property if nanograph supports it).

### decisions — Decision node scan

```
query list_decisions($domain: String, $status: String) {
    match {
        $d: Decision { domain: $domain, status: $status }
    }
    return { $d.slug, $d.title, $d.rationale, $d.decidedAt, $d.domain }
    order { $d.decidedAt desc }
}
```

### history — Concept timeline

```
query concept_history($slug: String) {
    match {
        $s: Session
        $s discusses $c
        $c: Concept { slug: $slug }
    }
    return {
        $s.slug, $s.projectPath,
        $s discusses $c as d
    }
    order { d.firstMention asc }
}
```

### stats — Count aggregations

```
query graph_stats() {
    match { $c: Concept }
    return { count($c) as concepts }
}
// + separate queries per type, or a single multi-return query
```

May need multiple queries (one per node type) since nanograph doesn't
support cross-type counting in a single query.

## Output Formatting

All results are returned as JSON strings for the LLM. Keep responses
concise — truncate long lists, include pagination hints.
