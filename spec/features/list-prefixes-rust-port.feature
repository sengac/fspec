@done
@rust
@querying
@cli
@RPC-248
Feature: Port list-prefixes command to Rust
  """
  New shared helper `io::ensure::read_prefixes_or_empty(cwd) -> Result<PrefixesData, FspecCoreError>` lives alongside `ensure_prefixes_file` but returns `Ok(PrefixesData::initial())` on ENOENT instead of auto-creating. This separates 'read-only' from 'load-or-init' semantics — list-prefixes uses the former (TS does NOT call ensurePrefixesFile), list-work-units continues to use the latter.
  New shared helper `io::ensure::read_work_units_or_empty(cwd) -> Result<WorkUnitsData, FspecCoreError>` returns `Ok(WorkUnitsData::initial(...))` on BOTH ENOENT and parse error — this captures TS's bare `catch {}` on work-units (silently empty on any failure). list-work-units continues to use `ensure_work_units_file` (which auto-creates AND escalates parse errors).
  The shape of each Prefix record in spec/prefixes.json is `{ prefix: string, description: string, createdAt: string }` (TS interface at src/commands/list-prefixes.ts:7-11). Add a typed `Prefix` struct to rust/fspec-core/src/types/prefix.rs (new file) and refactor `PrefixesData.prefixes` from `Map<String, Value>` to `IndexMap<String, Prefix>` with a `#[serde(flatten)] extra: Map<String, Value>` field on Prefix for forward-compat. This preserves insertion order (rule [6]).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `list-prefixes` MUST replace the NotYetPorted stub and return a real DispatchResult through the same `poll_sync_future` path the RPC-327 fix uses
  #   2. If spec/prefixes.json is missing, the command MUST return success with an empty prefixes list and MUST NOT auto-create the file (parity with the TS ENOENT-swallow path at src/commands/list-prefixes.ts:48-54)
  #   3. If spec/work-units.json is missing OR contains malformed JSON, the command MUST silently treat work-unit counts as zero for every prefix and MUST NOT throw (parity with the bare `catch {}` at src/commands/list-prefixes.ts:57-63 — work-unit read failures are deliberately swallowed)
  #   4. If spec/prefixes.json exists but is malformed, the command MUST propagate a structured error containing the substring 'Failed to parse prefixes.json' (prefixes-read failures are NOT swallowed; only the JSON.parse error path is escalated, matching the rethrow at src/commands/list-prefixes.ts:53)
  #   5. For each prefix in spec/prefixes.json the command MUST compute totalWorkUnits (work-units whose id starts with `${prefix}-`) and completedWorkUnits (subset with status=='done'); completionPercentage = round((completed/total)*100) when total > 0, else 0 (parity with src/commands/list-prefixes.ts:65-95)
  #   6. Output preserves insertion order of spec/prefixes.json (Object.values iteration in TS, IndexMap in Rust) — prefixes appear in the order they were registered
  #   7. Each prefix entry in the structured result MUST contain prefix (string), description (string), totalWorkUnits (number), completedWorkUnits (number), completionPercentage (number) — matching the TS PrefixWithProgress interface exactly
  #   8. The text format (default) prints 'No prefixes found' for empty lists; populated lists print '\nPrefixes (N)\n' header followed by each prefix as 'PREFIX\n  description\n[  Work Units: completed/total (pct%)\n]\n' — the 'Work Units' line only appears when totalWorkUnits > 0 (parity with src/commands/list-prefixes.ts:107-123)
  #   9. The JSON format wraps the result in `{ prefixes: [...] }` with 2-space indentation (parity with TS `JSON.stringify(result, null, 2)`); --format is NOT exposed at the TS CLI surface but the Rust shared `run()` accepts it for the dispatcher's structured-output path
  #   10. The standalone fspec binary at rust/fspec/src/main.rs MUST expose `list-prefixes` as a clap v4 derive subcommand with NO flags — matching the flag-less TS Commander.js registration at src/commands/list-prefixes.ts:101-104 (no --status, no --prefix, no --format)
  #   11. The clap subcommand action MUST delegate to the same fspec_core::commands::list_prefixes::run() function used by the LLM-facing dispatcher (two front doors, one source of truth — RPC-003 §7/§11) and MUST NOT duplicate prefix-aggregation, filter or rendering logic in the CLI bridge
  #   12. Shared infrastructure MUST be reused — the command reads spec/prefixes.json via a NEW shared helper (e.g. `io::read_prefixes_or_empty`) that is symmetric with the existing `ensure_prefixes_file` but returns `Ok(empty)` on ENOENT instead of auto-creating; the existing `WorkUnit`, `WorkUnitsData`, `PrefixesData`, and `project_root` modules MUST be reused without duplication
  #   13. The CLI wrapper MUST resolve the project root from current working directory (parity with TS `process.cwd()` default at src/commands/list-prefixes.ts:39), exit 0 on success, exit 1 on FspecCoreError, and write structured errors to stderr prefixed with `Error:` (same chalk-equivalent contract as RPC-253 rule [14])
  #
  # EXAMPLES:
  #   1. Dispatch `list-prefixes` against a tempdir with NO spec/ → command succeeds, returns JSON with empty prefixes array, NEITHER spec/prefixes.json NOR spec/work-units.json is created (parity with TS ENOENT short-circuit)
  #   2. Tempdir has prefixes.json with AUTH (description='Auth features') and DASH (description='Dashboard'), plus work-units.json with AUTH-001 (done), AUTH-002 (backlog), DASH-001 (done), DASH-002 (done) → dispatcher returns both prefixes with AUTH at 1/2 (50%) and DASH at 2/2 (100%)
  #   3. Tempdir has prefixes.json with AUTH but spec/work-units.json is MISSING → dispatcher returns AUTH with totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0 (work-units ENOENT silently treated as empty)
  #   4. Tempdir has prefixes.json with AUTH but spec/work-units.json contains '{ not json' → dispatcher still succeeds (does NOT escalate the parse error), returning AUTH with zero work-unit counts — TS's bare `catch {}` swallows malformed work-units silently for prefix-listing purposes
  #   5. Tempdir has prefixes.json containing invalid JSON syntax → dispatcher returns success=false with an error message containing the substring 'Failed to parse prefixes.json' (prefixes-read errors ARE escalated, unlike work-units errors)
  #   6. prefixes.json contains three prefixes registered in order ZED, AAA, MID → text output lists them as ZED, AAA, MID (insertion order, not alphabetical) under the 'Prefixes (3)' header
  #   7. prefixes.json contains AUTH with description 'Auth features' and zero matching work-units → text output prints 'AUTH\n  Auth features\n' with NO 'Work Units:' progress line (parity with TS guard at line 117: 'if (prefix.totalWorkUnits > 0)')
  #   8. prefixes.json has AUTH with 1 done and 2 backlog work-units → text output's 'Work Units:' line reads exactly '  Work Units: 1/3 (33%)' (Math.round semantics — 33.33% rounds DOWN to 33; verify with a 2/3 case rounding to 67%)
  #   9. Dispatcher receives `{"format":"json"}` against a prefixes.json containing AUTH (0/0) → DispatchResult.data is exactly `{\n  "prefixes": [\n    {\n      "prefix": "AUTH",\n      "description": "...",\n      "totalWorkUnits": 0,\n      "completedWorkUnits": 0,\n      "completionPercentage": 0\n    }\n  ]\n}` (2-space indent, no trailing newline)
  #   10. Running `./rust/target/release/fspec list-prefixes` in an empty directory prints 'No prefixes found' to stdout and exits 0 (does NOT auto-create spec/ since list-prefixes only reads)
  #   11. Running `./rust/target/release/fspec list-prefixes --help` prints clap-generated help with NO --status / --prefix / --epic / --format / --workspace flags listed (parity with TS Commander's flag-less registration and RPC-253's global-workspace exclusion)
  #   12. Running `./rust/target/release/fspec list-prefixes` against a directory whose spec/prefixes.json contains invalid JSON prints `Error: ... Failed to parse prefixes.json: ...` to stderr and exits with code 1
  #   13. Both invocation paths produce the SAME structured data: (a) dispatch_command("list-prefixes", `{"format":"json"}`, project_root) and (b) `./rust/target/release/fspec list-prefixes` against the same on-disk state — the only differences are how args are parsed (JSON vs flag-less clap) and how the result is delivered (DispatchResult.data vs stdout text)
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch list-prefixes from the agent loop and get the same prefix listing — with per-prefix work-unit completion progress — as the TypeScript implementation
    So that I can audit prefix registration and progress without relying on Node.js, sharing one source-of-truth between the LLM dispatcher and the CLI

  Scenario: Returns an empty prefixes list when spec/ does not exist and does not auto-create files
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch the list-prefixes command against that project root with format='json'
    Then the dispatcher returns success=true with an empty prefixes array
    Then spec/prefixes.json does not exist after the call
    Then spec/work-units.json does not exist after the call

  Scenario: Aggregates work-unit completion progress per prefix
    Given spec/prefixes.json contains AUTH (description 'Auth features') and DASH (description 'Dashboard') in that order
    Given spec/work-units.json contains AUTH-001 (done), AUTH-002 (backlog), DASH-001 (done), DASH-002 (done)
    When I dispatch list-prefixes with format='json'
    Then the prefixes array contains exactly two entries in order AUTH then DASH
    Then the AUTH entry has totalWorkUnits=2, completedWorkUnits=1, completionPercentage=50
    Then the DASH entry has totalWorkUnits=2, completedWorkUnits=2, completionPercentage=100

  Scenario: Treats missing work-units.json as zero counts without throwing
    Given spec/prefixes.json contains AUTH (description 'Auth features')
    Given spec/work-units.json does NOT exist
    When I dispatch list-prefixes with format='json'
    Then the dispatcher returns success=true
    Then the AUTH entry has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0

  Scenario: Treats malformed work-units.json as zero counts without throwing
    Given spec/prefixes.json contains AUTH (description 'Auth features')
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I dispatch list-prefixes with format='json'
    Then the dispatcher returns success=true
    Then the AUTH entry has totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0

  Scenario: Escalates malformed prefixes.json as a structured parse error
    Given spec/prefixes.json exists but contains invalid JSON syntax
    When I dispatch list-prefixes against that project root
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse prefixes.json'

  Scenario: Preserves insertion order of prefixes.json (not alphabetical)
    Given spec/prefixes.json contains three prefixes registered in order ZED, AAA, MID
    When I dispatch list-prefixes with format='text'
    Then the DispatchResult.data contains the substring 'Prefixes (3)'
    Then the substring 'ZED' appears before 'AAA' which appears before 'MID' in the output

  Scenario: Text format omits the 'Work Units' line when totalWorkUnits is zero
    Given spec/prefixes.json contains AUTH with description 'Auth features'
    Given spec/work-units.json contains no work units whose id starts with 'AUTH-'
    When I dispatch list-prefixes with format='text'
    Then the DispatchResult.data contains the line 'AUTH'
    Then the DispatchResult.data contains the line '  Auth features'
    Then the DispatchResult.data does NOT contain the substring 'Work Units:'

  Scenario: Text format renders completion progress with Math.round percentage semantics
    Given spec/prefixes.json contains AUTH with description 'Auth features'
    Given spec/work-units.json contains AUTH-001 (done), AUTH-002 (backlog), AUTH-003 (backlog)
    When I dispatch list-prefixes with format='text'
    Then the DispatchResult.data contains the exact line '  Work Units: 1/3 (33%)'

  Scenario: Text format rounds 2/3 progress to 67 percent
    Given spec/prefixes.json contains AUTH with description 'Auth features'
    Given spec/work-units.json contains AUTH-001 (done), AUTH-002 (done), AUTH-003 (backlog)
    When I dispatch list-prefixes with format='text'
    Then the DispatchResult.data contains the exact line '  Work Units: 2/3 (67%)'

  Scenario: JSON format emits two-space indented payload with the canonical field set
    Given spec/prefixes.json contains AUTH with description 'Auth features'
    Given spec/work-units.json does NOT exist
    When I dispatch list-prefixes with format='json'
    Then the DispatchResult.data parses as JSON whose root object has a 'prefixes' array of length 1
    Then the first prefixes entry contains fields prefix='AUTH', description='Auth features', totalWorkUnits=0, completedWorkUnits=0, completionPercentage=0
    Then the DispatchResult.data uses 2-space indentation

  Scenario: Text format prints 'No prefixes found' for an empty prefixes file
    Given spec/prefixes.json exists with an empty prefixes object
    When I dispatch list-prefixes with format='text'
    Then the DispatchResult.data is exactly the string 'No prefixes found'

  Scenario: Shared infrastructure modules exist under rust/fspec-core for reuse by other commands
    Given the rust/fspec-core crate is built
    When I inspect rust/fspec-core/src/
    Then the modules io::ensure::read_prefixes_or_empty and io::ensure::read_work_units_or_empty exist and are publicly accessible from the crate root
    Then types::prefix::Prefix exists and PrefixesData.prefixes is keyed by an IndexMap to preserve insertion order
    Then list_prefixes::run delegates to these shared modules rather than embedding its own filesystem logic
