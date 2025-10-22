@FEAT-016
@cli
@estimation
@system-reminder
Feature: Warn AI when estimate is > 13 points to break down work unit
  """
  TWO PLACEMENT POINTS: (1) update-work-unit-estimate.ts - immediate warning when estimate set, (2) show-work-unit.ts - persistent reminder via getLargeEstimateReminder() in system-reminder.ts
  SYSTEM-REMINDER MUST BE HIGHLY SPECIFIC: Include exact fspec commands (create-work-unit, add-dependency, create-epic), not generic advice like 'break this down'
  GUIDE FEATURE FILE ANALYSIS: Remind AI to 'Review feature file linked to this work unit. Look for scenario groupings that could be separate stories. Each group should deliver incremental value.'
  PERSISTENCE STRATEGY: Warning appears in TWO contexts (immediate + show-work-unit) creating 'sticky' reminder. AI will see it multiple times, increasing likelihood of following through.
  STEP-BY-STEP WORKFLOW IN WARNING: (1) Review feature file (or create if missing), (2) Identify boundaries, (3) Create child work units with fspec create-work-unit, (4) Link with fspec add-dependency --depends-on, (5) Optionally create epic to group, (6) Delete original work unit or convert to epic using fspec create-epic
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Warning must include SPECIFIC step-by-step commands (not generic advice) to break down the work unit
  #   2. Warning should guide AI to review feature file scenarios for natural split boundaries (not arbitrary splitting)
  #   3. Warning is non-blocking (allows estimate to be set) but strongly recommends breaking down
  #   4. Warning triggers IMMEDIATELY when estimate > 13 points (21 is too large, but 13 is acceptable)
  #   5. Warning ONLY applies to story and bug types, NOT task types (tasks can legitimately be large)
  #   6. Warning should suggest breaking into smaller stories (1-13 points each) for clear guidance
  #   7. Warning persists in show-work-unit while estimate > 13 AND status is not done
  #   8. Warning guidance adapts based on context: if feature file exists suggest reviewing it for natural boundaries, if no feature file exists suggest creating it first before breaking down
  #
  # EXAMPLES:
  #   1. AI estimates BUG-005 at 21 points → warning shown → AI ignores and continues → runs show-work-unit BUG-005 → warning shown again in system-reminder section → AI finally breaks it down
  #   2. AI estimates INFRA-001 (type=task, infrastructure setup) at 21 points → NO warning (task type exempt) → estimate accepted
  #   3. AI sees warning → reads feature file → identifies natural scenario groupings → creates child work units for each group → estimates each child (all <= 13) → links with dependencies
  #
  # ========================================
  Background: User Story
    As a AI agent estimating work units
    I want to receive clear warnings and guidance when estimate is too large
    So that I break down large work into manageable chunks without getting lost in the process

  Scenario: Immediate warning when estimating story/bug at 21 points with persistent reminder
    Given a work unit "BUG-005" exists with type "bug"
    And BUG-005 has a completed feature file without prefill placeholders
    When I run "fspec update-work-unit-estimate BUG-005 21"
    Then the command should succeed
    And the output should contain a system-reminder warning about estimate > 13 points
    And the warning should include specific fspec commands for breaking down the work unit
    And the warning should guide me to review the feature file for natural scenario boundaries
    When I later run "fspec show-work-unit BUG-005"
    Then the output should contain a system-reminder warning about estimate > 13 points
    And the warning should persist until estimate changes to <= 13 or status changes to done

  Scenario: No warning for task type work units with large estimates
    Given a work unit "INFRA-001" exists with type "task"
    And INFRA-001 has description "Infrastructure setup"
    When I run "fspec update-work-unit-estimate INFRA-001 21"
    Then the command should succeed
    And the output should NOT contain any warning about estimate size
    And the estimate should be set to 21 points without any system-reminder

  Scenario: AI follows warning guidance to break down large work unit
    Given a work unit "STORY-007" exists with type "story"
    And STORY-007 has a feature file with multiple scenario groupings
    And each scenario grouping could deliver incremental value
    When I run "fspec update-work-unit-estimate STORY-007 21"
    Then the command should succeed
    And the output should contain a system-reminder with step-by-step workflow guidance
    And the guidance should include: review feature file, identify boundaries, create child work units, link dependencies
    When I review the feature file for natural boundaries
    And I create child work units for each scenario group using "fspec create-work-unit"
    And I link child work units with "fspec add-dependency --depends-on STORY-007"
    And I estimate each child work unit (all <= 13 points)
    Then the original work unit can be deleted or converted to epic using "fspec create-epic"

  Scenario: Adaptive warning guidance when feature file is missing
    Given a work unit "AUTH-008" exists with type "story"
    And AUTH-008 has NO feature file
    When I run "fspec update-work-unit-estimate AUTH-008 21"
    Then the command should succeed
    And the output should contain a system-reminder warning about estimate > 13 points
    And the warning should suggest creating a feature file FIRST before breaking down
    And the warning should guide me to use "fspec generate-scenarios AUTH-008"

  Scenario: Warning stops when estimate is reduced to acceptable range
    Given a work unit "BUG-009" exists with type "bug"
    And BUG-009 has estimate of 21 points
    And BUG-009 status is "specifying"
    When I run "fspec show-work-unit BUG-009"
    Then the output should contain a system-reminder warning about estimate > 13 points
    When I break down BUG-009 into smaller work units
    And I run "fspec update-work-unit-estimate BUG-009 8"
    Then the command should succeed
    And the output should NOT contain any warning about estimate size
    When I run "fspec show-work-unit BUG-009"
    Then the output should NOT contain any warning about estimate size

  Scenario: Warning stops when work unit is marked done
    Given a work unit "STORY-010" exists with type "story"
    And STORY-010 has estimate of 21 points
    And STORY-010 status is "implementing"
    When I run "fspec show-work-unit STORY-010"
    Then the output should contain a system-reminder warning about estimate > 13 points
    When I complete the work and run "fspec update-work-unit-status STORY-010 done"
    Then the command should succeed
    When I run "fspec show-work-unit STORY-010"
    Then the output should NOT contain any warning about estimate size
