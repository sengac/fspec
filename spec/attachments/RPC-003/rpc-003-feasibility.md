# RPC-003 — Porting the fspec CLI to Rust (`codelet/fspec-core`): Feasibility & Architecture

> **Status:** Backlog. This document is exploratory. No rules, examples, or
> scenarios have been committed yet — this work unit must be broken down
> through Example Mapping before any implementation begins.
>
> **Pairs with:** `RPC-002` (Distributed Rust Frontend). Together they
> enable an end-to-end Rust fspec without taking the TypeScript stack
> offline at any point.

---

## 1. Goal

Move the **business logic** of every fspec CLI command out of TypeScript
and into a new Rust crate (`codelet/fspec-core`) inside the existing
`codelet/` Cargo workspace. The TypeScript CLI keeps working throughout
the migration by becoming a thin shell over the Rust crate via NAPI.
Once parity is reached, a pure-Rust `fspec` binary becomes possible and
the future ratatui frontend (RPC-002) calls directly into the same
crate — no NAPI hop, no IPC, just function calls.

## 2. Scope of the port (measured)

The TypeScript surface to migrate:

| Bucket | Count | LOC |
|---|---:|---:|
| `*.ts` files in `src/commands/` (excluding `*-help.ts` and `__tests__`) | ~190 | ~33,000 |
| `*-help.ts` static help configs | ~161 | ~3,500 (almost pure data) |
| Direct dependencies in `src/utils/` (file-manager, feature-parser, gherkin-formatter, ensure-files, similarity, temporal validation, …) | ~50 | ~7,000 |
| `src/types/` (TS interfaces → Rust structs) | 7 | ~700 |
| `src/migrations/` (registry + runner + 1 migration) | 5 | ~500 |
| `src/validators/` + `src/schemas/` (Ajv + JSON Schemas) | 6 | ~1,000 |
| `src/hooks/` (executor + conditions + virtual hooks) | ~9 | ~2,500 |
| Top-level (`src/index.ts`, `src/cli/program.ts`, `src/help.ts`, `src/utils/fspec-callback.ts`) | 4 | ~3,000 |
| **Total** | **~430 files** | **~48,200 LOC** |

Test suite that has to be reproduced (or replaced):
- 332 `.test.ts` files relevant to CLI logic, ~109,351 LOC.
- 230 in `src/commands/__tests__/` alone (~82,500 LOC).

The TUI (`src/tui/`, ~50+ React/Ink components, Zustand stores) is
**out of scope here** — it's covered by RPC-002.

## 3. Why this is feasible

### Tailwinds already in place

- `codelet/` is an existing Rust workspace; `codelet-napi` already
  exposes some logic to the TS layer (gitoxide, ast-grep, ghost
  checkpoints). The mechanism for "Rust does the work, TS hosts the
  CLI" is proven.
- The TS test convention is **integration tests against real OS temp
  dirs** (~95% of the suite), not heavy mocking. Rust ports of these
  tests are mechanical.
- Most commands are thin wrappers around `fileManager.transaction`
  (atomic JSON read-modify-write) — a pattern that maps cleanly onto
  Rust async + `tokio::sync::RwLock` + `tempfile::persist`.
- The codebase already has a strong **Gherkin describe/scenario test
  convention with `@step` comments** that survives a language port
  unchanged.

### Headwinds (the real costs)

The cost is concentrated in a small number of components — **most of
the 190 commands are trivial**, but a handful contain almost all the
algorithmic complexity. See §6.

## 4. Dependency mapping (TS → Rust)

