// Feature: spec/features/complete-big-picture-event-storm-schema-integration.feature

import { describe, it, expect } from 'vitest';
import { validateGenericFoundationObject } from '../generic-foundation-validator';
import type { GenericFoundation } from '../../types/generic-foundation';

describe('Feature: Complete Big Picture Event Storm Schema Integration', () => {
  describe('Scenario: Validate foundation with Event Storm containing bounded context', () => {
    it('should pass validation when eventStorm has level=big_picture', () => {
      // @step Given generic-foundation.schema.json includes eventStorm property definition
      // @step And the schema enforces level='big_picture' constraint
      const foundation: GenericFoundation = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [
            {
              name: 'Test Capability',
              description: 'Test capability description',
            },
          ],
        },
        eventStorm: {
          level: 'big_picture',
          items: [
            {
              id: 1,
              type: 'bounded_context',
              text: 'User Management',
              color: null,
              deleted: false,
              createdAt: '2025-01-15T10:00:00.000Z',
            },
          ],
          nextItemId: 2,
        },
      };

      // @step When I validate a foundation.json with eventStorm containing a bounded context
      // @step And the eventStorm.level equals 'big_picture'
      const result = validateGenericFoundationObject(foundation);

      // @step Then the validation should pass
      expect(result.valid).toBe(true);

      // @step And no schema errors should be reported
      expect(result.errors).toEqual([]);
    });
  });

  describe('Scenario: Reject foundation with invalid Event Storm level', () => {
    it('should fail validation when eventStorm.level is not big_picture', () => {
      // @step Given generic-foundation.schema.json includes eventStorm property definition
      // @step And the schema enforces level='big_picture' constraint
      const foundation = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [
            {
              name: 'Test Capability',
              description: 'Test capability description',
            },
          ],
        },
        eventStorm: {
          level: 'process_modeling', // Invalid for foundation
          items: [],
          nextItemId: 1,
        },
      };

      // @step When I validate a foundation.json with eventStorm.level='process_modeling'
      const result = validateGenericFoundationObject(foundation);

      // @step Then the validation should fail
      expect(result.valid).toBe(false);

      // @step And the error should indicate level must be 'big_picture'
      const levelError = result.errors?.find(
        err =>
          err.instancePath === '/eventStorm/level' && err.keyword === 'const'
      );
      expect(levelError).toBeDefined();
      expect(levelError?.params?.allowedValue).toBe('big_picture');

      // @step And the error should include the field path "eventStorm.level"
      expect(levelError?.instancePath).toBe('/eventStorm/level');
    });
  });

  describe('Scenario: Validate EventStormCommand item with type-specific fields', () => {
    it('should pass validation for command item with actor field', () => {
      // @step Given generic-foundation.schema.json includes eventStormItem discriminated union
      // @step And the schema defines EventStormCommand with actor field
      const foundation: GenericFoundation = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [
            {
              name: 'Test Capability',
              description: 'Test capability description',
            },
          ],
        },
        eventStorm: {
          level: 'big_picture',
          items: [
            {
              id: 1,
              type: 'command',
              text: 'Create Order',
              color: 'blue',
              deleted: false,
              createdAt: '2025-01-15T10:00:00.000Z',
              actor: 'Customer',
            },
          ],
          nextItemId: 2,
        },
      };

      // @step When I validate a foundation with EventStormCommand item including actor field
      // @step And the item type is 'command'
      // @step And the item color is 'blue'
      const result = validateGenericFoundationObject(foundation);

      // @step Then the validation should pass
      expect(result.valid).toBe(true);

      // @step And the actor field should be preserved in validated data
      expect(foundation.eventStorm?.items[0]).toHaveProperty(
        'actor',
        'Customer'
      );
    });
  });

  describe('Scenario: Validate EventStormBoundedContext with null color', () => {
    it('should pass validation for bounded_context with null color', () => {
      // @step Given generic-foundation.schema.json includes eventStormItem discriminated union
      // @step And the schema defines EventStormBoundedContext with color=null
      const foundation: GenericFoundation = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [
            {
              name: 'Test Capability',
              description: 'Test capability description',
            },
          ],
        },
        eventStorm: {
          level: 'big_picture',
          items: [
            {
              id: 1,
              type: 'bounded_context',
              text: 'Payment Processing',
              color: null,
              deleted: false,
              createdAt: '2025-01-15T10:00:00.000Z',
            },
          ],
          nextItemId: 2,
        },
      };

      // @step When I validate a foundation with EventStormBoundedContext item
      // @step And the item type is 'bounded_context'
      // @step And the item color is null
      const result = validateGenericFoundationObject(foundation);

      // @step Then the validation should pass
      expect(result.valid).toBe(true);

      // @step And the null color should be accepted as valid
      expect(foundation.eventStorm?.items[0].color).toBeNull();
    });
  });

  describe('Scenario: Reject draft with invalid Event Storm item type during finalization', () => {
    it('should fail validation when item has invalid type', () => {
      // @step Given I have a foundation.json.draft with eventStorm section
      // @step And the eventStorm contains an item with invalid type 'invalid_type'
      const foundation = {
        version: '2.0.0',
        project: {
          name: 'Test Project',
          vision: 'Test vision',
          projectType: 'cli-tool',
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Test description',
            impact: 'high',
          },
        },
        solutionSpace: {
          overview: 'Test solution',
          capabilities: [
            {
              name: 'Test Capability',
              description: 'Test capability description',
            },
          ],
        },
        eventStorm: {
          level: 'big_picture',
          items: [
            {
              id: 1,
              type: 'invalid_type', // Invalid type
              text: 'Invalid Item',
              color: 'red',
              deleted: false,
              createdAt: '2025-01-15T10:00:00.000Z',
            },
          ],
          nextItemId: 2,
        },
      };

      // @step When I run "fspec discover-foundation --finalize"
      // (Simulated via direct validation)
      const result = validateGenericFoundationObject(foundation);

      // @step Then the finalization should fail
      expect(result.valid).toBe(false);

      // @step And the error should indicate invalid item type
      const typeError = result.errors?.find(
        err =>
          err.instancePath.includes('/eventStorm/items') &&
          err.keyword === 'oneOf'
      );
      expect(typeError).toBeDefined();

      // @step And the draft file should not be deleted
      // @step And foundation.json should not be created
      // (Handled by discover-foundation command, not tested here)
    });
  });
});
