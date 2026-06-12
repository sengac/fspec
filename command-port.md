# Command Port Reference (TS → Rust)

Authoritative playbook for porting fspec CLI commands from `src/commands/<name>.ts`
(TypeScript / Commander.js) to the Rust workspace at `codelet/`. Distilled from
RPC-241..RPC-253 (the 12 `list-*` commands) and the RPC-327 dispatcher bug fix.

**Reference port: `list-prefixes` (RPC-248).** When in doubt, read its files
top-to-bottom and copy the shape.

---

## 1. Per-command file inventory (every port produces these 6 artifacts)

| # | Path | Role |
|---|------|------|
| 1 | `codelet/fspec-core/src/commands/<snake>.rs` | **Single source of truth.** `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` |
| 2 | `codelet/fspec/src/<snake>.rs` | **CLI bridge** — thin façade: `pub struct CliArgs { … }` + `pub async fn run(args: CliArgs) -> anyhow::Result<u8>` |
| 3 | `codelet/fspec/src/main.rs` | **clap variant** in `enum Mode` + match arm in `fn main` using the `forward!` macro |
| 4 | `codelet/fspec-core/src/help/configs/<snake>.rs` | **Help config** — `pub const CONFIG: CommandHelpConfig = …` |
| 5 | `codelet/fspec/tests/fixtures/help/<name>.txt` | **Byte-exact help fixture** — captured from `node dist/index.js <cmd> --help` piped to non-TTY |
| 6 | `codelet/fspec/tests/cli_<snake>.rs` | **Integration test** — one `#[test]` per Gherkin scenario, `@step` comments verbatim |

Typical sizes — core impl 200–440 LOC, CLI bridge 90–170 LOC, integration test 300–530 LOC.

---

## 2. Shared infrastructure under `codelet/fspec-core/src/`

```
codelet/fspec-core/src/
├── canonical.rs          # CANONICAL_COMMANDS + PORTED_COMMANDS + is_ported()
├── dispatch.rs           # dispatch_command() + run_ported() + run_stub() + poll_sync_future()
├── error.rs              # FspecCoreError { InvalidArgs, Io, ParseJson, NotYetPorted, … }
├── commands/             # one module per command; mod.rs registers all
├── help/
│   ├── mod.rs            # CommandHelpConfig + format_command_help()
│   └── configs/          # one CONFIG const per command + mod.rs
├── io/
│   ├── ensure.rs         # ensure_*_file + read_*_or_empty (file IO with load-or-init)
│   ├── feature_glob.rs   # glob_feature_files() — recursive spec/features/** walk
│   ├── locked_file.rs    # read_or_init_json + write_json_atomic (fs2 exclusive lock)
│   └── project_root.rs   # find_or_create_spec_directory()
└── types/
    ├── work_unit.rs      # WorkUnit / WorkUnitsData / WorkUnitStatus / WorkUnitType / Meta + states
    ├── prefix.rs         # Prefix / PrefixesData
    ├── epic.rs           # Epic / EpicsData
    └── tags.rs           # TagsData / TagCategory / Tag
```

### Golden rule for shared types

Every on-disk struct uses `#[serde(flatten)] pub extra: serde_json::Map<String, Value>`
to preserve unknown fields across a load → modify → save round-trip. TS interfaces are
runtime-unchecked; Rust must not drop fields like `migrationHistory`, `prefixCounters`,
or per-work-unit `virtualHooks` / `attachments` arrays.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkUnit {
    pub id: String,
    pub title: String,
    pub status: WorkUnitStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epic: Option<String>,
    // …typed known fields…
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
```

Per-unit ad-hoc fields (`virtualHooks`, `attachments`) are read out of `extra`
as `serde_json::Value` — they are NOT promoted to typed fields unless a command
needs structural access.

### IndexMap, not HashMap

`work_units: IndexMap<String, WorkUnit>` — preserves JSON insertion order. TS
`Object.entries(obj)` honours object-literal insertion order; `BTreeMap` would
alphabetize and break parity.

---

## 3. The Two-Front-Doors Pattern (RPC-003 §7/§11)

```
                  ┌──────────────────────────────────────────────────┐
                  │  fspec_core::commands::<name>::run               │
                  │    (args_json: &str, project_root: &Path)        │
                  │    → Result<String, FspecCoreError>              │
                  │    SINGLE SOURCE OF TRUTH                        │
                  └─────────▲──────────────────────────▲─────────────┘
                            │                          │
            ┌───────────────┘                          └────────────────┐
            │                                                           │
   FRONT DOOR #1 — Shell argv                       FRONT DOOR #2 — LLM tool call
   $ fspec list-prefixes                            {command, args:{...}}
            │                                                           │
   codelet/fspec/src/main.rs                        codelet/fspec-core/src/
   `fn main()` → Cli::parse()                        dispatch::dispatch_command
   → matches Mode::<Name> variant                   → canonical::lookup
            │                                       → canonical::is_ported
   codelet/fspec/src/<name>.rs::run(CliArgs)        → run_ported() match arm
   - project_root = env::current_dir()                                  │
   - marshal CliArgs → serde_json::Map               (passes args_json verbatim)
   - call fspec_core::commands::<name>::run(json, root)
            │                                                           │
            └─────────────────┬─────────────────────────────────────────┘
                              ▼
                       common implementation
```

**Both paths converge** on the same `async fn run`. The CLI bridge re-encodes
clap fields back to JSON (omitting `None` so serde `#[serde(default)]` arms
fire) instead of calling sibling Rust functions, because the dispatcher
already has `args_json` in JSON form. One marshalling format = one validator
implementation.

### Sync dispatch under async runtime — `poll_sync_future`

The dispatcher is called from inside the agent loop's outer `#[tokio::main]`
runtime. A nested `tokio::runtime::Runtime::block_on(...)` either panics
("Cannot start a runtime from within a runtime") or dead-locks. This was the
RPC-327 bug.

`dispatch.rs::poll_sync_future` polls the future once with `Waker::noop()`. All
ported `commands::*::run` functions touch only `std::fs` / `serde_json` and
resolve on first poll. A real `.await` returning `Pending` is caught loudly:

```rust
match Pin::as_mut(&mut future).poll(&mut cx) {
    Poll::Ready(v) => v,
    Poll::Pending => Err(FspecCoreError::InvalidArgs { command: "dispatch",
        reason: "ported command future returned Pending under sync dispatch — \
                 introduce a real async runtime or make the command sync".into() }),
}
```

**Do not introduce a real `.await` on tokio resources in command implementations.**
If you must, you need a different dispatch architecture (out of scope for list-* ports).

---

## 4. Help text intercept (byte-parity with TS Commander.js)

Help is intercepted in `codelet/fspec/src/main.rs::intercept_ts_help()` **before
clap parses argv**. clap's auto-generated `--help` block would diverge from the
TS Commander.js format — the TS rich `formatCommandHelp(config)` output is the
contract.

### Sequence

1. `fn main()` calls `intercept_ts_help()` first.
2. Helper inspects `std::env::args()`; if `argv[2..]` contains `--help` or `-h`,
   it matches `argv[1]` against the per-command arm, calls
   `format_command_help(&configs::<snake>::CONFIG)`, `println!`s it, returns `Some(0)`.
3. Otherwise returns `None`, and clap continues normally.

### `CommandHelpConfig` shape (`help/mod.rs`)

```rust
pub struct CommandHelpConfig {
    pub name: &'static str,                          // lowercased; rendered as uppercase header
    pub description: &'static str,
    pub usage: Option<&'static str>,                 // default "fspec <name>"
    pub arguments: &'static [CommandArgument],
    pub options: &'static [CommandOption],
    pub examples: &'static [CommandExample],
    pub related_commands: &'static [&'static str],
    pub when_to_use: Option<&'static str>,
    pub when_not_to_use: Option<&'static str>,
    pub prerequisites: &'static [&'static str],
    pub common_patterns: &'static [CommonPatternEntry],
    pub typical_workflow: Option<&'static str>,
    pub common_errors: &'static [CommonError],
    pub notes: &'static [&'static str],
}
```

