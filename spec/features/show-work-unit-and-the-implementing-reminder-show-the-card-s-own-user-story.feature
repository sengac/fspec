@done
@cli
@querying
@system-reminders
@bug-fix
@rust
@BUG-660
Feature: show-work-unit and the implementing reminder show the card's own user story
  """
  show-work-unit (rust/fspec-core/src/commands/show_work_unit.rs) projects the card's stored userStory into its JSON after relatesTo and before rules, as {role, action, benefit}, and prints it in the text output before the Rules section. A stored userStory that is not an object with string role, action and benefit is an exact error, never a blank or partial story. The implementing reminder (update_work_unit_status/reminders/enhanced.rs) reads the moved card's userStory from the board it has just written under the spec lock; investigation found no code path that reads another card's story (BUG-628's odd story was the one a paused helper had stored on that same card), and a test pins the reminder to the moved card's own story.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. show-work-unit --format json carries userStory as {role, action, benefit} when the card has one, placed before rules, and omits the key when the card has none
  #   2. show-work-unit's text output shows 'User Story: As a <role>, I want to <action>, so that <benefit>' before the Rules section when the card has one
  #   3. A stored userStory that is not an object with string role, action and benefit fails show-work-unit with an exact error naming the card and the field, never a blank or partial story
  #   4. The implementing reminder shows the user story of the card being moved, never another card's
  #
  # EXAMPLES:
  #   1. AUTH-001 has userStory {role: 'developer', action: 'log in', benefit: 'I reach my board'}; show-work-unit AUTH-001 --format json has userStory with exactly those three values
  #   2. AUTH-002 has no userStory; show-work-unit AUTH-002 --format json has no userStory key
  #   3. show-work-unit AUTH-001 (text) prints the line 'User Story: As a developer, I want to log in, so that I reach my board'
  #   4. AUTH-003's userStory is the string 'legacy'; show-work-unit AUTH-003 fails with "Work unit AUTH-003 has a malformed userStory field: expected an object with string role, action and benefit. Fix spec/work-units.json."
  #   5. AUTH-001 and AUTH-002 have different user stories; moving AUTH-002 to implementing prints AUTH-002's story in the USER STORY line and not AUTH-001's
  #
  # ========================================
  Background: User Story
    As a AI agent or developer reading a card
    I want to see the card's own user story in show-work-unit's JSON and text output and in the implementing reminder
    So that I work from the story that is actually set on the card

  Scenario: show-work-unit JSON carries the card's user story before its rules
    Given work unit "AUTH-001" has the user story role "developer", action "log in" and benefit "I reach my board", and one rule
    When I run show-work-unit for "AUTH-001" with format "json"
    Then the JSON field "userStory" is exactly {"role": "developer", "action": "log in", "benefit": "I reach my board"}
    And the key "userStory" comes after "status" and before "rules" in the JSON object

  Scenario: show-work-unit JSON has no userStory key for a card without a user story
    Given work unit "AUTH-002" has no user story
    When I run show-work-unit for "AUTH-002" with format "json"
    Then the JSON object has no "userStory" key

  Scenario: show-work-unit text output prints the user story line before the rules
    Given work unit "AUTH-001" has the user story role "developer", action "log in" and benefit "I reach my board", and one rule
    When I run show-work-unit for "AUTH-001" with the default text format
    Then the output has the line "User Story: As a developer, I want to log in, so that I reach my board"
    And that line comes before the "Rules:" line

  Scenario: show-work-unit fails with an exact error for a malformed stored user story
    Given work unit "AUTH-003" has the user story stored as the string "legacy"
    When I run show-work-unit for "AUTH-003"
    Then the command fails with "Work unit AUTH-003 has a malformed userStory field: expected an object with string role, action and benefit. Fix spec/work-units.json."

  Scenario: The implementing reminder shows the moved card's own user story
    Given task "AUTH-001" in testing has the user story role "developer", action "log in" and benefit "I reach my board"
    And task "AUTH-002" in testing has the user story role "reviewer", action "approve a change" and benefit "only reviewed code ships"
    When I move "AUTH-002" to implementing
    Then the output has the line 'USER STORY: "As a reviewer, I want to approve a change, so that only reviewed code ships"'
    And the output does not contain "As a developer, I want to log in"
