@done
@CLI-017
@rust
@querying
@cli
Feature: Board queries with list-work-units
  """
  fspec list-work-units is the one board query command (product owner, 2026-11-07: extend it, no second command).
  The core command rust/fspec-core/src/commands/list_work_units.rs reads spec/work-units.json once, applies exact filters (OR within a filter, AND across filters), then picks fields, sorts, limits, groups and renders.
  The clock and local time zone are injected: run() uses the system clock and zone, run_at() takes a BoardClock so tests use a fixed now and a named zone (TEST-042, no wall clock).
  The fspec CLI (rust/fspec/src/list_work_units.rs) only maps clap flags to the core's JSON args; the LLM dispatcher sends the same JSON.
  Feature files are read only when --tag is given, through list-features' feature-header reader.
  The init docs and the bootstrap doc tell agents to query the board with list-work-units and never read spec/work-units.json directly.
  """

  Background: User Story
    As a AI agent or developer working on an fspec board
    I want to find cards by time range, history, text, links and any other board field with fspec list-work-units
    So that nobody reads or parses spec/work-units.json with jq, grep or scripts

  Scenario: Cards done in the last two days are counted per local day
    Given a board in time zone Australia/Brisbane where now is 2026-11-07T06:00:00Z
    And CLI-001 was done at 2026-11-05T15:30:00Z, CLI-002 at 2026-11-06T23:00:00Z and CLI-003 at 2026-11-04T10:00:00Z
    When I list work units with "--status done --done-since 2d --group-by day"
    Then the output counts 1 card on 2026-11-06 and 1 card on 2026-11-07

  Scenario: Cards in testing or specifying with open questions
    Given CLI-001 in testing has an open question, CLI-002 in specifying has an open question, CLI-003 in testing has only an answered question and CLI-004 in implementing has an open question
    When I list work units with "--status testing,specifying --has-open-questions --format ids"
    Then the output is the ids CLI-001 and CLI-002

  Scenario: Bugs created since a date that are not done
    Given a board in time zone Australia/Brisbane with bug BUG-001 created 2026-10-29T14:00:00Z, bug BUG-002 created 2026-10-30T02:00:00Z, done bug BUG-003 created 2026-11-01T00:00:00Z and story CLI-001 created 2026-11-01T00:00:00Z
    When I list work units with "--type bug --created-since 2026-10-30 --status backlog,specifying,testing,implementing,validating,blocked --format ids"
    Then the output is the ids BUG-001 and BUG-002

  Scenario: Ids of the cards blocked by a card
    Given BUG-002 and BUG-003 are blocked by BUG-001 and BUG-004 depends on BUG-001
    When I list work units with "--blocked-by BUG-001 --format ids"
    Then the output is the ids BUG-002 and BUG-003

  Scenario: Cards with assumptions show their provisional decisions
    Given CLI-001 has the assumptions "uses the local zone" and "2d is 48 h" and CLI-002 has none
    When I list work units with "--has-assumptions --fields id,assumptions --format json"
    Then the JSON lists only CLI-001 with assumptions "uses the local zone" and "2d is 48 h"

  Scenario: Cards that entered validating today whatever their status now
    Given a board in time zone Australia/Brisbane where now is 2026-11-07T06:00:00Z
    And CLI-001 entered validating at 2026-11-06T15:00:00Z and is now done, CLI-002 entered validating at 2026-11-06T13:00:00Z and CLI-003 never entered validating
    When I list work units with "--entered validating --entered-since 2026-11-07 --format ids"
    Then the output is the ids CLI-001

  Scenario: A local day follows the zone's daylight saving change
    Given a board in time zone America/New_York where now is 2026-11-07T06:00:00Z
    And CLI-001 was created at 2026-11-02T04:30:00Z and CLI-002 at 2026-11-02T05:30:00Z
    When I list work units with "--created-since 2026-11-02 --format ids"
    Then the output is the ids CLI-002
    When I list work units with "--created-since 2026-11-01 --group-by day"
    Then the output counts 1 card on 2026-11-01 and 1 card on 2026-11-02

  Scenario: A date-only until value covers that whole local day
    Given a board in time zone Australia/Brisbane with CLI-001 created at 2026-10-30T13:59:00Z and CLI-002 created at 2026-10-30T14:00:00Z
    When I list work units with "--created-until 2026-10-30 --format ids"
    Then the output is the ids CLI-001

  Scenario: Relative durations, timestamps and now
    Given a board in time zone Australia/Brisbane where now is 2026-11-07T06:00:00Z
    And CLI-001 was updated at 2026-11-07T05:45:00Z, CLI-002 at 2026-11-07T01:00:00Z, CLI-003 at 2026-11-01T07:00:00Z and CLI-004 at 2026-10-20T00:00:00Z
    When I list work units with "--updated-since 30m --format ids"
    Then the output is the ids CLI-001
    When I list work units with "--updated-since 6h --updated-until now --format ids"
    Then the output is the ids CLI-001 and CLI-002
    When I list work units with "--updated-since 1w --updated-until 2026-11-07T11:30:00+10:00 --format ids"
    Then the output is the ids CLI-002 and CLI-003

  Scenario: An unknown status is an exact error listing the valid statuses
    Given a board with one card
    When I list work units with "--status donee"
    Then it fails with "Unknown status 'donee'. Valid statuses: backlog, specifying, testing, implementing, validating, done, blocked"

  Scenario: Unknown type, prefix, epic, field, format, group-by and sort key are exact errors listing the valid values
    Given a board with prefixes CLI and BUG and epic board-queries registered
    When I list work units with "--type feature"
    Then it fails with "Unknown type 'feature'. Valid types: story, task, bug"
    When I list work units with "--prefix NOPE"
    Then it fails with "Unknown prefix 'NOPE'. Valid prefixes: BUG, CLI"
    When I list work units with "--epic nope"
    Then it fails with "Unknown epic 'nope'. Valid epics: board-queries"
    When I list work units with "--fields id,colour"
    Then it fails with an error naming the field colour and listing the valid fields
    When I list work units with "--format text"
    Then it fails with "Unknown format 'text'. Valid formats: table, ids, json, jsonl, count, csv"
    When I list work units with "--group-by colour"
    Then it fails with "Unknown group-by 'colour'. Valid group-by values: status, type, prefix, epic, day, week"
    When I list work units with "--sort colour"
    Then it fails with an error naming the sort key colour and listing the valid fields

  Scenario: Invalid time values and ids are exact errors
    Given a board with one card CLI-001
    When I list work units with "--created-since yesterday"
    Then it fails with an error naming 'yesterday' and the accepted forms YYYY-MM-DD, an ISO-8601 timestamp, Nm, Nh, Nd, Nw and now
    When I list work units with "--entered-since 2d"
    Then it fails with "--entered-since and --entered-until need --entered <status>"
    When I list work units with "--blocked-by CLI-999"
    Then it fails with "Work unit 'CLI-999' is not on the board"
    When I list work units with "--work-unit BUG-001..CLI-010"
    Then it fails with "A --work-unit range must use one prefix: BUG-001..CLI-010"

  Scenario: Id lists, id ranges and field filters
    Given CLI-001 is the parent of CLI-002 and CLI-003, CLI-002 has estimate 3, CLI-003 has estimate 8 and CLI-004 has no estimate
    When I list work units with "--work-unit CLI-002..CLI-004 --format ids"
    Then the output is the ids CLI-002, CLI-003 and CLI-004
    When I list work units with "--work-unit CLI-001,CLI-004 --format ids"
    Then the output is the ids CLI-001 and CLI-004
    When I list work units with "--parent CLI-001 --estimate-min 5 --format ids"
    Then the output is the ids CLI-003
    When I list work units with "--has-children --format ids"
    Then the output is the ids CLI-001
    When I list work units with "--no-estimate --format ids"
    Then the output is the ids CLI-001 and CLI-004
    When I list work units with "--has-estimate --estimate-max 3 --format ids"
    Then the output is the ids CLI-002

  Scenario: Text and content filters ignore deleted items
    Given CLI-001 is titled "Board Query" with a rule and an example, CLI-002 has "board query" in its description, an attachment and an architecture note, and CLI-003 has only deleted rules
    When I list work units with "--text BOARD --format ids"
    Then the output is the ids CLI-001 and CLI-002
    When I list work units with "--title board --format ids"
    Then the output is the ids CLI-001
    When I list work units with "--has-rules --has-examples --format ids"
    Then the output is the ids CLI-001
    When I list work units with "--no-rules --format ids"
    Then the output is the ids CLI-002 and CLI-003
    When I list work units with "--has-attachments --has-architecture-notes --format ids"
    Then the output is the ids CLI-002

  Scenario: A tag on a card's feature file selects the card
    Given spec/features/a.feature carries the feature tags @CLI-001 and @cli, and spec/features/b.feature carries @CLI-002 and @tui
    When I list work units with "--tag @cli --format ids"
    Then the output is the ids CLI-001

  Scenario: Link filters take an id or any, and blocked cards include unfinished blockers
    Given BUG-002 depends on BUG-001, BUG-003 relates to BUG-001, BUG-004 is blocked by done BUG-005, BUG-006 is blocked by BUG-001 and BUG-007 has status blocked
    When I list work units with "--depends-on any --format ids"
    Then the output is the ids BUG-002
    When I list work units with "--relates-to BUG-001 --format ids"
    Then the output is the ids BUG-003
    When I list work units with "--blocks any --format ids"
    Then the output is the ids BUG-001 and BUG-005
    When I list work units with "--blocked --format ids"
    Then the output is the ids BUG-006 and BUG-007

  Scenario: The table format is the default and an empty result prints the sentinel
    Given CLI-001 is a backlog story in epic board-queries titled "Board queries" and CLI-002 is a done bug titled "Fix, then test" with no epic
    When I list work units with no options
    Then the output is a table with the header ID, TYPE, STATUS, EPIC and TITLE, one aligned row per card and the line "2 work units"
    When I list work units with "--status testing"
    Then the output is "No work units found"

  Scenario: The ids, json, jsonl, count and csv formats
    Given CLI-001 is a backlog story in epic board-queries titled "Board queries" and CLI-002 is a done bug titled "Fix, then test" with no epic
    When I list work units with "--format json"
    Then the JSON workUnits array holds CLI-001 with epic board-queries and CLI-002 without an epic key
    When I list work units with "--format jsonl"
    Then the output is two one-line JSON objects
    When I list work units with "--format count"
    Then the output is "2"
    When I list work units with "--format csv --fields id,title"
    Then the output is the lines "id,title", "CLI-001,Board queries" and "CLI-002,\"Fix, then test\""

  Scenario: Derived time fields print in local time with the zone
    Given a board in time zone Australia/Brisbane where now is 2026-11-07T06:00:00Z
    And CLI-001 was created at 2026-11-01T00:00:00Z, entered testing at 2026-11-03T02:00:00Z and was done at 2026-11-05T03:00:00Z, and CLI-002 was created at 2026-11-04T01:48:00Z and never changed status
    When I list work units with "--fields id,createdAt,enteredAt:testing,doneAt,ageInStatus --format json"
    Then CLI-001 has createdAt "2026-11-01T10:00:00+10:00", enteredAt:testing "2026-11-03T12:00:00+10:00", doneAt "2026-11-05T13:00:00+10:00" and ageInStatus "2d 3h 0m"
    And CLI-002 has ageInStatus "3d 4h 12m" and no doneAt or enteredAt:testing key

  Scenario: Open questions, dependencies and list fields
    Given CLI-001 has open question 1 "@human: which zone?", answered question 0 and dependsOn CLI-002, and CLI-002 blocks CLI-003
    When I list work units with "--work-unit CLI-001 --fields id,openQuestions,dependencies --format json"
    Then CLI-001 has openQuestions [{"id": 1, "text": "@human: which zone?"}] and dependencies {"dependsOn": ["CLI-002"]}
    When I list work units with "--work-unit CLI-001,CLI-002 --fields id,openQuestions,blocks --format csv"
    Then the output is the lines "id,openQuestions,blocks", "CLI-001,[1] @human: which zone?," and "CLI-002,,CLI-003"

  Scenario: Group by status, type, prefix, epic and week
    Given BUG-001 and BUG-002 are backlog bugs in epic board-queries, CLI-001 is a done story and CLI-002 is a testing story in epic board-queries
    When I list work units with "--group-by status"
    Then the output counts backlog 2, testing 1 and done 1 in workflow order
    When I list work units with "--group-by prefix --format csv"
    Then the output is the lines "prefix,count", "BUG,2" and "CLI,2"
    When I list work units with "--group-by epic --format json"
    Then the JSON groups are board-queries with 3 cards and the key null with CLI-001
    When I list work units with "--group-by type --format count"
    Then the output counts bug 2 and story 2

  Scenario: Group by day or week needs exactly one kind of time filter
    Given a board in time zone Australia/Brisbane where now is 2026-11-07T06:00:00Z
    And CLI-001 was created at 2026-11-01T00:00:00Z and CLI-002 at 2026-11-03T00:00:00Z
    When I list work units with "--created-since 2026-10-01 --group-by week"
    Then the output counts 1 card in 2026-W44 and 1 card in 2026-W45
    When I list work units with "--group-by day"
    Then it fails with "--group-by day needs exactly one kind of time filter (created, updated, done or entered)"
    When I list work units with "--created-since 2026-10-01 --done-since 2d --group-by day"
    Then it fails with "--group-by day needs exactly one kind of time filter (created, updated, done or entered)"
    When I list work units with "--created-since 2026-10-01 --group-by week --format ids"
    Then it fails with "--group-by works with --format table, count, csv or json"

  Scenario: Sort and limit
    Given CLI-001 was created at 2026-11-01T00:00:00Z, CLI-002 at 2026-11-03T00:00:00Z and CLI-003 at 2026-11-02T00:00:00Z
    When I list work units with "--sort createdAt --desc --limit 2 --format ids"
    Then the output is the ids CLI-002 and CLI-003
    When I list work units with "--sort createdAt --format ids"
    Then the output is the ids CLI-001, CLI-003 and CLI-002

  Scenario: A large board answers an exact query
    Given a generated board of 2500 cards of about 13 MB with known done times
    When I list work units with "--type bug --done-since 2d --group-by day --format json"
    Then the JSON group counts equal the counts computed from the generated cards

  Scenario: The fspec CLI passes every new flag to the core command
    Given a project whose board has bug BUG-001 created 2026-10-30T02:00:00Z and story CLI-001 created 2026-10-29T00:00:00Z
    When I run fspec with TZ=Australia/Brisbane and "list-work-units --type bug --created-since 2026-10-30 --fields id,createdAt --format csv"
    Then the command exits 0 and prints the lines "id,createdAt" and "BUG-001,2026-10-30T12:00:00+10:00"
    When I run fspec with TZ=Australia/Brisbane and "list-work-units --status donee"
    Then the command exits 1 and stderr contains "Unknown status 'donee'"

  Scenario: The list-work-units help documents the board queries
    Given the fspec binary has been compiled
    When I run "fspec list-work-units --help"
    Then stdout is byte-for-byte identical to the fixture rust/fspec/tests/fixtures/help/list-work-units.txt
    And the help lists --done-since, --entered, --has-open-questions, --blocked-by, --fields, --group-by and --sort

  Scenario: The workflow docs tell agents to query the board with list-work-units
    Given the init doc templates, their fixtures and the bootstrap doc
    When I read each of them
    Then each has a "Querying the board" section with list-work-units examples
    And each says never to read or parse spec/work-units.json directly with jq, grep or scripts