### Section render order (mirrors `src/utils/help-formatter.ts:44-187`)

Header → WHEN TO USE → WHEN NOT TO USE → PREREQUISITES → USAGE → ARGUMENTS →
OPTIONS → COMMON PATTERNS → TYPICAL WORKFLOW → EXAMPLES → COMMON ERRORS →
RELATED COMMANDS → NOTES.

### chalk wrappers are identity

TS uses `chalk.bold`, `chalk.cyan`, etc. — these reduce to identity when stdout
is non-TTY. The byte-parity contract is defined against piped/captured TS output,
so Rust mirrors the non-colour path only.

### Special case: bare Commander.js help

`list-foundation-sections` has NO `-help.ts` in TS → bare Commander.js Usage/
Description/Options block. Rust hard-codes that as `LIST_FOUNDATION_SECTIONS_HELP:
&str` in `main.rs` and special-cases its intercept arm to `print!` (no double newline).

---

## 5. The `forward!` macro and clap variant pattern

In `codelet/fspec/src/main.rs::main()` after `Cli::parse()`:

```rust
macro_rules! forward {
    ($bridge:path, $args:expr) => {
        match $bridge($args).await {
            Ok(code) => return std::process::ExitCode::from(code),
            Err(err) => { eprintln!("{err:#}"); return std::process::ExitCode::from(1); }
        }
    };
}

let res = match cli.cmd {
    None => combined::run(cli.workspace).await,
    Some(Mode::ListPrefixes) => forward!(list_prefixes::run, list_prefixes::CliArgs::default()),
    Some(Mode::ListWorkUnits { status, prefix, epic, r#type, format }) => forward!(
        list_work_units::run,
        list_work_units::CliArgs { status, prefix, epic, r#type, format: Some(format) }
    ),
    // …
};
```

### `Mode::` variant rules

- Use `#[command(name = "list-foo", about = "...")]`.
- Positional arguments: `#[arg(value_name = "WORK_UNIT_ID")] work_unit_id: String`.
- Optional flags: `#[arg(short, long, value_name = "STATUS")] status: Option<String>`.
- Required vs Optional matches TS: positional `<workUnitId>` is required (`String`),
  optional `[--status]` becomes `Option<String>`.
- Help text in `#[command(about=...)]` is **ignored** at runtime because the intercept
  short-circuits clap, but keep it parity-correct for `cargo doc`.

---

## 6. Core implementation template (`codelet/fspec-core/src/commands/<snake>.rs`)

```rust
//! `<name>` — Rust port of `src/commands/<name>.ts` (RPC-XXX).
//!
//! [Brief: what it reads, what it filters, what it emits.]
//! Both invocation paths (LLM dispatcher AND standalone CLI) call this single
//! function — RPC-003 §7/§11 two-front-doors invariant.

use std::path::Path;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::{ /* relevant helpers */ };
use crate::types::work_unit::WorkUnit;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Args {
    #[serde(default)]
    status: Option<String>,
    // …mirror TS flag names exactly. Use `#[serde(rename = "type")] r#type` for
    // reserved keywords. Use `#[serde(default, rename_all = "camelCase")]` at
    // the struct level so the dispatcher's camelCase JSON parses verbatim.
    #[serde(default)]
    format: Option<String>,
}

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: Args = serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
        command: "<name>",
        reason: format!("failed to parse args: {e}"),
    })?;

    // Load state via shared helpers (auto-create OR read-or-empty depending on TS parity).
    let data = /* ensure_… or read_…_or_empty(project_root)? */;

    let summaries = filter_and_summarize(&data, &args);
    let result = json!({ "items": summaries });

    match args.format.as_deref() {
        Some("json") => serde_json::to_string_pretty(&result).map_err(|e| {
            FspecCoreError::InvalidArgs { command: "<name>",
                reason: format!("failed to serialize result: {e}") }
        }),
        _ => Ok(render_text(&summaries)),
    }
}

fn filter_and_summarize(/* … */) -> Vec<Value> { /* … */ }
fn render_text(/* … */) -> String { /* … */ }

#[cfg(test)] mod tests { /* arg parsing, filter helpers, empty-result sentinel */ }
```

### Key invariants

- **Signature is non-negotiable**: `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`.
- **Never call `env::current_dir()` here** — project_root is passed in. One
  binary can serve multiple sessions with different working directories.
- **Errors via `FspecCoreError`**, never `anyhow` (the CLI bridge owns `anyhow`).
- **Output is a String** — JSON pretty-printed for `format=json`, plain text for default.
- **Insertion order matters**: when emitting JSON, prefer `#[derive(Serialize)]`
  structs over `json!{}` macros because `serde_json::Map` in `json!{}` is a
  `BTreeMap` and will alphabetize fields.

---

## 7. CLI bridge template (`codelet/fspec/src/<snake>.rs`)

```rust
//! `<name>` shell-facing CLI bridge (RPC-XXX).
//!
//! Two-front-doors pattern:
//!   - Shell argv         → clap → this module → fspec_core::commands::<name>::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::<name>::run

use std::env;
use std::path::PathBuf;
use anyhow::{Context, Result};
use codelet_fspec_core::commands::<name>;
use serde_json::{json, Value};

#[derive(Debug, Default)]
pub struct CliArgs {
    pub status: Option<String>,
    // …mirror clap variant fields…
}

pub async fn run(args: CliArgs) -> Result<u8> {
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Marshal CliArgs → JSON, omitting None so #[serde(default)] arms fire.
    let mut obj = serde_json::Map::new();
    if let Some(v) = args.status.as_ref() { obj.insert("status".into(), Value::String(v.clone())); }
    // …repeat for each field…
    let args_json = json!(obj).to_string();

    match <name>::run(&args_json, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') { println!(); }  // shell-friendly newline
            Ok(0)
        }
        Err(err) => {
            eprintln!("Error: {err}");                    // mirrors TS chalk.red('Error:', ...)
            Ok(1)
        }
    }
}
```

### Bridge MUST stay thin

The bridge is JSON marshalling only. No filter logic, no rendering logic, no
file IO except `env::current_dir()`. A peer reviewer should be able to skim it
in 30 seconds. The two-front-doors integration test
(`scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher`) enforces
this by comparing dispatcher JSON to CLI JSON.

---

## 8. Test infrastructure

### Shared helpers — `codelet/fspec/tests/common/mod.rs`

```rust
pub fn fspec_bin() -> &'static str { env!("CARGO_BIN_EXE_fspec") }  // cargo builds binary before tests
pub fn project_root() -> PathBuf       // walks CARGO_MANIFEST_DIR up to repo root
pub fn codelet_root() -> PathBuf
pub fn fspec_crate_root() -> PathBuf
pub fn make_workspace(units: &[(&str, &str, &str)]) -> (TempDir, PathBuf)
pub struct ChildGuard(pub Child);     // RAII drop = kill+wait, prevents orphan daemons
pub fn spawn_fspec_daemon(workspace: &Path) -> (ChildGuard, u16)
pub fn spawn_fspec_combined(workspace: &Path) -> (ChildGuard, u16)
```

### Per-command test file structure (`cli_<snake>.rs`)

```rust
mod common;
use common::fspec_bin;
use std::process::Command;
use tempfile::TempDir;

fn empty_workspace() -> TempDir { tempfile::tempdir().expect("tempdir") }

fn workspace_with_<file>_json(body: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("spec")).expect("mkdir spec");
    std::fs::write(dir.path().join("spec").join("<file>.json"), body).expect("write");
    dir
}

fn run_list_<name>(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let out = Command::new(fspec_bin())
        .arg("list-<name>").args(extra_args).current_dir(cwd)
        .output().expect("spawn");
    (out.status.code().unwrap_or(-1),
     String::from_utf8_lossy(&out.stdout).into_owned(),
     String::from_utf8_lossy(&out.stderr).into_owned())
}

fn canonical_<file>_json() -> String { /* fixture string used by multiple scenarios */ }

// One #[test] per Gherkin scenario, named scenario_<snake>_<…>
#[test]
fn scenario_<snake>_…() {
    // @step Given …
    // @step When …
    // @step Then …
}
```

