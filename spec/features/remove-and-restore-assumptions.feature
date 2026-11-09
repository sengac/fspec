@done
@cli
@assumption-management
@bug-fix
@rust
@BUG-633
Feature: Remove and restore assumptions
  """
  Assumptions were plain strings with no id or deleted flag, so a superseded provisional assumption could never be retracted. They now use the stable-id object form of rules ({id, text, deleted, createdAt, deletedAt?} with nextAssumptionId); a card's string assumptions are converted, keeping each index as its id, the first time a command writes them (add-assumption, remove-assumption, restore-assumption, answer-question --add-to assumption, compact-work-unit), and until then every reader takes a string at position i as active assumption i (product owner, 2026-11-08). remove-assumption / restore-assumption soft-delete and restore by stable id while specifying; show-deleted lists removed assumptions and compact-work-unit drops them.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Product owner, 2026-11-08: option (a). Each assumption is stored as {id, text, deleted, createdAt, deletedAt?} with a nextAssumptionId counter, like rules; existing assumptions keep their current index as their id. A card's string assumptions are converted the first time a command writes that card's assumptions (add-assumption, remove/restore-assumption, answer-question --add-to assumption); until then every reader reads a string at position i as active assumption i. show-work-unit --format json keeps 'assumptions' as the active texts.
  #   2. remove-assumption <id> <index> soft-deletes the assumption whose stable id is <index> (deleted: true, deletedAt); restore-assumption <id> <index> clears both. Both work only in specifying, like remove-question/restore-question; an unknown index or a card without assumptions is an error; removing an already removed (or restoring an active) assumption succeeds without writing
  #   3. Readers show only active assumptions with their stable ids: show-work-unit ([id] text), list-work-units --has-assumptions and --fields assumptions, generate-scenarios; validate-work-units accepts the string and the object form; export-example-map, import-example-map and query-example-mapping-stats treat assumptions like the other item kinds, and an imported assumption gets the card's next id
  #   4. show-deleted lists removed assumptions (after architecture notes) and compact-work-unit drops them, renumbers the survivors from 0 and resets nextAssumptionId, as it does for the other item kinds
  #   5. remove-assumption and restore-assumption have --help pages with usage, arguments and examples, and are named among the related commands of add-assumption and show-deleted
  #
  # EXAMPLES:
  #   1. A card with string assumptions ["A", "B"]: remove-assumption CARD 1 stores [{id 0, text A}, {id 1, text B, deleted, deletedAt}] with nextAssumptionId 2; show-work-unit lists only [0] A
  #   2. restore-assumption CARD 1 makes [1] B active again, with no deletedAt
  #   3. add-assumption on a card with string assumptions ["A"] stores [{id 0, text A}, {id 1, text C}] and nextAssumptionId 2
  #   4. remove-assumption CARD 5 on a card with two assumptions fails: Assumption with ID 5 not found
  #   5. show-deleted CARD lists the removed assumption B with its id and deletedAt; compact-work-unit CARD then leaves [{id 0, text A}] and nextAssumptionId 1
  #   6. A card nobody touched keeps ["A", "B"] on disk and show-work-unit lists [0] A and [1] B
  #
  # QUESTIONS (ANSWERED):
  #   Q: Assumptions are stored as plain strings (no id, no deleted flag), unlike rules/examples/questions/notes, so a soft delete needs a new stored form. Which? (a) RECOMMENDED: each assumption becomes {id, text, deleted, createdAt, deletedAt?} with a nextAssumptionId counter, like rules; existing assumptions keep their current index as their id, so '[0] ...' stays [0]. A card's string assumptions are converted to that form the first time a command writes that card's assumptions (add-assumption, remove/restore-assumption, answer-question --add-to assumption); until then every reader (show-work-unit, list-work-units, validate-work-units, export/import-example-map, generate-scenarios, stats, show-deleted, compact) reads a string at position i as active assumption i. show-work-unit --format json keeps 'assumptions' as the active texts (like rules). (b) Same object form, but a one-time 'fspec repair-work-units' converts every card and readers then refuse the string form with an error naming the card and the repair command (strict, but every existing board must run the repair once). (c) Keep strings; remove-assumption is a hard delete (indices shift, no restore, nothing in show-deleted).
  #   A: Product owner, 2026-11-08: option (a). Each assumption is stored as {id, text, deleted, createdAt, deletedAt?} with a nextAssumptionId counter, like rules; existing assumptions keep their current index as their id. A card's string assumptions are converted the first time a command writes that card's assumptions (add-assumption, remove/restore-assumption, answer-question --add-to assumption); until then every reader reads a string at position i as active assumption i. show-work-unit --format json keeps 'assumptions' as the active texts.
  #
  # ========================================
  Background: User Story
    As an agent or developer doing Example Mapping on a card
    I want to remove an assumption that a later answer superseded, and restore it if I removed the wrong one
    So that the card's assumptions show only what still holds, with stable indices and the removal visible in show-deleted

  Scenario: Removing an assumption converts the card's string assumptions and soft-deletes it
    Given a card in specifying with string assumptions "A" and "B"
    When "remove-assumption CARD 1" runs
    Then the card stores assumption 0 "A" active and assumption 1 "B" deleted with a deletedAt, and nextAssumptionId 2
    And show-work-unit lists only "[0] A" under Assumptions

  Scenario: Restoring an assumption makes it active again
    Given a card whose assumption 1 "B" was removed
    When "restore-assumption CARD 1" runs
    Then assumption 1 "B" is active with no deletedAt
    And show-work-unit lists "[0] A" and "[1] B"

  Scenario: Adding an assumption to a card with string assumptions continues the ids
    Given a card in specifying with the string assumption "A"
    When "fspec add-assumption CARD C" runs
    Then the card stores assumption 0 "A" and assumption 1 "C" as objects, and nextAssumptionId 2

  Scenario: An unknown assumption id is an error
    Given a card in specifying with two assumptions
    When "remove-assumption CARD 5" runs, and then "restore-assumption CARD 5"
    Then each fails with "Assumption with ID 5 not found" and the card is unchanged

  Scenario: Removing twice and restoring an active assumption change nothing
    Given a card whose assumption 1 was removed
    When "remove-assumption CARD 1" runs again, and "restore-assumption CARD 0" runs
    Then both succeed saying the assumption is already deleted or already active
    And the card's file is not rewritten

  Scenario: Assumptions are removed and restored only while specifying
    Given a card in testing with assumptions
    When "remove-assumption CARD 0" runs
    Then it fails saying assumptions can only be removed during discovery/specification

  Scenario: show-deleted lists removed assumptions and compaction drops them
    Given a card whose assumption 1 "B" was removed
    When "fspec show-deleted CARD" runs and then, with the card done, "fspec compact-work-unit CARD"
    Then show-deleted lists assumption 1 "B" with its deletedAt
    And after compaction the card stores only assumption 0 "A" and nextAssumptionId 1

  Scenario: Readers treat string and object assumptions alike and skip removed ones
    Given a card with string assumptions "A" and "B" and another card whose assumption 1 was removed
    When show-work-unit, list-work-units --has-assumptions --fields assumptions and validate-work-units read them
    Then the untouched card shows "[0] A" and "[1] B", the other shows only "[0] A", and validate-work-units reports no error

  Scenario: The new commands have help pages
    When "remove-assumption --help" and "restore-assumption --help" run
    Then each prints its usage, arguments and an example
    And "fspec add-assumption --help" and "fspec show-deleted --help" name remove-assumption and restore-assumption among related commands
