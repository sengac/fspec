# RPC-244 — CLI Error-Routing Parity Regression

**Discovered:** 2026-06-06 during post-restoration parity sweep
**Status:** Reopened (specifying)
**Severity:** Medium — wrong stream + wrong exit code on missing-file error

## Symptom

`./codelet/target/release/fspec list-feature-tags spec/features/missing.feature`
in a project where `missing.feature` does NOT exist:

| Stream / Code | TS reference (`node dist/index.js`) | Rust binary (current) |
|---|---|---|
| stdout | *(empty)* | `File not found: spec/features/missing.feature\n` |
| stderr | `Error: File not found: spec/features/missing.feature` | *(empty)* |
| exit code | **1** | **0** |

## TS reference (canonical, `src/commands/list-feature-tags.ts:120-123`)

```typescript
if (!result.success) {
  output.error('Error:', result.error);  // stderr, "Error:" prefix
  process.exit(1);                        // non-zero
}
```

## Root cause

`codelet/fspec-core/src/commands/list_feature_tags.rs` deliberately routes
structured errors (`{success:false, error:...}`) through `Ok(rendered_text)`
rather than `Err(FspecCoreError::...)` — this is correct for the LLM
dispatcher contract (architecture note [3]).

But `codelet/fspec/src/list_feature_tags.rs` then prints the rendered text
to stdout with exit 0 — losing the success/failure signal.

## Fix direction

CLI bridge needs to inspect the structured result, not the rendered text:

1. Add a public `ListFeatureTagsResultPublic` shape (or call core with
   `format: "json"` and parse).
2. If `success == false`, write `Error: <error>` to stderr and return 1.
3. Otherwise write the rendered text to stdout and return 0.

## Acceptance criteria gap

`spec/features/list-feature-tags-cli-subcommand.feature` has scenarios for
the help, happy-path, --show-categories, and bridge-no-duplication cases —
but NO scenario covering the structured-error CLI surface. Adding the
scenario and the rule that codifies it is the ACDD remediation.

## Parity fixture (reproduce)

```bash
TS="node /Users/rquast/projects/fspec/dist/index.js"
RUST="/Users/rquast/projects/fspec/codelet/target/release/fspec"
WORK=$(mktemp -d); cd "$WORK"; mkdir -p spec/features
cat > spec/features/sample.feature <<'GHK'
@critical
Feature: Sample
  Scenario: a thing
    Given x
GHK

echo "--- TS ---"
$TS list-feature-tags spec/features/missing.feature; echo "EXIT=$?"
echo "--- RUST ---"
$RUST list-feature-tags spec/features/missing.feature; echo "EXIT=$?"
```