### Mandatory `@step` comments

EVERY Gherkin step from the matching feature file MUST appear verbatim as a
`// @step …` comment in the corresponding test, right before the code that
executes that step. fspec's `link-coverage` and `audit-coverage` validation
will refuse to advance from `testing` → `implementing` if any step is missing.

### Help-fixture parity test

Every command has a scenario `<name>_help_matches_ts_formatcommandhelp_reference`
that asserts the binary's `--help` output is byte-for-byte identical to
`tests/fixtures/help/<name>.txt`. To regenerate the fixture:

```bash
node dist/index.js <command-name> --help 2>&1 > codelet/fspec/tests/fixtures/help/<name>.txt
```

Pipe to a file (not a TTY) so chalk wrappers reduce to identity.

### Two-front-doors parity test

```rust
#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // 1. Build temp workspace with seed JSON.
    // 2. Call codelet_fspec_core::dispatch_command(DispatchRequest{
    //      command: "list-<name>", args_json: r#"{"format":"json"}"#, project_root: ws.path() })
    //    → assert dispatcher JSON shape.
    // 3. Invoke the binary with the same args → assert identical JSON output.
    // 4. Optionally assert the bridge .rs file contains no inline rendering or
    //    filter logic (string-grep `tests/fixtures/parity_<name>.txt`).
}
```

---

## 9. Step-by-step porting workflow

### Pre-flight

1. Pick the next `RPC-XXX` work unit (`fspec board` or `fspec list-work-units --status backlog`).
2. Read the TS source (`src/commands/<name>.ts` + `src/commands/<name>-help.ts`)
   end-to-end. Note: every `console.log/error`, every `process.cwd()`, every
   `JSON.parse` / `JSON.stringify` indent, every `ensure*File` call, every
   filter ordering, every fallback default.
3. Read the existing feature file under `spec/features/<name>-*-cli-subcommand.feature`
   and `<name>-rust-port.feature` — they may already exist from earlier discovery.
4. Search for similar already-ported commands and copy their layout. **`list-prefixes`
   is the canonical reference** for read-only flag-less commands. **`list-work-units`**
   for filter-chain commands. **`list-virtual-hooks`** for positional-argument commands.

### Discovery / Example Mapping (only if scope unclear)

- `fspec discover-event-storm <id>` for complex domains.
- Otherwise jump straight to `fspec set-user-story`, `fspec add-rule`, `fspec add-example`.
- Add architecture notes describing the shared modules to be created.

### Specifying

- `fspec generate-scenarios <id>` produces the `.feature` file from the example map.
- Hand-edit to add CLI-subcommand scenarios (help intercept, exit codes, two-front-doors).
- `fspec validate` to confirm Gherkin syntax.
- `fspec update-work-unit-status <id> specifying` → `testing`.

### Testing (write failing tests FIRST)

1. Create `codelet/fspec/tests/cli_<snake>.rs` with one `#[test]` per scenario,
   `@step` comments verbatim.
2. Create `codelet/fspec/tests/fixtures/help/<name>.txt` by capturing TS output.
3. `fspec link-coverage <feature> --scenario "…" --test-file codelet/fspec/tests/cli_<snake>.rs --test-lines <range>`
4. Run `cargo test -p codelet-fspec --test cli_<snake>` and confirm they FAIL
   for the right reasons (no implementation yet → `NotYetPorted`).
5. `fspec update-work-unit-status <id> implementing`.

### Implementing

1. Add `<name>` to `PORTED_COMMANDS` in `canonical.rs`.
2. Add the dispatch arm in `dispatch.rs::run_ported` AND remove the `run_stub` arm.
3. Replace the stub at `codelet/fspec-core/src/commands/<snake>.rs` with the real impl.
4. Extend `codelet/fspec-core/src/io/` and `codelet/fspec-core/src/types/` with
   any new shared modules. **Prefer extending existing modules over creating new ones**.
5. Add `codelet/fspec-core/src/help/configs/<snake>.rs` + register in `configs/mod.rs`.
6. Add `Mode::<Pascal>` variant in `codelet/fspec/src/main.rs::Mode`.
7. Add the match arm calling `forward!`.
8. Add the intercept arm in `intercept_ts_help()`.
9. Create `codelet/fspec/src/<snake>.rs` bridge + `mod <snake>;` in main.rs.
10. Run tests, iterate until green.
11. Link impl: `fspec link-coverage <feature> --scenario "…" --test-file … --impl-file codelet/fspec-core/src/commands/<snake>.rs --impl-lines <range>`.

### Validating

- `cargo test --release -p codelet-fspec --test cli_<snake>` — targeted.
- `cargo test --release -p codelet-fspec-core --test <snake>` — core unit tests.
- `cargo test --release -p codelet-fspec --test cross_frontend_parity` —
  guards the dispatcher↔CLI invariant.
- **Do NOT run full workspace `cargo test`** — it pulls in fspec-tui's lance/
  tantivy/datafusion deps and compiles for 20+ minutes. Target the specific
  test binaries.
- `fspec validate` + `fspec validate-tags`.
- `fspec audit-coverage <feature>`.

### Done

- `fspec remove-tag-from-feature spec/features/<…>.feature @wip`
- `fspec add-tag-to-feature spec/features/<…>.feature @done`
- `fspec update-work-unit-status <id> done`.

---

## 10. Gotchas & lessons learned (RPC-241..253 + RPC-327)

### TS-parity edge cases

- **`ensureXFile` vs `readXOrEmpty`** — the TS code is inconsistent. `list-prefixes`
  reads `prefixes.json` directly with bare `catch {}` (ENOENT → empty, parse error
  → empty). `list-work-units` calls `ensureWorkUnitsFile` (auto-creates) AND
  `ensurePrefixesFile`. Match the TS behaviour for the specific command — do NOT
  unify on one helper.
- **Filter chain ordering matters** — TS applies filters left-to-right (status →
  prefix → epic → type). The output order is iteration order over the original
  IndexMap, with no re-sort. Preserve this exactly.
- **`(wu.type || 'story') === options.type`** — TS short-circuits missing types
  to `'story'`. Rust mirrors this via `WorkUnit::type_str()` which returns
  `"story"` when the field is None.
- **`Math.round()` semantics** — TS rounds 33.5 → 34, 66.5 → 67. Rust's `(x).round() as u32`
  matches. Don't use `as u32` on a float (truncates).
- **Insertion-order JSON output** — use `#[derive(Serialize)] struct` with
  explicit `#[serde(rename = "…")]` annotations. The field declaration order
  is the JSON output order. `json!{}` uses `serde_json::Map` (BTreeMap) and
  alphabetizes.
- **Empty-array vs missing field** — TS interfaces declare optional fields;
  the runtime may emit `"workUnits": []` or omit it entirely. Match the TS
  output exactly — check the actual `node dist/index.js …` output, not the
  TS source.

### Help-text divergence policy ("Framing A")

When TS rich help (`<name>-help.ts`) documents output examples that the TS CLI
itself does NOT produce (because `commander.action()` discards the result, e.g.
`list-hooks`), the **help doc is canon**. Rust must implement what the help
promises, even if it makes the binary output diverge from the broken TS CLI.

Document this in an architecture note on the work unit: "Framing A — TS shell
is broken (action discards result); Rust dispatcher correctly implements the
help-doc canon."

### Help intercept must run BEFORE clap parse

Otherwise clap's auto-generated `--help` wins and the fixture parity test
fails. The intercept short-circuits via `std::process::exit(0)` semantics
(actually returns `Some(0)` to main).

### clap derive quirks

- `r#type` for the `--type` flag — `type` is reserved.
- `value_name = "WORK_UNIT_ID"` (UPPER_SNAKE) is clap's default, but TS shows
  camelCase `<workUnitId>`. The intercept renders TS-style; clap-default only
  shows up if intercept fails. Tests should assert the TS-style string.
