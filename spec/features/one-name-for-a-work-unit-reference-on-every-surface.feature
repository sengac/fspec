@done
@CLI-020
@rust
@cli
@querying
Feature: One name for a work-unit reference on every surface: --work-unit, <workUnitId>, id on cards and workUnitId on references, with no aliases
  """
  CLI: list-work-units' Flags.id becomes work_unit (long --work-unit, value_name workUnitId, JSON key workUnitId) and prefix loses its short; --parent gains -p. query-metrics' Mode::QueryMetrics work_unit_id becomes --work-unit with JSON key workUnitId. query-work-units' prefix loses -p. Every clap value_name for a card id (WORK_UNIT_ID, work-unit-id, ID, IDS, PARENT, TARGET_ID, DEPENDS_ON_ID, FROM_ID, TO_ID) becomes workUnitId.
  Core: fspec-core list_work_units Args.id becomes work_unit_id (serde workUnitId, still a List with ranges); audit.rs and research.rs read workUnitId; auto_advance.rs writes doneAt. Help configs, help fixtures (rust/fspec/tests/fixtures/help), error-fix hints in commands/*.rs, init_doc_*.md and fixtures, bootstrap_doc.txt use <workUnitId>. RPC: the 11 MCP Args structs in rpc/src/mcp_dispatch/group_04/05/19/47.rs get #[serde(rename = "workUnitId")] on work_unit_id, so the catalog schema follows (WIRE CHANGE for the fspec-paseo plugin). TUI board_card/document.rs drops updated and statusHistory.
  Guard: an in-crate test module in rust/fspec walks Cli::command() (every subcommand and arg) and the rendered help of every help config, failing on a card-id value name or placeholder other than workUnitId and on a long flag named id or work-unit-id; the init docs and bootstrap doc are scanned the same way.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. --work-unit <workUnitId> is the one flag that selects cards by id: list-work-units --id and query-metrics --work-unit-id are renamed to --work-unit (list-work-units keeps its comma list, repeats and ranges); audit and research already use it (product owner, 2026-11-08)
  #   2. <workUnitId> is the one placeholder for a card id: help USAGE, ARGUMENTS and OPTIONS (link flags --parent, --depends-on, --blocks, --blocked-by, --relates-to, --before, --after, --from, --to and positionals such as [dependsOnId]), clap value_name, error-fix hints, the init docs, the bootstrap doc and the generated AGENTS.md text (product owner, 2026-11-08)
  #   3. A card object's JSON key stays id; every reference to a card from another object is workUnitId: the audit and research args (now workUnit), list-work-units' card filter (now id), query-metrics' args and the 11 MCP tool arg structs (now work_unit_id) (product owner, 2026-11-08)
  #   4. doneAt is the one name for when a card became done: auto-advance writes doneAt instead of completedAt (product owner, 2026-11-08)
  #   5. -p means --parent everywhere: create-story, create-bug, create-task, update-work-unit and list-work-units; list-work-units and query-work-units have no short flag for --prefix (product owner, 2026-11-08)
  #   6. The stored board is migrated once, in the card's commit: updated becomes updatedAt, statusHistory becomes stateHistory and completedAt becomes doneAt; afterwards no fspec code reads or writes the old keys, and the TUI card document shows only updatedAt and stateHistory (product owner, 2026-11-08)
  #   7. No aliases and no fallbacks: an old flag fails with clap's exact unexpected-argument error, and an old JSON args key (workUnit, id, work_unit_id) fails with the exact unknown-field error (product owner, 2026-11-08)
  #   8. An exact guard test fails on any clap value_name or help placeholder for a card id other than workUnitId, and on any --id or --work-unit-id flag, naming the command and the offending name (product owner, 2026-11-08)
  #
  # EXAMPLES:
  #   1. fspec list-work-units --work-unit BUG-600..BUG-602 --format ids prints BUG-600, BUG-601 and BUG-602, and fspec list-work-units --id BUG-600 fails with exactly: error: unknown option '--id'
  #   2. fspec query-metrics --work-unit AUTH-001 prints AUTH-001's metrics, and fspec query-metrics --work-unit-id AUTH-001 fails with exactly: error: unknown option '--work-unit-id'
  #   3. fspec list-work-units -p AUTH-001 lists the children of AUTH-001; fspec list-work-units --prefix AUTH still filters by prefix; fspec query-work-units -p AUTH fails with exactly: error: unknown option '-p'
  #   4. fspec add-dependency --help shows USAGE 'fspec add-dependency <workUnitId> [workUnitId] [options]' and the options --blocks <workUnitId>, --blocked-by <workUnitId>, --depends-on <workUnitId> and --relates-to <workUnitId>
  #   5. fspec copy-virtual-hooks --help shows USAGE 'fspec copy-virtual-hooks --from <workUnitId> --to <workUnitId> [--hook-name <name>]', and create-story --help shows '-p, --parent <workUnitId>'
  #   6. The audit and research JSON args take {"workUnitId": "AUTH-001"}; the old key {"workUnit": "AUTH-001"} fails with an unknown field `workUnit` error
  #   7. The MCP tool checkpoint_diff_files takes {"workUnitId": "AUTH-001", "name": "baseline"} and its catalog schema lists workUnitId; {"work_unit_id": "AUTH-001", "name": "baseline"} fails with an unknown field `work_unit_id` error
  #   8. auto-advance of AUTH-001 from validating to done on validation-pass writes doneAt on the card and no completedAt
  #   9. After the one-time migration the repository board has no card with updated, statusHistory or completedAt, every such card carries updatedAt, stateHistory or doneAt instead, and the TUI card document of a card shows its updatedAt and stateHistory
  #   10. A clap argument with value_name WORK_UNIT_ID, a help page showing <id> for a card or a --work-unit-id flag makes the guard test fail with a message naming the command and the offending name
  #   11. add-hotspot --help's COMMON ERRORS fix suggests 'fspec show-work-unit <workUnitId>', and the init docs, their fixtures and the bootstrap doc say 'fspec show-work-unit <workUnitId>' with no <work-unit-id> or card <id> left
  #
  # ASSUMPTIONS:
  #   1. The board migration is a one-time step in the card's commit, run with fspec-core's locked board read and write (the card allows an fspec command or a one-time step); no migration command or reader for the old keys stays in the code, and the commit message reports how many keys were rewritten
  #   2. A card holding both updated and updatedAt keeps updatedAt (fspec rewrites it on every change) and loses updated; a card holding both completedAt and doneAt keeps doneAt
  #   3. The model-facing agent tool argument work_unit_id (Read, Write, Done, inspect, search and the other agent-loop tools) is not one of the 11 MCP tool arg structs and is out of this card's scope; it is reported for its own card
  #   4. add-schedule's -p (--prompt) does not select a card, so '-p means --parent everywhere' applies to the commands that take a card: create-story, create-bug, create-task, update-work-unit and list-work-units
  #   5. Placeholders for other kinds of id (<epicId>, a rule or example <index>, a checkpoint <name>, a hook <name>) are not card ids and are unchanged
  #
  # ========================================
  Background: User Story
    As a developer or AI agent using fspec
    I want to refer to a work unit with one name on every surface: the --work-unit flag, the <workUnitId> placeholder, id on a card object and workUnitId on every reference
    So that I never have to guess which spelling a command, help page, JSON payload or stored card uses

  Scenario: list-work-units selects cards with --work-unit and rejects --id
    Given a board with the cards BUG-600, BUG-601, BUG-602 and BUG-603
    When I run "fspec list-work-units --work-unit BUG-600..BUG-602 --format ids"
    Then the output is BUG-600, BUG-601 and BUG-602, one per line
    When I run "fspec list-work-units --id BUG-600"
    Then the command fails with exactly "error: unknown option '--id'"

  Scenario: query-metrics takes --work-unit and rejects --work-unit-id
    Given a board with the card AUTH-001
    When I run "fspec query-metrics --work-unit AUTH-001 --format json"
    Then the output holds the metrics of AUTH-001
    When I run "fspec query-metrics --work-unit-id AUTH-001"
    Then the command fails with exactly "error: unknown option '--work-unit-id'"

  Scenario: -p means --parent and --prefix has no short flag
    Given a board with the card AUTH-001 and its child AUTH-002, and the card BUG-001
    When I run "fspec list-work-units -p AUTH-001 --format ids"
    Then the output is AUTH-002
    When I run "fspec list-work-units --prefix BUG --format ids"
    Then the output is BUG-001
    When I run "fspec query-work-units -p AUTH"
    Then the command fails with exactly "error: unknown option '-p'"

  Scenario: Link flags show the workUnitId placeholder
    When I run "fspec add-dependency --help"
    Then the USAGE is "fspec add-dependency <workUnitId> [workUnitId] [options]"
    And the options are "--blocks <workUnitId>", "--blocked-by <workUnitId>", "--depends-on <workUnitId>" and "--relates-to <workUnitId>"

  Scenario: copy-virtual-hooks and create-story show workUnitId for their card flags
    When I run "fspec copy-virtual-hooks --help"
    Then the USAGE is "fspec copy-virtual-hooks --from <workUnitId> --to <workUnitId> [--hook-name <name>]"
    When I run "fspec create-story --help"
    Then the options show "-p, --parent <workUnitId>"

  Scenario: audit and research JSON args reference the card as workUnitId
    Given a board with the card AUTH-001
    When the audit and research commands run with the args {"workUnitId": "AUTH-001"}
    Then both accept the card
    When they run with the args {"workUnit": "AUTH-001"}
    Then both fail with an unknown field `workUnit` error

  Scenario: MCP checkpoint tools take workUnitId
    When the MCP tool checkpoint_diff_files is validated with {"workUnitId": "AUTH-001", "name": "baseline"}
    Then the arguments are accepted
    And the catalog schema of every checkpoint tool lists the property workUnitId and no work_unit_id
    When it is validated with {"work_unit_id": "AUTH-001", "name": "baseline"}
    Then it fails with an unknown field `work_unit_id` error

  Scenario: auto-advance to done records doneAt
    Given the card AUTH-001 in validating
    When auto-advance moves AUTH-001 from validating on the validation-pass event
    Then AUTH-001 is done and carries doneAt
    And AUTH-001 has no completedAt

  Scenario: The board holds only the new keys after the one-time migration
    Given the repository board after the CLI-020 migration
    Then no card has the key updated, statusHistory or completedAt
    When the TUI renders the card document of a card that has updated and statusHistory keys
    Then the document shows its updatedAt and stateHistory and no updated or statusHistory entry in the History or Timestamps sections

  Scenario: The guard test rejects every other card-id name
    Given every fspec subcommand, argument and help page
    Then every clap value name for a card id is workUnitId
    And every help placeholder for a card id is <workUnitId>
    And no subcommand has an --id or --work-unit-id flag
    And a value name WORK_UNIT_ID, a placeholder <id> for a card or a --work-unit-id flag fails the guard with a message naming the command and the offending name

  Scenario: Error hints and docs name the card workUnitId
    When I run "fspec add-hotspot --help"
    Then its COMMON ERRORS fix says "Ensure work unit exists: fspec show-work-unit <workUnitId>"
    And the init docs, their fixtures and the bootstrap doc say "fspec show-work-unit <workUnitId>" and hold no "<work-unit-id>" and no card "<id>"
