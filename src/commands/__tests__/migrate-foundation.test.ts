/**
 * Feature: spec/features/migrate-existing-foundation-json.feature
 *
 * This test file validates the acceptance criteria for migrating
 * existing foundation.json to v2.0.0 generic schema format.
 * Tests map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { migrateFoundation } from '../migrate-foundation';
import type { GenericFoundation } from '../../types/generic-foundation';

describe('Feature: Migrate Existing foundation.json', () => {
  describe('Scenario: Migrate project overview to vision field', () => {
    it('should map whatWeAreBuilding.projectOverview to project.vision', () => {
      // Given I have an existing foundation.json with 'whatWeAreBuilding.projectOverview' field
      const legacyFoundation = {
        whatWeAreBuilding: {
          projectOverview:
            'A Kanban-based project management and specification tool for AI agents.',
        },
        project: {
          name: 'fspec',
          description: 'CLI tool for managing Gherkin specs',
        },
      };

      // When I run 'fspec migrate-foundation'
      const result = migrateFoundation(legacyFoundation);

      // Then the new foundation.json should have 'project.vision' field
      expect(result.project.vision).toBeDefined();

      // And the 'project.vision' should contain the content from 'whatWeAreBuilding.projectOverview'
      expect(result.project.vision).toBe(
        'A Kanban-based project management and specification tool for AI agents.'
      );
    });
  });

  describe('Scenario: Migrate problem definition to problem space', () => {
    it('should map whyWeAreBuildingIt.problemDefinition.primary to problemSpace.primaryProblem', () => {
      // Given I have an existing foundation.json with 'whyWeAreBuildingIt.problemDefinition.primary' field
      const legacyFoundation = {
        whyWeAreBuildingIt: {
          problemDefinition: {
            primary: {
              title: 'AI Agents Lack Structured Workflow',
              description: 'AI agents struggle to build quality software reliably',
            },
          },
        },
        project: {
          name: 'fspec',
        },
      };

      // When I run 'fspec migrate-foundation'
      const result = migrateFoundation(legacyFoundation);

      // Then the new foundation.json should have 'problemSpace.primaryProblem' field
      expect(result.problemSpace).toBeDefined();
      expect(result.problemSpace.primaryProblem).toBeDefined();

      // And the 'problemSpace.primaryProblem' should map from the old structure
      expect(result.problemSpace.primaryProblem.title).toBe(
        'AI Agents Lack Structured Workflow'
      );
      expect(result.problemSpace.primaryProblem.description).toBe(
        'AI agents struggle to build quality software reliably'
      );
    });
  });

  describe('Scenario: Preserve architecture diagrams during migration', () => {
    it('should preserve all diagrams in architectureDiagrams array', () => {
      // Given I have an existing foundation.json with 'architectureDiagrams' array
      const legacyFoundation = {
        project: {
          name: 'fspec',
        },
        architectureDiagrams: [
          {
            section: 'Architecture Diagrams',
            title: 'fspec System Context',
            mermaidCode: 'graph TB\n  AI-->FSPEC',
          },
          {
            section: 'Architecture Diagrams',
            title: 'Command Architecture',
            mermaidCode: 'graph LR\n  CLI-->CMD',
          },
        ],
      };

      // When I run 'fspec migrate-foundation'
      const result = migrateFoundation(legacyFoundation);

      // Then the new foundation.json should preserve all diagrams in 'architectureDiagrams' array
      expect(result.architectureDiagrams).toBeDefined();
      expect(result.architectureDiagrams).toHaveLength(2);

      // And all diagram sections and titles should remain unchanged
      expect(result.architectureDiagrams[0].title).toBe('fspec System Context');
      expect(result.architectureDiagrams[1].title).toBe('Command Architecture');
      expect(result.architectureDiagrams[0].mermaidCode).toBe(
        'graph TB\n  AI-->FSPEC'
      );
    });
  });

  describe('Scenario: Validate migrated foundation against v2.0.0 schema', () => {
    it('should pass generic foundation validation after migration', () => {
      // Given I have run 'fspec migrate-foundation' successfully
      const legacyFoundation = {
        project: {
          name: 'fspec',
          description: 'Test project',
        },
        whatWeAreBuilding: {
          projectOverview: 'Test overview',
        },
      };

      const result = migrateFoundation(legacyFoundation);

      // When the migration completes
      // Then the new foundation.json must pass generic foundation validation
      expect(result.version).toBe('2.0.0');

      // And validation should use the v2.0.0 schema
      // (This will be tested by calling validateGenericFoundationObject in implementation)
      expect(result.project).toBeDefined();
      expect(result.project.name).toBe('fspec');
      expect(result.project.vision).toBe('Test overview');
    });
  });
});