| TypeScript dependency | Use site | Rust equivalent | Effort |
|---|---|---|---|
| `@cucumber/gherkin` + `@cucumber/messages` | parser + AST traversal across ~30 commands | `gherkin` crate (official upstream Rust port) | Low — drop-in |
| Custom `GherkinFormatter` (421 LOC) | `fspec format` round-trip output stability | **Hand port** required | Medium — round-trip stability is a tested property |
| `mermaid` + `jsdom` | `add-diagram` semantic validation only | **No clean equivalent** — see §10 | High — outlier; deferred |
| `ajv` + `ajv-formats` | foundation/tags JSON schema validation (4 instantiations) | `jsonschema` crate (draft-07/2019-09/2020-12) | Low — error-shape mapping only |
| `proper-lockfile` | `LockedFileManager` (3-layer locks) | `fs4` crate (advisory locks) + `tokio::sync::RwLock` + `tempfile::persist` | Medium — **lock-file format changes**; do as single cutover |
| `tinyglobby` | ~28 sites, all `glob(['spec/features/**/*.feature'])` | `globwalk` or `ignore::WalkBuilder` | Trivial |
| `execa` | hooks executor, 2x git-context calls | `tokio::process::Command` + `select!` for timeout | Low |
| `chokidar` | one site (BoardView checkpoint refresh) — TUI-only | not needed for CLI port | n/a |
| `commander` | 1 setup + ~190 `register*Command` files | `clap` v4 derive | Medium — volume |
| `marked` + `marked-gfm-heading-id` | attachment HTTP server (out of scope) | `pulldown-cmark` if needed | Low |
| `winston` | one logger singleton → `~/.fspec/fspec.log` | `tracing` + `tracing-appender` | Trivial |
| Node `https` | research-tools (perplexity, jira, confluence) | `reqwest` (or `ureq` sync) | Low |
| `cron-validate` | schedule validation | `cron` crate | Trivial |
| `open` | `report-bug-to-github`, OAuth flows | `open` crate | Trivial |
| `string-width` | TUI widths — not CLI port | n/a here | n/a |
| `chalk` | every command | `nu-ansi-term` / `owo-colors` | Trivial |

## 5. Persistence layer inventory (the contract surface)

The crate's data model has to faithfully reproduce **15 distinct JSON
file shapes** under `spec/`:

| File | Type | Schema | Wrapped in `fileManager.transaction`? |
|---|---|---|---|
| `spec/work-units.json` | `WorkUnitsData` | (none — code-validated) | ✅ all writers |
| `spec/epics.json` | `EpicsData` | none | ✅ |
| `spec/prefixes.json` | `PrefixesData` | none | ✅ |
| `spec/tags.json` | `Tags` | `tags.schema.json` | ✅ |
| `spec/foundation.json` | `GenericFoundation` | `generic-foundation.schema.json` | ⚠️ 3 known direct-write gaps |
| `spec/foundation.json.draft` | partial | same | ⚠️ same gaps |
| `spec/schedules.json` | `SchedulesData` | `schedule.schema.json` | ✅ |
| `spec/fspec-config.json` (+ `~/.fspec/`) | untyped | none | ❌ direct write |
| `spec/fspec-hooks.json` | `HookConfig` | none | ✅ |
| `spec/features/*.feature` | Gherkin text | n/a | direct |
| `spec/features/*.feature.coverage` | `CoverageFile` | none | ⚠️ partial — auto-create writes raw |
| `spec/attachments/<id>/*` | filesystem | n/a | direct (index inside work-units.json) |
| `<tmp>/fspec-reverse-*.json` | `ReverseSession` | none | direct (tmp file) |
| `.git/fspec-checkpoints-index/<id>.json` | `CheckpointIndex` | none | written by Rust NAPI today |
| `spec/work-units.json.backup-*` | snapshot | none | direct (migration runner) |

Critical invariants the Rust crate must preserve:
- **Stable IDs with soft-delete** (`deleted: true`) for rules / examples
  / questions / architecture-notes. Auto-compact only on `done`
  transition.
- **`migrationHistory[]`** is append-only with backup-path references.
- **`prefixCounters`** monotonic, never reset.
- **`stateHistory[]`** append-only with timestamps used for temporal
  validation (see §6).

## 6. Complexity ranking — top files by algorithmic weight

