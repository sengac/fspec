# KGRAPH Refactoring Decision — Dual-Graph Architecture

## Date: 2026-03-22

## Problem
The original KGRAPH implementation (KGRAPH-002 through KGRAPH-012) indexed ALL conversation history into a single nanograph database. After 727 turns, the database consumed **7.6GB** of disk space due to:

1. Per-line JSONL loading creating Lance version amplification
2. No Lance compaction/GC — versions accumulated indefinitely
3. Turn nodes as highest volume (every assistant message → Turn + edges)
4. Write amplification from MergeInsert on frequently-updated Concept nodes
5. Fundamental design flaw: trying to index raw conversation data instead of structured knowledge

## Decision
Replace the single monolithic graph with **two purpose-built graphs**:

### Graph 1: AST Connection Graph (KGRAPH-014)
- Maps full codebase structure via tree-sitter/ast-grep static analysis
- Zero LLM cost, rebuilt on demand
- File → Module → Function → Type → Import → Dependency
- Project-scoped, estimated <10MB
- Child cards: KGRAPH-016 (schema), KGRAPH-017 (extraction), KGRAPH-018 (dependencies), KGRAPH-019 (queries)

### Graph 2: Learnings Graph (KGRAPH-015)
- Captures accumulated knowledge using the **Residue methodology** (Aquino-Michaels 2026)
- Extracts at session boundaries, not per-turn
- Learning → Exploration → Convention → Decision → CodePattern
- Strategy Register: eliminated approaches, active constraints, reformulations
- Global scope, estimated <5MB for 1000 sessions
- Child cards: KGRAPH-020 (schema), KGRAPH-021 (extraction), KGRAPH-022 (cross-session), KGRAPH-023 (queries)

### Migration: KGRAPH-024
- Migrate useful Concept/Decision nodes from old graph
- Remove Turn/Session/Mentions infrastructure
- Clean up 7.6GB of old data

## Hierarchy
```
KGRAPH-001 (Architecture & Research)
  └── KGRAPH-013 (Refactor: Dual-Graph Architecture)
        ├── KGRAPH-014 (AST Connection Graph — parent)
        │     ├── KGRAPH-016 (AST Data Model & Schema)
        │     ├── KGRAPH-017 (AST Extraction Pipeline)
        │     ├── KGRAPH-018 (Dependency Graph Population)
        │     └── KGRAPH-019 (AST Query Interface)
        ├── KGRAPH-015 (Learnings Graph — parent)
        │     ├── KGRAPH-020 (Learnings Data Model & Schema)
        │     ├── KGRAPH-021 (Extraction Pipeline)
        │     ├── KGRAPH-022 (Cross-Session Learning & Synthesis)
        │     └── KGRAPH-023 (Learnings Query Interface)
        └── KGRAPH-024 (Deprecate Old Graph & Migrate)
```

## References
- Knuth, D. (2026). "Claude's Cycles" — https://cs.stanford.edu/~knuth/papers/claude-cycles.pdf
- Aquino-Michaels, K. (2026). "Completing Claude's Cycles" — https://github.com/no-way-labs/residue
- Morrison, K. (2026). KnuthClaudeLean — https://github.com/kim-em/KnuthClaudeLean/
- Residue Prompt — https://github.com/no-way-labs/residue/blob/main/prompt/residue.md
