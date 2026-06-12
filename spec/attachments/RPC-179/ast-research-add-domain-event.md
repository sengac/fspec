# AST Research — RPC-179 port of `add-domain-event` to Rust

## Scope
Port `src/commands/add-domain-event.ts` → `codelet/fspec-core/src/commands/add_domain_event.rs`
(rewrite the stub) plus CLI bridge `codelet/fspec/src/add_domain_event.rs` and help config.

## TypeScript source surface (AstGrep)
- `src/commands/add-domain-event.ts:34` — `export async function addDomainEvent(options): Promise<AddDomainEventResult>`
  - Options: `{ workUnitId, text, timestamp?, boundedContext?, cwd? }`
  - Result: `{ success, error?, eventId? }`
- Registration (`registerAddDomainEventCommand`): `add-domain-event <workUnitId> <text>` with
  `--timestamp <ms>` (parseInt) and `--bounded-context <context>`.
- Success stdout (action callback): `✓ Added domain event "<text>" to <workUnitId> (ID: <eventId>)`.
- Failure stderr: `✗ Failed to add domain event: <error>` then `process.exit(1)`.

## Behavioural contract (INLINE — no shared util; carries BUG-087 dedup)
1. Missing `spec/work-units.json` → error `spec/work-units.json not found. Run fspec init first.`
   (existsSync check FIRST; NO auto-create — differs from add-rule's ensure helper).
2. Missing work unit → `Work unit <id> not found`.
3. Status `done`/`blocked` → `Cannot add Event Storm items to work unit in <status> state`.
4. Init `eventStorm` = `{ level: "process_modeling", items: [], nextItemId: 0 }` when absent.
5. BUG-087 dedup: scan non-deleted `type:"event"` items; case-insensitive text match →
   `Event '<text>' already exists (ID: <existingId>)`.
6. Append item field order: `id, type("event"), color("orange"), text, deleted(false), createdAt`,
   then optional `timestamp` / `boundedContext`. Post-increment `nextItemId`. Bump `updatedAt`.
7. Returns `{ success: true, eventId }`.

## Rust reference shapes (AstGrep)
- `codelet/fspec-core/src/commands/add_rule.rs:67` —
  `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`
  (canonical mutation port: parse camelCase args, mutate `wu.extra` map, `write_json_atomic`).
- `codelet/fspec-core/src/commands/show_event_storm.rs` — confirms eventStorm read path:
  items live at `wu.extra["eventStorm"]["items"]`.
- Dispatch already routes `"add-domain-event" => commands::add_domain_event::run(...)` (stub).
  Supervisor will (a) add to `canonical::is_ported`, (b) pass `project_root` in dispatch arm,
  (c) register CLI subcommand in `main.rs`.

## Divergence from add-rule
- NO `ensure_work_units_file` (that auto-creates). Inline: `existsSync` → read_json or error.
- Item lives under `eventStorm.items`, not a top-level `rules` array.
- Carries dedup logic absent from add-rule.
