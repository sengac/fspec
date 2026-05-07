# Formal Verification with Alloy

This document describes the formal verification process used in the fspec
codebase to mathematically prove correctness of safety- and correctness-critical
subsystems. We use **Alloy 6** (Daniel Jackson, MIT) — a lightweight formal
modeling language and bounded model checker — alongside conventional unit and
property-based tests.

> **Reference paper:** Daniel Jackson, *"Alloy: A Language and Tool for
> Exploring Software Designs"* (CACM, 2019).
> https://groups.csail.mit.edu/sdg/pubs/2019/alloy-cacm-18-feb-22-2019.pdf

---

## Why Alloy?

Conventional testing (Vitest, Rust unit tests, integration tests) verifies
**examples** — specific inputs produce specific outputs. Alloy verifies
**universally quantified properties** over a bounded universe — for every
possible state up to some scope, an invariant holds (or a counterexample is
exhibited).

Alloy is best for:

- **Structural invariants** across many interacting fields (e.g., "cumulative
  billed tokens never decreases across API calls")
- **State-machine correctness** (e.g., "no turn is silently lost during DAG
  construction")
- **Concurrency / ordering** properties (e.g., "ToolResult never trimmed without
  prior ToolUse registration")
- **Cross-cutting safety** properties that span many functions

Alloy is **not** a replacement for unit tests — it proves the *model* is
consistent, not that the *implementation* matches the model. The full
verification stack is:

```
┌─────────────────────────────────────────────────────────┐
│  Alloy model      → Proves DESIGN is correct            │
│  proptest tests   → Proves IMPL matches MODEL           │
│  Unit tests       → Proves specific examples work       │
│  Integration test → Proves end-to-end behaviour         │
└─────────────────────────────────────────────────────────┘
```

---

## Process

Each formal proof follows the same five-step process:

### 1. Identify invariants

Read the code and its doc comments. Extract every claim of the form "this is
always true" or "this never happens". These become candidate invariants. The
richest sources are:

- Module-level doc comments
- Type invariants ("X is monotonically increasing", "Y is absolute, not
  cumulative")
- Comments near `assert!`, `debug_assert!`, or panics
- Test names that describe rules ("does not double-count cache tokens")

### 2. Build an Alloy model

Write a `.als` file in the relevant subsystem's `spec/` directory that:

- Declares signatures (`sig`) for the domain objects
- Declares facts (`fact`) for structural constraints that always hold
- Declares predicates (`pred`) for state transitions / operations
- Declares assertions (`assert`) for invariants to be proved

Keep models **minimal** — abstract away anything not relevant to the property
being checked. A 100-line model that catches a real bug is worth far more than
a 1000-line model nobody can read.

### 3. Run bounded checks

Open the model in Alloy Analyzer 6 (or run via CLI) and execute:

```alloy
check NoTurnLost for 8       // bounded scope: up to 8 of each sig
check NoTurnLost for 5 but 10 ConversationTurn   // override one sig
```

Alloy explores **every possible structure** within scope. If a counterexample
exists, it shows it visually. If not within scope, the check passes — but
note: Alloy proves bounded correctness, not unbounded correctness. The
**small scope hypothesis** (Jackson's empirical observation) holds that most
design bugs manifest at small scopes.

### 4. Cross-check with property tests

For each invariant proved in Alloy, write a corresponding `proptest` (or
similar) test in Rust that checks the implementation upholds the same property
on randomly generated inputs. This catches the "model says X but code does Y"
class of bugs.

### 5. Document the link

Add a doc-comment annotation in the Rust source pointing to the Alloy file and
the specific assertion proved:

```rust
/// Invariant: cumulative_billed_input is monotonically non-decreasing.
/// Proved by: codelet/core/spec/compaction/token_tracker.als
///   assertion: CumulativeBilledMonotonic
fn record_api_response(&mut self, usage: &ApiTokenUsage) { ... }
```

This creates traceability so future engineers can find the proof when they
modify the function.

---

## Tooling

### Required

- **Alloy Analyzer 6**: https://alloytools.org/download.html
  - macOS (recommended): `brew install alloy-analyzer`
  - Or download `alloy.jar` and run with `java -jar alloy.jar`
- **JDK 17+** (Alloy 6.2 is compiled to class-file 61). On macOS:
  `brew install openjdk` (latest) or `brew install openjdk@17`.

### Headless runner

The repo ships a headless runner that executes every `check` and `run`
command in every model and reports pass/fail:

```bash
scripts/run-alloy.sh                     # run all models
scripts/run-alloy.sh trimmer.als         # run a specific model
scripts/run-alloy.sh --verbose           # show full Alloy output
```

The runner exits 0 if every assertion is proved (UNSAT for `check`, SAT for
`run`) and non-zero otherwise. JDK detection is automatic for common
Homebrew install paths; override via `JAVA_HOME=...` if needed.

### Optional supplements

- **`proptest` crate** (Rust) — property-based tests that mirror Alloy
  assertions
- **Kani** (Rust model checker) — bounded verification of actual Rust source
  for the highest-stakes functions
- **Stateright** — model checking specifically for state-machine systems in
  Rust

### CI integration (future)

Alloy can be invoked headlessly via `org.alloytools.alloy.cli` for CI smoke
checks. Initially we run models manually during design review and reference
them in PRs.

---

## File Layout Convention

Per-subsystem Alloy models live alongside the code they verify:

```
codelet/core/
├── src/compaction/             # Rust source
│   ├── trimmer.rs
│   ├── model.rs
│   └── ...
└── spec/compaction/            # Alloy models (this convention)
    ├── README.md               # How to run these models
    ├── trimmer.als
    ├── token_tracker.als
    └── dag_compaction.als
```

The `spec/` directory inside each Rust crate is **separate** from the
project-level `spec/` directory used by fspec's Gherkin features. They serve
different purposes:

| Directory                        | Purpose                              |
|----------------------------------|--------------------------------------|
| `spec/features/*.feature`        | Acceptance criteria (Gherkin)        |
| `codelet/<crate>/spec/<mod>/*.als` | Formal models (Alloy)              |

---

## Findings (open observations)

Properties surfaced during verification that are NOT bugs but represent
unenforced contracts in the implementation. Each is pinned by a test under
`codelet/core/src/compaction/__tests__/dag_node_proptest.test.rs` so any
future change tightening the contract will surface visibly.

| ID        | Severity | Source        | Description                                                  |
|-----------|----------|---------------|--------------------------------------------------------------|
| FV-001-a  | Low      | `trimmer.rs`  | `HashMap::insert` silently overwrites entries — relies on Anthropic UUID uniqueness for `tool_use_id`. No `debug_assert!` guard. |
| FV-003-a  | Low      | `parse_dag_nodes` | Accepts `turns="50-10"` (start > end). Pinned by `limitation_parser_does_not_validate_start_le_end`. |
| FV-003-b  | Info     | `parse_dag_nodes` | Logs a warning but ACCEPTS overlapping same-depth ranges. Pinned by `limitation_parser_does_not_reject_overlap`. |
| FV-003-c  | Low      | `parse_dag_nodes` | Clamping `turn_end` to `message_count - 1` can produce an inverted range when `turn_start ≥ message_count`. Pinned by `limitation_clamping_can_invert_range`. |

None of these are defects — but each represents a gap between what the
formal model assumes and what the implementation enforces. If hardening any
of them is desired, the corresponding `limitation_*` test will surface a
deliberate decision rather than allowing silent contract drift.

---

## Proofs

The list below tracks every formal proof in the codebase, its status, and the
files involved.

| Proof ID    | Subsystem            | Alloy        | proptest     | Model file                                          |
|-------------|----------------------|--------------|--------------|-----------------------------------------------------|
| FV-001      | Compaction trimmer   | ✅ Proved    | 📝 Planned   | `codelet/core/spec/compaction/trimmer.als`          |
| FV-002      | Token tracker        | ✅ Proved    | ✅ Cross-checked | `codelet/core/spec/compaction/token_tracker.als` |
| FV-003      | DAG compaction       | ✅ Proved    | ✅ Cross-checked + 3 limitations pinned | `codelet/core/spec/compaction/dag_compaction.als`   |

**Status legend:**
- 📝 Planned — invariants identified, model not yet written
- 🚧 Drafted — model exists, assertions failing or not yet checked
- ✅ Proved — all assertions pass within stated scope
- ⚠️ Counterexample — model exposed a real defect (see linked issue)

---

### FV-001: Compaction Trimmer

**Subsystem:** Structurally lossless trimmer (Layer 0 of hierarchical
compaction).

**Source files verified:**
- `codelet/core/src/compaction/trimmer.rs`
- `codelet/core/src/compaction/trimmer_base64.rs`
- `codelet/core/src/compaction/trimmer_metadata.rs`
- `codelet/core/src/compaction/mod.rs`

**Existing tests (regression coverage):**
- `codelet/core/src/compaction/__tests__/trimmer.test.rs`
- `codelet/core/src/compaction/__tests__/structural_annotation.test.rs`
- `codelet/core/src/compaction/__tests__/annotation_detector.test.rs`

**Model file:** `codelet/core/spec/compaction/trimmer.als` *(to be created)*

**Invariants to prove:**

1. **Tool-registry append-only within a session.** Once `trim_assistant_message`
   inserts a `tool_use_id`, no later message overwrites it with a different
   `name` or `input`.
2. **No ToolResult trimmed without prior ToolUse.** If a user message contains
   a `tool_use_id` not in the registry, the trimmer falls back to
   `trim_by_content_heuristics` — never to a tool-specific path that would
   misinterpret the content.
3. **Lossless-by-substitution boundary.** Write/Edit content is replaced with
   a *human-readable summary* string that is structurally distinguishable from
   any valid original content (begins with `[Write:` or `[Edit:`).
4. **Trimming is idempotent.** Trimming an already-trimmed message produces the
   same output (modulo registered tool uses).
5. **Bash exit-code parsing soundness.** `parse_exit_code_from_content` either
   returns `None` (and falls back to `is_error` flag) or returns a valid
   non-negative integer matching the prefix in the content.

---

### FV-002: Token Tracker

**Subsystem:** Token usage accounting (`TokenTracker`, `ApiTokenUsage`,
`TokenState`).

**Source files verified:**
- `codelet/core/src/compaction/model.rs` (TokenTracker, lines documenting
  CTX-003)
- `codelet/core/src/token_usage.rs` (ApiTokenUsage)
- `codelet/core/src/compaction_hook.rs` (TokenState, threshold logic)

**Existing tests (regression coverage):**
- `codelet/core/tests/token_tracker_update_test.rs`
- `codelet/core/tests/cache_token_extraction_test.rs`
- `codelet/core/tests/rig_012_reasoning_token_propagation_test.rs`

**Model file:** `codelet/core/spec/compaction/token_tracker.als` *(to be created)*

**Invariants to prove (from `model.rs` doc comment):**

1. **`input_tokens` is absolute, not cumulative.** After any sequence of API
   responses, `input_tokens` equals the *latest* response's total context size,
   never a sum across responses.
2. **`output_tokens` is cumulative.** `output_tokens` after N responses equals
   the sum of `output_tokens` from each individual response.
3. **`cumulative_billed_input` is monotonically non-decreasing.** Across any
   sequence of API responses, `cumulative_billed_input` only ever grows.
4. **`total_input() = input + cache_read + cache_creation`** — this identity
   holds for every API response and for the resulting `TokenTracker` state.
5. **No double-counting of cache tokens.** A token reported in
   `cache_read_input_tokens` is never simultaneously reported in
   `cache_creation_input_tokens` for the same response.
6. **Threshold checks use absolute, not cumulative, values.** The compaction
   hook fires based on current context size (`total_input`), not on
   `cumulative_billed_input`.

---

### FV-003: DAG Compaction

**Subsystem:** Hierarchical lossless context compaction via in-view DAG
construction.

**Source files verified:**
- `codelet/core/src/compaction/model.rs` (`ConversationTurn`, `DagDepth`,
  `DagNodeMeta`, `parse_dag_nodes`, `wrap_dag_content`)
- `codelet/core/src/compaction_hook.rs`
- `codelet/core/src/compaction/annotation_detector.rs`
- `codelet/tools/src/inject_summary.rs`

**Existing tests (regression coverage):**
- `codelet/core/src/compaction/__tests__/dag_node_parsing.test.rs`
- `codelet/core/tests/legacy_compaction_cleanup_test.rs`

**Model file:** `codelet/core/spec/compaction/dag_compaction.als` *(to be created)*

**Invariants to prove:**

1. **No turn silently lost.** Every `ConversationTurn` in the input is, after
   compaction, either:
   - Preserved verbatim, OR
   - Trimmed by Layer 0 (still present, content reduced), OR
   - Subsumed into a DAG node whose `DagNodeMeta` covers its turn index.
2. **DAG depth is bounded.** `parse_dag_nodes` and `wrap_dag_content` cannot
   produce nodes deeper than `DagDepth::MAX` (no recursive blow-up).
3. **DAG node coverage is contiguous.** A DAG node covering turns
   `[i, j]` includes every turn in that range — no gaps.
4. **DAG node coverage is non-overlapping.** No two DAG nodes at the same
   depth cover the same turn index.
5. **Restoration round-trip.** Parsing a DAG-wrapped content with
   `parse_dag_nodes` and rewrapping it produces equivalent structure.
6. **Cache stability across compaction.** A compaction event does not
   invalidate cache entries for turns prior to the earliest compacted turn.

---

## Adding a New Proof

When adding a new formal proof to this codebase:

1. **Allocate a Proof ID** in the table above (`FV-NNN`, sequential).
2. **Identify invariants** from doc comments, tests, and code review.
3. **Create the `.als` model** in the subsystem's `spec/` directory.
4. **Write a per-subsystem `README.md`** explaining how to run the model and
   what each assertion proves.
5. **Update the table** with the proof's status and file location.
6. **Cross-link from Rust source** with `Invariant: ... Proved by: ...` doc
   comments on the relevant functions/types.
7. **Mirror invariants in `proptest`** to catch implementation drift.

---

## References

- Jackson, D. *Alloy: A Language and Tool for Exploring Software Designs.*
  CACM, 2019.
  https://groups.csail.mit.edu/sdg/pubs/2019/alloy-cacm-18-feb-22-2019.pdf
- Jackson, D. *Software Abstractions: Logic, Language, and Analysis.* MIT
  Press, 2nd ed., 2012.
- Alloy Analyzer 6: https://alloytools.org
- Alloy tutorial: https://alloytools.org/tutorials/online/
