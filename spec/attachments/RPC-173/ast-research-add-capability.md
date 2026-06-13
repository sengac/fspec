# AST Research — RPC-173 `add-capability` Rust port

Date: 2026-06-12 · Phase A (specifying) discovery artifact

## 1. Current Rust stub (rewrite target)

`codelet/fspec-core/src/commands/add_capability.rs`

```
pub async fn run(_args_json: &str) -> Result<String, FspecCoreError>   // 1-arg stub, returns NotYetPorted
```

AstGrep (`pub async fn run($$$ARGS) -> $RET { $$$BODY }`) confirms the stub
is a single 1-arg fn. Must become the **2-arg** signature:

```
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```

## 2. Reference template — `add_diagram.rs` (RPC-178, foundation mutation)

AstGrep match: `add_diagram.rs:55  pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`

Pattern to copy:
- `#[derive(Debug, Default, Deserialize)] #[serde(default)] struct AddCapabilityArgs { name: String, description: String }`
- parse args → `serde_json::from_str` → on error `FspecCoreError::InvalidArgs { command: "add-capability", reason }`
- mutate a `serde_json::Value` root object (preserves unknown fields)
- serialize result with `serde_json::to_string(&json!({...}))`

KEY DIVERGENCE from add_diagram: add_diagram uses `ensure_foundation_file`
(auto-creates). **add-capability MUST NOT** — see §4.

## 3. TS source semantics (`src/commands/add-capability.ts`)

Two module-private helpers (no AstGrep match because they're `function`
declarations the tool elides, read directly):
- `isPlaceholderCapability({name,description})` → regex `/\[QUESTION:|\[DETECTED:/`
  tested against name OR description.
- `hasOnlyPlaceholders(caps)` → `caps.length>0 && caps.every(isPlaceholder)`.

`addCapability(cwd, name, description)` flow:
1. `draftPath = spec/foundation.json.draft`, `foundationPath = spec/foundation.json`.
2. `fs.access(draftPath)` → if ok, target=draft, isDraft=true; else target=foundation.
3. `readFile(target)` → JSON.parse. ENOENT branch: prints `✗ foundation.json not found`
   + yellow hint, `throw new Error('foundation.json not found')`. **No file created.**
4. `if (!solutionSpace.capabilities) capabilities = []`.
5. `if (hasOnlyPlaceholders(caps)) { removedCount = caps.length; caps = [] }`.
6. `caps.push({name, description})`.
7. `writeFile(target, JSON.stringify(foundation, null, 2) + '\n')`.  ← trailing `\n`
8. stdout: optional `Removed N placeholder capability(ies)` (yellow), then
   `✓ Added capability to <fileName>`, `  Name: <name>`, `  Description: <description>`.

## 4. IO helpers (`codelet/fspec-core/src/io/`)

- `ensure::ensure_foundation_file(cwd) -> Result<serde_json::Value, _>` — AUTO-CREATES.
  **DO NOT USE** for add-capability (TS throws on ENOENT, never creates).
  → Probe `project_root.join("spec/foundation.json.draft").exists()` manually;
    read via `std::fs::read_to_string` + `serde_json::from_str::<Value>`; on
    ENOENT of both files return `FspecCoreError::InvalidArgs { reason: "foundation.json not found" }`.
- `locked_file::write_json_atomic_trailing_newline(path, value)` — **ALREADY EXISTS**,
  public, and its doc-comment explicitly names `add-capability`/`remove-capability`
  as intended callers. Emits `to_string_pretty` + single `\n`. ✅ Resolves the
  trailing-newline parity concern — **NO shared-file change needed**, no
  module-local write helper required.

## 5. Error variants available (`error.rs`)

`InvalidArgs { command:&'static str, reason:String }` (dispatcher-wrapped),
`Io`, `ParseJson { file, reason }`, `FoundationMissing(String)`.
Plan: use `InvalidArgs` with reason `"foundation.json not found"` (CLI bridge
strips the dispatcher envelope via `common::strip_dispatch_envelope` /
`render_core_error`).

## 6. CLI bridge pattern (`codelet/fspec/src/`)

- `render_core_error(err)` (common.rs:489) → returns `reason` for InvalidArgs,
  else Display verbatim. Bridges print `eprintln!("Error: {}", render_core_error(&err))`.
- `add_diagram.rs:63` uses `Error:` prefix; bounded-context uses `strip_dispatch_envelope`.
- New bridge `codelet/fspec/src/add_capability.rs`: build JSON `{name, description}`,
  `dispatch_command`, on success print the success block, on failure exit 1.

## 7. Dispatcher / wiring (SUPERVISOR-owned — for Phase C)

- `dispatch.rs:142 fn run_ported(name, args_json, project_root)` routes ported
  commands; `add-diagram` lives here (`:356`). add-capability currently at
  `:437` in the NOT-ported block calling 1-arg `run(args_json)` → must move to
  run_ported, 2-arg.
- `canonical.rs:33` declares the command; `:275` is the PORTED list (add to it).
- `main.rs:1022/1677/2176` show the add-diagram clap Mode + dispatch + help routing.
- `help/configs/mod.rs:80` registers help configs.
