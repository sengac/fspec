# Compaction — Formal Models (Alloy 6)

This directory contains Alloy 6 models that formally verify invariants of the
context compaction subsystem.

> **Background:** see `docs/FORMAL_VERIFICATION.md` for the
> overall verification process and proof index.

## Models in this directory

| File                      | Proof ID | Verifies                                                |
|---------------------------|----------|---------------------------------------------------------|
| `token_tracker.als`       | FV-002   | TokenTracker invariants (CTX-003, PROV-001)             |
| `trimmer.als`             | FV-001   | Structurally lossless trimmer registry/dispatch safety  |
| `dag_compaction.als`      | FV-003   | Hierarchical DAG construction — no turn lost, no overlap |

## Running the models

### Quick start (headless)

From the repo root:

```bash
scripts/run-alloy.sh
```

This runs every `check` and `run` command across all `.als` files in this
directory and reports a pass/fail summary. Expected output (current state):

```
Total: 26   ✅ pass: 26   ❌ fail: 0   ⚠️  unknown: 0
```

Run a single model: `scripts/run-alloy.sh trimmer.als`
Verbose output:     `scripts/run-alloy.sh --verbose`

### Prerequisites

- **JDK 17 or later** (`java -version` to check). On macOS:
  `brew install openjdk` or `brew install openjdk@17`.
- **Alloy Analyzer 6**: `brew install alloy-analyzer`

### GUI workflow

```bash
# macOS Homebrew install:
alloy

# Or with the JAR directly:
java -jar /opt/homebrew/Cellar/alloy-analyzer/<version>/libexec/org.alloytools.alloy.dist-<version>.jar
```

Then in the Alloy GUI:
1. **File → Open** → select one of the `.als` files in this directory.
2. **Execute → Execute All** to run every `check` and `run` command.
3. The text panel shows results. For any `check`, Alloy reports:
   - `No counterexample found.` — invariant holds within the stated scope.
   - `Counterexample found.` — click to view the offending instance visually.

### CLI workflow (single command)

```bash
JAVA_HOME=/opt/homebrew/opt/openjdk/libexec/openjdk.jdk/Contents/Home \
  alloy exec -q -f -c <CommandName> <model>.als
```

Use `-c '*'` to run all commands in a file. Solution files (counterexamples
or `run` instances) are written to a directory named after the model.

## What each model proves

### `token_tracker.als` (FV-002)

Verifies the token-accounting invariants documented in
`rust/core/src/compaction/model.rs` (CTX-003) and
`rust/core/src/token_usage.rs` (PROV-001):

- **TotalInputIdentity** — `total_input() = input + cache_read + cache_creation`
- **CumulativeBilledInputMonotonic** — `cumulative_billed_input` only grows
- **CumulativeBilledOutputMonotonic** — `cumulative_billed_output` only grows
- **InputTokensAbsolute** — after `update_from_usage`, `input_tokens` equals
  *this* request's `total_input`, never a sum across requests
- **CompactionPreservesCumulativeBilling** — `reset_after_compaction` does not
  zero out cumulative billing
- **CompactionClearsTransientState** — output, reasoning, and cache values
  are cleared on compaction
- **CumulativeBilledLowerBound** — every `update_from_usage` adds at least
  `usage.input_tokens` to the cumulative
- **InputTokensRequireUpdate** — tracker doesn't spontaneously gain tokens

### `trimmer.als` (FV-001)

Verifies the safety properties of the Trimmer's tool-use registry and
dispatch logic in `rust/core/src/compaction/trimmer.rs`:

- **RegistryAppendOnly** — once an id is registered, it is not replaced with
  a different tool name
- **NoDuplicateIds** — no two distinct ToolUse entries share an id
- **ToolPathRequiresRegistration** — Read/Bash/Grep paths only fire if the
  registry contains a matching entry
- **UnknownIdFallsBackSafely** — unknown id ⇒ heuristic or image path,
  never a tool-specific path
- **ImageAlwaysImagePath** — base64-image content always takes the image
  path regardless of registry state
- **DispatchDeterministic** — every (msg, registry) pair has exactly one
  outcome

### `dag_compaction.als` (FV-003)

Verifies the structural invariants of hierarchical DAG-based compaction.
The model encodes the **algorithm's guarantees** (G1, G2) as facts and
verifies derived properties:

**Algorithm guarantees (facts):**
- **G1 (RemovedTurnsCovered)** — every removed turn is covered by at least
  one DAG node (no silent loss).
- **G2 (SameDepthNonOverlapping)** — at any single depth, no two DAG nodes
  share a turn.

**Derived properties (assertions):**
- **NoTurnLost** — every turn is either present in the log or covered by
  a DAG node (follows from G1).
- **SameDepthNonOverlapping** — restatement of G2 as an assertion.
- **NodeCoverageContiguous** — a node covering `[start, end]` covers every
  turn in that range.
- **DepthBounded** — only D0/D1/D2 — no depth blow-up.
- **RemovedRequiresCoverage** — contrapositive of NoTurnLost.
- **EmptySessionNoDag** — empty input ⇒ empty output.
- **NodeRangeBounded** — a node's range size ≤ total turn count.

> **Note:** The Rust implementation MUST uphold G1 and G2. If it doesn't, the
> Alloy proof is vacuous. The accompanying `proptest` tests are the
> implementation-side checks that lock these guarantees.

## Scope notes

Alloy proves bounded correctness — every `check` runs against a finite scope
(`for N` or `for N but K steps`). Per Jackson's **small-scope hypothesis**,
most design defects manifest at small scopes (≤ 5–8 elements). When you
modify a model, increase the scope until either:
- a counterexample appears, or
- the analyser slows beyond your patience.

If a model passes at scope 5 but you suspect a high-cardinality issue, run
again at scope 8 or 10. If a real-world configuration could exceed the
modelled scope, document why the small-scope hypothesis is expected to hold
in that file's comments.

## Maintenance

When the corresponding Rust code changes, the model **may** need updating.
The doc-comment annotation on each verified function points to the specific
assertion proved. If you change a function's semantics:

1. Update the Rust code.
2. Update the relevant `.als` model.
3. Re-run `check` commands.
4. If a counterexample appears, decide whether the new design is correct
   (update the assertion) or buggy (revert the code change).

If a model becomes stale because the code it referenced was removed, mark
the assertion as `// HISTORICAL —` and either delete or generalise it.
