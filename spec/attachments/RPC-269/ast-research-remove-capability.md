# AST Research — RPC-269 `remove-capability` Rust port

Date: 2026-06-12 · Phase A (specifying) discovery artifact

## 1. Current Rust stub (rewrite target)

`codelet/fspec-core/src/commands/remove_capability.rs`

```
pub async fn run(_args_json: &str) -> Result<String, FspecCoreError>   // 1-arg stub, returns NotYetPorted
```

AstGrep (`pub async fn run($$$ARGS) -> $RET { $$$BODY }`) confirms the stub
is a single 1-arg fn. Must become the **2-arg** signature:

```
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```

Args struct (camelCase): `{ name: String }`.

## 2. Reference template — `add_diagram.rs` (RPC-178, foundation mutation)

AstGrep match: `add_diagram.rs:55  pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`

Copy: args struct + `serde_json::from_str` → `InvalidArgs`, mutate
`serde_json::Value` root (preserves unknown fields), serialize result with
`to_string(&json!({...}))`. As with add-capability, do NOT use
`ensure_foundation_file` (no auto-create — see §4).

## 3. TS source semantics (`src/commands/remove-capability.ts`)

`removeCapability(cwd, name)` flow (no module-private helpers):
1. draft precedence: `fs.access(spec/foundation.json.draft)` → target=draft else foundation.
2. `readFile(target)` → JSON.parse. ENOENT: prints `✗ foundation.json not found`
   + hint, `throw new Error('foundation.json not found')`. **No file created.**
3. Empty/missing guard: `if (!caps || caps.length === 0)` →
   prints `✗ Capability "<name>" not found` + `  No capabilities exist in foundation`,
   `throw new Error('Capability "<name>" not found')`.
4. Lookup: `index = caps.findIndex(c => c.name === name)` (EXACT, case-sensitive,
   FIRST match). `if (index === -1)` →
   prints `✗ Capability "<name>" not found` + `  Available capabilities: <names.join(', ')>`,
   throw same.
5. `caps.splice(index, 1)`  (only ONE entry removed; non-idempotent guard is the
   exact-match itself).
6. `writeFile(target, JSON.stringify(foundation, null, 2) + '\n')`.  ← trailing `\n`
7. stdout: `✓ Removed capability "<name>" from <fileName>`.

## 4. IO helpers (`codelet/fspec-core/src/io/`)

- `ensure::ensure_foundation_file` — AUTO-CREATES → **DO NOT USE**. Probe draft
  `.exists()` manually; `std::fs::read_to_string` + `from_str::<Value>`; both
  missing → `InvalidArgs { reason: "foundation.json not found" }`.
- `locked_file::write_json_atomic_trailing_newline(path, value)` — **ALREADY
  EXISTS & PUBLIC**; doc-comment explicitly names `remove-capability` as an
  intended caller. ✅ NO shared-file change needed for trailing-newline parity.

Removal mechanics in Rust: navigate
`root["solutionSpace"]["capabilities"].as_array_mut()`; find first index where
`entry["name"].as_str() == Some(name)`; `vec.remove(index)`.

## 5. Error variants (`error.rs`) + detail-line strategy

`InvalidArgs { command, reason }`. The TS not-found paths emit TWO stderr lines
(`✗ ...not found` + a detail line). Strategy: pack BOTH lines into the
`reason` string (e.g. `Capability "X" not found\nNo capabilities exist in
foundation` / `... \nAvailable capabilities: A, B`) so the dispatcher contract
and the CLI bridge both surface the detail. Feature scenarios assert on the
substrings `'Capability "X" not found'`, `'No capabilities exist in foundation'`,
and `'Available capabilities: Reporting, Search'`.

## 6. CLI bridge pattern (`codelet/fspec/src/`)

`render_core_error` (common.rs:489) returns `reason` for InvalidArgs verbatim
(including embedded `\n` detail lines). New bridge
`codelet/fspec/src/remove_capability.rs`: build JSON `{name}`,
`dispatch_command`, success → `✓ Removed capability "<name>" from <fileName>`,
failure → exit 1 printing the rendered reason.

## 7. Dispatcher / wiring (SUPERVISOR-owned — for Phase C)

- `dispatch.rs:580` currently routes `remove-capability` in the NOT-ported
  block calling 1-arg `run(args_json)` → move to `run_ported` (`:142`), 2-arg.
- `canonical.rs:129` declares the command; `:275`-area PORTED list (add it).
- `main.rs`: add clap `Mode::RemoveCapability { name }` + dispatch + help routing
  (mirror add_diagram @ 1022/1677/2176).
- `help/configs/mod.rs:80`: register `pub mod remove_capability;`.
