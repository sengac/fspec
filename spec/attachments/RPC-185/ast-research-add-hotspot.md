# AST Research — RPC-185 port of `add-hotspot` to Rust

## Scope
Port `src/commands/add-hotspot.ts` → `codelet/fspec-core/src/commands/add_hotspot.rs`
(rewrite the stub) plus CLI bridge `codelet/fspec/src/add_hotspot.rs` and help config.

## TypeScript source surface (AstGrep)
- `src/commands/add-hotspot.ts:33` — `export async function addHotspot(options): Promise<AddHotspotResult>`
  - Options: `{ workUnitId, text, concern?, timestamp?, boundedContext?, cwd? }`
  - Result: `{ success, error?, hotspotId? }`
- Delegates to the SHARED util `src/commands/event-storm-utils.ts:29` —
  `export async function addEventStormItem<T extends EventStormItem>(options): Promise<AddEventStormItemResult>`.
- Registration (`registerAddHotspotCommand`): `add-hotspot <workUnitId> <text>` with
  `--concern <description>`, `--timestamp <ms>` (parseInt), `--bounded-context <name>`.
- Success stdout: `✓ Hotspot added to <workUnitId> (id: <hotspotId>)`.
- Failure stderr: `✗ Failed to add hotspot: <error>` then `process.exit(1)`.

## Behavioural contract (SHARED util — NO dedup)
1. Missing `spec/work-units.json` → error `spec/work-units.json not found. Run fspec init first.`
   (existsSync check FIRST; NO auto-create).
2. Missing work unit → `Work unit <id> not found`.
3. Status `done`/`blocked` → `Cannot add Event Storm items to work unit in <status> state`.
4. Init `eventStorm` = `{ level: "process_modeling", items: [], nextItemId: 0 }` when absent.
5. NO dedup — same hotspot text may be added repeatedly (distinguishes it from add-domain-event).
6. Item construction (via util spread): itemData fields first —
   `type("hotspot"), color("red"), text, [concern], [timestamp], [boundedContext]` — then
   `id, deleted(false), createdAt` appended. Post-increment `nextItemId`.
7. Returns `{ success: true, hotspotId }` (mapped from util's `itemId`).
8. Util wraps caught errors as `Failed to add Event Storm item: <msg>`, but the three validation
   errors (missing file / missing WU / done-blocked) return directly WITHOUT that wrapper.

## Rust reference shapes (AstGrep)
- `codelet/fspec-core/src/commands/add_rule.rs:67` —
  `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`
  (canonical mutation port shape to mirror).
- `codelet/fspec-core/src/commands/show_event_storm.rs` — items at `wu.extra["eventStorm"]["items"]`.
- Dispatch already routes `"add-hotspot" => commands::add_hotspot::run(...)` (stub). Supervisor
  wires `canonical::is_ported`, dispatch project_root, and `main.rs` subcommand registration.

## Divergence from add-domain-event (sibling card RPC-179)
- add-hotspot uses the SHARED util and has NO dedup; add-domain-event INLINES + dedups.
- color "red" vs "orange"; extra `--concern` option; success line wording differs.
