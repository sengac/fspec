# RPC-003 — Rust Gherkin Parser Research

**Status:** Research note (supplements `rpc-003-feasibility.md`)
**Date:** 2026-05-07
**Question:** What Rust crate(s) should `codelet/fspec-core` use for Gherkin parsing/formatting, and how big is the gap vs the current `@cucumber/gherkin` (JS) dependency?

---

## TL;DR

- **No official Rust port** exists in `cucumber/gherkin`. That repo ships C, C++, Dart, .NET, Go, Java, JS, Perl, PHP, Python, and Ruby — Rust is conspicuously absent.
- **The de-facto Rust impl is `cucumber-rs/gherkin`** (crates.io: `gherkin`), MIT OR Apache-2.0, maintained by Brendan Molloy and the cucumber-rs org. It is the parser used by the `cucumber` Rust BDD crate.
- **Latest release: `0.16.0` (2026-04-23)**, MSRV 1.88, `forbid(unsafe_code)`, ~1,837 LOC across 4 files. Active and stable.
- **Recommendation: depend on `gherkin = "0.16"`** for `codelet/fspec-core` parsing. Write a **thin adapter** that maps the cucumber-rs AST to fspec's internal types. **Port fspec's existing custom formatter** to Rust ourselves (the cucumber-rs crate does NOT ship a formatter / pretty-printer).
- **Conformance risk:** the cucumber-rs crate does NOT run the official `cucumber/gherkin/testdata/` conformance suite (282 fixture files, ndjson AST/pickles/tokens). We should add a conformance harness in `codelet/fspec-core` that runs against that suite to lock down behaviour and catch divergences before they hit our `validate` command.

---

## 1. What's available in the Rust ecosystem

I cloned both candidate repos to `/tmp` and inspected them:

| Repo | Rust support | Notes |
|---|---|---|
| `github.com/cucumber/gherkin` | ❌ none | Polyglot home of the official parser. No Rust folder. Latest commit 2026-05-07. |
| `github.com/cucumber-rs/gherkin` | ✅ yes (the only viable choice) | crates.io `gherkin = "0.16"`. Pure Rust. Used by `cucumber-rs/cucumber`. |

There is no plausible second-source Rust Gherkin parser (the only other crates on crates.io that mention "gherkin" are either abandoned forks or downstream BDD frameworks consuming `gherkin`).

---

## 2. cucumber-rs/gherkin — anatomy

Cloned: `/tmp/cucumber-rs-gherkin` (commit `aaa7d59`, "Prepare 0.16.0 release").

### Source layout (1,837 LOC total)

```
src/
├── lib.rs        419 lines — public AST types + Feature::parse / parse_path
├── parser.rs    1078 lines — single peg::parser! { ... } grammar block + GherkinEnv
├── keywords.rs   152 lines — Keywords<'a> struct + lookup
├── tagexpr.rs    188 lines — TagOperation::{And, Or, Not, Tag} + FromStr
└── languages.json — copied verbatim from cucumber/gherkin's gherkin-languages.json
build.rs           — codegen: reads languages.json, emits a const Keywords for each lang
```

### Cargo features

```toml
default = ["parser"]
parser  = ["dep:typed-builder"]   # AST + parse functions
serde   = [...]                    # Serialize/Deserialize on every AST node
juniper = [...]                    # GraphQL types (irrelevant for us)
```

For fspec we want `default-features = false, features = ["parser", "serde"]` — we need serde for JSON output (`show-feature --format=json`, etc.) and we don't want juniper.

### Dependencies (lean)

- `peg = 0.6.3` — PEG parser-generator (compile-time grammar)
- `textwrap = 0.16` — whitespace handling
- `thiserror = 2.0` — error derives
- `typed-builder = 0.23` (optional, with `parser` feature)
- `serde` (optional)

Build deps: `heck`, `quote`, `serde`, `serde_json`, `syn` — all for the i18n keyword codegen.

