# Batch 7 Worker Instructions (read in full BEFORE starting)

PROJECT ROOT: `/home/rquast/projects/fspec`
SUPERVISOR SESSION_ID: `30167441-9c28-4e18-8db9-11e53befc0a4`
CARGO RUNNER AGENT ID: `d6a0bcba-6ac6-4943-92fc-6f1287df22cc`

## Authoritative playbook
- `command-port.md` (root)  — full porting reference
- `codelet/fspec-core/src/commands/list_prefixes.rs` — canonical impl
- `codelet/fspec/src/list_prefixes.rs` — CLI bridge reference
- `codelet/fspec/tests/cli_list_prefixes.rs` — CLI test reference
- `codelet/fspec-core/src/help/configs/list_prefixes.rs` — help config
- `codelet/fspec/tests/fixtures/help/list-prefixes.txt` — help fixture
- `codelet/fspec-core/src/io/ensure.rs` + `io/locked_file.rs` — IO helpers
- `codelet/fspec-core/src/types/{prefix,epic,tags,work_unit}.rs` — shared types
- `codelet/fspec-core/src/commands/show_epic.rs` — example reading epics.json

## File ownership — YOU MAY CREATE / EDIT (parallel-safe)
- `spec/features/<your-cmd-kebab>-rust-port.feature`
- `spec/features/<your-cmd-kebab>-cli-subcommand.feature`
- `spec/features/<your-cmd-kebab>-*.feature.coverage` (auto)
- `spec/attachments/<RPC-ID>/ast-research-<your-cmd-kebab>.md`
- `codelet/fspec-core/src/commands/<your_cmd_snake>.rs` (rewrite the stub)
- `codelet/fspec-core/tests/<your_cmd_snake>.rs` (NEW dispatcher test)
- `codelet/fspec-core/src/help/configs/<your_cmd_snake>.rs` (NEW help config)
- `codelet/fspec/src/<your_cmd_snake>.rs` (NEW CLI bridge)
- `codelet/fspec/tests/cli_<your_cmd_snake>.rs` (NEW CLI test)
- `codelet/fspec/tests/fixtures/help/<your-cmd-kebab>.txt` (NEW help fixture)

## YOU MUST NOT TOUCH (supervisor-only — ask supervisor for changes)
- `codelet/fspec-core/src/canonical.rs`
- `codelet/fspec-core/src/dispatch.rs`
- `codelet/fspec-core/src/commands/mod.rs`
- `codelet/fspec-core/src/types/mod.rs`
- `codelet/fspec-core/src/help/configs/mod.rs`
- `codelet/fspec-core/src/io/ensure.rs` (READ-ONLY; ASK supervisor for new helpers)
- `codelet/fspec-core/src/types/*.rs` (READ-ONLY for changes)
- `codelet/fspec/src/main.rs`
- `codelet/fspec/tests/cargo_shape.rs`
- `Cargo.toml` files
- Any other worker's feature files

## 3-Phase cycle for each command

### PHASE A — SPECIFYING (work unit already in `specifying`)
1. Read canonical reference files + TS source for THIS command (`src/commands/<name>.ts`).
2. Save AST research to `spec/attachments/<RPC-ID>/ast-research-<cmd>.md`.
3. Fspec: `add-attachment`, `set-user-story`, `add-rule` (per TS observation), `add-example` (concrete scenarios), `add-architecture-note` (file layout).
4. Fspec: `update-work-unit-estimate` (Fibonacci 1/2/3/5).
5. Fspec: `generate-scenarios` TWICE: once for `*-rust-port.feature` (dispatcher contract), once for `*-cli-subcommand.feature` (clap surface).
6. Fspec: `add-tag-to-feature @wip` on both files.
7. Fspec: `validate` (only on YOUR two new files).
8. Fspec: `update-work-unit-status <id> testing`.

### PHASE B — TESTING
1. Write `codelet/fspec-core/tests/<cmd>.rs` — dispatcher test, one fn per Gherkin scenario, `@step` comments verbatim.
2. Write `codelet/fspec/tests/cli_<cmd>.rs` — CLI shell test.
3. Capture help fixture. TS build is NOT cargo and does NOT conflict — you may run it yourself:
   ```
   cd /home/rquast/projects/fspec && node dist/index.js <name> --help > codelet/fspec/tests/fixtures/help/<name>.txt 2>&1
   ```
   If `dist/` is stale ask CARGO RUNNER to `npm run build` first.
4. Fspec `link-coverage` for each scenario → test file + lines.
5. Fspec: `update-work-unit-status <id> implementing`.

### PHASE C — IMPLEMENTING
1. Replace stub at `codelet/fspec-core/src/commands/<cmd>.rs`.
2. Write `codelet/fspec-core/src/help/configs/<cmd>.rs`.
3. Write `codelet/fspec/src/<cmd>.rs` CLI bridge.
4. Fspec `link-coverage` with impl-file + impl-lines.
5. REPORT BACK to supervisor: file paths, current state, any shared-file change requests (e.g. "supervisor please add `read_<x>_or_empty` to `io/ensure.rs`"). STOP. Wait for supervisor.

## CARGO RUNNER PROTOCOL
- NEVER run `cargo build`, `cargo test`, etc. yourself.
- Send messages to CARGO RUNNER:
  ```
  AgentManager.message session_id=d6a0bcba-6ac6-4943-92fc-6f1287df22cc
    message="Please run: cd codelet && cargo test --release -p codelet-fspec-core --test <cmd> 2>&1 | tee /tmp/test-<cmd>.txt"
  ```
- Then `AgentManager.await_idle session_id=d6a0bcba-6ac6-4943-92fc-6f1287df22cc`.
- Read the tee file when it reports back.
- `npm run build` IS allowed via cargo runner (or run yourself — but cargo runner serializes everything cleanly).

## Worker philosophy
- Tests must FAIL with `NotYetPorted` at end of Phase B (supervisor hasn't wired dispatch yet) — that's correct.
- Tests should PASS at end of Phase C — but only AFTER supervisor wires shared files. Phase C green tests are NOT your responsibility; supervisor triggers them.
- Core impl 200-440 LOC, CLI bridge 90-170 LOC, integration test 300-530 LOC.
- Use `#[derive(Serialize)]` structs for JSON output (preserves field order — `json!{}` alphabetizes).
- ALL on-disk structs need `#[serde(flatten)] pub extra: serde_json::Map<String, Value>` to preserve unknown fields.
- For mutation commands, use `io::locked_file::write_json_atomic` for atomic writes.
- Use `IndexMap` for ordered maps (not `HashMap` / `BTreeMap`).

## Two-front-doors invariant (RPC-003 §7/§11)
Both invocation paths (LLM dispatcher AND CLI shell) call the SAME `async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` in `commands/<cmd>.rs`. The CLI bridge marshals clap args to JSON and re-enters via `<cmd>::run(&args_json, &project_root).await`. NO logic in the bridge — JSON marshalling only.

## After completing each command
- Move work unit to `validating` then later supervisor will move to `done`.
- Each worker handles 2 commands. After Phase C of command #1, STOP — supervisor wires dispatch, validates, then tells you to start command #2.
