# Proposed Schema: Agent Memory Graph

## Schema Definition (`agent-memory.pg`)

```
// ═══════════════════════════════════════════════════════════════
// CODELET AGENT MEMORY GRAPH — NanoGraph Schema
// Captures concepts, decisions, and relationships across all
// agent session history for relational memory retrieval.
// ═══════════════════════════════════════════════════════════════

// ── Core Knowledge Nodes ──────────────────────────────────────

node Concept
  @description("A named idea, technology, pattern, or domain term extracted from conversations.")
{
    slug: String @key
      @description("Stable identifier: lowercase-kebab, e.g. 'jwt-authentication'")
    name: String @unique
      @description("Human-readable concept name, e.g. 'JWT Authentication'")
    category: enum(
      architecture, convention, decision, dependency,
      domain_term, error_class, feature, library,
      pattern, person, platform, process, technology, tool
    )
    summary: String
      @description("One-paragraph description synthesized from all mentions.")
    embedding: Vector(1536) @embed(summary) @index
      @description("Semantic embedding for nearest-neighbor concept search.")
    mentionCount: I32
      @description("Total number of session turns that reference this concept.")
    firstSeen: DateTime
    lastSeen: DateTime
    confidence: enum(high, medium, low)
      @description("Extraction confidence: high=explicitly named, medium=inferred, low=speculative.")
    tags: [String]?
      @description("Freeform tags for filtering, e.g. ['backend', 'security']")
}

node Decision
  @description("An explicit decision or conclusion reached during a session.")
{
    slug: String @key
    title: String @unique
    rationale: String
      @description("Why this decision was made — the reasoning chain.")
    embedding: Vector(1536) @embed(rationale) @index
    status: enum(active, proposed, reversed, superseded)
    domain: enum(
      architecture, convention, dependency, deployment,
      design, implementation, process, testing
    )
    decidedAt: DateTime
    createdAt: DateTime
}

node CodeEntity
  @description("A file, function, class, module, or other addressable code artifact.")
{
    slug: String @key
      @description("Normalized path or qualified name, e.g. 'src/auth/login.ts::validateCredentials'")
    name: String
    entityType: enum(class, endpoint, file, function, module, package, schema, test)
    filePath: String?
    language: String?
    lastModified: DateTime?
    createdAt: DateTime
}

node WorkUnit
  @description("Mirror of an fspec work unit for cross-referencing graph knowledge with project state.")
{
    slug: String @key
      @description("Matches the fspec work unit ID exactly, e.g. 'AUTH-001'.")
    title: String
    status: enum(backlog, blocked, done, implementing, specifying, testing, validating)
    workType: enum(bug, story, task)
    createdAt: DateTime
    updatedAt: DateTime
}

// ── Session & Provenance Nodes ──────────────────────────────────

node Session
  @description("An agent conversation session. Links knowledge back to source context.")
{
    slug: String @key
      @description("The session UUID from fspec.")
    projectPath: String?
    startedAt: DateTime
    lastIndexedAt: DateTime
      @description("Timestamp of the last turn that was indexed from this session.")
    turnCount: I32
    indexedTurnCount: I32
}

node Turn
  @description("A single turn (message) in a session. Finest-grained provenance anchor.")
{
    slug: String @key
      @description("Format: '{session_uuid}:{turn_index}'")
    sessionSlug: String
    turnIndex: I32
    role: enum(assistant, user)
    timestamp: DateTime
    preview: String?
      @description("First 200 chars of turn content for context.")
}

// ── Relationship Edges ──────────────────────────────────────────

edge Mentions: Turn -> Concept {
    confidence: enum(high, medium, low)
    extractedAt: DateTime
}

edge Discusses: Session -> Concept {
    turnCount: I32
      @description("How many turns in this session mention this concept.")
    firstMention: DateTime
    lastMention: DateTime
}

edge Decides: Turn -> Decision {
    extractedAt: DateTime
}

edge Implements: CodeEntity -> Decision {
    linkedAt: DateTime
}

edge Modifies: Turn -> CodeEntity {
    operation: enum(created, modified, deleted, reviewed)
    extractedAt: DateTime
}

edge RelatesTo: Concept -> Concept {
    strength: F32
      @description("Relationship strength: 0.0-1.0, based on co-occurrence frequency.")
    relationType: enum(
      causes, composes, conflicts_with, depends_on,
      extends, implements, similar_to, supersedes, uses
    )
    firstSeen: DateTime
    lastSeen: DateTime
    coOccurrenceCount: I32
}

edge Supersedes: Decision -> Decision {
    supersededAt: DateTime
    reason: String?
}

edge WorksOn: Session -> WorkUnit {
    linkedAt: DateTime
}

edge References: Decision -> WorkUnit {
    linkedAt: DateTime
}

edge ContainsTurn: Session -> Turn {
    turnIndex: I32
}
```

## Node Type Summary

| Node | Purpose | Key Field | 
|------|---------|-----------|
| `Concept` | Named ideas, technologies, patterns | `slug` (kebab-case) |
| `Decision` | Explicit decisions reached in conversations | `slug` |
| `CodeEntity` | Files, functions, modules discussed/modified | `slug` (path::name) |
| `WorkUnit` | Mirror of fspec work units | `slug` (e.g. AUTH-001) |
| `Session` | Agent conversation sessions | `slug` (UUID) |
| `Turn` | Individual messages in sessions | `slug` (uuid:index) |

## Edge Type Summary

| Edge | From → To | Purpose |
|------|-----------|---------|
| `Mentions` | Turn → Concept | Fine-grained concept extraction |
| `Discusses` | Session → Concept | Aggregated session-concept link |
| `Decides` | Turn → Decision | Where a decision was made |
| `Implements` | CodeEntity → Decision | Code that implements a decision |
| `Modifies` | Turn → CodeEntity | Code changes tracked per turn |
| `RelatesTo` | Concept → Concept | Concept-to-concept relationships |
| `Supersedes` | Decision → Decision | Decision revision chain |
| `WorksOn` | Session → WorkUnit | Session-to-work-unit link |
| `References` | Decision → WorkUnit | Decision-to-work-unit link |
| `ContainsTurn` | Session → Turn | Session containment |