| Rank | File | LOC | What's tricky |
|---|---|---:|---|
| 1 | `update-work-unit-status.ts` | 1,404 | 7-state Kanban FSM with ~12 cascading guards (transition table, type-specific variants, prefill detection, blocker check, temporal ordering, scenario+coverage prerequisites, `@step` docstring validation, parent/child completion, virtual+global hooks, auto-checkpoint, auto-compact, sorted insertion). |
| 2 | `dependencies.ts` | 1,100 | BFS cycle detection scoped per relationship type, **bidirectional consistency repair** (blocks↔blockedBy, relatesTo↔relatesTo), critical-path DFS+memo, transitive impact BFS, mermaid/JSON export. |
| 3 | `work-unit.ts` | 1,060 | `getNextWorkUnitId` prefix scanning, parent/child reparenting, epic moves, schema validation, state-array reconciliation, prioritization (top/bottom/before/after), board rendering. |
| 4 | `discover-foundation.ts` | 859 | Draft-driven file-state wizard with `[QUESTION:]`/`[DETECTED:]` placeholder scanning, manual-edit detection with **automatic revert**, Ajv error formatting, cascading FOUND-* work-unit creation. |
| 5 | `reverse.ts` | 687 | 4-strategy reverse-ACDD orchestration; gap analysis (spec/test/coverage); session persistence in tmp-file; AST-driven feature suggestion. |
| 6 | `generate-scenarios.ts` | 673 | Reads Example Mapping, categorizes architecture notes, **uses Gherkin AST to modify existing files**, scenario similarity matching for duplicate detection, regenerates coverage. |
| 7 | `review.ts` | 587 | ACDD compliance checks; emits structured system-reminders for AI deep-review. |
| 8 | `example-mapping.ts` | 541 | All add/remove/restore for rules/examples/questions/assumptions/architecture-notes; soft-delete; `answer-question` auto-promotion to rule/example. |
| 9 | `show-coverage.ts` | 511 | Single-file & all-files renderer (markdown/JSON), stale-scenario detection, statistics. |
| 10 | `show-work-unit.ts` | 475 | Composite renderer over Example Mapping + dependencies + virtual hooks + attachments + state history. |
| 11 | `report-bug-to-github.ts` | 415 | System context gathering + URL-encoded GitHub issue URL + browser launch (no HTTP). |
| 12 | `research.ts` | 408 | Plugin dispatcher (bundled + custom in `spec/research-scripts/`), config validation, `spawn`. |
| 13 | `init.ts` | 385 | spec/ tree creation, default JSON templates, hook scripts. |
| 14 | `query.ts` | 376 | Multi-criteria querying with JSON/markdown/table output. |
| 15 | `delete-scenarios-by-tag.ts` | 350 | Tag-based bulk delete with dry-run, Gherkin re-parse, coverage update. |

The remaining ~175 commands are short (50–200 LOC) and follow the
same "load JSON → mutate → save in a transaction" pattern.

## 7. Proposed crate layout

All new crates live in the existing `codelet/` workspace alongside
`napi/`, `cli/`, `core/`, `tools/`, etc.:

```
codelet/
├── fspec-types/      # NEW — pure-serde types (WorkUnit, Epic, RuleItem,
│                     #       GenericFoundation, Tags, SchedulesData, …)
│                     #       Mirrors src/types/. Zero deps beyond serde.
├── fspec-storage/    # NEW — LockedFileManager port (3-layer locking,
│                     #       atomic write-replace), ensure-files,
│                     #       migrations runner + registry.
├── fspec-gherkin/    # NEW — wrapper over `gherkin` crate + the custom
│                     #       formatter (the part that needed hand-porting).
│                     #       Coverage file reader/writer also lives here.
├── fspec-validators/ # NEW — JSON Schema validation (jsonschema crate),
│                     #       prefill detection, temporal validation,
│                     #       review validation, scenario similarity.
├── fspec-hooks/      # NEW — HookConfig loader, executor (tokio::process),
│                     #       discovery, conditions, virtual hook script
│                     #       generation, formatting (system-reminder
│                     #       wrapping for blocking failures).
├── fspec-core/       # NEW — THE BUSINESS LOGIC. One module per command
│                     #       group: work_unit/, example_mapping/,
│                     #       dependencies/, foundation/, gherkin_ops/,
│                     #       coverage/, schedule/, attachments/, query/,
│                     #       reverse/, init/, etc. Each module exposes
│                     #       typed `async fn` entry points returning
│                     #       structured Result<R, E>. NO `process::exit`.
│                     #       NO println!/eprintln! — output via OutputSink.
├── fspec-cli/        # NEW — clap-derive front-end. Each command is a
│                     #       small file calling into fspec-core. Help
│                     #       configs are static `const` data (replacing
│                     #       the *-help.ts files).
├── fspec-bin/        # NEW — pure-Rust `fspec` binary. Just main() →
│                     #       fspec-cli. Replaces dist/index.js once
│                     #       parity is reached.
├── napi/             # UNCHANGED for now — gradually grows facade
│                     #       functions delegating to fspec-core, so the
│                     #       TS CLI keeps working byte-identically.
└── cli/, core/, tools/, providers/, git/, common/, tui/   # UNCHANGED
```

