# KGRAPH-005: LLM Extraction Pipeline — Implementation Notes

## Extraction Prompt

The prompt is the core of this card. It must be precise enough to avoid
hallucination while capturing meaningful concepts.

```
You are a knowledge graph extractor. Given a batch of agent conversation
turns, extract structured entities.

## Rules
- Only extract what is EXPLICITLY discussed — never infer or speculate
- Set confidence=high for explicitly named items, medium for contextually
  clear items, low for ambiguous references
- Use kebab-case for all slugs (e.g. "jwt-authentication", not "JWT Auth")
- Merge duplicates within a batch by slug
- Relations must connect two concepts that BOTH appear in this batch or
  were explicitly mentioned together

## Extract These Entity Types

### Concepts
Named ideas, technologies, patterns, domain terms.
```json
{ "slug": "string", "name": "string", "category": "string", "summary": "string", "confidence": "high|medium|low" }
```
Categories: architecture, convention, decision, dependency, domain_term,
error_class, feature, library, pattern, person, platform, process,
technology, tool

### Decisions
Explicit choices or conclusions reached (not hypotheticals).
```json
{ "slug": "string", "title": "string", "rationale": "string", "domain": "string", "confidence": "high|medium|low" }
```
Domains: architecture, convention, dependency, deployment, design,
implementation, process, testing

### Relations
How two concepts relate. Both concepts must be in this batch.
```json
{ "from": "concept-slug", "to": "concept-slug", "type": "string", "strength": 0.0-1.0 }
```
Types: causes, composes, conflicts_with, depends_on, extends,
implements, similar_to, supersedes, uses

## Output Format
Return a single JSON object:
```json
{
  "concepts": [...],
  "decisions": [...],
  "relations": [...]
}
```
Return empty arrays if nothing meaningful to extract.
```

## Batching Strategy

- **Batch size:** 5-10 turns per LLM call
- **Turn selection:** Only user + assistant turns, skip tool results
  (tool results are handled by structural extractors)
- **Content limit:** Truncate each turn to 2000 chars for the extraction
  prompt to stay within context limits
- **Max concurrent:** 3 parallel extraction calls

## Response Parsing

```rust
#[derive(Deserialize)]
struct ExtractionResult {
    concepts: Vec<ExtractedConcept>,
    decisions: Vec<ExtractedDecision>,
    relations: Vec<ExtractedRelation>,
}
```

Validation rules:
- Reject concepts with empty slug or name
- Reject relations where from == to
- Reject strength values outside 0.0-1.0
- Log and skip malformed entries (don't fail the whole batch)

## Model Selection

Use the cheapest fast model available — extraction is high-volume, low-stakes:
- Default: `claude-sonnet-4-20250514` (good extraction, reasonable cost)
- Configurable via skills file `extraction.llmExtraction.model`
- Falls back to the user's configured default model if specified model unavailable
