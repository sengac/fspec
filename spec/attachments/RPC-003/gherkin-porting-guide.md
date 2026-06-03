# Gherkin Porting Guide (TypeScript → Rust)

**Applies to:** All 40 RPC cards under epic `rust-cli-port` that touch `.feature` files.
**Sources studied:**
- `tmp/cucumber-rs/` — https://github.com/cucumber-rs/cucumber v0.23 (BDD test runner)
- `tmp/gherkin-rs/` — https://github.com/cucumber-rs/gherkin v0.16 (the underlying parser)

---

## 1. Crate Choice — `gherkin`, NOT `cucumber`

The `cucumber` crate is a **BDD test runner** with step macros, async dispatch, JUnit/JSON output writers, etc. It does NOT add any feature-file parsing beyond delegating to the `gherkin` crate (`tmp/cucumber-rs/src/parser/basic.rs:74` calls `gherkin::Feature::parse_path` directly).

**fspec's TypeScript code uses `@cucumber/gherkin` only for parse / mutate / format**, never for running tests. The matching Rust dependency is therefore:

```toml
[dependencies]
gherkin = { version = "0.16", features = ["parser", "serde"] }
```

- `parser` (default-on) — enables the PEG parser and `TypedBuilder` derives.
- `serde` — needed for JSON round-trip (used by `get-scenarios`, `show-acceptance-criteria`, etc.).

Add `cucumber` only if/when we port a future BDD runner — none of the current 40 commands need it.

---

## 2. Critical Gap: Comment Preservation

**The `gherkin` crate does not preserve comments.** From `tmp/gherkin-rs/src/lib.rs:33`:
> *"Indentation and comments are ignored by the parser."*

Confirmed by parser grammar (`tmp/gherkin-rs/src/parser.rs:162-168`) — comments are matched inside `quiet!{…}` whitespace rules and discarded. No `Comment` field exists on any AST struct.

**TS `gherkin-formatter.ts` preserves comments**, so we have **two options** in Rust:
1. **Drop comment preservation** in the initial port (document as a known regression).
2. **Build a sidecar comment-collector** — pre-scan source lines for `#…` comments with their (line, column), re-attach during emit by matching against AST node `position` / `span` ranges. (Recommended for the `format` command at minimum.)

The `# language: xx` directive IS recognized as a parser directive (`parser.rs:203-206`) but also not stored in the AST.

---

## 3. AST Schema (all fields `pub`, all have `TypedBuilder`)

All in `gherkin::*` (re-exported from `lib.rs`):

| Type | Fields (selected) | Has tags? |
|---|---|---|
| `Feature` | `keyword, name, description, background, scenarios, rules, tags, span, position, path` | yes |
| `Background` | `keyword, name, description, steps, span, position` | **no** (spec correct) |
| `Rule` | `keyword, name, description, background, scenarios, tags, span, position` | yes |
| `Scenario` | `keyword, name, description, steps, examples, tags, span, position` | yes |
| `Examples` | `keyword, name, description, table, tags, span, position` | yes |
| `Step` | `keyword, ty (StepType), value, docstring (Option<String>), table (Option<Table>), span, position` | n/a |
| `Table` | `rows: Vec<Vec<String>>, span, position` (row 0 is header by convention) | n/a |