### Boundaries

- **`fspec-types`** has zero behaviour. Just structs + serde.
- **`fspec-storage`** owns `LockedFileManager`. Pinned to `fspec-types`.
- **`fspec-gherkin`** depends on the `gherkin` crate plus
  `fspec-storage` for coverage I/O.
- **`fspec-validators`** is pure functions — JSON in, errors out. No
  filesystem coupling beyond reading schemas at compile time
  (`include_str!`).
- **`fspec-hooks`** depends on `fspec-types` + `fspec-storage`.
  `tokio::process::Command` for execution.
- **`fspec-core`** is where the 190 commands live. Each `pub async fn`
  takes a typed input struct + an `&Context` (cwd, output sink,
  optional cancellation token) and returns a typed result.
- **`fspec-cli`** never reaches into `fspec-storage` or `fspec-types`
  except through `fspec-core`.
- **`napi/`** grows facades that delegate to `fspec-core`.

### Why this split

- The same `fspec-core::*` functions are called from **three**
  consumers: the TS CLI (via NAPI), the future pure-Rust CLI, and the
  ratatui frontend (RPC-002, embedded mode). One source of truth.
- A separate `fspec-bin` keeps the binary lean (no NAPI symbols).
- `fspec-types` being its own crate prevents accidental coupling — and
  is what RPC-002's tarpc trait will reuse (the same `WorkUnit`,
  `RuleItem`, etc. become wire types).

## 8. Output and exit-code model

The TS code ends almost every command with `process.exit(N)`. The Rust
port replaces that with **explicit return values** at the `fspec-core`
boundary:

```rust
pub struct CommandResult<T> {
    pub data: T,
    pub exit_code: i32,
    pub system_reminders: Vec<SystemReminder>,
}

pub trait OutputSink: Send {
    fn log(&mut self, msg: &str);
    fn error(&mut self, msg: &str);
    fn json<T: Serialize>(&mut self, value: &T);
}
```

- `fspec-core` functions never call `std::process::exit`.
- They never write to stdout/stderr directly. They write to an
  injected `&mut dyn OutputSink`.
- `fspec-cli` provides a `TerminalOutputSink` that goes to stdout;
  tests provide a `BufferedOutputSink` capturing into `Vec<String>`.
- The in-process invocation entry point (equivalent to today's
  `src/utils/fspec-callback.ts`) returns the captured output without
  touching the process exit code.

This eliminates the existing `__FSPEC_EXIT_OVERRIDE__:N` hack used by
the current TS callback to recover exit codes from in-process
invocations.

## 9. The fileManager contract — Rust port detail

The current `LockedFileManager` (391 LOC) gives:

1. **Layer 1** — inter-process advisory file lock via `proper-lockfile`
   (which uses a `<file>.lock/` *directory*, not OS-level locks).
2. **Layer 2** — in-process readers-writer lock via Promise queues.
3. **Layer 3** — atomic write-replace via `<file>.tmp.<uuid>` + rename.

Rust port options for Layer 1:

- **Option A: Keep `proper-lockfile` semantics.** Reimplement the
  lock-directory protocol in Rust so old/new fspec instances interlock
  during the migration period. Custom code, ~50 LOC.
- **Option B: Switch to OS advisory locks (`fs4`).** Cleaner, but **not
  compatible** with the lock-directory protocol — old TS instances
  would not see the new locks. Must be a single cutover.
- **Recommendation: Option A** until parity is reached, then a final
  cutover to Option B once the TS CLI is retired.

Layer 2 maps onto `tokio::sync::RwLock<()>` per-path
(`DashMap<PathBuf, Arc<RwLock<()>>>`).

Layer 3 maps onto `tempfile::NamedTempFile::persist`.

The `transaction(path, fn)` mutator API translates straightforwardly:

```rust
pub async fn transaction<T, F, Fut>(path: &Path, mutator: F) -> Result<()>
where
    T: DeserializeOwned + Serialize + Default,
    F: FnOnce(&mut T) -> Fut,
    Fut: Future<Output = Result<()>>;
```

Tests of the existing TS file-manager (in `src/utils/__tests__/`) port
1:1.

## 10. The mermaid problem

`add-diagram` validates Mermaid syntax by **running the
browser-targeted `mermaid` library inside JSDOM** (with manual
`SVGElement.getBBox` polyfills). There is no equivalent Rust crate
covering the full Mermaid grammar.

Three options, deferred to a later work unit:

1. **Drop semantic validation.** Keep only the regex pre-checks
   already in `mermaid-validation.ts`. Acceptable if Mermaid usage is
   limited.
2. **Sidecar Node process.** Shell out to a single
   `node mermaid-validate.js` script for diagram validation only. Adds
   a Node dependency to one command.
3. **Hand-written subset validator.** Cover the diagram types fspec
   actually uses (probably just `graph TD` / `flowchart` / `sequence`).
   Acceptable if scoped narrowly.

The architecture document for that work unit must enumerate the
Mermaid diagram types currently used in the codebase to inform the
choice.

## 11. Commander → clap mapping

Per-command pattern today:

```ts
program
  .command('add-rule')
  .description('Add a business rule …')
  .argument('<workUnitId>', 'Work unit ID')
  .argument('<rule>', 'Business rule description')
  .action(async (workUnitId, rule) => { … });
```

Becomes:

```rust
#[derive(clap::Args)]
pub struct AddRuleArgs {
    /// Work unit ID
    pub work_unit_id: String,
    /// Business rule description
    pub rule: String,
}

pub async fn run(args: AddRuleArgs, ctx: &Context) -> Result<i32> {
    let result = fspec_core::example_mapping::add_rule(
        &fspec_core::example_mapping::AddRule {
            work_unit_id: args.work_unit_id,
            rule: args.rule,
        },
        ctx,
    ).await?;
    ctx.output().log("Rule added successfully");
    Ok(if result.success { 0 } else { 1 })
}
```

`*-help.ts` files become `const HELP: HelpConfig = HelpConfig { … }`
items in the same module, registered in a static `phf` map at compile
time (replacing the Vite `import.meta.glob` build-time discovery).
## 12. Test strategy

Match the TS strategy 1:1 for portability of intent:

- **OS-level temp dirs** (`tempfile::TempDir`, drop-based cleanup
  matching TS `cleanup()`).
- **Function-level integration tests, NOT process-level.** Tests call
  `fspec_core::example_mapping::add_rule(input, &ctx)` with a
  `BufferedOutputSink`. Verify return value + filesystem state. This
  matches ~95% of the TS suite.
- **Builder helpers** in a `fspec-test-helpers/` crate replicating
  `setupWorkUnitTest` / `setupFoundationTest` / `setupFullTest` /
  `createTestFiles({path: data})`.
- **Gherkin-style test naming**: prefer `cucumber-rs` for tests where
  the BDD structure is meaningful (state machine tests especially);
  for the rest, modules + `#[test] fn feature_X__scenario_Y__should_Z()`.
