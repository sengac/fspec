# AST Research — RPC-186 `add-persona` Rust port

Tooling: AstGrep (rust/typescript) + Grep. Date 2026-06-12.

## 1. Current Rust stub — `codelet/fspec-core/src/commands/add_persona.rs`

AstGrep `pub async fn run($$$ARGS) -> Result<String, FspecCoreError> { $$$BODY }`:

```
add_persona.rs:6:  pub async fn run(_args_json: &str) -> Result<String, FspecCoreError>
```

- **1-arg** stub returning `FspecCoreError::NotYetPorted { command:"add-persona", work_unit:"RPC-186" }`.
- Must be rewritten to the **2-arg** form (see §2). This is a SHARED-FILE-impacting change
  (dispatch.rs / canonical.rs route the stub) — flagged to supervisor.

## 2. Reference template — `codelet/fspec-core/src/commands/add_diagram.rs`

AstGrep confirms the canonical foundation-mutation signature:

```
add_diagram.rs:55:  pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```

Pattern to copy:
- `#[derive(Debug, Default, Deserialize)] #[serde(default, rename_all="camelCase")] struct AddPersonaArgs`
- arg-parse → `FspecCoreError::InvalidArgs { command, reason: "failed to parse args: {e}" }`
- mutate a `serde_json::Value` root (preserve_order keeps unknown top-level keys + key order)
- serialize result via `serde_json::to_string(&json!({ "success": true, ... }))`.

**Divergence from add_diagram**: add_diagram uses `ensure_foundation_file` (auto-creates).
add-persona must NOT auto-create — see §4 rule 2.

## 3. IO helpers — `codelet/fspec-core/src/io/`

- `io::ensure::ensure_foundation_file` — auto-creates canonical v2.0.0 default. **NOT used here**
  (TS add-persona errors on ENOENT instead of creating).
- `io::ensure::foundation_initial()` — shows canonical `personas` shape:
  `[{ "name","description","goals":[...] }]`.
- `io::locked_file::write_json_atomic` — pretty 2-space, **NO** trailing newline.
- `io::locked_file::write_json_atomic_trailing_newline` (locked_file.rs:110, added by supervisor) —
  pretty 2-space **+ single trailing `\n`**. **USE THIS** — TS appends `JSON.stringify(...,2)+'\n'`.
  Note it `create_dir_all`s the parent; harmless since we only write when the foundation file exists.

## 4. TS source — `src/commands/add-persona.ts` (behaviour map)

- `isPlaceholderPersona(p)`: regex `/\[QUESTION:|\[DETECTED:/` tested against
  `p.name`, `p.description`, and **each** `p.goals[]`.
- `hasOnlyPlaceholders(personas)`: `false` when array empty; else `every(isPlaceholder)`.
- `addPersona(cwd,name,description,goals)`:
  1. **Draft precedence**: `fs.access(spec/foundation.json.draft)` → target=draft, isDraft=true;
     else target=`spec/foundation.json`.
  2. Read+parse target. **ENOENT** → print `✗ foundation.json not found` + hint,
     `throw new Error('foundation.json not found')`. **No auto-create.**
  3. `if (!foundation.personas) foundation.personas = []`.
  4. `if (hasOnlyPlaceholders(personas)) { removedCount = len; personas = [] }`.
  5. `personas.push({ name, description, goals })`.
  6. `fs.writeFile(target, JSON.stringify(foundation,null,2) + '\n')`.
  7. stdout: optional `Removed ${removedCount} placeholder persona(s)`,
     then `✓ Added persona to ${fileName}`, `  Name: ${name}`,
     `  Description: ${description}`, `  Goals: ${goals.join(', ')}`.
  - `fileName` = `foundation.json.draft` when isDraft else `foundation.json`.

CLI registration (`register-add-persona.ts`): positional `<name> <description>`,
repeatable `--goal` accumulating into `string[]` (default `[]`); error → `output.error` + `exit(1)`.

## 5. Type shape — `src/types/generic-foundation.ts:154`

```ts
interface Persona { name: string; description: string; goals?: string[]; painPoints?: string[]; }
```

**KEY FINDING**: Persona may carry `painPoints`. add-persona only WRITES `{name,description,goals}`,
but existing personas may have `painPoints`. To preserve unknown fields on round-trip, model the
personas array as untyped `serde_json::Value` (NOT a typed struct) when reading/rewriting —
mirrors add_diagram's `Value`-root approach.

## 6. Planned core signature & result

```rust
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
// args (camelCase): { name: String, description: String, goals: Vec<String> = [] }
// result: { success: true, fileName, removedPlaceholders, name, description, goals }
```
New persona inserted as a `serde_json::Map` in key order `name, description, goals`.
