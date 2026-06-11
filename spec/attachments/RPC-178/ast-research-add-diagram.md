# AST Research — `add-diagram` Rust Port (RPC-178)

## TS source-of-truth

- `src/commands/add-diagram.ts` (170 LOC)
- `src/commands/add-diagram-help.ts` (~100 LOC)
- `src/utils/mermaid-validation.ts` (~176 LOC) — JSDOM-based mermaid.parse() validation
- `src/utils/ensure-files.ts` (ensureFoundationFile)
- `src/generators/foundation-md.ts` (generateFoundationMd)
- `src/validators/validate-json-schema.ts` (validateFoundationJson)

## Public surface

```typescript
addDiagram({ section, title, code, description?, cwd? })
  → Promise<{ success: boolean; message?: string; error?: string }>
addDiagramCommand(section, title, code) → process.exit(0|1)
```

CLI: `fspec add-diagram <section> <title> <code>`

## Behaviour summary

1. Validate non-empty section / title / code → return error sentinel.
2. Mermaid syntax validation via `validateMermaidSyntax(code)` (JSDOM + mermaid.render).
3. `ensureFoundationFile(cwd)` loads (or initialises) `spec/foundation.json`.
4. Ensure `architectureDiagrams: []` exists on the foundation object.
5. Find existing diagram by `title` (NOTE: `section` argument is accepted but NOT persisted to JSON — the generic schema v2.0.0 has no `section` field).
6. Build `{ title, mermaidCode, [description] }` payload.
7. If existing title found → replace at index; else push to end.
8. Atomic write `foundation.json` (pretty 2-space indent).
9. Schema validation via Ajv (`validateFoundationJson`) — fail with detailed error list.
10. Regenerate `spec/FOUNDATION.md` via `generateFoundationMd(data)`.
11. Success message: `Updated diagram "<title>"` or `Added diagram "<title>"`.

CLI prints two extra lines:
```
  Updated: spec/foundation.json
  Regenerated: spec/FOUNDATION.md
```

## Existing Rust scaffolding available

- `ensure_foundation_file(cwd)` in `codelet/fspec-core/src/io/ensure.rs` returns `serde_json::Value`.
- `write_json_atomic` in `codelet/fspec-core/src/io/locked_file.rs`.
- `dispatch.rs` already routes `add-diagram` → `commands::add_diagram::run`.
- `canonical.rs` already lists `add-diagram` and `delete-diagram` as canonical commands.
- `generate-foundation-md` is itself a stub (RPC-233 NotYetPorted) — we cannot regenerate FOUNDATION.md
  byte-for-byte yet.
- `validate-foundation-schema` already ported (commands/validate_foundation_schema.rs).

## Framing A divergences (port-time deviations from TS)

### Mermaid validation
The TS implementation uses `JSDOM` + `mermaid.parse()` to render-validate the diagram code.
Rust has no equivalent of the Mermaid renderer without shelling out to node, which the
port playbook forbids. **Rust port accepts any non-empty code string and performs only
the TS pre-render guards that are pure regex** (`subgraph "quoted"` rejection +
non-alphanumeric subgraph identifier rejection at `src/utils/mermaid-validation.ts:25-48`).
A test fixture documenting this divergence is added to the architecture notes on RPC-178.

### FOUNDATION.md regeneration
Because RPC-233 is unported, the Rust `add-diagram` skips the FOUNDATION.md regeneration
step but STILL prints the same status-line to stdout for CLI parity. This is documented
as an architecture note on the work unit. A follow-up RPC will wire generate-foundation-md
in once RPC-233 lands.

### Schema validation
The Rust port uses the same Ajv schema (`spec/foundation.schema.json`) via the existing
`validate_foundation_schema` helper (RPC-307 / show_foundation parity). Failure surfaces
identical "Updated foundation.json failed schema validation: <messages>" error text.

## Edge cases covered by TS tests

- New diagram added to empty architectureDiagrams
- Existing title replaced (idempotent — array length unchanged)
- Multiple diagrams in array
- foundation.json missing → ensureFoundationFile creates canonical default first
- Empty section / title / code → distinct error sentinels
- Invalid Mermaid syntax → reject with error containing the mermaid error message
- description optional → only present in JSON when supplied

## Out of scope for THIS port

- generate-foundation-md (RPC-233): skipped, status-line still printed
- Direct JSDOM/Mermaid integration in Rust: replaced with regex pre-checks only