- `#[arg(short, long)]` adds both `-s` and `--status`. If TS only has `--status`,
  drop `short`.
- Boolean flags: `#[arg(long)] json: bool` — no `Option` wrapper.

### Build / test runner gotchas

- **Never `cargo test` the full workspace.** It compiles fspec-tui's massive
  dependency tree (lance, tantivy, datafusion) — 20+ minutes minimum.
  Always target: `cargo test -p codelet-fspec --test cli_<name>` or
  `cargo test -p codelet-fspec-core --test <name>`.
- **`tee` test output**, never `head`/`tail`/`grep`. Tests are expensive;
  re-running to see different output slices wastes time.
  Pattern: `cargo test … 2>&1 | tee /tmp/test-out.txt` then `Read /tmp/test-out.txt`.
- **TUI E2E tests** (under `e2e/*.test.ts`) use `@microsoft/tui-test` via
  `./scripts/run-tui-test.sh`. NOT cargo. Out of scope for command ports.
- **Combined-mode tests grab `/dev/tty`** so they are `#[ignore = "RPC-026: ..."]`'d
  and shouldn't be run in normal flow.
- **`scrollback_pty_rpc078.rs` is `#[ignore]`'d** — leave it alone.

### State walk discipline

- Move `done → testing` (not `done → specifying`) when only test/spec
  details changed — preserves the architecture-note arc.
- Use `skipTemporalValidation: true` only when walking already-done work
  back through states for fixes. Document why in the next state transition.
- The auto-checkpoint feature creates a git stash on each transition.
  Auto-cleanup deletes them on the next move-forward; manual checkpoints
  persist.

### RPC-327: nested tokio runtime panic

The very first port (RPC-253) shipped with `tokio::runtime::Builder::new_current_thread().block_on()`
inside `dispatch_command`. When the dispatcher was called from the agent
loop's outer `#[tokio::main]` runtime, this panicked or dead-locked.

**Fix:** `poll_sync_future` with `Waker::noop()`. Polls once. Returns
`InvalidArgs` if the future returns `Pending` (caught loudly during
development, never silently hangs).

**Lesson:** every ported command must be effectively synchronous internally.
Async signature is a transport contract; no real `.await` allowed.

---

## 11. Quick reference — file locations cheat sheet

| Need | Path |
|------|------|
| Add a new ported command name | `codelet/fspec-core/src/canonical.rs::PORTED_COMMANDS` |
| Wire dispatcher | `codelet/fspec-core/src/dispatch.rs::run_ported` (add arm + remove from `run_stub`) |
| Core impl | `codelet/fspec-core/src/commands/<snake>.rs` |
| Register core module | `codelet/fspec-core/src/commands/mod.rs` |
| Shared IO helpers | `codelet/fspec-core/src/io/{ensure,locked_file,project_root,feature_glob}.rs` |
| Shared types | `codelet/fspec-core/src/types/{work_unit,prefix,epic,tags}.rs` |
| Help config | `codelet/fspec-core/src/help/configs/<snake>.rs` |
| Register help config | `codelet/fspec-core/src/help/configs/mod.rs` |
| clap variant | `codelet/fspec/src/main.rs::Mode` |
| clap dispatch arm | `codelet/fspec/src/main.rs::main` (use `forward!` macro) |
| Help intercept arm | `codelet/fspec/src/main.rs::intercept_ts_help` |
| CLI bridge | `codelet/fspec/src/<snake>.rs` (+ `mod <snake>;` in main.rs) |
| Integration tests | `codelet/fspec/tests/cli_<snake>.rs` |
| Test helpers | `codelet/fspec/tests/common/mod.rs` |
| Help fixture | `codelet/fspec/tests/fixtures/help/<name>.txt` |
| Feature file | `spec/features/<name>-cli-subcommand.feature` + `<name>-rust-port.feature` |
| TS reference | `src/commands/<name>.ts` + `src/commands/<name>-help.ts` |
| TS help formatter | `src/utils/help-formatter.ts` |

---

## 12. Canonical port lineage (read in order)

When you need a concrete example, read these in order — they build on each other:

1. **RPC-248 `list-prefixes`** — the canonical reference. Flag-less, read-only,
   no auto-create, minimal serde. **Start here.**
2. **RPC-253 `list-work-units`** — filter chain (status/prefix/epic/type),
   auto-create files, IndexMap iteration, WorkUnitType enum.
3. **RPC-252 `list-virtual-hooks`** — positional argument, reads `extra` map.
4. **RPC-245 `list-features`** — uses `io::feature_glob`, embedded gherkin parsing.
5. **RPC-244 `list-feature-tags`** — most complex; full gherkin re-parse with
   category grouping.
6. **RPC-247 `list-hooks`** — Framing A divergence (TS broken; Rust correct).
7. **RPC-246 `list-foundation-sections`** — special-case help (bare Commander.js).

---

## 13. Parallel orchestration (Supervisor + Workers + Cargo Serial Worker)

The 12 `list-*` ports were landed in batches of 5 using a three-role
AgentManager topology. This section is the playbook for repeating that.

### Roles

```
┌─────────────────────────────────────────────────────────────────┐
│              SUPERVISOR (this session)                          │
│  - Picks the batch of 5 work units                              │
│  - Spawns + closes worker sessions                              │
│  - Drives phase transitions (A → B → C)                         │
│  - Edits SHARED files (canonical.rs, dispatch.rs, main.rs, …)   │
│  - Calls fspec Fspec tool for all state moves                   │
└─────────────────────────────────────────────────────────────────┘
   │                              │
   │ spawn 5 workers              │ spawn 1 cargo serial worker
   ▼                              ▼
┌──────────────────────────┐ ┌──────────────────────────────────┐
│  WORKER ×5               │ │  CARGO SERIAL WORKER             │
│  (each owns 1 RPC card)  │ │  (the ONLY agent allowed to run  │
│                          │ │   cargo build / cargo test /     │
│  - Reads TS source       │ │   binary invocations)            │
│  - Owns ISOLATED files   │ │                                  │
│  - Never edits shared    │ │  - Receives requests from        │
│    files                 │ │    supervisor + workers          │
│  - Asks cargo runner for │ │  - Serializes them               │
│    builds/tests          │ │  - Streams output back via       │
│  - Reports back at each  │ │    file paths (tee /tmp/*.txt)   │
│    phase boundary        │ │                                  │
└──────────────────────────┘ └──────────────────────────────────┘
```

### Why a Cargo Serial Worker?

Parallel `cargo build` / `cargo test` invocations from multiple agents
**corrupt the build cache** (`target/` directory locking, .rlib clashes,
incremental compilation invalidation). The Cargo Serial Worker is a single
agent session whose sole purpose is to receive build/test requests and run
them one at a time. All other agents `message` it instead of touching cargo
themselves.

### Cargo Serial Worker role prompt (template)

```
You are the CARGO SERIAL WORKER for the fspec project.

Your sole purpose: serialize ALL cargo invocations + binary builds + binary
runs so the supervisor and worker agents do NOT corrupt the build cache by
running cargo in parallel.

RULES:
- You are the ONLY agent allowed to run `cargo build`, `cargo test`,
  `cargo clippy`, `cargo run`, or invoke `./codelet/target/release/fspec`.
- Other agents send you requests via AgentManager.message — process them
  one at a time, in arrival order.
- ALWAYS tee output to a stable filename so the requesting agent can read it:
    cd /Users/rquast/projects/fspec/codelet
    cargo test --release -p codelet-fspec --test cli_list_<name> 2>&1 \
      | tee /tmp/test-<name>-$(date +%s).txt
- After running, REPLY with: (a) the tee filename, (b) exit code,
  (c) a 5–10 line summary of pass/fail counts.
- NEVER run full-workspace `cargo test` (pulls fspec-tui deps, 20+ min).
  Always target -p <crate> --test <test_binary>.
- NEVER pipe cargo output through head/tail/grep — tee then Read.
```