- **`@step` comment convention** survives unchanged — Rust line
  comments work the same way for `link-coverage`.
- **Hook tests** use real subprocess execution against bash scripts in
  tmp dirs (matching today).
- **Skip TUI/components/server tests** — out of scope for this epic.

## 13. Migration plan (incremental)

The TypeScript stack must work the entire time. Pure-Rust binary is
the *last* deliverable, not the first.

**Phase 0 — Spike (~1 week).** `fspec-types` + `fspec-storage` + 5
ported commands (`add-rule`, `add-example`, `remove-rule`,
`set-user-story`, `list-prefixes`). NAPI facade exposes them. The TS
CLI for those 5 commands becomes a one-line shim. Validates the whole
pipeline end-to-end.

**Phase 1 — `fspec-types` + `fspec-storage` complete.** All structs,
all 7 ensure-files variants, full `LockedFileManager`, migration
runner + registry, all soft-delete invariants. Test parity with
`src/utils/__tests__/`.

**Phase 2 — `fspec-gherkin` + `fspec-validators`.** Includes the
hand-ported formatter (this is where round-trip stability matters).
JSON schema validation for foundation/tags/schedules.

**Phase 3 — `fspec-hooks`.** Full hooks system + virtual hooks +
git-context script generation. Tests run real bash scripts.

**Phase 4 — `fspec-core` for the simple commands.** ~150 thin-wrapper
commands ported in mechanical batches (10–20 per work unit). Each
batch ships behind NAPI; TS CLI commands become one-line shims as the
ports land.

**Phase 5 — `fspec-core` for the heavy hitters.** Each top-15 command
is its own work unit:
- `dependencies.ts` (graph algorithms) — ~3-5 days
- `update-work-unit-status.ts` (state machine) — ~5–8 days. Largest
  single port.
- `work-unit.ts`
- `generate-scenarios.ts` (depends on `fspec-gherkin` from Phase 2)
- `discover-foundation.ts`
- `reverse.ts`
- `review.ts`, `report-bug-to-github.ts`, `research.ts`, `init.ts`,
  `query.ts`, `show-coverage.ts`, `show-work-unit.ts`,
  `delete-scenarios-by-tag.ts`, `example-mapping.ts` — one work unit
  each or grouped where they share helpers

**Phase 6 — `fspec-cli` + `fspec-bin` (pure Rust).** Build a Rust
`fspec` binary that does NOT go through Node. Side-by-side with the
TS CLI behind a CLI flag (`--engine=rust`) until parity is verified.

**Phase 7 — Cutover.** TS CLI becomes a deprecation shim that prints a
warning + invokes the Rust binary. Eventually deleted.

## 14. Effort estimate (rough, pre-Example-Mapping)

| Phase | Risk | Order-of-magnitude |
|---|---|---|
| 0. Spike (5 commands end-to-end) | Low | 1 week |
| 1. `fspec-types` + `fspec-storage` | Low–Medium | 2 weeks |
| 2. `fspec-gherkin` + `fspec-validators` | Medium (formatter parity) | 2 weeks |
| 3. `fspec-hooks` | Low–Medium | 1 week |
| 4. ~150 thin-wrapper commands | Low (mechanical, but volume) | 4–6 weeks |
| 5. Top-15 algorithmic commands | Medium–High | 6–10 weeks |
| 6. `fspec-cli` + `fspec-bin` parity | Medium | 2 weeks |
| 7. Cutover & deprecation | Low | 1 week |
| **Total** | — | **~20–25 weeks of focused work** |

The bottleneck is **Phase 5**: porting `update-work-unit-status.ts`
faithfully (with all 12 guards in the right order) and porting the
graph algorithms in `dependencies.ts`. Everything else is volume, not
difficulty.

## 15. Risks & open questions (Example Mapping fodder)

1. **Lock-file format compatibility.** During the parallel period, do
   we keep the lock-directory protocol or break it? (See §9.)
2. **Mermaid validation.** Drop / sidecar / port? (See §10.)
3. **Error message stability.** Do existing `expect(error.message)`
   tests in TS need byte-identical error strings from Rust? (Probably
   not — but agent prompts may match on substrings.)