No tokio. No async. No unsafe. Single-threaded synchronous parser. Perfect for our use case (fspec is CLI-bound, parses one feature file at a time, parallelism is at the fs-walk level).

### AST shape (lib.rs)

```rust
pub struct Feature {
    pub keyword: String, pub name: String,
    pub description: Option<String>,
    pub background: Option<Background>,
    pub scenarios:  Vec<Scenario>,
    pub rules:      Vec<Rule>,
    pub tags:       Vec<String>,
    pub span: Span, pub position: LineCol,
    pub path: Option<PathBuf>,
}
pub struct Rule        { keyword, name, description, background, scenarios, tags, span, position }
pub struct Scenario    { keyword, name, description, steps, examples, tags, span, position }
pub struct Background  { keyword, name, description, steps,         span, position }
pub struct Examples    { keyword, name, description, table, tags,   span, position }
pub struct Step        { keyword, ty: StepType, value, docstring: Option<String>, table: Option<Table>, span, position }
pub enum   StepType    { Given, When, Then }            // And/But are resolved contextually
pub struct Table       { rows: Vec<Vec<String>>, span, position }
pub struct Span        { start: usize, end: usize }
pub struct LineCol     { line: usize, col: usize }

pub fn is_language_supported(lang: &str) -> bool;
```

Entry points:

```rust
Feature::parse_path(path, GherkinEnv::new("en")?)  -> Result<Feature, ParseFileError>
Feature::parse     (str,  GherkinEnv::new("en")?)  -> Result<Feature, ParseError>
```

Tag expressions (we already use these for `--tag` filters in fspec):

```rust
let op: TagOperation = "@a and not @b".parse()?;     // FromStr
```

### Errors

`ParseError { position: LineCol, expected: HashSet<&'static str> }` — peg gives precise positions and an "expected token set", which is more useful than `@cucumber/gherkin`'s string-only error in JS.

### What's NOT in the crate

