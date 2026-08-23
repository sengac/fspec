# BLOCK-012 Research — Auto-installing the default system blocklist template

## Goal

Every user should get sensible default blocklist protections (dangerous
commands, sensitive file paths, agent-loop protection) on first run, with no
manual setup. The template is the contents of `~/blocklist.json` (version
`1.0.0`, 68 rules covering Windows **and** Linux). It is embedded in the
binary at compile time and written to `~/.fspec/blocklist.json` the moment
the blocklist loader checks for that file and finds it missing — then loaded
and used in the same process, same call.

## Current state (as of research)

### Blocklist loading — `rust/tools/src/blocklist/`

- `config.rs` — `BlocklistConfig { version, rules: Vec<BlocklistRule> }`,
  `BlocklistRule { id, pattern, action, reason, guidance }`,
  `BlocklistAction { Block, Allow, Prompt }`. Has `load_from_file` /
  `save_to_file` (creates parent dirs).
- `middleware.rs` — the runtime layer:
  - `system_config_path()` → `dirs::home_dir()/.fspec/blocklist.json`
    (on Linux this is `$HOME/.fspec/blocklist.json` — tests can redirect
    `HOME` to isolate it).
  - `project_config_path(root)` → `<root>/.fspec/blocklist.json`.
  - `load_blocklist_config(project_root: Option<&Path>)` — **the single
    chokepoint**. It checks `system_path.exists()`, loads it, does the same
    for the project path, then `BlocklistConfig::merge(system, project)`.
    Today, a missing system file simply yields `BlocklistConfig::empty()`
    — no rules, no protection.
  - `init_blocklist(project_root)` — called at service startup
    (`rust/fspec/src/common.rs::build_service`, per RPC-407). Also calls
    `load_blocklist_config`.
  - `check_bash_command` / `check_file_path` — **hot-reload**: every check
    call re-invokes `load_blocklist_config` and rebuilds the matcher. So any
    logic placed in `load_blocklist_config` is exercised on *every* command
    and file check, not just at startup.
  - `reload_blocklist` also goes through `load_blocklist_config`.

### Callers of the chokepoint

| Caller | Crate | Notes |
|---|---|---|
| `init_blocklist` | codelet-tools (startup via `build_service`, RPC-407) | daemon + combined modes |
| `check_bash_command` | codelet-tools — `bash.rs`, `unified_exec/tool.rs` | every Bash/unified-exec tool call |
| `check_file_path` | codelet-tools — `read.rs`, `write.rs`, `edit.rs`, `apply_patch/mod.rs` | every file tool call |
| `reload_blocklist` | codelet-tools | TUI /blocklist refresh |
| napi `blocklist_load` | codelet-napi (`napi/src/blocklist.rs`) | legacy TS shell path |

Because `load_blocklist_config` is the one function every path funnels
through, placing the check-then-write there covers all entry points with a
single change.

### Precedent for embedded templates in this workspace

- `rust/sessions/src/codex_allowlist.rs` (PROV-129):
  `const BUNDLED_ALLOWLIST_JSON: &str = include_str!("../data/codex-models.json");`
  with a user-override at `~/.fspec/codex-models.json` taking precedence.
  Unit test asserts the bundled JSON parses and is non-empty.
- `rust/fspec-core/src/generators/foundation_schema.rs` /
  `tags_schema.rs`: `include_str!` of bundled JSON schemas.
- `rust/fspec-core/src/commands/init.rs`: `include_str!` of bundled markdown
  docs.

So `include_str!` of a `data/*.json` file inside the crate is an established
pattern. `codelet-tools` has no `data/` dir yet — we create
`rust/tools/data/default-blocklist.json`.

### Test isolation precedent

`rust/fspec/src/blocklist_init_tests.rs` (RPC-407) defines a
`GlobalBlocklistGuard` RAII guard:

- redirects `HOME` to a fresh empty tempdir for the test's duration
  (so the real user system blocklist can never interfere),
- on drop: `clear_session_allowances()`, `init_blocklist(None)`, restores
  `HOME` — even on panic.

All tests touching the process-global blocklist state are `#[serial]`
(`serial_test`). The new tests must use the same guard pattern because
auto-install writes to `$HOME/.fspec/blocklist.json`.

⚠️ **Side effect on this machine**: the real `~/.fspec/blocklist.json`
does not currently exist here, so the first real run of the fspec binary
after this lands will create it from the template. That is the intended
product behavior.

## Design

### 1. Template file

- Copy `~/blocklist.json` verbatim to `rust/tools/data/default-blocklist.json`.
- Keep it byte-stable (it is the source of truth for what users get).

### 2. Embedding (in `rust/tools/src/blocklist/config.rs` or a new
`template.rs` — decision: new `template.rs` to keep `config.rs` under the
300-line guideline)

```rust
/// Bundled default blocklist template — embedded at compile time (BLOCK-012).
pub const DEFAULT_BLOCKLIST_TEMPLATE: &str =
    include_str!("../../data/default-blocklist.json");

/// Parse the embedded template.
pub fn default_blocklist_config() -> Result<BlocklistConfig, serde_json::Error> {
    serde_json::from_str(DEFAULT_BLOCKLIST_TEMPLATE)
}
```

Unit test (rule 4): `default_blocklist_config()` must parse, have version
`1.0.0` and exactly 68 rules. This fails the test suite if the template is
ever corrupted.

### 3. Check-then-write (in `middleware.rs::load_blocklist_config`)

