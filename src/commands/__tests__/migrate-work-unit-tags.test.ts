/**
 * Test suite for: spec/features/remove-work-unit-id-tags-from-generate-scenarios.feature
 * Scenario: Script moves scenario-level work unit ID tags to feature level
 */

import { describe, it, expect } from 'vitest';

describe('Feature: Remove work unit ID tags from generate-scenarios', () => {
  describe('Scenario: Script moves scenario-level work unit ID tags to feature level', () => {
    it('should move scenario-level work unit ID tags to feature-level', () => {
      // Given I have a feature file with scenario-level work unit ID tags
      const inputFeatureContent = `@phase1
@cli
Feature: Example Feature

  @COV-001
  Scenario: First scenario
    Given some precondition
    When some action
    Then some result

  @COV-002
  Scenario: Second scenario
    Given another precondition
    When another action
    Then another result
`;

      // When I run the migration script (simulated by migration function)
      const migratedContent = migrateWorkUnitTags(inputFeatureContent);

      // Then the @COV-001 tag should be moved to feature-level
      const lines = migratedContent.split('\n');
      const featureLine = lines.findIndex(line => line.startsWith('Feature:'));
      const featureLevelTags = lines
        .slice(0, featureLine)
        .filter(line => line.trim().startsWith('@'));

      expect(featureLevelTags.some(tag => tag.includes('@COV-001'))).toBe(true);
      expect(featureLevelTags.some(tag => tag.includes('@COV-002'))).toBe(true);

      // And the scenario should no longer have @COV-001 tag
      const scenarioSections = migratedContent.split(/\n(?=  Scenario:)/);

      for (const section of scenarioSections) {
        // Skip non-scenario sections
        if (!section.includes('Scenario:')) {
          continue;
        }

        // Check if this section has any scenario-level tags (indented with 2 spaces)
        const scenarioLines = section.split('\n');
        const scenarioLevelTags = scenarioLines.filter(
          line =>
            line.match(/^  @/) && // Indented tag (scenario-level)
            (line.includes('@COV-') || line.includes('@AUTH-'))
        );

        // No work unit ID tags should remain at scenario level
        expect(scenarioLevelTags).toEqual([]);
      }
    });
  });
});

/**
 * Migration utility function: Moves work unit ID tags from scenario level to feature level
 *
 * Work unit ID tags are tags that match the pattern @PREFIX-NNN (e.g., @AUTH-001, @COV-001)
 * or @PREFIX-NNN-NNN (e.g., @AUTH-001-002 for subtasks).
 */
function migrateWorkUnitTags(featureContent: string): string {
  const lines = featureContent.split('\n');
  const featureLine = lines.findIndex(line => line.startsWith('Feature:'));

  if (featureLine === -1) {
    throw new Error('No Feature: line found in feature file');
  }

  // Collect all work unit ID tags from scenario level
  const workUnitTagPattern = /^@[A-Z]+-\d+(-\d+)?$/; // Matches @PREFIX-001 or @PREFIX-001-002 (without leading spaces)
  const scenarioLevelWorkUnitTags: string[] = [];
  const linesToRemove: number[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Check for scenario-level work unit ID tags (indented with 2 spaces)
    const trimmed = line.trim();
    if (workUnitTagPattern.test(trimmed) && line.startsWith('  @')) {
      if (!scenarioLevelWorkUnitTags.includes(trimmed)) {
        scenarioLevelWorkUnitTags.push(trimmed);
      }
      linesToRemove.push(i);
    }
  }

  // Remove scenario-level work unit ID tags (in reverse order to maintain indices)
  for (const index of linesToRemove.reverse()) {
    lines.splice(index, 1);
  }

  // Add collected work unit ID tags to feature level (without indentation)
  const featureTagsInsertIndex = featureLine;
  for (const tag of scenarioLevelWorkUnitTags.reverse()) {
    lines.splice(featureTagsInsertIndex, 0, tag.replace(/^  /, '')); // Remove indentation
  }

  return lines.join('\n');
}
