@done
@BUG-358
@COV-065
@cli
@tag-management
@rust
Feature: validate-tags passes for every feature file
  """
  fspec validate-tags failed project-wide (547 of 2334 files): 420 lacked a
  feature-group tag, 311 a component tag, 303 work-unit ID tags sat at
  scenario level, about 390 tags were unregistered (among them CamelCase,
  snake_case, component:* and rule:* tags) and five features referenced work
  units that do not exist.
  Fixed: work-unit tags moved to feature level; odd-format tags normalised
  to kebab-case (AgentView -> agent-view, component:session-footer ->
  session-footer) and the rule:* scenario annotations removed; remaining tags
  registered under Technical Tags; missing component tags inferred from the
  crate of each feature's mapped tests (else its name), feature-group tags
  from the feature name; references to non-existent work units removed and
  the layer-1 / layer-2 tags renamed to layer-one / layer-two.
  The regression test runs the validate-tags command over the repository's
  spec directory.
  Tests: rust/fspec-core/tests/bug358_validate_tags_passes_project_wide.rs.
  """

  Background: User Story
    As a maintainer validating the specification
    I want to have fspec validate-tags pass for every feature file
    So that tag violations show up as regressions instead of hiding in hundreds of known failures

  Scenario: validate-tags reports no invalid feature files in the repository
    Given the repository's spec directory
    When the validate-tags command runs over every feature file
    Then no feature file has a tag violation