Notes:
- **No `ScenarioOutline` type** — outlines reuse `Scenario` with non-empty `examples`. Distinguish via `keyword` (e.g. `"Scenario Outline"`) or `!s.examples.is_empty()`.
- **No `Tag` struct** — tags are bare `String` values *without* the leading `@` (parser strips it at `parser.rs:469-470`).
- **No `DocString` type** — stored as `Option<String>` on `Step.docstring`. Fence style (`"""` vs ` ``` `) is normalized.
- **`Step.keyword`** preserves raw source including `"And"`/`"But"`. `Step.ty` is the contextually-resolved `Given`/`When`/`Then`.
- `Span { start, end }` carries byte offsets. `LineCol { line, col }` are 1-based.

---

## 4. Parsing

```rust
use gherkin::{Feature, GherkinEnv};

// From string
let env = GherkinEnv::default();                  // English; pass GherkinEnv::new("xx") for others
let feat = Feature::parse(src, env)?;             // Result<Feature, ParseError>

// From path
let feat = Feature::parse_path("spec/login.feature", GherkinEnv::default())?;
// Result<Feature, ParseFileError>
```

Quirk: parser auto-appends a trailing `\n` if missing (`lib.rs:236-240`).

---

## 5. Error Reporting

```rust
// tmp/gherkin-rs/src/lib.rs:395-419
pub struct ParseError { /* position: LineCol, expected: HashSet<&'static str>  -- both PRIVATE */ }
pub enum ParseFileError { Reading{...}, Parsing { path, error: Option<EnvError>, source: ParseError } }
```

**Caveat:** `ParseError`'s `position` and `expected` fields are **not `pub`** — you can only access them via `Display`/`Debug`. For richer error UX (line snippets like the TS version), format with `{e}` then re-parse the message, OR maintain your own source-line cache keyed by line number.

No source-snippet is provided by the crate; surrounding lines must come from the original source string.

---

## 6. Mutation Patterns

All fields are `pub` → direct mutation. Use `TypedBuilder` to build new nodes.

```rust
use gherkin::{Scenario, Step, StepType};

// Add a scenario
feat.scenarios.push(
    Scenario::builder()
        .keyword("Scenario".to_string())
        .name("Login succeeds".to_string())
        .steps(vec![
            Step::builder().ty(StepType::Given).keyword("Given ".to_string())
                .value("a valid user".to_string()).build(),
            Step::builder().ty(StepType::When).keyword("When ".to_string())
                .value("they submit credentials".to_string()).build(),
            Step::builder().ty(StepType::Then).keyword("Then ".to_string())
                .value("they reach the dashboard".to_string()).build(),
        ])
        .build()
);

// Remove a scenario
feat.scenarios.retain(|s| s.name != "Deprecated");

// Mutate a step in place
if let Some(scen) = feat.scenarios.iter_mut().find(|s| s.name == "Login") {
    if let Some(step) = scen.steps.iter_mut().find(|s| s.value.contains("old")) {
        step.value = "new value".to_string();
    }
}
```

---

## 7. Tag Manipulation

Tags live in `Vec<String>` (no `@`) on `Feature`, `Rule`, `Scenario`, `Examples`. Backgrounds cannot be tagged (spec-correct).

```rust
fn add_tag(tags: &mut Vec<String>, tag: &str) {
    let bare = tag.trim_start_matches('@').to_string();
    if !tags.iter().any(|t| t == &bare) { tags.push(bare); }
}

fn remove_tag(tags: &mut Vec<String>, tag: &str) {
    let bare = tag.trim_start_matches('@');
    tags.retain(|t| t != bare);
}

fn has_tag(tags: &[String], tag: &str) -> bool {
    let bare = tag.trim_start_matches('@');
    tags.iter().any(|t| t == bare)
}
```

For tag-expression evaluation (e.g. `@a and not @b`), use `gherkin::tagexpr::TagOperation` (`lib.rs:49`, grammar at `parser.rs:565-574`).

---

## 8. Re-Serialization (Formatter) — Build Our Own

**There is no `Display`/`to_string`/`to_pretty_string` on `Feature`, `Scenario`, `Background`, `Rule`, `Examples`, or `Table`.** The only `Display` impl is on `Step` (`lib.rs:357-360`) and it only emits `"{keyword} {value}"` — no indentation, no docstring, no table.

We must write our own emitter that mirrors the canonical Gherkin layout the TS formatter produces:

```text
@tag1 @tag2
Feature: Name
  Description line 1
  Description line 2

  Background: <name>
    Given …

  @scenario-tag
  Scenario: Name
    Given …
    When …
    Then …

  @outline-tag
  Scenario Outline: Name
    Given <x>

    Examples:
      | x   |
      | one |
```

Standard indentation:
- Feature-level tags & `Feature:` — column 0
- Scenario/Background/Rule — `  ` (2 spaces)
- Steps — `    ` (4 spaces)
- Table rows / Examples table — `      ` (6 spaces) with column-aligned padding

Pull the emitter into a single shared helper (recommended location: `codelet/fspec-core/src/gherkin_emit.rs`) so every mutation command and the `format` command share one canonical writer.

---

## 9. Shared Rust Modules to Create

To avoid duplicating logic across all 40 cards, create these shared modules in `codelet/fspec-core/src/`:

| Module | Purpose | Maps to TS file |
|---|---|---|
| `gherkin_io.rs` | `load_feature(path) -> Feature`, `save_feature(path, &Feature)` (wraps emit) | `src/utils/feature-parser.ts` (subset) |
| `gherkin_emit.rs` | Canonical text serializer (§8) | `src/utils/gherkin-formatter.ts` |
| `gherkin_query.rs` | `parse_all_features`, `find_features_by_tag`, `search_scenarios`, `get_scenario_steps` | `src/utils/feature-parser.ts` |
| `gherkin_tags.rs` | `add_tag/remove_tag/has_tag` helpers + `extract_work_unit_tags` | `src/utils/work-unit-tags.ts` |
| `gherkin_validate.rs` | Syntax-only validation with line-snippet enrichment | (used by `validate`, `check`) |
| `coverage_file.rs` | `CoverageFile` struct + create/update/load using `gherkin_io` | `src/utils/coverage-file.ts` |

Every individual command card (RPC-167, RPC-171, RPC-190, etc.) should depend on these modules rather than re-importing `gherkin::` directly. **A separate card should be opened for each shared module BEFORE the dependent command cards begin.**

---

## 10. Known Limitations to Document Up-Front

1. **No comment preservation** unless we add a sidecar collector. The TS `gherkin-formatter-comment-preservation.test.ts` will need a Rust equivalent or be replaced with a documented regression note.
2. **Trailing newline injection** on parse — re-emit must explicitly normalize.
3. **`ParseError` field privacy** — limits richness of error UI; consider upstream PR or local fork.
4. **DocString fence normalization** — `"""` vs ` ``` ` distinction is lost on re-emit.
5. **Cell whitespace** — table cells are trimmed by the parser (`parser.rs:225-249`); padding for column alignment is the formatter's job.
6. **Step `And`/`But`** — `Step.keyword` preserves source verbatim, so re-emit will faithfully output `And`/`But`. `Step.ty` is resolved (Given/When/Then) — use it for semantic queries, not for emit.

---

## 11. Per-Command Category Map

Each of the 40 affected RPC cards has its own attachment with a category tag. Categories:

- **(A) Parse/Validate** — `validate`, `validate-tags`
- **(B) AST Mutation** — `add-architecture`, `add-background`, `add-scenario`, `add-step`, `add-tag-to-feature`, `add-tag-to-scenario`, `delete-features`, `delete-scenario`, `delete-scenarios`, `delete-step`, `format`, `remove-tag-from-feature`, `remove-tag-from-scenario`, `retag`, `update-scenario`, `update-step`
- **(C) Read/Query** — `compare-implementations`, `get-scenarios`, `list-feature-tags`, `list-features`, `list-scenario-tags`, `search-scenarios`, `show-acceptance-criteria`, `show-feature`, `show-test-patterns`, `show-work-unit`, `tag-stats`
- **(D) Coverage Linking** — `audit-coverage`, `link-coverage`, `show-coverage`, `unlink-coverage`
- **(E) Generation** — `create-feature`, `generate-coverage`, `generate-scenarios`
- **(F) Cross-cutting** — `check`, `review`, `update-work-unit-status`

---

## 12. Recommended Porting Order

1. **Foundation modules** (gherkin_io, gherkin_emit, gherkin_query, gherkin_tags, gherkin_validate, coverage_file) — open a child card under RPC-003 for each.
2. **(A) Parse/Validate** — establishes the parsing pipeline end-to-end.
3. **(E) `generate-scenarios`** — exercises the AST-builder pipeline.
4. **`format`** — exercises the emitter; once green, all (B) mutators inherit it.
5. **(B) Mutators** — straightforward once parser + emitter are stable.
6. **(C) Queries** — read-only consumers.
7. **(D) Coverage** — depends on (C) + `coverage_file`.
8. **(F) Cross-cutting** — depends on everything else.

---

## 13. Source File References (TypeScript Side, to Mirror)

| TS module | Rust target | Cards depending on it |
|---|---|---|
| `src/utils/gherkin-formatter.ts` | `gherkin_emit.rs` | All (B), `format`, `check` |
| `src/utils/feature-parser.ts` | `gherkin_query.rs` | All (C), `update-work-unit-status`, `compare-implementations`, `show-test-patterns`, `search-scenarios` |
| `src/utils/coverage-file.ts` | `coverage_file.rs` | All (D), `create-feature`, `generate-coverage` |
| `src/utils/work-unit-tags.ts` | `gherkin_tags.rs` | `add-tag-*`, `remove-tag-*`, `validate-tags`, `show-work-unit`, `update-work-unit-status` |
