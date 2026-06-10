# RPC-217 — `delete-epic` Rust port: AST/behaviour research

## TypeScript source
- Primary: `src/commands/delete-epic.ts` (110 lines)
- Help: `src/commands/delete-epic-help.ts`

## Behaviour observations from TS source

### Inputs (CLI surface, src/commands/delete-epic.ts:92-108)
- Positional `<epicId>` (required)
- Option `--force` (optional, declared but **unused** by TS impl — passed but never inspected)

### Inputs (programmatic surface, lines 37-40)
- `options.epicId: string`
- `options.cwd?: string` (defaults to `process.cwd()`)

### Side effects
1. **Read+modify+write `spec/epics.json`** via `fileManager.transaction()`:
   - If `epicsData.epics[options.epicId]` is missing → throw `"Epic ${id} not found"` (wrapped by outer catch → `"Failed to delete epic: Epic ${id} not found"`).
   - Otherwise `delete epicsData.epics[options.epicId]` (mutation).
2. **Read+modify+write `spec/prefixes.json`** via `fileManager.transaction()` wrapped in try/catch:
   - For each prefix in `prefixesData.prefixes`, if `prefix.epicId === deletedId` then `delete prefix.epicId`.
   - **Catch swallowed**: if prefixes.json doesn't exist (or fails to read), silently continue.
3. **Read+modify+write `spec/work-units.json`** via `fileManager.transaction()` wrapped in try/catch:
   - For each work unit, if `workUnit.epic === deletedId` then `delete workUnit.epic`.
   - **Catch swallowed**: if work-units.json doesn't exist (or fails to read), silently continue.

### Output (CLI wrapper, lines 99-107)
On success:
```
✓ Epic <id> deleted successfully
```
Exit code 0.

### Output (errors)
`output.error('✗ Failed to delete epic:', error.message)` to stderr, exit 1.
Error messages start with `Failed to delete epic: ` (per outer-catch wrap).

### Important nuance — transaction call shape
The `fileManager.transaction(epicsFile, async epicsData => { ... })` will probably AUTO-CREATE the file if missing (since `LockedFileManager.transaction` typically reads-or-inits). Looking at TS line 48-55: if the file doesn't exist, transaction creates an empty `{ epics: {} }`, then the lookup at line 49 fails → throws "Epic <id> not found". So **deleting from a missing-epics.json correctly yields "Epic <id> not found"** as observable behaviour.

For Rust port we mirror this: use `ensure_epics_file` (read-or-init creating empty store), then check for missing key. Wait — currently `ensure_epics_file` does NOT exist in `io/ensure.rs`. We have `read_epics_or_empty` (read-only, ENOENT → empty store, parse error → escalate). For delete-epic, the easier route is:
- Use `read_epics_or_empty` to fetch the current state
- If the requested id isn't there → return "Epic <id> not found" wrapped
- Otherwise remove it and write back via `write_json_atomic`

This actually preserves TS semantics better — the auto-create behaviour TS exhibits (write empty, then fail) leaves an artifact file behind. Our approach (don't write when there's nothing to do) is slightly cleaner BUT it changes a side effect: TS leaves `spec/epics.json: { "epics": {} }` on disk after a failed delete against a missing file. We should match TS for parity.

**Decision**: Match TS exactly. Use `read_epics_or_empty` then if file did not exist, write the empty store first (via `write_json_atomic`) so the artifact appears. This is the simplest faithful mirror.

Actually, **simpler**: Use a new `ensure_epics_file` helper that load-or-inits. Then duplicate-check after. Side effects match TS exactly:
- If file missing → file created with empty store → lookup fails → "Epic <id> not found"
- If file present → lookup → maybe modify → atomic write

But the supervisor instruction says "You will need a write helper for epics.json — ask supervisor for `write_epics_file` or `ensure_epics_file` (creates+writes)." So I need a request.

Actually, I can build it inline by:
1. Check if `spec/epics.json` exists.
2. If not, write empty `{epics:{}}` via `write_json_atomic`.
3. Then read via `read_epics_or_empty`.
4. Check key; missing → error.
5. Else remove + `write_json_atomic`.

Same observable behavior. No new shared helper required. 

### Prefixes / work-units side effects parity
For each side-effect file:
- Build a list of changes to make.
- Try to read via `read_*_or_empty`. On ANY failure (ENOENT or parse-error per TS bare-catch), silently SKIP.
- Apply mutation.
- Write back via `write_json_atomic`.

**Important** — what about a file that DID exist but had no matching entries to remove? TS still calls the transaction (rewrites file with no changes). Our parity could either rewrite-unchanged or skip. To minimize disk churn, **skip the write when no changes**. This matches user-observable behavior because nothing changes byte-for-byte.

Actually, `fileManager.transaction` always writes after the callback (even when unchanged). To play it safe, we'll also always write back when the file existed. This matches the "lastUpdated"-style nuance and any field-order normalizations.

Hmm — but rewriting unchanged files MAY alter formatting (e.g. trailing newline, key ordering). Easier and equally faithful: only write back when a mutation was applied. We'll go with skip-when-unchanged for cleanliness, document the choice in a code comment, and the tests won't constrain unchanged-file rewriting.

### Field reads
- `Prefix.epicId` — currently the typed `Prefix` struct in `types/prefix.rs` may not have an explicit `epic_id` field. Need to check.

## File layout
- `codelet/fspec-core/src/commands/delete_epic.rs` — async fn run, ~250 LOC
- `codelet/fspec-core/src/help/configs/delete_epic.rs`
- `codelet/fspec/src/delete_epic.rs` — CLI bridge
- `codelet/fspec-core/tests/delete_epic.rs`
- `codelet/fspec/tests/cli_delete_epic.rs`
- `codelet/fspec/tests/fixtures/help/delete-epic.txt`

## Scenario inventory (preview)

### Dispatcher (`delete-epic-rust-port.feature`)
1. Deletes existing epic from epics.json
2. Removes epicId references from prefixes.json
3. Removes epic references from work-units.json
4. Returns "Epic <id> not found" wrapped error when missing
5. Tolerates missing prefixes.json silently
6. Tolerates missing work-units.json silently
7. Tolerates malformed prefixes.json silently (TS bare-catch)
8. Tolerates malformed work-units.json silently (TS bare-catch)
9. Preserves non-deleted epics
10. Returns canonical success text

### CLI (`delete-epic-cli-subcommand.feature`)
1. clap exposes delete-epic with positional + --force flag
2. CLI deletes existing epic, exit 0 + success message
3. CLI accepts --force without error (TS impl ignores it)
4. CLI exits 1 on missing epic with stderr Error prefix
5. CLI byte-for-byte help matches TS fixture
