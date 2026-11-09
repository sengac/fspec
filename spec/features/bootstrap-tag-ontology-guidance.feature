@done
@cli
@bootstrap
@documentation
@BUG-531
Feature: Bootstrap tag ontology guidance names real commands
  """
  The Foundation Event Storm section of the bootstrap text (rust/fspec-core/src/commands/bootstrap_doc.txt, served by fspec bootstrap) told agents to run fspec derive-tags-from-foundation, a command that never existed in the TypeScript or Rust CLI. The section now gives the real workflow: one fspec register-tag per bounded context in "Component Tags", then fspec generate-tags-md. DISC-003 made the same change in the tool guidance (rust/tools). The general check that every named command parses with the real CLI is CLI-016. Tests: rust/fspec-core/tests/bug531_bootstrap_tag_ontology_guidance.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The bootstrap section 'Integration with Tag Ontology (EXMAP-004)' tells agents to register one component tag per bounded context with fspec register-tag <tag> "Component Tags" <description> and then regenerate TAGS.md with fspec generate-tags-md, as DISC-003 did for the tool guidance
  #   2. The bootstrap output never names derive-tags-from-foundation, and claims no automatic tag derivation from bounded contexts
  #   3. The general check that every fspec command named in guidance parses with the real CLI belongs to CLI-016 (it supersedes the 'add a check' part of this card)
  #
  # EXAMPLES:
  #   1. An agent finishes the Foundation Event Storm with bounded contexts Work Management and Specification, reads the bootstrap tag ontology section, and runs fspec register-tag @work-management "Component Tags" "Work Management bounded context" for each context and then fspec generate-tags-md
  #   2. Searching the fspec bootstrap output for derive-tags-from-foundation finds nothing
  #
  # ========================================
  Background: User Story
    As a AI agent following fspec bootstrap
    I want to turn the Foundation Event Storm's bounded contexts into tags with commands that exist
    So that the tag ontology step works instead of failing on an unknown command

  Scenario: The tag ontology section gives the register-tag and generate-tags-md workflow
    Given a project whose Foundation Event Storm has bounded contexts
    When the agent runs fspec bootstrap and reads the section "Integration with Tag Ontology (EXMAP-004)"
    Then the section tells it to run fspec register-tag with the "Component Tags" category for each bounded context
    And the section tells it to run fspec generate-tags-md

  Scenario: The bootstrap output never names derive-tags-from-foundation
    Given any project
    When the agent runs fspec bootstrap
    Then the output does not contain "derive-tags-from-foundation"
