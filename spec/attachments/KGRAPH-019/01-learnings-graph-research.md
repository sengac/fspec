# Learnings Graph — Residue-Methodology Research

## Inspiration: The Residue Methodology

This graph design is inspired by two related works:

1. **Don Knuth's "Claude's Cycles"** (February 2026) — Documents how Claude Opus 4.6 solved an open combinatorics problem through 31 numbered explorations, with explicit documentation of progress at each step.

2. **Keston Aquino-Michaels' "Completing Claude's Cycles"** (March 2026) — Introduces the **Residue methodology** for structured multi-agent exploration, demonstrating how two AI agents with complementary capabilities (one theoretical, one computational) can solve problems neither could solve alone, mediated by an orchestrator that transfers data and tools between them at critical moments.

The key insight from the Residue methodology is that **the solution may be in the residue of previous failures**. A well-structured record of what was tried, what failed, why it failed, what survived, and what reformulations emerged is more valuable than raw conversation history.

## Core Concepts: The Strategy Register Pattern

The Residue prompt defines a **Strategy Register** — a running summary maintained at the top of an exploration log with three lists:

### 1. Eliminated Approach Classes
Not specific failed attempts, but **categories of approaches** that have been ruled out, with the structural reason. Example: "Approaches requiring cyclic symmetry — ruled out because the problem lacks Z_n invariance for even n."

**Graph model**: `EliminatedApproach` nodes with `className`, `reason`, `eliminatedAt`, linked to the `Exploration` that discovered the elimination.

### 2. Active Structural Constraints
Facts **discovered about the problem** through attempts, whether or not those attempts succeeded. These are persistent truths that constrain all future approaches. Example: "The map must be injective on the fiber over s=0."

**Graph model**: `Constraint` nodes with `description`, `discoveredAt`, `confirmedAt`, linked to relevant `Exploration` nodes.

### 3. Known Reformulations
Alternative representations of the problem discovered during exploration. These are reusable even when the strategy that produced them failed. Example: "Fiber coordinates (i,j) with k = (s-i-j) mod m — makes quotient map visible."

**Graph model**: `Reformulation` nodes with `description`, `discoveredAt`, linked to `Concept` nodes for the domain.

## Graph Schema

### Node Types

#### 1. `Learning` (primary)
- **Key**: slug (e.g., `use-batch-loading-for-lance`)
- **Properties**:
  - `title`: Short description
  - `category`: convention | pattern | anti-pattern | decision | discovery | constraint | reformulation
  - `summary`: Detailed description
  - `confidence`: high | medium | low
  - `domain`: architecture | testing | performance | ux | security | tooling | process | domain-model
  - `firstSeen`, `lastSeen`, `mentionCount`
  - `status`: active | superseded | questioned

#### 2. `Exploration`
- **Key**: slug (e.g., `explore-042-batch-lance-loading`)
- **Properties**:
  - `number`: Sequential exploration number
  - `strategy`: One-sentence description of what was tried and why
  - `outcome`: succeeded | failed | abandoned | partial
  - `failureConstraint`: If failed — the specific structural reason (not "it didn't work")
  - `whatThisRulesOut`: What class of approaches this eliminates
  - `survivingStructure`: Partial results that survived the failure
  - `reformulations`: New ways to see the problem discovered during the attempt
  - `sessionId`: Which session this exploration occurred in
  - `timestamp`

#### 3. `Convention`
- **Key**: slug (e.g., `never-use-any-type`)
- **Properties**:
  - `rule`: The convention statement
  - `rationale`: Why this convention exists
  - `scope`: project | file | module | global
  - `enforcementLevel`: strict | recommended | advisory
  - `source`: manual | discovered | extracted

#### 4. `Decision`
- **Key**: slug (same as current KGRAPH)
- **Properties**: `title`, `rationale`, `status`, `domain`, `decidedAt`, `alternatives[]`

#### 5. `CodePattern`
- **Key**: slug (e.g., `command-with-hooks-pattern`)
- **Properties**:
  - `patternName`: Human-readable name
  - `description`: What the pattern does
  - `exampleFiles[]`: Files that exemplify this pattern
  - `applicableContexts[]`: Where to use this pattern

### Edge Types

#### 1. `Discovered` (Exploration → Learning)
An exploration that discovered or confirmed a learning.

#### 2. `Eliminates` (Exploration → Learning)
An exploration that eliminated an approach class.

#### 3. `Supersedes` (Learning → Learning)
A newer learning that replaces an older one.

#### 4. `RelatesTo` (Learning ↔ Learning)
Two learnings that are related, with `strength` and `relationType`.

#### 5. `InformedBy` (Exploration → Exploration)
An exploration that was informed by a previous one's surviving structure.

#### 6. `Applies` (Convention → CodePattern)
A convention that applies to a specific code pattern.

#### 7. `Contradicts` (Learning ↔ Learning)
Two learnings that are in tension — useful for surfacing unresolved design conflicts.

## Extraction Strategy

### When to Extract (Session Boundaries, Not Per-Turn)

The critical mistake of the old system was extracting from every tool call and every turn. The Learnings graph extracts at **natural boundaries**:

1. **Session end / compaction**: When a session ends or is compacted, extract learnings from the DAG summary (not raw turns).
2. **Work unit completion**: When a work unit moves to `done`, extract learnings from the work unit's session history.
3. **Explicit `index` command**: User/agent requests graph indexing via `GraphSearch index`.
4. **Periodic synthesis**: Every N sessions, perform a synthesis pass (analogous to the Residue prompt's "every 5 explorations" synthesis).

### What to Extract

Instead of extracting raw entities from every turn, extract **high-level learnings**:

1. **Decisions made** — What was decided and why (from assistant messages with decision language)
2. **Patterns discovered** — New code patterns or architectural approaches (from successful implementations)
3. **Anti-patterns identified** — Things that didn't work and why (from debugging sessions)
4. **Conventions established** — Rules for how code should be written (from review discussions)
5. **Reformulations** — New ways to think about problems (from exploration sessions)
6. **Constraints discovered** — Hard limits or requirements found during implementation

### LLM Extraction Prompt (Residue-Informed)

The extraction prompt should mirror the Residue methodology's structured logging:

```
You are analyzing a completed agent session to extract lasting knowledge.
Do NOT extract transient details (file names edited, commands run).
DO extract:

1. LEARNINGS: What lasting knowledge was gained?
   - Conventions established or confirmed
   - Patterns that worked well
   - Anti-patterns that should be avoided
   - Architectural decisions and their rationale

2. EXPLORATIONS: What approaches were tried?
   - For each approach: strategy, outcome, failure reason (if failed)
   - What class of approaches was eliminated by failures?
   - What surviving structure remained from failed attempts?

3. REFORMULATIONS: Were there new ways to understand existing problems?
   - New frameworks for thinking about the codebase
   - Insights that apply beyond the immediate task

4. CONSTRAINTS: What hard facts were discovered?
   - Technical limitations
   - Performance characteristics
   - Compatibility requirements
```

### Volume Estimate

A typical session produces:
- 0-3 Learning nodes
- 0-5 Exploration nodes
- 0-2 Convention nodes
- 0-1 Decision nodes
- 5-15 edges

After 1000 sessions: ~3000 nodes, ~10000 edges. Storage: **<5MB** (vs 7.6GB for the old system).

## Query Use Cases

### For Code Quality Improvement
1. **"What conventions exist for this codebase?"** → All Convention nodes, sorted by enforcement level
2. **"What anti-patterns have been identified?"** → Learning nodes with category=anti-pattern
3. **"What approaches have been tried for problem X?"** → Search Exploration nodes, traverse InformedBy edges
4. **"What decisions affect module X?"** → Decisions linked to relevant code patterns

### For Better Context
1. **"What did we learn from implementing feature Y?"** → Learnings linked to Explorations in that feature's sessions
2. **"What constraints apply to this area?"** → Constraint-type learnings related to the domain
3. **"What reformulations exist for this problem?"** → Reformulation-type learnings

### For Cross-Session Learning
1. **"What approaches were eliminated?"** → EliminatedApproach-linked Explorations
2. **"What surviving structure exists from past failures?"** → Exploration nodes with non-empty survivingStructure
3. **"When was the last synthesis?"** → Most recent Exploration with synthesis flag

## The Multi-Agent Dimension

The Residue methodology's key innovation is **cross-agent synthesis** — where one agent's failures become another agent's building blocks. The Learnings graph enables this for the codelet ecosystem:

1. **Supervisor/subordinate knowledge sharing**: When a supervisor spawns a subordinate (via AgentManager), the subordinate's DeepSearch system prompt can include relevant learnings from the graph.

2. **Cross-session continuity**: When resuming work on a work unit, the system can inject relevant learnings (decisions, constraints, conventions) from previous sessions on the same work unit.

3. **Failure propagation**: When an approach fails in one session, the structured record (with failure reason and eliminated class) prevents other sessions from repeating the same mistake.

4. **Periodic synthesis**: A scheduled job can perform synthesis across recent sessions, looking for patterns in the Concrete Artifacts of recent Explorations — exactly like the Residue prompt's "every 5 explorations" synthesis.

## Comparison: Old System vs. New

| Dimension | Old KGRAPH | Learnings Graph |
|-----------|-----------|-----------------|
| **Extraction trigger** | Every tool call + batch scan of all turns | Session boundaries + work unit completion |
| **Extraction granularity** | Per-turn (Turn nodes, Mentions edges) | Per-session (Learning, Exploration nodes) |
| **LLM cost** | High (batch scan of all turns) | Low (summary extraction at boundaries) |
| **Storage** | 7.6GB for 727 turns | Estimated <5MB for 1000 sessions |
| **Query value** | "Which turn mentioned concept X?" (low value) | "What was decided about X and why?" (high value) |
| **Cross-session utility** | Limited (concept mention counts) | High (Strategy Register, failure propagation) |

## References

- Knuth, D. (2026). "Claude's Cycles." https://cs.stanford.edu/~knuth/papers/claude-cycles.pdf
- Aquino-Michaels, K. (2026). "Completing Claude's Cycles: Multi-agent structured exploration on an open combinatorial problem." https://github.com/no-way-labs/residue
- Morrison, K. (2026). KnuthClaudeLean: Lean 4 formalization. https://github.com/kim-em/KnuthClaudeLean/
- Residue Prompt: https://github.com/no-way-labs/residue/blob/main/prompt/residue.md