### Worker role prompt (template)

```
You are a subordinate WORKER agent porting one fspec CLI command from
TypeScript to Rust as part of a 5-worker parallel orchestration. Your
supervisor (session <SUPERVISOR_ID>) coordinates phase transitions.

PROJECT ROOT: /Users/rquast/projects/fspec
CARGO RUNNER AGENT ID: <CARGO_WORKER_ID>
YOUR WORK UNIT: <RPC-XXX> — Port `<command-name>` to Rust

REFERENCE IMPLEMENTATION (canonical pattern — read these first):
  - codelet/fspec-core/src/commands/list_prefixes.rs
  - codelet/fspec-core/tests/list_prefixes.rs
  - codelet/fspec/src/list_prefixes.rs
  - codelet/fspec/tests/cli_list_prefixes.rs
  - spec/features/list-prefixes-rust-port.feature
  - spec/features/list-prefixes-cli-subcommand.feature
  - TS source of truth: src/commands/list-prefixes.ts

PHASED EXECUTION (each phase gated by supervisor):
  - PHASE A — SPECIFYING: Example Mapping + feature files. STOP, report.
  - PHASE B — TESTING:    Write failing tests in isolated files. STOP, report.
  - PHASE C — IMPLEMENTING: Write the Rust impl. STOP, report.

═══════════════════════════════════════════════════════════════════════════
HARD RULES — FILE OWNERSHIP (DO NOT VIOLATE):
═══════════════════════════════════════════════════════════════════════════

YOU MAY CREATE / EDIT (isolated, parallel-safe):
- spec/features/<your-cmd-kebab>-rust-port.feature
- spec/features/<your-cmd-kebab>-cli-subcommand.feature
- spec/features/<your-cmd-kebab>-*.feature.coverage (auto)
- spec/attachments/<RPC-ID>/ast-research-<your-cmd-kebab>.md
- codelet/fspec-core/src/commands/<your_cmd_snake>.rs    (rewrite the stub)
- codelet/fspec-core/tests/<your_cmd_snake>.rs           (NEW dispatcher test)
- codelet/fspec-core/src/help/configs/<your_cmd_snake>.rs (NEW help config)
- codelet/fspec/src/<your_cmd_snake>.rs                  (NEW CLI bridge)
- codelet/fspec/tests/cli_<your_cmd_snake>.rs            (NEW CLI test)
- codelet/fspec/tests/fixtures/help/<your-cmd-kebab>.txt (NEW help fixture)
- NEW type files: codelet/fspec-core/src/types/<your_type>.rs

YOU MUST NOT TOUCH (shared — supervisor-only):
- codelet/fspec-core/src/canonical.rs
- codelet/fspec-core/src/dispatch.rs
- codelet/fspec-core/src/commands/mod.rs
- codelet/fspec-core/src/types/mod.rs
- codelet/fspec-core/src/help/configs/mod.rs
- codelet/fspec-core/src/io/ensure.rs       (READ-ONLY; ask supervisor)
- codelet/fspec/src/main.rs
- codelet/fspec/tests/cargo_shape.rs
- Cargo.toml files
- spec/features/list-work-units-*.feature   (canonical reference)
- spec/features/list-prefixes-*.feature     (canonical reference)

If you need a shared-file change, ASK your supervisor in your report.

═══════════════════════════════════════════════════════════════════════════
ACDD DISCIPLINE — use the Fspec tool, not bash CLI:
═══════════════════════════════════════════════════════════════════════════

PHASE A — SPECIFYING:
  1. Read canonical reference files.
  2. Read TS impl: src/commands/<your-cmd>.ts + <your-cmd>-help.ts.
  3. Save AST research to spec/attachments/<RPC-ID>/ast-research-<your-cmd-kebab>.md.
  4. Fspec: add-attachment.
  5. Fspec: set-user-story.
  6. Fspec: add-rule (one per TS behaviour observation).
  7. Fspec: add-example (concrete scenarios).
  8. Fspec: add-architecture-note (file layout, wiring intent).
  9. Fspec: update-work-unit-estimate (Fibonacci 1/2/3/5).
  10. Fspec: generate-scenarios TWICE: once for *-rust-port.feature
      (dispatcher contract), once for *-cli-subcommand.feature (clap surface).
  11. Fspec: add-tag-to-feature @wip on both files.
  12. Fspec: validate.
  13. REPORT BACK: feature file paths, scenario counts, estimate,
      rules summary. STOP.

PHASE B — TESTING:
  1. Write codelet/fspec-core/tests/<your_cmd_snake>.rs — dispatcher test,
     one fn per Gherkin scenario, `@step` comments verbatim.
  2. Write codelet/fspec/tests/cli_<your_cmd_snake>.rs — CLI shell test.
  3. Capture help fixture: `node dist/index.js <name> --help 2>&1 >
     codelet/fspec/tests/fixtures/help/<name>.txt`.
  4. Ask CARGO RUNNER to verify tests FAIL with NotYetPorted:
       cargo test --release -p codelet-fspec-core --test <your_cmd_snake>
       cargo test --release -p codelet-fspec --test cli_<your_cmd_snake>
  5. Fspec link-coverage for each scenario → test file + lines.
  6. REPORT BACK: file paths, test counts, supervisor change requests
     (e.g. "add `read_<x>_or_empty` to io/ensure.rs"). STOP.

PHASE C — IMPLEMENTING:
  1. Replace stub at codelet/fspec-core/src/commands/<your_cmd_snake>.rs.
  2. Write codelet/fspec-core/src/help/configs/<your_cmd_snake>.rs.
  3. Write codelet/fspec/src/<your_cmd_snake>.rs CLI bridge.
  4. Ask CARGO RUNNER for incremental builds:
       cargo build --release -p codelet-fspec-core
  5. WAIT FOR SUPERVISOR to wire shared files (canonical/dispatch/main/help-configs/mod).
  6. Ask CARGO RUNNER for green-phase test runs (targeted, not workspace).
  7. Fspec link-coverage with impl-file + impl-lines.
  8. REPORT BACK: file paths, test pass counts, any remaining concerns. STOP.

CARGO RUNNER PROTOCOL:
  Send messages like:
    "Please run: cd codelet && cargo test --release -p codelet-fspec
     --test cli_<name> 2>&1 | tee /tmp/test-<name>.txt"
  Then await_idle on the cargo runner. Read the tee file when it reports back.
  NEVER run cargo yourself.
```

### Supervisor playbook (step-by-step)

**Setup**

```
1. Fspec: board → identify the 5 next list-* / port candidates (look for
   straightforward read-only commands first).

2. Fspec: update-work-unit-status <id> specifying  (× 5)

3. AgentManager: spawn the Cargo Serial Worker with the role prompt above.
   Save its session_id as CARGO_WORKER_ID.

4. AgentManager: spawn 5 worker sessions. For each one:
   - Use the worker role prompt above, substituting the worker's RPC ID,
     command name, and the CARGO_WORKER_ID.
   - Save each worker's session_id.

5. Track everything in a markdown scratch file (e.g. spec/attachments/
   <BATCH-ID>/orchestration-state.md) — see template below.
```

**Phase A (Specifying) — fan out, fan in**

```
6. message all 5 workers: "Begin PHASE A. Report back when done."

7. AgentManager: await_idle on all 5 workers (no timeout, this can take
   30+ minutes for example mapping).

8. For each worker, AgentManager: get_status or SessionSearch on their
   session — extract the phase A report (feature paths, scenario counts,
   shared-file change requests).

9. Update orchestration-state.md with each worker's report.

10. Fspec: validate to confirm all 10 new feature files parse.
```

**Phase B (Testing) — fan out, fan in, single cargo run**

```
11. message all 5 workers: "Begin PHASE B. Write isolated tests. ASK
    the cargo runner ONCE at the end to confirm tests fail with
    NotYetPorted. STOP after that. NEVER run cargo yourself."

12. await_idle on all 5 workers.

13. Collect their reports. Read any tee files the cargo runner produced.

14. CRITICAL — supervisor's own job in phase B:
    - Add NEW shared helpers requested by workers (e.g. read_epics_or_empty
      in io/ensure.rs).
    - DO NOT add `PORTED_COMMANDS` entries yet — that would move the
      dispatcher onto the new impls, but the impls don't exist yet.
    - Workers' tests should still FAIL with NotYetPorted at this point.
```

