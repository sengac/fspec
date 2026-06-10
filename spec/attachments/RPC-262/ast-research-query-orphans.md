# RPC-262 — AST Research: `query-orphans`

## Source of truth (TypeScript)

- Implementation: `src/commands/query-orphans.ts`
- Help config: `src/commands/query-orphans-help.ts`

## TS behavioural inventory

### 1. Inputs

- Constructor option: `cwd?: string` (defaults to `process.cwd()`).
- Loads `spec/work-units.json` via `ensureWorkUnitsFile(cwd)` — **auto-creates** the file with the canonical initial structure if missing.
- CLI flags:
  - `--output <format>` (default `text`)
  - `--exclude-done` (boolean, default `false`)

### 2. Orphan detection

For each work unit:

```ts
hasEpic = wu.epic && wu.epic.trim().length > 0
hasRelationships =
  (wu.blocks && wu.blocks.length > 0) ||
  (wu.blockedBy && wu.blockedBy.length > 0) ||
  (wu.dependsOn && wu.dependsOn.length > 0) ||
  (wu.relatesTo && wu.relatesTo.length > 0)
isOrphaned = !hasEpic && !hasRelationships
```

If `isOrphaned`:
- If `excludeDone && wu.status === 'done'` → skip.
- Otherwise push `{id, title, status, suggestedActions: ['Assign epic', 'Add relationship', 'Delete']}`.

### 3. Result shape (JSON field order — declaration order)

```jsonc
{
  "orphans": [
    {
      "id": "MISC-001",
      "title": "Update documentation",
      "status": "backlog",
      "suggestedActions": ["Assign epic", "Add relationship", "Delete"]
    }
  ]
}
```

### 4. CLI wrapper (`registerQueryOrphansCommand`)

```ts
.option('--output <format>', '...', 'text')
.option('--exclude-done', '...', false)
.action(async ({output, excludeDone}) => {
  const result = await queryOrphans({output, excludeDone});
  if (output === 'json') {
    output.log(JSON.stringify(result, null, 2));
  } else {
    // text rendering — see TS lines 99-139
  }
});
```

Text rendering empty:

```
✓ No orphaned work units found.
<dim>All work units have either an epic assignment or dependency relationships.</dim>
```

Text rendering populated:

```
<yellow>
Found <N> orphaned work unit(s):
</yellow>

1. <cyan>MISC-001</cyan> - Update documentation (<dim>backlog</dim>)
   <red>⚠</red> No epic or dependency relationships
   <bold>Suggested actions:</bold>
     • Assign epic
     • Add relationship
     • Delete

To fix orphaned work units:
<dim>  fspec update-work-unit <id> --epic=<epic-name></dim>
<dim>  fspec add-dependency <id> --depends-on=<other-id>  (or --blocks, --relates-to)</dim>
  fspec delete-work-unit <id>
```

Note chalk wrappers reduce to identity for non-TTY. Whitespace details:
- `\nFound N orphaned work unit(s):\n` — has leading and trailing newlines.
- Per-orphan trailing `output.log('')` adds blank line between entries.

Errors caught → `output.error('✗ Failed to query orphans:', err.message)` → `process.exit(1)`.

### 5. Rust port targets

| Layer    | TS                                  | Rust                                                       |
|----------|-------------------------------------|------------------------------------------------------------|
| Core fn  | `src/commands/query-orphans.ts`     | `codelet/fspec-core/src/commands/query_orphans.rs`         |
| Help cfg | `src/commands/query-orphans-help.ts` | `codelet/fspec-core/src/help/configs/query_orphans.rs`    |
| CLI br   | (Commander.js registration)         | `codelet/fspec/src/query_orphans.rs`                       |

### 6. WorkUnit fields consumed

- `id`, `title`, `status`, `epic` — typed fields on the Rust `WorkUnit`.
- `blocks`, `blockedBy`, `dependsOn`, `relatesTo` — read from `extra` map via `extra.get(k).and_then(Value::as_array)`.

### 7. Corner cases to test

- Empty workspace → auto-create + `orphans: []`.
- Unit with `epic="auth"` and no relationships → NOT orphaned.
- Unit with `epic=""` (empty string) → trimmed, considered no epic.
- Unit with `epic="   "` (whitespace) → trimmed, considered no epic → orphaned (if no relations).
- Unit with `epic=null/undefined` and no relations → orphaned.
- Unit with `blocks=[]` (empty array) → considered no relations (length 0 check).
- Unit with `blocks=["X"]` and no epic → NOT orphaned (has relation).
- `--exclude-done` true: done orphans skipped.
- `--exclude-done` false (default): done orphans included.
- Insertion order preserved in output.
- Missing `work-units.json` → auto-created → empty.
- Malformed JSON → ParseJson error.

### 8. Output rendering matrix

| `--output` | `--exclude-done` | Stdout | Exit code |
|-----------|------------------|--------|-----------|
| `text` (default) | (any) | Multi-line text rendering, `✓ No orphaned work units found.` when empty | 0 |
| `json` | (any) | `JSON.stringify({orphans:[...]}, null, 2)` | 0 |
| (error) | — | stderr: `✗ Failed to query orphans: <msg>` | 1 |

End of research.
