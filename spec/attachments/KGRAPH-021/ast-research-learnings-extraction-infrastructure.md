# AST Research: Learnings Extraction Infrastructure

## Existing LLM Extraction Pipeline

### Architecture
```
session_scanner.rs (orchestrator)
  ├── Phase 1: Structural extraction (zero-cost)
  │     └── extractors.rs
  └── Phase 2: LLM extraction
        ├── llm_extraction.rs — prompts, types, turn filtering
        ├── llm_caller.rs — batching, LLM invocation
        └── llm_validation.rs — entity validation
```

### Key Files
- `codelet/napi/src/graph/llm_extraction.rs` — ExtractionResult types, EXTRACTION_SYSTEM_PROMPT, turn filtering
- `codelet/napi/src/graph/llm_caller.rs` — prepare_turn_batches(), build_batch_prompts(), extract_from_session_turns(), call_extraction_llm()
- `codelet/napi/src/graph/llm_validation.rs` — parse_and_validate_response(), validates concepts/decisions/relations
- `codelet/napi/src/graph/session_scanner.rs` — Top-level scan_and_index_sessions()

### Reusable Components for Learnings Pipeline
1. **llm_caller.rs::call_extraction_llm()** — Can be reused with different prompt
2. **llm_caller.rs::extract_json_from_response()** — Universal JSON extraction from LLM responses
3. **GraphDatabase::load_entities()** — Batch loading API (from database.rs)
4. **GraphEntity enum** — Shared entity representation

### Learnings Schema (learnings.pg)
- Node types: Learning, Exploration, Convention, Decision, CodePattern
- Edge types: Discovered, Eliminates, Supersedes, RelatesTo, InformedBy, Applies, Contradicts
- All node types have `slug: String @key` for upsert semantics

### Design Principles for New Pipeline
1. Use LEARNINGS_GRAPH registry name, not AGENT_MEMORY_GRAPH
2. Session boundary triggers only (not per-turn)
3. Residue methodology prompt structure
4. 5-20 entities per extraction (volume constrained)
5. Reuse llm_caller infrastructure, create new prompt + validation
