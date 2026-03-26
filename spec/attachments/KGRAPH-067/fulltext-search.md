# KGRAPH-067: Full-Text and Content Search Within Graph

## Dependency

**Requires KGRAPH-063** (Source Code and Metadata Storage) — full-text search needs `source` and `docstring` properties on nodes.

## Problem

Our `ast_search` does client-side case-insensitive substring matching on `name`, `slug`, `path`, `qualifiedName`. It can't search inside function bodies or documentation. An agent searching for "authentication" won't find a function called `process_user_login` even though its body contains authentication logic.

## CGC Reference Implementation

### Full-text index creation — `graph_builder.py` lines 186–191

```python
# Neo4j: Create fulltext index across Function, Class, Variable nodes
session.run("""
    CREATE FULLTEXT INDEX code_search_index IF NOT EXISTS
    FOR (n:Function|Class|Variable)
    ON EACH [n.name, n.source, n.docstring]
""")
```

### FalkorDB fallback — `graph_builder.py` lines 179–185

```python
# FalkorDB: per-label fulltext index
for label in ['Function', 'Class']:
    session.run(f"CALL db.idx.fulltext.createNodeIndex('{label}', 'name', 'source', 'docstring')")
```

### Neo4j fulltext search — `code_finder.py` lines 100–122

```python
def find_by_content(self, search_term, repo_path=None):
    """Find code by content matching in source or docstrings."""
    result = session.run(f"""
        CALL db.index.fulltext.queryNodes("code_search_index", $search_term) 
        YIELD node, score
        WITH node, score
        WHERE (node:Function OR node:Class OR node:Variable)
        MATCH (node)<-[:CONTAINS]-(f:File)
        RETURN
            CASE WHEN node:Function THEN 'function'
                 WHEN node:Class THEN 'class'
                 ELSE 'variable' END as type,
            node.name as name, f.path as path,
            node.line_number as line_number, node.source as source,
            node.docstring as docstring
        ORDER BY score DESC
        LIMIT 20
    """, search_term=search_term)
```

### FalkorDB content search fallback — `code_finder.py` lines 124–150

```python
def _find_by_content_falkordb(self, search_term, repo_path=None):
    """FalkorDB-compatible: pure Cypher CONTAINS matching."""
    for label, type_name in [('Function', 'function'), ('Class', 'class')]:
        result = session.run(f"""
            MATCH (node:{label})
            WHERE (toLower(node.name) CONTAINS toLower($search_term)
                OR (node.source IS NOT NULL AND toLower(node.source) CONTAINS toLower($search_term))
                OR (node.docstring IS NOT NULL AND toLower(node.docstring) CONTAINS toLower($search_term)))
            RETURN '{type_name}' as type, node.name as name, node.path as path, ...
            ORDER BY node.is_dependency ASC, node.name
            LIMIT 20
        """, search_term=search_term)
```

### Fuzzy search with Lucene — `code_finder.py` lines 181–191

```python
def find_related_code(self, user_query, fuzzy_search, edit_distance, repo_path=None):
    if fuzzy_search:
        # Lucene fuzzy syntax: "term~2" means edit distance of 2
        user_query_normalized = " ".join(
            map(lambda x: f"{x}~{edit_distance}", user_query.split(" "))
        )
```

### Ranked multi-strategy search — `code_finder.py` lines 193–229

CGC combines multiple search strategies and ranks results:

```python
results = {
    "functions_by_name": self.find_by_function_name(...),     # score: 0.9
    "classes_by_name": self.find_by_class_name(...),          # score: 0.8
    "variables_by_name": self.find_by_variable_name(...),     # score: 0.7
    "content_matches": self.find_by_content(...),             # score: 0.6
}
# Dependencies get lower scores (0.7, 0.6, 0.5, 0.4)
all_results.sort(key=lambda x: x["relevance_score"], reverse=True)
results["ranked_results"] = all_results[:15]
```

## What We Need to Implement

### Option A: nanograph-native text search

If nanograph supports text search or LIKE/CONTAINS operators in its query language, use them directly:

```
match {
  $fn: Function
  where contains($fn.source, $search_term)
     or contains($fn.docstring, $search_term)
     or contains($fn.name, $search_term)
}
return $fn
order by $fn.name
limit 20
```

### Option B: Client-side search over enriched properties

Extend our existing client-side search in `dispatch_helpers.rs` to also match against `source` and `docstring` fields (once KGRAPH-063 stores them):

```rust
const SEARCHABLE_FIELDS: &[&str] = &[
    "name", "slug", "path", "qualifiedName",
    "source",    // NEW
    "docstring", // NEW
];
```

### Option C: External search index

Build a separate search index (tantivy, SQLite FTS5) alongside nanograph for full-text queries. This gives us proper tokenization, stemming, and ranking — but adds complexity.

### Recommendation

Start with **Option B** (client-side search with enriched fields). It's the simplest path:
1. KGRAPH-063 adds `source` + `docstring` to nodes
2. We expand `SEARCHABLE_FIELDS` in `dispatch_helpers.rs`
3. Agent searches "authentication" → matches function bodies containing that word

If performance becomes an issue (scanning all nodes), upgrade to Option A or C.

### Enhanced `ast_search` action

```rust
AstSearch {
    query: String,
    entity_type: Option<EntityType>,
    limit: Option<u32>,
    path: Option<String>,
    search_mode: Option<SearchMode>, // NEW: "name" (default), "content", "all"
}
```

- `name` — current behavior, search name/slug/path only
- `content` — search source code and docstrings
- `all` — search everything (name + content)

### Files to modify

| File | Change |
|------|--------|
| `codelet/napi/src/graph/dispatch_helpers.rs` | Add source/docstring to searchable fields |
| `codelet/tools/src/graph_search/types.rs` | Add `search_mode` parameter |
| `codelet/napi/src/graph_search_handler.rs` | Adjust search logic based on mode |

### Effort estimate

**Medium** — Depends on KGRAPH-063 being done first. Once source is stored, extending the client-side search is straightforward. Fuzzy/ranked search is a nice-to-have follow-up.
