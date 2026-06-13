# AST Research — RPC-277 `remove-persona` Rust port

Tooling: AstGrep (rust/typescript) + Grep. Date 2026-06-12.

## 1. Current Rust stub — `codelet/fspec-core/src/commands/remove_persona.rs`

AstGrep `pub async fn run($$$ARGS) -> Result<String, FspecCoreError> { $$$BODY }`:

```
remove_persona.rs:6:  pub async fn run(_args_json: &str) -> Result<String, FspecCoreError>
```

- **1-arg** stub returning `FspecCoreError::NotYetPorted { command:"remove-persona", work_unit:"RPC-277" }`.
- Rewrite to **2-arg** form `run(args_json, project_root)` (SHARED-FILE-impacting; flagged to supervisor).

## 2. Reference template — `codelet/fspec-core/src/commands/add_diagram.rs`

```
add_diagram.rs:55:  pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```
Copy: camelCase `#[serde(default)]` args struct, `Value`-root mutation (preserve_order keeps unknown
top-level keys + ordering), `InvalidArgs` for parse/serialize failures, `json!` result envelope.

## 3. IO helpers — `codelet/fspec-core/src/io/`

- `io::ensure::ensure_foundation_file` — auto-creates. **NOT used** (remove-persona errors on ENOENT).
- `io::locked_file::write_json_atomic_trailing_newline` (locked_file.rs:110) — pretty 2-space **+ `\n`**.
  **USE THIS** — TS `removePersona` writes `JSON.stringify(foundation,null,2) + '\n'`.
- Plain `write_json_atomic` omits the newline → would break byte parity. Do not use it.

## 4. TS source — `src/commands/remove-persona.ts` (behaviour map)

`removePersona(cwd, name)`:
1. **Draft precedence**: `fs.access(spec/foundation.json.draft)` → target=draft, isDraft=true;
   else target=`spec/foundation.json`.
2. Read+parse target. **ENOENT** → `✗ foundation.json not found` + hint,
   `throw new Error('foundation.json not found')`. **No auto-create.**
3. **Empty/absent personas**: `if (!foundation.personas || personas.length === 0)` →
   `✗ Persona "${name}" not found`, `  No personas exist in foundation`,
   `throw new Error('Persona "${name}" not found')`.
4. **Match**: `index = personas.findIndex(p => p.name === name)` — exact, **case-sensitive**.
5. **No match** (`index === -1`): build `availableNames = personas.map(p=>p.name).join(', ')`,
   `✗ Persona "${name}" not found`, `  Available personas: ${availableNames}`,
   `throw new Error('Persona "${name}" not found')`.
6. **Remove**: `personas.splice(index, 1)` — removes **only the FIRST** match.
7. `fs.writeFile(target, JSON.stringify(foundation,null,2) + '\n')`.
8. stdout: `✓ Removed persona "${name}" from ${fileName}`.
   `fileName` = `foundation.json.draft` when isDraft else `foundation.json`.

CLI registration (`register-remove-persona.ts`): positional `<name>`; success `exit(0)`;
any error → `exit(1)` (NB: register-remove-persona swallows the message and just exits 1 — the
human-facing message text is emitted by `removePersona` itself via `output.error`).

## 5. Type shape — `src/types/generic-foundation.ts:154`

```ts
interface Persona { name: string; description: string; goals?: string[]; painPoints?: string[]; }
```
Match/removal only touches `name`. Other personas (incl. `painPoints`) round-trip untouched →
model the personas array as untyped `serde_json::Value` to preserve unknown fields & ordering.

## 6. Planned core signature & result

```rust
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
// args (camelCase): { name: String }
// success result: { success: true, fileName, name }
// failures -> FspecCoreError::InvalidArgs (CLI bridge strips dispatch envelope, exit 1)
```

### Error-message parity targets (substring-asserted by tests)
- `foundation.json not found`
- `Persona "<name>" not found`
- `No personas exist in foundation`
- `Available personas: <names joined ', '>`