**Phase C (Implementing) — fan out, supervisor wires shared files in
parallel, single cargo run at the end**

```
15. message all 5 workers: "Begin PHASE C. Write the impl in your isolated
    module + CLI bridge + help config. Do NOT touch shared files. When
    done, REPORT BACK and STOP."

16. await_idle on all 5 workers.

17. Supervisor wires shared files (ALL files in ONE edit pass to minimize
    rebuild thrash):
    - canonical.rs: add 5 lines to PORTED_COMMANDS
    - dispatch.rs::run_ported: add 5 match arms; remove 5 arms from run_stub
    - commands/mod.rs: register 5 new modules (replace stub registrations)
    - help/configs/mod.rs: register 5 new help configs
    - main.rs: add 5 Mode:: variants + 5 forward! arms + 5 intercept arms
    - main.rs: add `mod <snake>;` for each new bridge

18. message CARGO_WORKER once: "Please run targeted tests for the 5 new
    commands and tee each to a separate file:
      cargo test --release -p codelet-fspec-core --test list_<a>  → /tmp/a.txt
      cargo test --release -p codelet-fspec-core --test list_<b>  → /tmp/b.txt
      …
      cargo test --release -p codelet-fspec --test cli_list_<a>   → /tmp/cli-a.txt
      …
    Report exit codes."

19. await_idle on cargo runner. Read all tee files.

20. If any failures: re-message the relevant worker with the failure
    snippet ("Test X failed because…"), iterate until green.

21. For each work unit:
    - Fspec: update-work-unit-status <id> validating
    - Fspec: show-coverage <feature> (sanity check)
    - Fspec: remove-tag-from-feature @wip; add-tag-to-feature @done
    - Fspec: update-work-unit-status <id> done
```

**Cleanup**

```
22. AgentManager: close each worker session (only spawner can close).
23. AgentManager: close the cargo worker.
24. Final cargo test run to ensure no regression:
    cd codelet && cargo test --release -p codelet-fspec-core -p codelet-fspec
```

### Phase boundary discipline (critical)

The phases exist to prevent two failure modes:

1. **Stub-on-stub race**: if Worker A finishes and registers in PORTED_COMMANDS
   while Worker B is still in PHASE B (tests must fail with NotYetPorted),
   Worker B's tests start failing with a real error message → false signal.
   **Fix:** the supervisor only edits canonical/dispatch AFTER all 5 workers
   are at Phase C green.

2. **Cargo cache thrash**: if 5 workers each invoke cargo build in parallel,
   the `target/` directory locks contend and incremental compilation
   silently drops .rlib files. Builds appear to succeed but link errors
   appear later in unrelated tests. **Fix:** all cargo calls go through the
   one Cargo Serial Worker.

### Tracking the orchestration in this markdown

Update the table below at every batch start. After each batch lands,
move the rows into the "Completed batches" log at the bottom.

#### Current batch

| Slot | RPC ID | Command | Worker session_id | Phase | Notes |
|------|--------|---------|-------------------|-------|-------|
| 1    |        |         |                   |       |       |
| 2    |        |         |                   |       |       |
| 3    |        |         |                   |       |       |
| 4    |        |         |                   |       |       |
| 5    |        |         |                   |       |       |

**Cargo Serial Worker session_id:**  `<paste here>`
**Supervisor session_id:**  `<paste here>`

#### Shared-file change requests (pending supervisor action)

| Requested by | File | Change | Status |
|--------------|------|--------|--------|
|              |      |        |        |

#### Completed batches log

| Batch | RPC IDs | Completed (UTC) | Notes |
|-------|---------|-----------------|-------|
| 1 (RPC-248 reference) | RPC-248 | 2026-06-03 | Foundational port; established io/, types/, dispatch/ patterns |
| 2 | RPC-253, RPC-243, RPC-251, RPC-245, RPC-241 | 2026-06-04 | Filter-chain + read-only commands; established two-front-doors test |
| 3 | RPC-244, RPC-246, RPC-249, RPC-250, RPC-252 | 2026-06-05 | Positional-arg commands + bare Commander.js help special case |
| 4 | RPC-247 | 2026-06-05 | Solo port; Framing A divergence (TS broken; Rust correct) |
| 5 (fix-up) | RPC-252, RPC-253 (re-done) | 2026-06-07 | Help-text fixture/assertion alignment after intercept landed |
| 6 | RPC-242, RPC-301, RPC-302, RPC-304, RPC-310 + RPC-308, RPC-257, RPC-258, RPC-261, RPC-263 + RPC-256, RPC-259, RPC-260, RPC-262, RPC-299, RPC-300, RPC-303, RPC-305, RPC-306, RPC-307 | 2026-06-09 | show-* / query-* commands; deepest read-only batch |
| 7 | RPC-211, RPC-217, RPC-213, RPC-313, RPC-265, RPC-316, RPC-222, RPC-176, RPC-271, RPC-204 | 2026-06-10 | First **mutation** batch (10 commands): create/delete/update for epic/prefix/tag + add-dependencies/remove-dependency/clear-dependencies. Established write_json_atomic + IndexMap discipline. Lessons: (a) workers must NOT edit dispatch.rs, (b) Framing A inverse — when TS has no rich -help.ts, Rust formatter becomes canon, (c) clap variadic per-flag uses `num_args = 1..` (not `value_delimiter = ','`). main.rs cap bumped 850→1100. |
| 8 | RPC-189, RPC-279, RPC-169, RPC-181, RPC-273, RPC-188, RPC-278, RPC-168, RPC-267, RPC-298 | 2026-06-11 | Example Mapping mutation batch (10 commands): add/remove rule, add assumption, add/remove example, add/remove question, add/remove architecture-note, set-user-story. All operate on `spec/work-units.json` work-unit Example Mapping sub-objects. Lessons: (a) workers spawn empty → ALWAYS send a kickoff message immediately; (b) one worker silently abandoned mid-PHASE-B — spawn a replacement when await_idle returns and the report is incomplete; (c) `link-coverage` with the same `<scenario, testFile, testLines>` tuple creates duplicate entries; (d) test-fixture JSON with duplicate keys passes Rust's parse step but fails at runtime — always pretty-print + visually inspect the seeded JSON. Five worker sessions + one cargo serial worker. |
| 10 | RPC-170, RPC-268, RPC-195, RPC-283, RPC-205, RPC-209, RPC-184, RPC-275, RPC-178, RPC-216 | 2026-06-11 | Attachments + virtual hooks + hooks + diagrams batch (10 commands): add/remove-attachment, add/remove-virtual-hook, clear/copy-virtual-hooks, add/remove-hook, add/delete-diagram. Five worker sessions + one cargo serial worker. Each worker owned a tight pair sharing infrastructure. Lessons: (a) substring assertions on bridge source for "no writes" must strip comments first — `MUST NOT perform on-disk writes` doc-comment triggers false-positives; (b) help fixture COMMON PATTERNS reference sibling commands' flags so blanket "must not mention --blocking" assertions fail — assert on OPTIONS section content instead; (c) cargo_shape's `main_cap` (1500 lines) and lock-list need bumping each batch — supervisor task. Final: 162/162 core tests + 71/71 CLI tests + cargo_shape green. |
| 12 | RPC-317, RPC-318, RPC-223, RPC-206, RPC-255, RPC-284, RPC-264, RPC-229, RPC-228, RPC-227 | 2026-06-12 | work-units.json mutation + export batch (10 commands): update-work-unit, update-work-unit-estimate, delete-work-unit, compact-work-unit, prioritize-work-unit, repair-work-units, record-iteration, export-work-units, export-example-map, export-dependencies. Five workers + one cargo serial worker. All reuse `WorkUnitsData`/`EpicsData` + `write_json_atomic`; no FOUNDATION.md chain (deliberately avoided foundation event-storm cmds). cargo_shape lock-list 88→98; main.rs 2250 lines (< 2300 cap, no bump). Final: core cmd tests 65 + dispatcher 6 + CLI tests 47 + cross_frontend_parity 8 + core lib 452, all green. Lessons: (a) **workers ran ahead from PHASE B into PHASE C** — when a worker rewrites its command stub to the 2-arg `(args_json, project_root)` signature, dispatch.rs `run_stub` (1-arg) stops compiling and blocks ALL sibling workers' test builds; collapse B+C and wire dispatch.rs as soon as impls land. (b) **W1 used `regex` in an impl** but it was only a `[dev-dependencies]` entry → lib failed to compile; supervisor moved `regex` to `[dependencies]` (playbook says hand-roll, but a one-line dep move is the faster recovery once written). (c) **enum-edit footgun**: replacing the last `Mode` variant's trailing `}\n}` region can accidentally delete the `#[tokio::main] async fn main()` signature that immediately followed — re-add it. (d) help-config type is `CommonError`, NOT `CommandError` (W2 typo broke the build). (e) **1 feature = 1 test file**: a worker that merges dispatcher+CLI scenarios into ONE feature then links both a core test AND a cli test to it gets BLOCKED at the testing→validating gate — enforce the `<cmd>-rust-port.feature` (→core test) / `<cmd>-cli-subcommand.feature` (→cli test) split up front. (f) **cli-subcommand scenarios need IMPL mappings too** (impl-file = the CLI bridge) or the validating gate fails with "implementation coverage is incomplete". |

