# AST Research — `delete-diagram` Rust Port (RPC-216)

## TS source-of-truth

- `src/commands/delete-diagram.ts` (111 LOC)
- `src/commands/delete-diagram-help.ts` (~28 LOC)
- `src/types/foundation.ts` (DiagramSection — note: TS interface includes `section` field but the
  GENERIC schema v2.0.0 written by ensureFoundationFile does NOT — see Framing A divergence below)
- `src/generators/foundation-md.ts` (generateFoundationMd — stub in Rust)

## Public surface

```typescript
deleteDiagram({ section, title, cwd? })
  → Promise<{ success: boolean; message?: string; error?: string }>
deleteDiagramCommand(section, title) → process.exit(0|1)
```

CLI: `fspec delete-diagram <section> <title>`

## Behaviour summary

1. Check `spec/foundation.json` exists (no auto-create) — missing → error sentinel
   `'foundation.json not found: spec/foundation.json'`.
2. Read + JSON.parse foundation.json.
3. `findIndex(d => d.section === section && d.title === title)` — note: TS uses the legacy
   `section` field on `DiagramSection`, which DOES NOT exist in the v2.0.0 generic schema
   written by `add-diagram` (Framing A divergence below).
4. Not found → `'Diagram '<title>' not found in section '<section>''`.
5. `architectureDiagrams.splice(index, 1)`.
6. Atomic write foundation.json (pretty 2-space indent).
7. Regenerate FOUNDATION.md via `generateFoundationMd`.
8. Success message: `Deleted diagram '<title>' from section '<section>'`.

CLI prints:
```
✓ Deleted diagram '<title>' from section '<section>'
  Updated: spec/foundation.json
  Regenerated: spec/FOUNDATION.md
```

## Framing A divergences (port-time deviations from TS)

### Section field is legacy
`add-diagram` (Rust port and TS) writes `{title, mermaidCode, description?}` — no `section`.
The TS `delete-diagram.ts` THEN filters by `d.section === section`, meaning every freshly-added
diagram is invisible to delete-diagram in the TS world! This is a latent TS bug.

**Rust port behaviour**: match by `title` ONLY. The `section` argument is accepted (for CLI
shape parity) but functions as a label that's echoed in error and success messages. If multiple
diagrams share the same title (rare — add-diagram replaces by title), the FIRST is removed.

### FOUNDATION.md regeneration
Mirror RPC-178: skip the regen step, still emit the `  Regenerated: spec/FOUNDATION.md`
status-line to stdout for CLI parity. Wire up once RPC-233 (generate-foundation-md) ports.

### foundation.json does NOT auto-create
Unlike `add-diagram`, the TS delete-diagram uses `existsSync` directly and returns an error
sentinel when missing. The Rust port preserves this — `std::fs::read_to_string` with explicit
ENOENT check, NOT through `ensure_foundation_file`.

## Existing Rust scaffolding available

- `write_json_atomic` in `codelet/fspec-core/src/io/locked_file.rs`.
- `dispatch.rs` already routes `delete-diagram` → `commands::delete_diagram::run`.
- `canonical.rs` already lists `delete-diagram`.

## Edge cases covered by TS tests

- Delete by section + title (positive)
- One of multiple diagrams in array
- Non-existent diagram → error
- Last diagram in section (architectureDiagrams becomes empty array)
- Other foundation.json sections preserved (project/personas/etc untouched)
- foundation.json missing → error

## Out of scope for THIS port

- generate-foundation-md (RPC-233): skipped, status-line still printed