Insert at the top of the system-config section, **before** the load:

```rust
if let Some(system_path) = system_config_path() {
    if !system_path.exists() {
        install_default_system_blocklist(&system_path);
    }
    if system_path.exists() {
        // ... existing load ...
    }
}
```

```rust
/// Write the embedded default template to `path` if it is missing.
/// Failures (no home dir, read-only fs) are logged and swallowed — a failed
/// install must never break command checking (rule 5).
fn install_default_system_blocklist(path: &Path) {
    match default_blocklist_config() {
        Ok(config) => match config.save_to_file(path) {
            Ok(()) => info!("Installed default blocklist template at {path:?}"),
            Err(e) => warn!("Failed to install default blocklist at {path:?}: {e}"),
        },
        Err(e) => warn!("Embedded default blocklist template failed to parse: {e}"),
    }
}
```

Why this satisfies the "same process, same call" requirement:
`load_blocklist_config` is called synchronously by `check_bash_command` /
`check_file_path` on every check. The write happens *before* the load in the
same function, so the freshly written file is the one that gets loaded and
matched against in that very call. No startup-only seam, no restart needed.

Idempotency (rule 6): the `!exists()` guard makes the check-then-write safe
to repeat on every call; if the user deletes the file mid-session, the next
check re-installs it.

Existing-file safety (rule 2): the `!exists()` guard means a user-edited
blocklist is never touched.

### 4. What is NOT changed

- Project blocklist (`.fspec/blocklist.json`) behavior — unchanged.
- Merge order (project first, then system) — unchanged.
- `init_blocklist` / RPC-407 startup seam — unchanged (it inherits the
  behavior for free).
- Session allowances, prompt flow, TUI /blocklist view — unchanged.

## Risks / edge cases

| Case | Handling |
|---|---|
| `HOME` unset / `dirs::home_dir()` → `None` | `system_config_path()` returns `None`; no install, no error (existing behavior) |
| `~/.fspec` not writable / read-only fs | `save_to_file` fails → `warn!` log, check proceeds with empty system config (graceful degrade, rule 5) |
| User deletes file mid-session | Next check re-installs (idempotent, rule 6) |
| Concurrency: two processes check simultaneously | Both may write the same template content; last writer wins with identical bytes — harmless. (No lock needed; could add a temp-file-rename atomic write later if desired.) |
| Template drift between releases | Template is embedded per-binary; users who already have a file keep theirs (rule 2). Documented limitation: upgrades do not refresh an existing file. |
| Tests that redirect `HOME` to a tempdir | Auto-install now writes into the tempdir — which is exactly what the tests want; the `GlobalBlocklistGuard` pattern (restore `HOME` on drop) keeps the real home clean |

## Test plan (ACDD phase 2)

New integration test file: `rust/tools/tests/block012_default_template_install.rs`
(feature: `spec/features/default-system-blocklist-template-install.feature`).

Shared `HomeGuard` (mirrors RPC-407's `GlobalBlocklistGuard`):
redirect `HOME` → fresh tempdir; on drop restore `HOME`,
`clear_session_allowances()`, `init_blocklist(None)`. All tests `#[serial]`.

Scenarios → tests:

1. **Template installs and is active in the same call**
   - Given: `HOME` = empty tempdir (no `~/.fspec/blocklist.json`)
   - When: `check_bash_command("git checkout main", uuid)`
   - Then: `~/.fspec/blocklist.json` exists on disk
   - And: the command is blocked with rule id `git-checkout-block`
   - And: the on-disk JSON parses as `BlocklistConfig` with 68 rules

2. **Existing user blocklist is never overwritten**
   - Given: `HOME` tempdir with a custom `~/.fspec/blocklist.json`
     (1 rule: block pattern `sentinel-block012`)
   - When: `load_blocklist_config(None)` (via `init_blocklist` + a check)
   - Then: file content is byte-identical to the custom one
   - And: the sentinel command is blocked by the user rule
   - And: the template's `git-checkout-block` rule is NOT present in the file

3. **Deleted file is re-installed on the next check (idempotent)**
   - Given: `HOME` tempdir; one check already installed the template
   - When: delete `~/.fspec/blocklist.json`, then run another
     `check_bash_command("git checkout main", uuid)`
   - Then: the file exists again and the command is blocked

4. **Write failure degrades gracefully**
   - Given: `HOME` tempdir where `~/.fspec` exists as a **regular file**
     (so `create_dir_all`/write fails)
   - When: `check_bash_command("echo hello", uuid)`
   - Then: the call returns `Ok(())` (no panic, no error from the install
     path) — checking still works

Unit tests (inline in `template.rs`):

5. **Embedded template is valid** — parses to `BlocklistConfig`,
   version `1.0.0`, exactly 68 rules, rule ids unique.

## File change summary

| File | Change |
|---|---|
| `rust/tools/data/default-blocklist.json` | NEW — copy of `~/blocklist.json` |
| `rust/tools/src/blocklist/template.rs` | NEW — embedded const + parse + install fn + unit tests |
| `rust/tools/src/blocklist/mod.rs` | + `mod template;` and re-exports |
| `rust/tools/src/blocklist/middleware.rs` | check-then-write in `load_blocklist_config` |
| `rust/tools/tests/block012_default_template_install.rs` | NEW — integration tests |
| `spec/features/default-system-blocklist-template-install.feature` | NEW — Gherkin |
