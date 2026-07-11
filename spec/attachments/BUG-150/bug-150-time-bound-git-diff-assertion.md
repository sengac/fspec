# BUG-150: `bug146_napi_attribute_scoping` pins a time-bound uncommitted git-diff shape

## Summary

The test `scenario_fix_replaces_field_level_attrs_and_no_other_changes` in
`codelet/rpc-types/tests/bug146_napi_attribute_scoping.rs` (line ~373) asserts the shape of the
**uncommitted git working-tree diff** of `codelet/rpc-types/src/lib.rs`. It requires that the
diff contains **exactly 34 added `serde(rename` lines** and 33–34 removed
`cfg_attr(...napi(js_name...)` lines. This is a *time-bound git-state assertion*: it was only
ever true in the specific working tree that existed while the BUG-146 fix sat uncommitted. On
any tree whose diff diverges — including a clean tree after the fix is committed, or a tree with
additional legitimate edits — the test goes red even though the production code is correct.

- **Status when filed:** pre-existing red test, known stale since the BUG-147 era.
- **Deepened by:** CONT-007 / CONT-008, whose rpc-types edits added **+6 `serde(rename` lines**
  to the working tree, pushing the added-line count past 34.
- **Related to:** CONT-008 (`relatesTo` dependency already recorded on the card).

## Affected file and test

| Item | Value |
|---|---|
| Test file | `codelet/rpc-types/tests/bug146_napi_attribute_scoping.rs` (693 lines) |
| Failing scenario | `scenario_fix_replaces_field_level_attrs_and_no_other_changes` (line ~373) |
| Feature under test | BUG-146 Option B: replace 34 field-level `#[cfg_attr(feature = "napi", napi(js_name = "X"))]` decorations with `#[serde(rename = "X")]` |
| Subject file | `codelet/rpc-types/src/lib.rs` |

## The offending assertions (in execution order)

1. **Line ~378–398 — `git diff codelet/rpc-types/src/lib.rs --stat`**
   Expects the stat output to contain `34 insertions(+)` **and** `34 deletions(-)`. There is
   already a soft-fallback note (line ~395–398) acknowledging the working tree may have more
   changes — evidence the authors knew this was fragile.

2. **Line ~404–460 — full `git diff` parse**
   Collects every diff line matching `+ ... serde(rename` and asserts the count is
   **exactly 34** (`assert_eq!(added.len(), 34, ...)` at line ~456–460).

3. **Line ~465–470 — removed-line count**
   Asserts the diff-removed set contains **33 or 34** `cfg_attr...napi(js_name` lines.

4. **Line ~478–522 — pairwise replacement check**
   For each removed `napi(js_name = "X")`, asserts an added `serde(rename = "X")` with the same
   `X` exists *in the diff*.

5. **Line ~551 — `git diff Cargo.toml`** — also inspects uncommitted state.

Note that the test **already contains the correct, durable form** of the pin at line ~437–446:
it asserts the committed/on-disk source `lib.rs` contains **at least 34** `#[serde(rename = `
lines. That content-based assertion is sound; the git-diff assertions layered on top of it are
what go stale.

## Why this is a bug

- **Non-hermetic:** the test's outcome depends on `git status` of the developer's tree, not on
  the code being compiled and tested. Any unrelated legitimate edit to `lib.rs` (e.g.
  CONT-007/CONT-008 adding new renamed fields) breaks it.
- **One-shot semantics:** the assertions only describe the *transition* (the fix diff), not the
  *invariant* (the fixed code). Once the fix is committed, the diff is empty and the test can
  never pass again without artificially reconstructing a dirty tree.
- **Misleading red:** it currently fails on a fully correct tree, training people to ignore red
  tests in `rpc-types`.

## Current observed failure mode (2026-07-10)

Working tree contains the committed BUG-146 fix **plus** CONT-007/CONT-008 edits, so:

- `git diff --stat` no longer reports `34 insertions(+) / 34 deletions(-)`.
- The added-`serde(rename` count in the diff is not 34 (the +6 new rename lines from
  CONT-007/CONT-008 shift the count; on a clean tree it would be 0).

## Proposed fix

**Retire the git-state assertions and convert the scenario to a content-based pin on the
committed source.** Concretely:

1. Delete (or rewrite) the `git diff` / `git diff --stat` steps in
   `scenario_fix_replaces_field_level_attrs_and_no_other_changes`.
2. Replace them with assertions against `codelet/rpc-types/src/lib.rs` contents:
   - The file contains **zero** field-level `#[cfg_attr(feature = "napi", napi(js_name = ` at
     the 34 documented field sites (the site list already lives in the test at line ~65–68).
   - Each of the 34 documented renames `X` appears as `#[serde(rename = "X")]` on the
     corresponding field (reuse the existing expected-rename table already used by the
     `index.d.ts` scenario at line ~584–610).
   - Keep the existing `>= 34` count assertion (line ~443–446) — or tighten it to an exact
     count *of field-level sites only* if the site list is used, so new legitimate renames
     elsewhere don't break it.
3. Update the corresponding Gherkin scenario steps in the BUG-146 feature file so the feature,
   `@step` comments, and coverage links stay in sync (`fspec link-coverage` re-link after the
   test edit).
4. Sanity-check the sibling scenario `scenario_repro_before_fix_shows_34_unscoped_napi_errors`
   (line ~122): it reconstructs the *pre-fix* source and pins exactly 34
   ``cannot find attribute `napi` in this scope`` errors. If it derives the pre-fix source from
   git history rather than synthesizing it, it has the same time-bound disease and must be
   fixed in the same pass.

### Rejected alternative

Pinning the diff against a fixed base commit (`git diff <sha> -- lib.rs`) was considered and
rejected: it merely moves the time-bound coupling to a hard-coded SHA and still breaks on every
legitimate follow-up edit to `lib.rs`.

## Acceptance sketch

- `cargo test -p codelet-rpc-types --test bug146_napi_attribute_scoping` passes on a **clean**
  tree at HEAD.
- The same test still passes after an unrelated edit to `lib.rs` (e.g. adding a new struct with
  its own `serde(rename)` fields).
- The test still **fails** if any of the 34 documented field-level renames is reverted to
  `napi(js_name)` or removed — the actual BUG-146 regression it exists to guard.

## Verification commands

```bash
cargo test -p codelet-rpc-types --test bug146_napi_attribute_scoping
git -C . diff codelet/rpc-types/src/lib.rs --stat   # must NOT influence the test outcome
```
