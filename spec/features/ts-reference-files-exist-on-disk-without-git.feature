@BUG-153
Feature: TS reference files exist on disk without consulting git

  """
  Architecture notes:
  - rpc027_dialog_parity_ij.rs: typescript_ink_dialog_files_are_not_modified_by_this_refactor
    Remove git status invocation. Replace with direct file existence checks.
  - Test must pass regardless of git branch or working-tree state
  """

  Background: User Story
    As a developer
    I want to run shape tests deterministically
    So that tests pass regardless of git branch or working-tree state

  Scenario: TS reference files exist on disk without consulting git
    Given the TS reference files are listed in TS_REFERENCE_FILES
    When I check each file exists on disk
    Then Dialog.tsx exists and has content
    And ThinkingLevelDialog.tsx exists and has content
    And AttachmentDialog.tsx exists and has content
    And TurnContentModal.tsx exists and has content
    And FileSearchPopup.tsx exists and has content
    And SlashCommandPalette.tsx exists and has content
    And ThreeButtonDialog.tsx exists and has content
