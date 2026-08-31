@done
@querying
@cli
@astgrep
@CLI-015
Feature: Harness workflow guidance references the AstGrep tool

  """
  FSPEC_WORKFLOW_GUIDANCE (rust/tools/src/fspec_workflow_guidance.rs) is
  harness-only — it is injected into the native agent system prompt, never
  into CLI users. Per CLI-015 its discovery/Research-Tools sections name the
  native AstGrep tool for code search and carry no `research --tool=ast`
  references.
  """

  Background: User Story
    As the native agent receiving the FSPEC workflow guidance
    I want code-search guidance that names the AstGrep tool
    So that I use a tool that actually exists in my context

  Scenario: harness workflow guidance references the AstGrep tool, not research ast
    Given the harness-only FSPEC workflow guidance constant
    When I inspect the guidance text
    Then it references `AstGrep` for code search during discovery
    And it does not reference `research` with the `ast` tool