### Pitfalls encountered (don't repeat)

- **Worker spawn delay**: `AgentManager spawn` returns a session_id but the
  worker is idle with no task. **Always** follow `spawn` with `message` to
  hand it the role context + first instruction. Forgetting this leaves the
  worker silent and you'll think it crashed.
- **`await_idle` timeout pitfall**: omitting `timeout` blocks indefinitely.
  For Phase A on 5 workers, 30+ minutes is normal. Use `timeout: 3600` if
  you want a safety upper bound but be prepared to extend.
- **Cargo runner timeouts**: the cargo runner's `await_idle` will say
  `timed_out` while a long compile is still running. Re-poll. The session
  is alive; the timeout is your timeout, not its.
- **Workers asking each other**: workers must NEVER message each other —
  they don't know about each other's file scopes. Cross-worker coordination
  goes through the supervisor only.
- **TS reference parity drift**: do not blindly copy the TS `console.log`
  behaviour if the TS CLI is observably broken (Framing A). Always check
  the captured non-TTY output, not the source.
- **Empty `node dist/index.js …` output**: if the TS CLI silently produces
  nothing (broken `.action()`), the help-doc reference is canon. Document
  Framing A in the architecture note.
- **Tag-validation cross-contamination**: validating all spec/features/
  while another worker is editing files can produce false failures. Run
  `fspec validate` only on the specific new files during batch work.
- **Don't full-workspace `cargo test`**: pulls fspec-tui's lance/tantivy/
  datafusion (20+ min). Cargo runner role prompt forbids it.

### When NOT to use the parallel orchestration

- Single complex command (e.g. anything 13+ story points) — supervise it
  yourself in this session.
- Commands that need shared-type additions covered by a different in-flight
  card — serialize them.
- Fix-up / TS-parity work on already-done cards — do these solo (this is
  what happened with RPC-252/RPC-253 in batch 5 above).

---

## 14. Testing discipline — `@microsoft/tui-test` + real fixtures (NOT mocks)

> Authoritative reference: [`TESTING.md`](./TESTING.md) in the repo root —
> read it before writing any new test. This section is the *port-specific*
> distillation: the testing pyramid the Rust port must respect and the
> shape of every test artifact that a worker produces.

### 14.1 The four test layers (where does this test belong?)

```
┌────────────────────────────────────────────────────────────────────┐
│  Layer 4  ─ @microsoft/tui-test  (TS, e2e/*.test.ts)               │
│            Spawns a real PTY, runs the *real Rust binary*,         │
│            asserts on what an end-user actually sees.              │
│            Slow (seconds), but it catches the bugs the other       │
│            three layers cannot: ANSI rendering, cursor placement,  │
│            scrollback, panics in `block_on`, real key events.      │
├────────────────────────────────────────────────────────────────────┤
│  Layer 3  ─ `cargo test -p codelet-fspec` (Rust, tests/cli_*.rs)   │
│            Spawns the compiled `fspec` binary via std::process::    │
│            Command in a real OS temp directory. Asserts stdout /   │
│            stderr / exit code against fixture JSON / fixture       │
│            help text. THIS is where every ported list-* command    │
│            lives.                                                  │
├────────────────────────────────────────────────────────────────────┤
│  Layer 2  ─ `cargo test -p codelet-fspec-tui`                      │
│            Drives the `App` reducer in-process with an             │
│            `Arc<MockBackend>` (the 2876-LoC test double in         │
│            `codelet/fspec-tui/tests/common/mod.rs`). The Mock      │
│            here is a *real Rust object* with deterministic         │
│            counters — NOT a `vi.fn()` interceptor. Same            │
│            philosophy: redirect, don't intercept.                  │
├────────────────────────────────────────────────────────────────────┤
│  Layer 1  ─ `cargo test -p codelet-fspec-core` (unit)              │
│            Pure-function tests over `commands/<snake>.rs`. Real    │
│            in-memory JSON. No filesystem, no process spawn.        │
└────────────────────────────────────────────────────────────────────┘
```

**Every ported command MUST exist at Layers 1 + 3.** Layers 2 + 4 are
only added when the command surface includes a TUI view or interactive
behaviour.

### 14.2 `@microsoft/tui-test` — when, and how

**When** — only when the test cannot be expressed at Layer 3. Specifically:

- The behaviour depends on PTY framing (cursor save/restore, `\x1b[?2004h`
  bracketed paste, scrollback truncation at term height).
- Reproducing a screenshot bug (`RPC-068` is the canonical example —
  `e2e/rpc-068-rust-binary-smoke.test.ts` writes the rendered buffer to
  `/tmp/rust_fspec_real_buffer.txt` and greps for the panic substring).