4. **In-process invocation semantics.** The current callback captures
   stdout via `output` context. Rust replacement should be
   stronger-typed but must avoid breaking the codelet-tools agent
   integration.
5. **Help-config build-time discovery.** The Vite eager-glob becomes a
   `build.rs` script + generated `phf::Map`, OR `inventory` /
   `linkme` for distributed registration. Pick early.
6. **Cron expression validation.** TS uses `cron-validate` which is
   permissive about syntax variants. Pick a Rust crate that matches
   (`cron` vs `croner` — note the workspace already uses `croner`).
7. **Distribution.** Pure-Rust `fspec` binary breaks the `npm install`
   story. Use `cargo install fspec`? Pre-built signed binaries on
   each platform? Reuse the Node SEA packaging only for the duration
   of Phase 7?
8. **Migration runner backups.** The TS `migrationHistory[]` carries
   `backupPath` strings — Rust must read/write them in the exact
   same shape so a TS-CLI can roll back a Rust-CLI-applied migration
   and vice versa.
9. **Network research tools.** Perplexity / Jira / Confluence go via
   Node `https`. Rust port to `reqwest` — but rustls vs
   native-tls choice (codelet workspace already uses
   `rustls-tls-webpki-roots` for `tokio-tungstenite`).
10. **Stable IDs across language ports.** Auto-incrementing counters
    must be preserved exactly — no off-by-one regressions.

## 16. What this work unit explicitly does NOT do

- It does not delete `src/commands/` until Phase 7.
- It does not change the JSON file shapes on disk.
- It does not change the CLI surface (commands, flags, exit codes,
  output text — all stable).
- It does not change the test naming convention (Gherkin describe /
  scenario, `@step` comments).
- It does not commit to a Mermaid validation strategy.
- It does not commit to a Rust binary distribution mechanism.
- It does not require RPC-002 (the ratatui frontend) to ship first —
  but ratatui-mode-embedded becomes much more attractive once
  `fspec-core` exists.

## 17. Reference material checked while writing this

- `src/commands/` — 338 .ts files; 48,204 LOC.
- `src/cli/program.ts` — 365 lines, ~140 register-command calls,
  static-imported.
- `src/types/index.ts` (221 lines) — `WorkUnit`, `WorkUnitsData`,
  `Epic`, `Prefix`, all `EventStorm*`, `RuleItem` / `ExampleItem` /
  etc.
- `src/utils/file-manager.ts` (391 lines) — `LockedFileManager`,
  three-layer locking, `transaction()` API.
- `src/utils/ensure-files.ts` — 7 ensure functions covering
  work-units / epics / prefixes / tags / schedules / hooks plus the
  draft & finalized variants of the project bootstrap doc.
- `src/migrations/registry.ts` — `CURRENT_VERSION = "0.7.1"`, 1
  migration registered (001-stable-indices).
- `src/schemas/` — three JSON schemas (generic project bootstrap,
  tags, schedule).
- `src/validators/` — three validator modules built around Ajv.
- `src/hooks/` — executor / conditions / discovery / config / types /
  git-context / script-generation / formatting / command-utils.
- `src/commands/update-work-unit-status.ts` (1,404 lines) — state
  machine cascade.
- `src/commands/dependencies.ts` (1,100 lines) — graph algorithms.
- `src/commands/work-unit.ts` (1,060 lines) — core CRUD + display.
- `src/commands/discover-foundation.ts` (859 lines) — draft FSM.
- `src/commands/reverse.ts` (687 lines) — reverse-ACDD orchestration.
- `src/commands/generate-scenarios.ts` (673 lines) — Gherkin codegen.
- `package.json` — npm dependency manifest.
- `codelet/Cargo.toml` — Rust workspace, confirmed deps already
  pulled in: `tokio`, `serde`, `tracing`, `reqwest`,
  `tokio-tungstenite`, `croner`, `globset`, `ignore`, `ast-grep-core`,
  `ast-grep-language`.
