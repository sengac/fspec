@done
@querying
@cli
@rust
@RPC-241
Feature: Port list-attachments command to Rust

  """
  New typed `attachments: Option<Vec<String>>` field on `codelet/fspec-core/src/types/work_unit.rs::WorkUnit` with `#[serde(default, skip_serializing_if = "Option::is_none")]` — preserves on-disk shape (no field emitted when absent) and allows typed access from `list_attachments::run`. The existing `extra: serde_json::Map<...>` flatten map continues to round-trip every OTHER unknown field unchanged
  Reuses `crate::io::ensure::ensure_work_units_file` (NOT the read-only `read_work_units_or_empty` twin) so the load-or-init + escalating-parse-error semantics match TS's `await ensureWorkUnitsFile(cwd)` at src/commands/list-attachments.ts:20
  Two new dispatcher-side helpers: `format_size_kb(bytes: u64) -> String` returning `"{:.2}"`-formatted KB (parity with JS `(n/1024).toFixed(2)`), and `format_mtime(modified: SystemTime) -> String` returning a deterministic UTC ISO-like string (e.g. "2026-06-04 14:32:17 UTC"). These live inside `codelet/fspec-core/src/commands/list_attachments.rs` because they are command-specific and not reused elsewhere
  Args struct in fspec-core: `ListAttachmentsArgs { work_unit_id: Option<String>, format: Option<String> }` with `#[serde(default, rename_all = "camelCase")]` so the dispatcher accepts both `workUnitId` (JS-canonical) and the missing-field case (validated explicitly with a clear InvalidArgs error)
  Clap variant in main.rs: `ListAttachments { work_unit_id: String }` — required positional named `work_unit_id` (clap renders it as `<WORK_UNIT_ID>` in help). No flags. The action arm marshals `{"workUnitId": "..."}` JSON, then delegates to the bridge module's `run()` which returns the exit code (0 success / 1 FspecCoreError). Clap's own validation (missing-positional → exit 2) executes BEFORE the bridge — that's the correct contract per RPC-253
  CLI bridge module `codelet/fspec/src/list_attachments.rs` defines `pub struct CliArgs { pub work_unit_id: String }` and `pub async fn run(args: CliArgs) -> Result<u8>` — symmetric with the list_prefixes/list_work_units bridges so future flag additions land as field additions only. The bridge contains NO inline rendering or filesystem logic — only JSON arg marshalling + `print!(rendered)` + `eprintln!("Error: {err}")`
  Dispatcher wiring: move `"list-attachments"` arm from `dispatch.rs::run_stub` into `dispatch.rs::run_ported` and add `"list-attachments"` to `canonical.rs::PORTED_COMMANDS`. The lock-list in `cargo_shape.rs::scenario_fspec_src_contains_exactly_the_locked_file_layout` grows from 8 → 9 with `"list_attachments.rs"` added (alongside list_prefixes.rs / list_work_units.rs)
  Modified-timestamp parity: JS `Date.toLocaleString()` is host-locale/TZ-dependent and CANNOT be bit-stably reproduced from Rust. Acceptance criteria assert ONLY the `    Modified: ` line prefix + a non-empty suffix — never the literal time text. This deviation is documented in the feature file architecture doc-string
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `list-attachments` MUST replace the NotYetPorted stub and return a real DispatchResult through the same `poll_sync_future` path used by RPC-248/RPC-253
  #   2. The command MUST require a `workUnitId` argument; if it is missing or empty the dispatcher MUST return success=false with an InvalidArgs error whose message names the missing field (parity with TS Commander's `<workUnitId>` required positional)
  #   3. If spec/work-units.json is missing it MUST be auto-created with the canonical initial structure before lookup (parity with `ensureWorkUnitsFile` at src/commands/list-attachments.ts:20, which delegates to the load-or-init helper — NOT the read-only twin)
  #   4. If spec/work-units.json exists but is malformed, the command MUST propagate a structured error containing the substring 'Failed to parse work-units.json' (parity with `ensureWorkUnitsFile` escalating parse errors, unlike `read_work_units_or_empty` which silently swallows)
  #   5. If the requested workUnitId does not exist in data.workUnits, the dispatcher MUST return success=false with an error message containing exactly the substring "Work unit '<id>' does not exist" (parity with src/commands/list-attachments.ts:24)
  #   6. If the work unit exists but `attachments` is missing OR an empty array, the command MUST return success=true with output containing exactly the substring "No attachments found for work unit <id>" (parity with src/commands/list-attachments.ts:30-34 — empty list is NOT an error)
  #   7. Populated text output MUST begin with a leading blank line then the header `Attachments for <id> (<N>):` (parity with TS line 39-41 which embeds `\n` at start and `:\n` at end of the chalk.bold header)
  #   8. For each attachment whose file exists on disk under `<cwd>/<relativePath>`, the text output MUST render three lines in this order: `  ✓ <relativePath>`, `    Size: <KB> KB` (bytes/1024 formatted to 2 decimal places), `    Modified: <timestamp>` (parity with src/commands/list-attachments.ts:48-53)
  #   9. For each attachment whose file is missing OR unreadable, the text output MUST render exactly two lines: `  ✗ <relativePath>` and `    File not found on filesystem` (parity with TS catch block at lines 54-58 — stat errors are swallowed into the ✗ marker, never escalated)
  #   10. The Modified timestamp string is deliberately informational and its exact format is NOT bit-stable with TypeScript's `Date.toLocaleString()` (which is locale- and TZ-dependent). Acceptance tests MUST only assert the `    Modified: ` line prefix and a non-empty suffix — never the literal time content
  #   11. Each populated attachment block MUST be separated from the next by a blank line (parity with TS embedding `\n` inside the last console.log of each block, which combines with console.log's own trailing newline to produce a blank line)
  #   12. The attachments array MUST be iterated in stored order (TS `for (const attachment of workUnit.attachments)`) — Rust MUST preserve the insertion order of the array as it appears in spec/work-units.json
  #   13. The JSON format ({"format":"json"} on the dispatcher surface) MUST emit a 2-space-indented payload with shape `{ "workUnitId": "<id>", "attachments": [ { "path": "<rel>", "exists": <bool>, ... } ] }` where exists=true entries also carry `sizeKb` (string from `{:.2}`) and `modified` (string), and exists=false entries omit those fields. --format is NOT exposed at the TS CLI surface but the Rust shared run() accepts it for the dispatcher's structured-output path (parity convention with RPC-248)
  #   14. The standalone fspec binary at codelet/fspec/src/main.rs MUST expose `list-attachments` as a clap v4 derive subcommand with a single required positional argument `<work_unit_id>` and NO flags — matching the TS Commander registration at src/commands/list-attachments.ts:62-66 (no --status, no --prefix, no --format, no --workspace)
  #   15. The clap subcommand action MUST delegate to the same fspec_core::commands::list_attachments::run() function used by the LLM-facing dispatcher (two front doors, one source of truth — RPC-003 §7/§11) and MUST NOT duplicate work-unit lookup, attachment-iteration, file-stat or rendering logic in the CLI bridge
  #   16. The CLI wrapper MUST resolve the project root from current working directory (parity with TS `process.cwd()` default at src/commands/list-attachments.ts:17), exit 0 on success (including the empty-attachments sentinel and the ✗ missing-file case), exit 1 on any FspecCoreError (unknown work-unit, missing argument, parse failure), and write structured errors to stderr prefixed with `Error:` (same chalk-equivalent contract as RPC-253 rule [14] and RPC-248)
  #   17. Shared infrastructure MUST be reused — the command reads spec/work-units.json via the existing `ensure_work_units_file` helper (NOT the read-only twin, because TS uses the load-or-init helper); the existing `WorkUnit` / `WorkUnitsData` modules MUST be reused without duplication. The `WorkUnit` struct gains a typed `attachments: Option<Vec<String>>` field with `#[serde(default, skip_serializing_if = "Option::is_none")]` so the field round-trips losslessly and is invisible on disk when absent
  #
  # EXAMPLES:
  #   1. Dispatch list-attachments with workUnitId='AUTH-001' against a tempdir with NO spec/ → command returns success=false with the substring "Work unit 'AUTH-001' does not exist" because the auto-created work-units.json is empty
  #   2. Tempdir has work-units.json with AUTH-001 (no attachments field at all) → dispatcher returns success=true and the text output contains exactly the line "No attachments found for work unit AUTH-001"
  #   3. Tempdir has work-units.json with AUTH-001 (attachments=[]) → dispatcher returns success=true and the text output contains exactly the line "No attachments found for work unit AUTH-001" (empty array behaves identically to missing field)
  #   4. Tempdir has work-units.json with AUTH-001 (attachments=["spec/attachments/AUTH-001/diagram.png"]) AND that file exists on disk with size 2048 bytes → text output contains the header "Attachments for AUTH-001 (1):", line "  ✓ spec/attachments/AUTH-001/diagram.png", line "    Size: 2.00 KB", and a line starting with "    Modified: " (the exact timestamp content is NOT asserted because it is locale/TZ-sensitive)
  #   5. Tempdir has work-units.json with AUTH-001 (attachments=["spec/attachments/AUTH-001/missing.png"]) but that file does NOT exist on disk → text output contains "  ✗ spec/attachments/AUTH-001/missing.png" followed by "    File not found on filesystem" and exits 0 (stat errors are silently downgraded)
  #   6. Tempdir has work-units.json with AUTH-001 (attachments=["spec/attachments/AUTH-001/a.png","spec/attachments/AUTH-001/b.png"]) where a.png exists (1234 bytes) and b.png is missing → text output renders a.png with "  ✓" + "    Size: 1.21 KB" + Modified line, then b.png with "  ✗" + "    File not found on filesystem", and the header reports (2). The substring 'a.png' appears before 'b.png' (insertion order preserved)
  #   7. Dispatch list-attachments with empty args object {} (no workUnitId field) → dispatcher returns success=false with an InvalidArgs error message that names the missing field (parity with TS Commander rejecting a missing required positional)
  #   8. Tempdir has spec/work-units.json containing invalid JSON syntax → dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json' (work-units parse errors ARE escalated when using ensure_work_units_file, matching TS rethrow at src/utils/ensure-files.ts:49-52)
  #   9. Dispatcher receives `{"workUnitId":"AUTH-001","format":"json"}` against a tempdir whose AUTH-001 has attachments=["spec/attachments/AUTH-001/x.png"] (exists, 1024 bytes) → DispatchResult.data parses to JSON object with workUnitId="AUTH-001" and attachments array of length 1 whose first element has path="spec/attachments/AUTH-001/x.png", exists=true, sizeKb="1.00", and a non-empty modified string. The output uses 2-space indentation
  #   10. Running `./codelet/target/release/fspec list-attachments AUTH-001` in a tempdir whose AUTH-001 has no attachments prints "No attachments found for work unit AUTH-001" to stdout and exits 0
  #   11. Running `./codelet/target/release/fspec list-attachments --help` prints clap-generated help with the required <WORK_UNIT_ID> positional and NO --status / --prefix / --epic / --format / --workspace flags listed
  #   12. Running `./codelet/target/release/fspec list-attachments NONEXISTENT-001` in a tempdir whose work-units.json does not contain NONEXISTENT-001 prints `Error: Work unit 'NONEXISTENT-001' does not exist` to stderr and exits with code 1
  #   13. Running `./codelet/target/release/fspec list-attachments` (no positional) exits with clap's standard usage error (code 2) and stderr names the missing required argument — does NOT delegate to fspec_core because clap validates first
  #   14. Both invocation paths produce the SAME structured data: (a) dispatch_command("list-attachments", `{"workUnitId":"AUTH-001","format":"json"}`, project_root) and (b) `./codelet/target/release/fspec list-attachments AUTH-001` against the same on-disk state — the only differences are how args are parsed (JSON vs clap positional) and how the result is delivered (DispatchResult.data JSON vs stdout text)
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch list-attachments from the agent loop AND invoke `fspec list-attachments <workUnitId>` from a shell and get the same attachment listing — with size and modification time for each file — as the TypeScript implementation
    So that I can audit work-unit attachments without relying on Node.js, sharing one source-of-truth between the LLM dispatcher and the CLI

  Scenario: Returns a structured error when the requested work unit does not exist
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch list-attachments with workUnitId='AUTH-001' against that project root
    Then the dispatcher returns success=false
    Then the error message contains the substring "Work unit 'AUTH-001' does not exist"
    Then spec/work-units.json was auto-created with an empty workUnits map

  Scenario: Returns the empty-attachments sentinel when the attachments field is missing
    Given spec/work-units.json contains AUTH-001 with NO attachments field
    When I dispatch list-attachments with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains exactly the line "No attachments found for work unit AUTH-001"

  Scenario: Returns the empty-attachments sentinel when the attachments array is empty
    Given spec/work-units.json contains AUTH-001 with attachments=[]
    When I dispatch list-attachments with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains exactly the line "No attachments found for work unit AUTH-001"

  Scenario: Renders a present attachment with size and modified-line prefix
    Given spec/work-units.json contains AUTH-001 with attachments=["spec/attachments/AUTH-001/diagram.png"]
    Given the file spec/attachments/AUTH-001/diagram.png exists on disk with exactly 2048 bytes
    When I dispatch list-attachments with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the substring "Attachments for AUTH-001 (1):"
    Then the DispatchResult.data contains the exact line "  ✓ spec/attachments/AUTH-001/diagram.png"
    Then the DispatchResult.data contains the exact line "    Size: 2.00 KB"
    Then the DispatchResult.data contains a line starting with "    Modified: "

  Scenario: Renders a missing attachment with the ✗ marker and the canonical not-found message
    Given spec/work-units.json contains AUTH-001 with attachments=["spec/attachments/AUTH-001/missing.png"]
    Given no file exists at spec/attachments/AUTH-001/missing.png
    When I dispatch list-attachments with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the exact line "  ✗ spec/attachments/AUTH-001/missing.png"
    Then the DispatchResult.data contains the exact line "    File not found on filesystem"
    Then the DispatchResult.data does NOT contain the substring "Size:" for this entry

  Scenario: Preserves attachment-array insertion order and mixes present/missing markers
    Given spec/work-units.json contains AUTH-001 with attachments=["spec/attachments/AUTH-001/a.png","spec/attachments/AUTH-001/b.png"]
    Given the file spec/attachments/AUTH-001/a.png exists on disk with exactly 1234 bytes
    Given no file exists at spec/attachments/AUTH-001/b.png
    When I dispatch list-attachments with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    Then the DispatchResult.data contains the substring "Attachments for AUTH-001 (2):"
    Then the DispatchResult.data contains the exact line "  ✓ spec/attachments/AUTH-001/a.png"
    Then the DispatchResult.data contains the exact line "    Size: 1.21 KB"
    Then the DispatchResult.data contains the exact line "  ✗ spec/attachments/AUTH-001/b.png"
    Then the DispatchResult.data contains the exact line "    File not found on filesystem"
    Then the substring 'a.png' appears before 'b.png' in the DispatchResult.data

  Scenario: Rejects an empty arguments object with a structured InvalidArgs error
    Given an empty project root directory
    When I dispatch list-attachments with the empty JSON args object {}
    Then the dispatcher returns success=false
    Then the error message names the missing field workUnitId

  Scenario: Escalates malformed work-units.json as a structured parse error
    Given spec/work-units.json exists but contains invalid JSON syntax
    When I dispatch list-attachments with workUnitId='AUTH-001'
    Then the dispatcher returns success=false
    Then the error message contains the substring 'Failed to parse work-units.json'

  Scenario: JSON format emits two-space indented payload with the canonical field set
    Given spec/work-units.json contains AUTH-001 with attachments=["spec/attachments/AUTH-001/x.png"]
    Given the file spec/attachments/AUTH-001/x.png exists on disk with exactly 1024 bytes
    When I dispatch list-attachments with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the DispatchResult.data parses as JSON whose root object has workUnitId='AUTH-001'
    Then the JSON root object has an attachments array of length 1
    Then the first attachments entry has path='spec/attachments/AUTH-001/x.png', exists=true, and sizeKb='1.00'
    Then the first attachments entry has a non-empty modified string
    Then the DispatchResult.data uses 2-space indentation

  Scenario: Shared infrastructure and ported wiring are in place
    Given the codelet/fspec-core crate is built
    When I inspect codelet/fspec-core/src/
    Then commands/list_attachments.rs does NOT return FspecCoreError::NotYetPorted
    Then commands/list_attachments.rs delegates to ensure_work_units_file
    Then commands/list_attachments.rs reads the attachments field via the WorkUnit extra map
    Then canonical.rs lists "list-attachments" in PORTED_COMMANDS
