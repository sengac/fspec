export function getAcddPrinciplesSection(): string {
  return `## Key ACDD Principles

1. **Example Mapping First** - Interactive conversation with human (rules, examples, questions)
2. **Feature Second** - Generate or write Gherkin feature file from example map
3. **Test Third** - Write test file with header comment linking to feature file
4. **Link Coverage Immediately** - After writing tests, link them to scenarios with \`fspec link-coverage\`
5. **Tests Must Fail** - Verify tests fail (red) before implementing (proves they work)
6. **Implement Fourth** - Write code AND wire up integration points (CREATION + CONNECTION)
7. **Link Implementation Immediately** - After implementing, link code to test mappings with \`fspec link-coverage\`
8. **Validate All Tests** - Run ALL tests to ensure nothing broke
9. **No Skipping** - Must follow ACDD order: Discovery → Feature → Test → Coverage → Implementation → Coverage → Validation
10. **Kanban Tracking** - Move work units through board states as you progress
11. **Tags Reflect State** - Add \`@wip\` when starting, change to \`@done\` when complete
12. **Feature-Test Link** - Always add feature file path in test file header comment
13. **Coverage Traceability** - Always maintain scenario-to-test-to-implementation mappings

### Test File Header Template

Every test file MUST start with this header comment:

\`\`\`typescript
/**
 * Feature: spec/features/[feature-name].feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

describe('Feature: [Feature Name]', () => {
  describe('Scenario: [Scenario Name]', () => {
    it('should [expected behavior]', async () => {
      // Given: [precondition]
      // When: [action]
      // Then: [expected outcome]
    });
  });
});
\`\`\`

`;
}
