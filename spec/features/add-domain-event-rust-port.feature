@event-storm
@cli
@event-storming
@rust
@RPC-179
Feature: Port add-domain-event command to Rust
  """
  Core impl at codelet/fspec-core/src/commands/add_domain_event.rs. INLINES the logic (does NOT use the shared addEventStormItem util) because it has the BUG-087 dedup step. Reads spec/work-units.json (existsSync check first, NO auto-create on missing), mutates wu.extra['eventStorm'] map, write_json_atomic. Item field order: id, type, color, text, deleted, createdAt, then optional timestamp/boundedContext appended.
  Returns {success, eventId} from core; CLI bridge codelet/fspec/src/add_domain_event.rs is a clap-derived struct (workUnitId, text, --timestamp, --bounded-context) that marshals to dispatch and formats the success/failure stdout/stderr lines (no domain logic).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When spec/work-units.json does not exist, return error 'spec/work-units.json not found. Run fspec init first.'
  #   2. When the work unit ID is not found, return error 'Work unit <id> not found'
  #   3. When the work unit status is done or blocked, return error 'Cannot add Event Storm items to work unit in <status> state'
  #   4. The eventStorm section is initialized with level 'process_modeling', empty items, nextItemId 0 when missing
  #   5. BUG-087: before adding, scan non-deleted items of type 'event' for a case-insensitive text match; if found return 'Event '<text>' already exists (ID: <existingId>)'
  #   6. A successful add appends an item with type 'event', color 'orange', id=nextItemId, deleted=false, createdAt set; then increments nextItemId; returns the eventId
  #   7. Optional --timestamp and --bounded-context are appended to the item only when provided
  #   8. A soft-deleted event with the same text does not block re-adding (dedup ignores deleted items)
  #
  # EXAMPLES:
  #   1. Given a work unit RPC-179 in specifying state with no eventStorm, when add_domain_event adds 'UserRegistered', then an item with id 0, type 'event', color 'orange', deleted false is appended and eventId 0 is returned
  #   2. Adding 'UserRegistered' twice (same case) returns error "Event 'UserRegistered' already exists (ID: 0)" and the file is unchanged
  #   3. Adding 'userregistered' after 'UserRegistered' triggers the case-insensitive dedup error
  #   4. Adding with --timestamp 1000 --bounded-context Sales appends timestamp 1000 and boundedContext 'Sales' to the item
  #   5. Adding an event to a missing work unit ID 'NOPE-1' returns 'Work unit NOPE-1 not found'
  #   6. Adding an event when spec/work-units.json is absent returns 'spec/work-units.json not found. Run fspec init first.'
  #   7. Adding an event to a work unit in 'done' state returns 'Cannot add Event Storm items to work unit in done state'
  #   8. CLI: fspec add-domain-event RPC-179 'UserRegistered' exits 0 and stdout shows '✓ Added domain event "UserRegistered" to RPC-179 (ID: 0)'
  #   9. CLI: fspec add-domain-event with a duplicate event exits 1 and stderr shows '✗ Failed to add domain event:' with the dedup error
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to port the add-domain-event command to Rust (fspec-core)
    So that the dispatcher and standalone binary can add domain events to a work unit's Event Storm with byte-for-byte parity to the TypeScript implementation

  Scenario: Add first domain event to a work unit with no Event Storm
    given a work unit "RPC-179" in the "specifying" state with no Event Storm
    when I add a domain event "UserRegistered"
    then the command returns eventId 0
    And the Event Storm contains an item with id 0, type "event", color "orange", deleted false
    And the eventStorm has level "process_modeling" and nextItemId 1

  Scenario: Reject duplicate event with same case
    given a work unit "RPC-179" with a non-deleted event "UserRegistered" at id 0
    when I add a domain event "UserRegistered"
    then the command fails with error "Event 'UserRegistered' already exists (ID: 0)"
    And the work units file is unchanged

  Scenario: Append optional timestamp and bounded context
    given a work unit "RPC-179" in the "specifying" state with no Event Storm
    when I add a domain event "OrderPlaced" with timestamp 1000 and bounded context "Sales"
    then the item has timestamp 1000 and boundedContext "Sales"

  Scenario: Re-add an event whose prior occurrence was soft-deleted
    given a work unit "RPC-179" with a soft-deleted event "UserRegistered" at id 0
    when I add a domain event "UserRegistered"
    then the command succeeds and a new non-deleted event is appended

  Scenario: Reject add for a missing work unit
    given a work units file that does not contain "NOPE-1"
    when I add a domain event "X" to "NOPE-1"
    then the command fails with error "Work unit NOPE-1 not found"

  Scenario: Reject add when work units file is absent
    given there is no spec/work-units.json file
    when I add a domain event "X" to "RPC-179"
    then the command fails with error "spec/work-units.json not found. Run fspec init first."
    And no spec/work-units.json file is created

  Scenario: Reject add for a work unit in done state
    given a work unit "RPC-179" in the "done" state
    when I add a domain event "X" to "RPC-179"
    then the command fails with error "Cannot add Event Storm items to work unit in done state"

  Scenario: Reject duplicate event case-insensitively
    given a work unit "RPC-179" with a non-deleted event "UserRegistered" at id 0
    when I add a domain event "userregistered"
    then the command fails with error "Event 'userregistered' already exists (ID: 0)"
