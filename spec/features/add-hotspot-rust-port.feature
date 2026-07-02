@event-storm
@cli
@event-storming
@rust
@RPC-185
Feature: Port add-hotspot command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/add_hotspot.rs. Uses the SHARED addEventStormItem util (event-storm-utils style — NOT inlined, NO dedup), because hotspots may repeat. Reads spec/work-units.json (existsSync check first, NO auto-create), mutates wu.extra['eventStorm'] map, write_json_atomic. Item construction: itemData fields (type 'hotspot', color 'red', text, optional concern/timestamp/boundedContext) first, then id, deleted, createdAt appended.
  Returns {success, hotspotId} from core; CLI bridge codelet/fspec/src/add_hotspot.rs is a clap-derived struct (workUnitId, text, --concern, --timestamp, --bounded-context) that marshals to dispatch and formats success/failure stdout/stderr lines (no domain logic). Success line: '✓ Hotspot added to <workUnitId> (id: <hotspotId>)'; failure: '✗ Failed to add hotspot: <message>'.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When spec/work-units.json does not exist, return error 'spec/work-units.json not found. Run fspec init first.'
  #   2. When the work unit ID is not found, return error 'Work unit <id> not found'
  #   3. When the work unit status is done or blocked, return error 'Cannot add Event Storm items to work unit in <status> state'
  #   4. Uses the shared addEventStormItem util (no dedup): unlike add-domain-event, the same hotspot text may be added multiple times
  #   5. The eventStorm section is initialized with level 'process_modeling', empty items, nextItemId 0 when missing
  #   6. A successful add appends an item via the util: itemData fields (type 'hotspot', color 'red', text, optional concern/timestamp/boundedContext) come first, then id=nextItemId, deleted=false, createdAt; nextItemId increments; returns hotspotId
  #   7. Optional --concern, --timestamp and --bounded-context are appended to the item only when provided
  #
  # EXAMPLES:
  #   1. Given a work unit RPC-185 in specifying state with no eventStorm, when add_hotspot adds 'Unclear retry policy', then an item with id 0, type 'hotspot', color 'red', deleted false is appended and hotspotId 0 is returned
  #   2. Adding the same hotspot text twice succeeds both times (no dedup), producing ids 0 and 1
  #   3. Adding with --concern 'How long to wait?' --timestamp 500 --bounded-context Payments appends concern, timestamp 500 and boundedContext 'Payments'
  #   4. Adding a hotspot to a missing work unit ID 'NOPE-1' returns 'Work unit NOPE-1 not found'
  #   5. Adding a hotspot when spec/work-units.json is absent returns 'spec/work-units.json not found. Run fspec init first.' and no file is created
  #   6. Adding a hotspot to a work unit in 'blocked' state returns 'Cannot add Event Storm items to work unit in blocked state'
  #   7. CLI: fspec add-hotspot RPC-185 'Unclear retry policy' exits 0 and stdout shows '✓ Hotspot added to RPC-185 (id: 0)'
  #   8. CLI: fspec add-hotspot for a missing work unit exits 1 and stderr shows '✗ Failed to add hotspot:' with the not-found error
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to port the add-hotspot command to Rust (fspec-core)
    So that the dispatcher and standalone binary can add hotspots to a work unit's Event Storm with byte-for-byte parity to the TypeScript implementation

  Scenario: Add first hotspot to a work unit with no Event Storm
    given a work unit "RPC-185" in the "specifying" state with no Event Storm
    when I add a hotspot "Unclear retry policy"
    then the command returns hotspotId 0
    And the Event Storm contains an item with id 0, type "hotspot", color "red", deleted false
    And the eventStorm has level "process_modeling" and nextItemId 1

  Scenario: Add the same hotspot text twice without deduplication
    given a work unit "RPC-185" with a non-deleted hotspot "Unclear retry policy" at id 0
    when I add a hotspot "Unclear retry policy" again
    then the command succeeds and returns hotspotId 1
    And the Event Storm now contains two non-deleted hotspots with the same text

  Scenario: Append optional concern, timestamp and bounded context
    given a work unit "RPC-185" in the "specifying" state with no Event Storm
    when I add a hotspot "Timeout unknown" with concern "How long to wait?", timestamp 500 and bounded context "Payments"
    then the item has concern "How long to wait?", timestamp 500 and boundedContext "Payments"

  Scenario: Reject add for a missing work unit
    given a work units file that does not contain "NOPE-1"
    when I add a hotspot "X" to "NOPE-1"
    then the command fails with error "Work unit NOPE-1 not found"

  Scenario: Reject add when work units file is absent
    given there is no spec/work-units.json file
    when I add a hotspot "X" to "RPC-185"
    then the command fails with error "spec/work-units.json not found. Run fspec init first."
    And no spec/work-units.json file is created

  Scenario: Reject add for a work unit in blocked state
    given a work unit "RPC-185" in the "blocked" state
    when I add a hotspot "X" to "RPC-185"
    then the command fails with error "Cannot add Event Storm items to work unit in blocked state"
