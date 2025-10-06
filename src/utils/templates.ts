export function generateFeatureTemplate(featureName: string): string {
  return `@phase1 @component @feature-group
Feature: ${featureName}

  """
  Architecture notes:
  - TODO: Add key architectural decisions
  - TODO: Add dependencies and integrations
  - TODO: Add critical implementation requirements
  """

  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]

  Scenario: [Scenario name]
    Given [precondition]
    When [action]
    Then [expected outcome]
`;
}
