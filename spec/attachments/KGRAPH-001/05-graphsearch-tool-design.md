# GraphSearch Tool Design

## Overview

`GraphSearch` is a new tool available to agents alongside `SessionSearch`, `DeepSearch`, and `AgentManager`. While SessionSearch finds raw text in conversations, GraphSearch queries the **relational structure** — finding connected concepts, decision chains, code lineage, and cross-session knowledge paths.

## Tool Interface

```typescript
interface GraphSearchArgs {
  action_type: 'search' | 'neighbors' | 'path' | 'related' | 'decisions' | 'history' | 'stats' | 'index';
  
  // For 'search' — semantic + text concept search
  query?: string;               // Natural language or concept name
  category?: string;            // Filter by concept category
  limit?: number;               // Max results (default: 20)
  
  // For 'neighbors' — graph neighborhood exploration
  node_id?: string;             // e.g. "concept:jwt-tokens" or "decision:use-redis-sessions"
  depth?: number;               // Traversal depth (default: 1, max: 3)
  edge_types?: string[];        // Filter by edge type
  
  // For 'path' — find shortest path between nodes
  from?: string;                // Source node ID
  to?: string;                  // Target node ID
  max_hops?: number;            // Max path length (default: 5)
  
  // For 'related' — find concepts related to a topic
  topic?: string;               // Topic to explore
  min_strength?: number;        // Minimum RelatesTo strength (0.0-1.0)
  
  // For 'decisions' — query decision history
  domain?: string;              // architecture, implementation, etc.
  status?: string;              // active, superseded, reversed
  since?: string;               // ISO timestamp
  
  // For 'history' — concept evolution over time
  concept?: string;             // Concept slug to trace
  
  // For 'index' — trigger indexing
  scope?: string;               // 'recent' | 'all' | 'session:<uuid>'
  since?: string;               // ISO timestamp for incremental indexing
}
```

## Example Interactions

### Search for concepts
```
GraphSearch(action_type='search', query='authentication patterns')
→ Returns: matching Concept nodes ranked by embedding similarity + text match
  [
    { slug: "jwt-authentication", name: "JWT Authentication", category: "pattern",
      summary: "Token-based auth using JSON Web Tokens...", mentionCount: 47,
      lastSeen: "2026-03-18T...", confidence: "high" },
    { slug: "session-management", name: "Session Management", category: "architecture",
      summary: "Redis-backed session store with 24h TTL...", mentionCount: 23, ... }
  ]
```

### Explore concept neighborhood
```
GraphSearch(action_type='neighbors', node_id='concept:jwt-authentication', depth=2)
→ Returns: subgraph around JWT authentication
  {
    center: { slug: "jwt-authentication", ... },
    neighbors: [
      { node: { slug: "session-management", ... }, edge: "relates_to", strength: 0.85 },
      { node: { slug: "redis-cache", ... }, edge: "depends_on", strength: 0.72 },
      { node: { slug: "use-jwt-over-cookies", type: "Decision", ... }, edge: "implements" }
    ]
  }
```

### Find path between concepts
```
GraphSearch(action_type='path', from='concept:authentication', to='concept:rate-limiting')
→ Returns: shortest path through the graph
  {
    hops: 3,
    path: [
      { node: "concept:authentication" },
      { edge: "relates_to", relation: "uses" },
      { node: "concept:api-gateway" },
      { edge: "relates_to", relation: "implements" },
      { node: "concept:rate-limiting" }
    ]
  }
```

### Query decisions
```
GraphSearch(action_type='decisions', domain='architecture', status='active')
→ Returns: active architecture decisions with provenance
  [
    { slug: "use-lance-storage", title: "Use Lance for graph storage",
      rationale: "Sub-ms opens, ACID, time-travel...", decidedAt: "2026-03-10T...",
      sourceSession: "a6bdbefb-...", sourceTurn: 42 }
  ]
```

### Trace concept history
```
GraphSearch(action_type='history', concept='authentication')
→ Returns: timeline of all sessions and turns mentioning authentication
  {
    concept: { slug: "authentication", firstSeen: "2026-01-15T...", mentionCount: 47 },
    timeline: [
      { session: "a6bdbefb-...", turnCount: 12, firstMention: "2026-01-15T...",
        decisions: ["use-jwt-over-cookies"] },
      { session: "b7cedfac-...", turnCount: 5, firstMention: "2026-02-20T...",
        decisions: ["add-oauth-support"] }
    ]
  }
```

### Get graph stats
```
GraphSearch(action_type='stats')
→ Returns: { concepts: 342, decisions: 67, codeEntities: 891,
             sessions: 156, turns: 4230, edges: 12847,
             lastIndexed: "2026-03-19T02:00:00Z" }
```

## Implementation Architecture

```
LLM invokes GraphSearch
    ↓
GraphSearchTool::call(args)           [fspec tools, implements rig::tool::Tool]
    ↓
execute_graph_search(session_id, action)  [handler registry]
    ↓
HashMap<Uuid, GraphSearchHandler>     [per-session isolation]
    ↓
Handler → NanographBridge             [Rust wrapper around nanograph-ts Database]
    ├── search  → hybrid query (embedding nearest + text fuzzy)
    ├── neighbors → bounded expansion query
    ├── path    → multi-hop traversal query
    ├── related → RelatesTo edge query with strength filter
    ├── decisions → Decision node scan with filters
    ├── history → Join Turn→Mentions→Concept with timeline
    ├── stats   → count aggregations per type
    └── index   → trigger indexing pipeline
```

## Nanograph Queries (compiled `.gq` files)

```
// Search concepts by embedding similarity
query search_concepts($query_embedding: Vector(1536), $limit: I32) {
    match {
        $c: Concept
        nearest($c.embedding, $query_embedding)
    }
    return { $c.slug, $c.name, $c.category, $c.summary, $c.mentionCount, $c.lastSeen }
    limit $limit
}

// Get concept neighborhood
query concept_neighbors($slug: String) {
    match {
        $c: Concept { slug: $slug }
        $c relatesTo $neighbor
    }
    return {
        $neighbor.slug, $neighbor.name, $neighbor.category,
        $c relatesTo $neighbor as rel
    }
}

// Find path (bounded expansion)
query find_path($from_slug: String, $to_slug: String) {
    match {
        $src: Concept { slug: $from_slug }
        $src relatesTo{1,5} $dst
        $dst: Concept { slug: $to_slug }
    }
    return { $src.slug, $dst.slug }
}

// Active decisions in a domain
query active_decisions($domain: String) {
    match {
        $d: Decision { domain: $domain, status: "active" }
    }
    return { $d.slug, $d.title, $d.rationale, $d.decidedAt }
    order { $d.decidedAt desc }
}
```