| Need | cucumber-rs/gherkin | Status |
|---|---|---|
| Parse `.feature` → AST | ✅ | done |
| i18n keywords (47 langs) | ✅ | codegen from upstream `gherkin-languages.json` |
| Tag expressions | ✅ | `tagexpr` module |
| Spans / line-col | ✅ | every AST node |
| Pickle compiler (scenario outline expansion) | ❌ | lives in `cucumber-rs/cucumber` separately. **fspec doesn't need it** — we don't run pickled steps, we just edit/format/validate AST. |
| Pretty formatter (`fspec format`) | ❌ | **WE MUST WRITE THIS** — port from existing TS formatter in `src/utils/gherkin-formatter*.ts` |
| Tokens / source-envelope ndjson | ❌ | not needed for fspec |
| Markdown-with-Gherkin | ❌ | not needed (we don't use `.feature.md`) |
| AST→source roundtrip | ❌ (no `Display` for `Feature`) | covered by our formatter port |

### Maturity signals

- Used in production by `cucumber` crate (≈3M+ downloads on crates.io).
- 0.16.0 release notes (2026-04-23) include BC-break "Fixed precedence of operations in tag expressions to align with upstream" — i.e. they actively chase upstream conformance.
- `forbid(non_ascii_idents, unsafe_code)`, `clippy::allow_attributes` warnings.
- 12 issue/PR cross-references in CHANGELOG since project inception — small surface, low churn.
- Last release ~2 weeks ago. Healthy maintenance cadence (0.13→0.14→0.15→0.16 over ~12 months).

---

## 3. Mapping cucumber-rs AST ↔ fspec's current TS AST

fspec currently uses `@cucumber/gherkin` (`Parser` + `AstBuilder` + `GherkinClassicTokenMatcher`) — found in 20+ command files and `src/utils/feature-parser.ts`. The shape we consume from JS:

```ts
GherkinDocument {
  feature: Feature {
    tags: Tag[],                  // Tag.name = "@foo"
    keyword, name, description,
    children: FeatureChild[]      // union of { background?, scenario?, rule? }
  }
}
Scenario {
  tags, keyword, name, description,
  steps: Step[],                  // Step.text, Step.keyword, Step.dataTable, Step.docString.content
  examples: Examples[]
}
```

### Differences vs cucumber-rs (Rust)

| TS @cucumber/gherkin | Rust cucumber-rs/gherkin | Adapter work |
|---|---|---|
| `Feature.children: (Background\|Scenario\|Rule)[]` | `Feature { background, scenarios, rules }` direct fields | Trivial — Rust shape is actually nicer. fspec consumers iterate by type anyway. |
| `Tag { name: "@foo" }` | `Vec<String>` (already includes leading `@`) | Trivial — drop the wrapper. |
| `Step.text` | `Step.value` | Rename in adapter. |
| `Step.docString.content` | `Step.docstring: Option<String>` | Trivial. |
| `Step.dataTable.rows[].cells[].value` | `Step.table.rows: Vec<Vec<String>>` | Rust shape is flatter / better. |
| `Scenario.keyword == "Scenario Outline"` distinguishes outline | Same — `keyword` carries the raw word; `examples: Vec<Examples>` empty for plain Scenario | Already idiomatic. |
| Pickles (outline expansion) | Not in crate; we don't need them | None. |
| `IdGenerator.uuid()` IDs on every node | No IDs | fspec coverage files reference scenarios by **name**, not ID. Already compatible. |

**Verdict:** The Rust AST is structurally a superset (it includes spans/line-col uniformly, which JS doesn't on every node). The adapter layer in `codelet/fspec-core` will be a small `From<gherkin::Feature> for FspecFeature` with field renames — well under 200 LOC.

---

## 4. The formatter gap (most important takeaway)

`@cucumber/gherkin` itself does not ship a pretty-printer either — fspec already wrote its own AST-based formatter (see `src/utils/gherkin-formatter.ts` and friends, ~600 LOC). That formatter is what powers `fspec format`.

**Action item for the migration:** the formatter port is independent of the parser choice. We can:

1. Walk the cucumber-rs AST in the same order our TS formatter walks the JS AST.
2. Reproduce the same indent/alignment rules (2-space indent for steps, table-cell column alignment, tag-line wrapping at 80 cols, etc.).
3. Reuse the **same fixture inputs** the TS formatter is regression-tested against (`src/utils/__tests__/gherkin-formatter-comment-preservation.test.ts` and similar) by running both formatters and diffing output → byte-identical or fail.

This is a tractable ~1-2 day port and should be tracked as its own child story under RPC-003 (suggested ID: `RPC-006 Port AST-based Gherkin formatter to Rust`).

---

## 5. Conformance test harness (gap to close BEFORE swapping parsers)

The official `cucumber/gherkin` repo ships **282 test data files** under `testdata/{good,bad}/` — for every `.feature` fixture there's a `.feature.ast.ndjson`, `.feature.pickles.ndjson`, `.feature.tokens`, and `.feature.source.ndjson`. All polyglot ports run against this suite to guarantee identical AST output.

**cucumber-rs/gherkin does NOT run this suite.** Their tests live in `tests/cucumber.rs` / `tests/bad.rs` and exercise their own fixtures.

**Risk:** any divergence between cucumber-rs and the official spec will surface as a regression in fspec's `validate` command — feature files that pass today on the JS parser may fail on the Rust parser, or vice versa.

**Mitigation (proposed child story for RPC-003):**

> **RPC-007 — Add Gherkin conformance suite to fspec-core**
> Vendor `cucumber/gherkin/testdata/good/*.feature` (and a curated subset of `bad/`) into `codelet/fspec-core/testdata/`. For each fixture, parse with `gherkin = "0.16"` and assert that the produced AST round-trips and matches the official `*.feature.ast.ndjson` modulo serde-json field ordering. Lock this down in CI before any TS commands switch to the Rust core.

This is the single biggest risk-reduction lever on the whole RPC-003 epic. Doing it early (before NAPI integration) means we discover any cucumber-rs gaps while we still have the option to either upstream a fix or work around it in our adapter layer.

---

## 6. Licensing

| | License | Compatible with fspec (MIT)? |
|---|---|---|
| `gherkin` (cucumber-rs) | MIT OR Apache-2.0 | ✅ |
| `gherkin-languages.json` (vendored) | MIT (cucumber/gherkin) | ✅ — already attributed in cucumber-rs README; we re-attribute. |
| `peg` | MIT | ✅ |
| `textwrap`, `thiserror`, `typed-builder`, `serde` | MIT/Apache-2.0 dual | ✅ |

No GPL/LGPL contamination. Clear path.

---

## 7. Concrete recommendations for RPC-003 breakdown

When this epic gets broken into stories (do **not** start Example Mapping yet, per the umbrella note), the following parser-related stories should appear:

| Suggested ID | Title | Est | Notes |
|---|---|---|---|
| RPC-005 | Add `gherkin = "0.16"` dep + adapter `gherkin::Feature → fspec::Feature` to `codelet/fspec-core` | 3 | Pure plumbing. |
| RPC-006 | Port AST-based Gherkin formatter from TS to Rust | 5 | Behaviour parity required; reuse fixtures. |
| RPC-007 | Vendor official `cucumber/gherkin/testdata` and add conformance harness | 3 | Run BEFORE any production swap. |
| RPC-008 | Re-implement `validate` command on top of Rust parser (NAPI-shim from existing `validate.ts`) | 3 | First user-visible swap. |
| RPC-009 | Re-implement `format` command on top of Rust formatter | 3 | Depends on RPC-006. |
| RPC-010 | Migrate read-side commands (`list-features`, `show-feature`, `get-scenarios`, `tag-stats`, etc.) to Rust core via NAPI | 8 | ~10 commands, all use the same parse-and-traverse pattern. |
| RPC-011 | Migrate write-side commands (`add-scenario`, `update-scenario`, `add-step`, tag mutations, …) | 13 | These need the formatter (RPC-006) since they round-trip. Largest single chunk. |

(Estimates are placeholders pending Example Mapping. Bands chosen to honour the ≤13 ceiling from the foundation guidelines.)

---

## 8. Open questions to resolve before implementation begins

1. **Comment preservation.** `@cucumber/gherkin` strips comments; fspec's formatter has bespoke logic to re-insert them from a side-channel parse (see `gherkin-formatter-comment-preservation.test.ts`). cucumber-rs/gherkin **also** strips comments at the grammar level. Does the round-trip story require us to fork peg's parser to preserve comment tokens? Or can we keep the same "second-pass scan for `# `-comments aligned by line-col" approach we use today? **Suggested: defer to RPC-006; current side-channel approach should work because cucumber-rs gives us `LineCol` on every node.**
2. **Markdown-with-Gherkin.** Upstream supports `.feature.md`. fspec does NOT use it today; do we want to keep it that way for the foreseeable future? Recommend **yes, drop it from scope.**
3. **Pickle generation.** Truly not needed — fspec is a spec-management tool, not a runner. Confirm with stakeholders that no future feature requires pickle compilation.
4. **i18n.** fspec accepts only English keywords today (the `# language: ...` directive is technically supported by the parser but not exercised). cucumber-rs supports all 47 languages. Recommend **passthrough, no extra work**.

---

## 9. Sources / artifacts

- Local clones (will be deleted before commit):
  - `/tmp/gherkin-official` — `cucumber/gherkin@7926c23` (2026-05-07)
  - `/tmp/cucumber-rs-gherkin` — `cucumber-rs/gherkin@aaa7d59` (2026-04-23, v0.16.0)
- crates.io: <https://crates.io/crates/gherkin>
- docs.rs: <https://docs.rs/gherkin/0.16.0>
- Official conformance fixtures: `cucumber/gherkin/testdata/{good,bad}/` (282 files)
- fspec's current usage surface: 20+ command files importing `@cucumber/gherkin` (see grep audit attached separately if needed)