- Asserting that the *combined-mode* (`/dev/tty`-grabbing) binary doesn't
  deadlock. (`#[ignore]`'d cargo tests literally cannot exercise this —
  cargo's test harness already owns the TTY.)

**How** — the workflow is fixed; do not invent alternatives:

```bash
# 1. Build the Rust binary first (debug or release; debug is fine).
./scripts/cargo-runner.sh "cargo build -p codelet-fspec"

# 2. Run the JS-side harness. NEVER `npx @microsoft/tui-test` directly —
#    always go through the wrapper. It (a) stashes `tmp/` so swc doesn't
#    explode on cloned-repo JSX in plain .js, (b) clears `.tui-test/
#    cache`, (c) restores `tmp/` even on Ctrl-C.
./scripts/run-tui-test.sh                          # run all e2e tests
./scripts/run-tui-test.sh rpc-068                  # filter by name
./scripts/run-tui-test.sh --trace prov-095         # capture trace.zip
```

**File layout** — every tui-test file goes in `e2e/`, named
`<rpc-id>-<slug>.test.ts`, and the program under test is the *built
Rust binary at `codelet/target/debug/fspec`*. Never run a node script.

**Canonical skeleton** (mirrors `rpc-068-rust-binary-smoke.test.ts`):

```ts
import { test, expect } from '@microsoft/tui-test';
import { homedir, tmpdir } from 'os';
import { join } from 'path';
import { mkdtempSync, writeFileSync } from 'fs';

const rustFspec = join(homedir(), 'projects', 'fspec',
                       'codelet', 'target', 'debug', 'fspec');
// REAL OS temp dir — no memfs, no in-process redirection.
const tmpWorkspace = mkdtempSync(join(tmpdir(), '<rpc-id>-'));

function bufferToText(buffer: ReadonlyArray<ReadonlyArray<string>>): string {
  return buffer.map(row => row.join('').trimEnd()).join('\n');
}

test.describe('<scenario name from feature file>', () => {
  test.use({
    program: { file: rustFspec, args: ['--workspace', tmpWorkspace] },
    rows: 40,
    columns: 160,
  });

  test('<then assertion>', async ({ terminal }) => {
    // @step Given … (set up real files in tmpWorkspace)
    // @step When … (real keystrokes via terminal.keyboard.* or terminal.write)
    // @step Then …
    await terminal.expectStable();                  // wait for repaint
    const text = bufferToText(terminal.getBuffer());
    writeFileSync(`/tmp/<rpc-id>_buffer.txt`, text); // for post-mortem
    await expect(terminal.getByText(/expected substring/)).toBeVisible();
    await expect(terminal.getByText(/panic|block_on/)).not.toBeVisible();
  });
});
```

**Rules** — non-negotiable:

1. **Real binary, real PTY, real temp dir.** No mocks. No mocked stdin.
   No spawn of `node dist/index.js` — this is a *Rust* port test.
2. **Always dump the buffer to `/tmp/<rpc-id>_buffer.txt`** before the
   assertion. The supervisor will read that file when the test fails;
   debugging an invisible PTY blind is a waste of cycles.
3. **One `test.describe` per Gherkin scenario.** `program.args` go in
   `test.use({...})` so each scenario gets its own PTY.
4. **Never widen `rows` / `columns` past 200 × 60** without a recorded
   reason — tui-test ships those values into the PTY and large terminals
   change ANSI output.
5. **`@step` comments are still mandatory** in TS tests. `link-coverage`
   verifies them in `.ts` files just like it does in `.rs`.

### 14.3 Real fixtures vs mocks — the line, drawn explicitly

> The cardinal rule, stated once and never broken: **fixtures, not mocks.**
> A *fixture* is real input the system processes (JSON files on disk,
> seeded SessionIds, captured help text). A *mock* is a stand-in that
> intercepts behaviour. Layers 3 and 4 of the pyramid use fixtures
> exclusively. Layer 2 uses an in-process *test double* (which Rust
> calls "Mock" by convention) but it is still a real implementation of
> the `FspecBackend` trait — no method interception, no `vi.fn()`-style
> spies. Layer 1 uses neither.

**The cheat sheet** (port-specific extract of `TESTING.md §"Integration
Tests Without Mocks"`):

| You're tempted to mock…                        | Instead, do this                                                   |
|------------------------------------------------|--------------------------------------------------------------------|
| `process.cwd()` / `env::current_dir()`         | `Command::new(fspec_bin()).current_dir(tempdir.path())`            |
| `$HOME` / `dirs::home_dir()`                   | Set `HOME` via `.env("HOME", tempdir.path())` on the `Command`     |
| `fs::read_to_string(config)`                   | Write the config to the real temp dir; let the binary read it      |
| A NAPI call                                    | Build the real NAPI module; invoke it (see TESTING.md §"Real NAPI")|
| A spawned shell hook                           | Drop a real `.sh` into the temp workspace and let it run           |
| `fetch()` / HTTP client                        | Use a real `httpmock::MockServer` on a real port (only exception)  |
| Backend trait calls (TUI tests, layer 2)       | `Arc<MockBackend>` — the real Rust struct with deterministic state |
| Stdin keystrokes (TUI tests, layer 4)          | `terminal.keyboard.press('Enter')` against the real PTY            |
| Time (`Instant::now`, animations)              | Inject a `Clock` trait — pass a deterministic clock fixture in     |
| Random / UUIDs                                 | Seed the RNG via a fixture; do not stub the call site              |

**`Arc<MockBackend>` is not a "mock" in the misleading sense.** It's a
real `impl FspecBackend` that records actions in counters and replays
them — every test that uses it (e.g. `behaviour_parity_rpc065.rs`,
`slash_clear_rpc046.rs`) drives the *real* `App` reducer through *real*
`Action` enum variants. The name predates this discipline. Do not invent
new mocks; reach for `MockBackend` (or extend it via a real method) when
Layer 2 is appropriate.

### 14.4 Fixture file conventions (Rust side)

Fixtures live in `codelet/fspec/tests/fixtures/`:

```
codelet/fspec/tests/fixtures/
├── help/
│   └── <command>.txt           ← byte-for-byte TS Commander.js help output
│                                 captured via: node dist/index.js <cmd> --help
│                                 > codelet/fspec/tests/fixtures/help/<cmd>.txt
│                                 (pipe to file — NOT TTY — so chalk reduces
│                                 to identity)
└── parity_<command>.txt        ← optional grep-style assertion that the
                                  bridge .rs has no inline rendering
```

**Seeded JSON fixtures** are inline string functions inside `cli_<name>.rs`,
not separate files (keeps each test self-contained):

```rust
fn canonical_<file>_json() -> String { r#"{ "...real schema..." }"# }

fn workspace_with_<file>_json(body: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("spec")).expect("mkdir spec");
    std::fs::write(dir.path().join("spec").join("<file>.json"), body).expect("write");
    dir
}
```

Why inline strings and not a `.json` file under `fixtures/`? Because every
ported command's seed shape is small (< 1 KB) and lives next to the test
that owns it. Shared fixtures across commands invite cross-test coupling
and silent breakage when a schema field is added — exactly what bit batch 3
(see `## 10. Gotchas`).

### 14.5 What a worker is REQUIRED to produce

When a worker delivers a port batch, the supervisor will reject the work
unless every one of these exists:

1. `codelet/fspec-core/src/commands/<snake>.rs` — unit-test module at
   bottom (Layer 1) using only in-memory JSON.
2. `codelet/fspec/tests/cli_<snake>.rs` — at least one `#[test] fn
   scenario_<snake>_<…>()` per Gherkin scenario in the feature file
   (Layer 3) with real `tempfile::TempDir`, real `Command::new(fspec_bin())`,
   real captured stdout/stderr.
3. `codelet/fspec/tests/fixtures/help/<command>.txt` — the help fixture,
   regenerated from the TS CLI (or marked Framing A in the architecture note
   if the TS CLI is broken — see `## 10`).
4. Optional `e2e/<rpc-id>-<slug>.test.ts` *only if* the command has TUI
   surface area. Do not write tui-test files for plain JSON-out commands.
5. `@step` comments matching the feature file 1:1, in every test file,
   in every layer that was written.
6. `link-coverage` invocations that wire the feature file's scenarios to
   the test file/line ranges.

If a worker submits a test file containing the strings `vi.fn(`, `jest.mock(`,
`unimplemented!()`, `todo!()`, or `#[mockall::*]` — the supervisor rejects
the batch and the worker rewrites against a real fixture.

### 14.6 Pitfalls (testing-specific addendum to `## 10`)

- **`#[ignore = "RPC-026: combined mode grabs /dev/tty"]`** — keep that
  ignore in place. If a test needs to exercise combined mode, write it
  at Layer 4 (tui-test), not Layer 3 (cargo).
- **`scrollback_pty_rpc078.rs` is `#[ignore]`'d on purpose** — porting work
  cannot un-ignore it casually. Cross-reference the RPC card first.
- **Don't widen the `tui-test` PTY past 200 columns** to "make the
  assertion fit" — change the assertion regex instead. Wide PTYs change
  wrap semantics and the test stops representing reality.
- **`./scripts/run-tui-test.sh` MUST be the entry point.** Bare
  `npx @microsoft/tui-test` skips the `tmp/` stash and produces swc
  parse errors that look like test failures but aren't.
- **Help fixtures get stale.** If `node dist/index.js <cmd> --help`
  changes (Commander.js minor bump, chalk wrap change), the fixture
  regenerates — but only against a clean `dist/` build. Run
  `npm run build` before re-capturing.
- **Layer 4 tests run last in CI** and they are the slowest. Do not
  reach for tui-test when a Layer 3 stdout assertion would do.
