@diff-display
@git-integration
@RPC-355
Feature: Git Change Type Derivation
  """
  Change-type derivation is gitoxide-based with no shelling out. A=untracked or absent-from-HEAD, D=indexed but missing from workdir, M=otherwise, R best-effort defaults to M. get_staged_files_with_change_type / get_unstaged_files_with_change_type return ChangedFileStatus { path, change_type } where ChangeType is an enum with as_letter().
  """

  Background: User Story
    As a Rust TUI developer
    I want to derive A/M/D change types for staged and unstaged files in codelet/git
    So that the transport can report each changed file with a correct single-letter status

  Scenario: Staged tracked-but-modified file is reported as change type M
    Given a temporary git repository with a committed file
    And the file is modified and staged in the index
    When get_staged_files_with_change_type is called against that repo
    Then the staged file is reported with change_type "M"

  Scenario: Unstaged file deleted from the working directory is reported as change type D
    Given a temporary git repository with a committed file
    And the file is deleted from the working directory
    When get_unstaged_files_with_change_type is called against that repo
    Then the missing file is reported with change_type "D"

  Scenario: Untracked working-tree file appears as change type A and unstaged
    Given a temporary git repository with a committed file
    And a new untracked file exists in the working directory
    When changed_files is collected against that repo
    Then the untracked file appears with change_type "A" and staged false
